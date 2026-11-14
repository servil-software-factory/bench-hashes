//! The multithreaded contender's owned-buffer and receiving cells.
//!
//! Rooms keep the existing 1 MiB input budget. Short messages share a room;
//! every nonfinal commit of a longer message is a whole room. Each call
//! drains its results, so samples include completion and reuse starts clean.
//! The synchronous lent-buffer cells remain the baseline in main.rs.

use blake3_servil::throughput::{block_on, Batch, Block, Event, Limits, Pipeline, Room, ROOM_LEN};
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

    /// Whole messages, each produced directly into rooms. The room's unused
    /// tail can hold the next short message; a long message starts a fresh
    /// room so every nonfinal commit has ROOM_LEN bytes at offset zero.
    fn run<'a>(
        &mut self,
        inputs: impl IntoIterator<Item = &'a [u8]>,
        mut consume: impl FnMut(&[u8]),
    ) {
        let mut room: Option<Room> = None;
        let mut pending = 0;
        for input in inputs {
            if pending == self.capacity {
                self.consume(&mut consume);
                pending -= 1;
            }
            let mut message = block_on(self.pipe.message((), Some(input.len() as u64)));
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
