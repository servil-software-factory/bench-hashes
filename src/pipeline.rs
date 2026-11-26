//! The multithreaded contender's owned-buffer and receiving cells.
//!
//! Rooms keep the existing 1 MiB input budget. Short messages share a room;
//! every nonfinal commit of a longer message is a whole room. Each call
//! drains its results, so samples include completion and reuse starts clean.
//! The synchronous lent-buffer cells remain the baseline in main.rs.

use blake3_servil::throughput::{
    block_on, Batch, Block, Event, Limits, Message, Pipeline, Room, ROOM_LEN,
};
use blake3_servil::Mode;
use std::cell::RefCell;
use std::hint::black_box;

const LIMITS: Limits = Limits {
    messages: crate::IN_FLIGHT_BUFFERS,
    rooms: crate::IN_FLIGHT_BYTES / ROOM_LEN,
};

thread_local! {
    // Constructed during calibration, kept independently by each producer.
    static MESSAGES: RefCell<Option<Messages>> = const { RefCell::new(None) };
    // Kept separate from MESSAGES so per-message custom mode never alters
    // default-mode ContinuousMessages pipelines in the same process.
    static KEYED_MESSAGES: RefCell<Option<Messages>> = const { RefCell::new(None) };
    static BATCHES: RefCell<crate::Map<usize, Batches>> = RefCell::new(crate::Map::default());
}

/// Wait for one completion. The caller has an ended message in flight.
fn completed(pipe: &mut Pipeline<()>) -> Event<()> {
    if let Some(event) = pipe.next_event() {
        return event;
    }
    block_on(pipe.ready());
    pipe.next_event().expect("ready supplies an event")
}

struct Messages {
    pipe: Pipeline<()>,
    capacity: usize,
}

impl Messages {
    fn new(limits: Limits) -> Self {
        Self {
            pipe: Pipeline::new(Mode::Plain, limits),
            capacity: limits.messages,
        }
    }

    fn consume(&mut self, consume: &mut impl FnMut(&[u8])) {
        let Event::Hashed {
            hash, owner: None, ..
        } = completed(&mut self.pipe)
        else {
            panic!("a room-fed message supplies one hash");
        };
        consume(hash.as_bytes());
    }

    /// Shared room-feed and drain protocol for whole messages. The `open`
    /// closure specializes message creation (`Pipeline::message` vs
    /// `Pipeline::message_with`) so plain runs carry no per-message or
    /// per-piece mode branch, while plain and keyed traffic still keep
    /// separate `Pipeline` instances.
    #[inline(always)]
    fn run_with<'a, Item>(
        &mut self,
        items: impl IntoIterator<Item = Item>,
        mut open: impl FnMut(&mut Pipeline<()>, Item) -> (&'a [u8], Message),
        mut consume: impl FnMut(&[u8]),
    ) {
        let mut room: Option<Room> = None;
        let mut pending = 0;
        for item in items {
            if pending == self.capacity {
                self.consume(&mut consume);
                pending -= 1;
            }
            let (input, mut message) = open(&mut self.pipe, item);
            for piece in black_box(input).chunks(ROOM_LEN) {
                if room.as_ref().is_none_or(|r| r.len() < piece.len()) {
                    // Release our reference before asking for another room.
                    drop(room.take());
                    room = Some(block_on(self.pipe.room()));
                }
                let r = room.as_mut().unwrap();
                r.bytes_mut()[..piece.len()].copy_from_slice(piece);
                self.pipe.commit(&mut message, r, piece.len());
            }
            self.pipe.end(message);
            pending += 1;
        }
        drop(room);
        for _ in 0..pending {
            self.consume(&mut consume);
        }
    }

    /// Whole messages, each produced directly into rooms. The room's unused
    /// tail can hold the next short message; a long message starts a fresh
    /// room so every nonfinal commit has ROOM_LEN bytes at offset zero.
    fn run<'a>(&mut self, inputs: impl IntoIterator<Item = &'a [u8]>, consume: impl FnMut(&[u8])) {
        self.run_with(
            inputs,
            |pipe, input| {
                let message = block_on(pipe.message((), Some(input.len() as u64)));
                (input, message)
            },
            consume,
        );
    }

    /// Whole messages, each authenticated under its own per-message key
    /// (`Pipeline::message_with(Mode::Keyed(key), ...)`), produced directly
    /// into rooms with the same room-sharing and drain rules as `run`.
    fn run_keyed<'a>(
        &mut self,
        inputs: impl IntoIterator<Item = (&'a [u8], &'a [u8; 32])>,
        consume: impl FnMut(&[u8]),
    ) {
        self.run_with(
            inputs,
            |pipe, (input, key)| {
                let message = block_on(pipe.message_with(
                    Mode::Keyed(black_box(key)),
                    (),
                    Some(input.len() as u64),
                ));
                (input, message)
            },
            consume,
        );
    }

    /// The source declares no length. Accumulate arrivals directly in a room
    /// until full, committing a partial room only when the source ends.
    fn receive(
        &mut self,
        input: &[u8],
        iterations: usize,
        arrivals: impl Fn(&[u8], &mut dyn FnMut(&[u8])),
        mut consume: impl FnMut(&[u8]),
    ) {
        for _ in 0..iterations {
            let mut message = block_on(self.pipe.message((), None));
            let mut room: Option<Room> = None;
            let mut filled = 0;
            arrivals(black_box(input), &mut |mut fragment| {
                while !fragment.is_empty() {
                    let r = room.get_or_insert_with(|| block_on(self.pipe.room()));
                    let n = fragment.len().min(ROOM_LEN - filled);
                    r.bytes_mut()[filled..filled + n].copy_from_slice(&fragment[..n]);
                    filled += n;
                    fragment = &fragment[n..];
                    if filled == ROOM_LEN {
                        self.pipe.commit(&mut message, r, filled);
                        drop(room.take());
                        filled = 0;
                    }
                }
            });
            if let Some(mut r) = room {
                self.pipe.commit(&mut message, &mut r, filled);
            }
            self.pipe.end(message);
            self.consume(&mut consume);
        }
    }
}

/// One owned input after another, including every final digest in the call.
#[inline(always)]
pub(crate) fn messages<'a>(inputs: impl IntoIterator<Item = &'a [u8]>, consume: impl FnMut(&[u8])) {
    MESSAGES.with(|kept| {
        kept.borrow_mut()
            .get_or_insert_with(|| Messages::new(LIMITS))
            .run(inputs, consume)
    });
}

/// One owned input after another, each authenticated with its own key
/// (`Pipeline::message_with`), including every final digest in the call.
#[inline(always)]
pub(crate) fn keyed_messages<'a>(
    inputs: impl IntoIterator<Item = (&'a [u8], &'a [u8; 32])>,
    consume: impl FnMut(&[u8]),
) {
    KEYED_MESSAGES.with(|kept| {
        kept.borrow_mut()
            .get_or_insert_with(|| Messages::new(LIMITS))
            .run_keyed(inputs, consume)
    });
}

#[inline(always)]
pub(crate) fn receive(
    input: &[u8],
    iterations: usize,
    arrivals: impl Fn(&[u8], &mut dyn FnMut(&[u8])),
    consume: impl FnMut(&[u8]),
) {
    MESSAGES.with(|kept| {
        kept.borrow_mut()
            .get_or_insert_with(|| Messages::new(LIMITS))
            .receive(input, iterations, arrivals, consume);
    });
}

type BatchBuffers = (Batch<'static>, Box<[[u8; 32]]>);

/// One open message of a turn cell (many messages at once, their pieces
/// arriving interleaved), as the Pipeline's caller holds it: the message,
/// and a Room of its own that arrivals land in. The caller knows no
/// lengths, so a message of any length is served the same way: a full
/// Room is committed, and what is left when the message ends is its last
/// commit.
pub(crate) struct TurnSlot {
    message: Option<Message>,
    room: Option<Room>,
    filled: usize,
}

impl TurnSlot {
    pub(crate) fn new() -> Self {
        Self {
            message: None,
            room: None,
            filled: 0,
        }
    }
}

impl Drop for TurnSlot {
    /// The turn cells keep their messages open from call to call for the
    /// producer's life. An open `Message` must be ended or aborted, never
    /// dropped; at the thread's end it is left as it is (the process ends).
    fn drop(&mut self) {
        if let Some(message) = self.message.take() {
            std::mem::forget(message);
        }
    }
}

/// A turn schedule's Pipeline, kept for its producer's life (its messages
/// stay open between calls, so it is never dropped).
struct Turns {
    pipe: Pipeline<()>,
    /// Messages ended whose event is not yet taken.
    pending: usize,
    open: usize,
}

thread_local! {
    /// Kept by contender and schedule, as the schedules themselves are.
    static TURNS: RefCell<Vec<((&'static str, &'static str), std::mem::ManuallyDrop<Turns>)>> = const { RefCell::new(Vec::new()) };
}

fn with_turns<R>(
    key: (&'static str, &'static str),
    open: usize,
    f: impl FnOnce(&mut Turns) -> R,
) -> R {
    let mut turns = TURNS
        .with(|all| {
            let mut all = all.borrow_mut();
            all.iter()
                .position(|(k, _)| *k == key)
                .map(|i| all.swap_remove(i).1)
        })
        .unwrap_or_else(|| {
            std::mem::ManuallyDrop::new(Turns {
                // As many messages again may be in the workers' hands or
                // waiting as events, and as many Rooms: `turn_finish` keeps
                // the ended messages at or under `open`, so opening a
                // message never waits.
                pipe: Pipeline::new(
                    Mode::Plain,
                    Limits {
                        messages: 2 * open,
                        rooms: 2 * open + 16,
                    },
                ),
                pending: 0,
                open,
            })
        });
    let result = f(&mut turns);
    TURNS.with(|all| all.borrow_mut().push((key, turns)));
    result
}

impl Turns {
    fn take(&mut self, consume: &mut dyn FnMut(&[u8])) {
        let Event::Hashed {
            hash, owner: None, ..
        } = completed(&mut self.pipe)
        else {
            panic!("a room-fed message supplies one hash");
        };
        consume(hash.as_bytes());
        self.pending -= 1;
    }
}

/// A turn's arrivals: each piece is read straight into its message's Room
/// (the one copy every contender pays for an arrival), and a Room that
/// fills is committed.
pub(crate) fn turn_update(
    key: (&'static str, &'static str),
    slots: &mut [TurnSlot],
    arriving: &[(usize, &[u8])],
) {
    with_turns(key, slots.len(), |turns| {
        for &(slot, mut piece) in arriving {
            let slot = &mut slots[slot];
            if slot.message.is_none() {
                slot.message = Some(block_on(turns.pipe.message((), None)));
            }
            while !piece.is_empty() {
                let room = slot.room.get_or_insert_with(|| block_on(turns.pipe.room()));
                let n = piece.len().min(ROOM_LEN - slot.filled);
                room.bytes_mut()[slot.filled..slot.filled + n].copy_from_slice(&piece[..n]);
                slot.filled += n;
                piece = &piece[n..];
                if slot.filled == ROOM_LEN {
                    turns
                        .pipe
                        .commit(slot.message.as_mut().unwrap(), room, ROOM_LEN);
                    slot.room = None;
                    slot.filled = 0;
                }
            }
        }
    });
}

/// The messages that ended in a turn: what is left in each one's Room is
/// its last commit, and the message is ended. Digests are consumed as they
/// are ready, and by waiting once more ended messages are outstanding than
/// the schedule holds open.
pub(crate) fn turn_finish(
    key: (&'static str, &'static str),
    slots: &mut [TurnSlot],
    ended: &[usize],
    consume: &mut dyn FnMut(&[u8]),
) {
    with_turns(key, slots.len(), |turns| {
        for &slot in ended {
            let slot = &mut slots[slot];
            let mut message = slot.message.take().expect("an ended message had a piece");
            if let Some(mut room) = slot.room.take() {
                turns.pipe.commit(&mut message, &mut room, slot.filled);
                slot.filled = 0;
            }
            turns.pipe.end(message);
            turns.pending += 1;
        }
        while let Some(event) = turns.pipe.next_event() {
            let Event::Hashed { hash, .. } = event else {
                panic!("a room-fed message supplies one hash");
            };
            consume(hash.as_bytes());
            turns.pending -= 1;
        }
        while turns.pending > turns.open {
            turns.take(consume);
        }
    });
}

/// Before a call returns: every ended message's digest consumed, so each
/// sample holds its own completions.
pub(crate) fn turn_drain(
    key: (&'static str, &'static str),
    open: usize,
    consume: &mut dyn FnMut(&[u8]),
) {
    with_turns(key, open, |turns| {
        while turns.pending > 0 {
            turns.take(consume);
        }
    });
}

struct Batches {
    pipe: Pipeline<()>,
    free: Vec<BatchBuffers>,
}

impl Batches {
    fn new(messages: usize, buffers: usize) -> Self {
        Self {
            pipe: Pipeline::new(
                Mode::Plain,
                Limits {
                    messages: buffers,
                    rooms: 0,
                },
            ),
            free: (0..buffers)
                .map(|_| {
                    (
                        Batch::new(crate::MESSAGE_LEN, messages),
                        vec![[0; 32]; messages].into_boxed_slice(),
                    )
                })
                .collect(),
        }
    }

    fn consume(&mut self, consume: &mut impl FnMut(&[u8])) {
        let Event::Batch { batch, digests, .. } = completed(&mut self.pipe) else {
            panic!("a submitted batch returns its input and digests");
        };
        consume(digests.as_flattened());
        self.free.push((batch, digests));
    }

    fn run(&mut self, input: &[u8], iterations: usize, mut consume: impl FnMut(&[u8])) {
        let buffers = self.free.len();
        debug_assert!(buffers > 0, "each call starts with all buffers returned");
        for _ in 0..iterations {
            if self.free.is_empty() {
                self.consume(&mut consume);
            }
            let (mut batch, digests) = self.free.pop().unwrap();
            // Continuous batches contain whole 64-byte messages. One bulk
            // copy models the producer's read, just as in the lent baseline.
            Block::bytes_mut(batch.blocks_mut()).copy_from_slice(black_box(input));
            let message = block_on(self.pipe.message((), None));
            self.pipe.batch(message, batch, digests);
        }
        while self.free.len() < buffers {
            self.consume(&mut consume);
        }
    }
}

#[inline(always)]
pub(crate) fn batches(
    input: &[u8],
    messages: usize,
    iterations: usize,
    consume: impl FnMut(&[u8]),
) {
    BATCHES.with(|kept| {
        kept.borrow_mut()
            .entry(messages)
            .or_insert_with(|| Batches::new(messages, crate::in_flight(input.len())))
            .run(input, iterations, consume);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start() {
        blake3_servil::initialize_multithreaded(std::num::NonZeroUsize::new(2));
    }

    #[test]
    fn fixed_vectors_keep_order_across_backpressure_and_reuse() {
        start();
        // Published BLAKE3 vectors, independent of either implementation here.
        let empty = "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262";
        let abc = "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";
        let mut state = Messages::new(Limits {
            messages: 2,
            rooms: 1,
        });
        for _ in 0..2 {
            let mut got = Vec::new();
            state.run([&b""[..], b"abc"].into_iter().cycle().take(9), |digest| {
                got.push(
                    digest
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>(),
                )
            });
            assert_eq!(
                got,
                [empty, abc].into_iter().cycle().take(9).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn room_boundaries_and_partial_room_reuse_deliver_every_byte() {
        start();
        let inputs: Vec<Vec<u8>> = [
            0,
            1,
            63,
            64,
            65,
            1000,
            ROOM_LEN - 1,
            ROOM_LEN,
            ROOM_LEN + 1,
            3 * ROOM_LEN + 1000,
        ]
        .into_iter()
        .enumerate()
        .map(|(seed, len)| crate::make_input_seeded(len, seed as u64))
        .collect();
        let expected: Vec<_> = inputs.iter().map(|m| blake3::hash(m)).collect();
        let mut state = Messages::new(Limits {
            messages: 2,
            rooms: 1,
        });
        for _ in 0..2 {
            let mut got = Vec::new();
            state.run(inputs.iter().map(Vec::as_slice), |d| got.push(d.to_vec()));
            assert_eq!(
                got,
                expected
                    .iter()
                    .map(|h| h.as_bytes().to_vec())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn receiving_preserves_fragment_order_waits_and_finalization() {
        start();
        let mut state = Messages::new(Limits {
            messages: 1,
            rooms: 1,
        });
        for len in [
            0,
            1,
            1448,
            ROOM_LEN - 1,
            ROOM_LEN,
            ROOM_LEN + 1,
            3 * ROOM_LEN + 1000,
        ] {
            let input = crate::make_input_seeded(len, 7);
            let mut fragments = 0usize;
            crate::receive::arrivals(&input, |_| fragments += 1, || {});
            let waits = std::cell::Cell::new(0);
            let mut got = Vec::new();
            state.receive(
                &input,
                3,
                |input, each| crate::receive::arrivals(input, each, || waits.set(waits.get() + 1)),
                |d| got.push(d.to_vec()),
            );
            assert_eq!(waits.get(), 3 * fragments.saturating_sub(1));
            assert_eq!(
                got,
                vec![blake3::hash(&input).as_bytes().to_vec(); 3],
                "{len} bytes"
            );
        }
    }

    #[test]
    fn shared_producers_keep_independent_inputs_and_completions() {
        start();
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            let jobs: Vec<_> = [3, 11]
                .into_iter()
                .map(|seed| {
                    let barrier = &barrier;
                    scope.spawn(move || {
                        let input = crate::make_input_seeded(ROOM_LEN + 65, seed);
                        let expected = blake3::hash(&input);
                        barrier.wait();
                        for _ in 0..2 {
                            let mut seen = 0;
                            messages(std::iter::repeat_n(input.as_slice(), 17), |digest| {
                                assert_eq!(digest, expected.as_bytes());
                                seen += 1;
                            });
                            assert_eq!(seen, 17);
                        }
                    })
                })
                .collect();
            for job in jobs {
                job.join().unwrap();
            }
        });
    }

    #[test]
    fn turns_hash_each_message_whatever_its_length_across_calls_and_rooms() {
        start();
        // Messages shorter than a piece, of several Rooms, and ending on a
        // Room's last byte; pieces that straddle Rooms; two calls, the
        // messages open between them.
        let lengths = [64usize, 1000, ROOM_LEN, 3 * ROOM_LEN + 7, 100_000, 5];
        let inputs: Vec<Vec<u8>> = lengths
            .iter()
            .enumerate()
            .map(|(i, &len)| (0..len).map(|j| (i * 31 + j * 7) as u8).collect())
            .collect();
        let key = ("turn-test", "lengths");
        let mut slots: Vec<TurnSlot> = lengths.iter().map(|_| TurnSlot::new()).collect();
        let mut sent = vec![0usize; lengths.len()];
        let mut digests: Vec<[u8; 32]> = Vec::new();
        let piece = 40_000;
        for call in 0..2 {
            loop {
                let mut arriving: Vec<(usize, &[u8])> = Vec::new();
                let mut ended = Vec::new();
                for (i, input) in inputs.iter().enumerate() {
                    // The first call stops halfway through each message.
                    let until = if call == 0 {
                        input.len() / 2
                    } else {
                        input.len()
                    };
                    let n = piece.min(until - sent[i]);
                    if n > 0 {
                        arriving.push((i, &input[sent[i]..sent[i] + n]));
                        sent[i] += n;
                        if sent[i] == input.len() {
                            ended.push(i);
                        }
                    }
                }
                if arriving.is_empty() {
                    break;
                }
                turn_update(key, &mut slots, &arriving);
                turn_finish(key, &mut slots, &ended, &mut |d| {
                    digests.push(d.try_into().unwrap())
                });
            }
            turn_drain(key, slots.len(), &mut |d| {
                digests.push(d.try_into().unwrap())
            });
        }
        let mut want: Vec<[u8; 32]> = inputs
            .iter()
            .map(|input| *blake3_servil::convenience::hash(input).as_bytes())
            .collect();
        want.sort();
        digests.sort();
        assert_eq!(digests, want);
        // The slots are reused: a second message through each.
        let again: Vec<(usize, &[u8])> = inputs
            .iter()
            .enumerate()
            .map(|(i, m)| (i, &m[..5]))
            .collect();
        let all: Vec<usize> = (0..inputs.len()).collect();
        let mut second = Vec::new();
        turn_update(key, &mut slots, &again);
        turn_finish(key, &mut slots, &all, &mut |d| {
            second.push(<[u8; 32]>::try_from(d).unwrap())
        });
        turn_drain(key, slots.len(), &mut |d| {
            second.push(d.try_into().unwrap())
        });
        assert_eq!(second.len(), inputs.len());
        for (i, input) in inputs.iter().enumerate() {
            assert!(
                second.contains(blake3_servil::convenience::hash(&input[..5]).as_bytes()),
                "message {i}"
            );
        }
    }

    #[test]
    fn keyed_messages_match_blake3_keyed_hash_across_sizes() {
        start();
        let keys = [[0x11u8; 32], [0x22u8; 32]];
        for len in [64, 4 * 1024, ROOM_LEN + 1024] {
            let input = crate::make_input_seeded(len, 19);
            let mut got = Vec::new();
            keyed_messages((0..6).map(|i| (input.as_slice(), &keys[i & 1])), |d| {
                got.push(<[u8; 32]>::try_from(d).unwrap())
            });
            let want: Vec<[u8; 32]> = (0..6)
                .map(|i| *blake3::keyed_hash(&keys[i & 1], &input).as_bytes())
                .collect();
            assert_eq!(got, want, "len {len}");
        }
    }

    #[test]
    fn batches_return_storage_and_every_digest_before_reuse() {
        start();
        for messages in [1, 16, 1024, 1025] {
            let mut state = Batches::new(messages, 2);
            for seed in [2, 7] {
                let input = crate::make_input_seeded(messages * crate::MESSAGE_LEN, seed);
                let expected: Vec<u8> = input
                    .chunks(crate::MESSAGE_LEN)
                    .flat_map(|m| *blake3::hash(m).as_bytes())
                    .collect();
                let mut got = Vec::new();
                state.run(&input, 7, |digests| got.push(digests.to_vec()));
                assert_eq!(got, vec![expected; 7]);
                assert_eq!(state.free.len(), 2);
                assert!(state.pipe.next_event().is_none());
            }
        }
    }
}
