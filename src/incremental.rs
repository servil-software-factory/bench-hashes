//! One complete incremental-hasher lifetime, shared by the stream and
//! short-message workloads. The producer controls the pieces, the caller
//! controls input preparation, and every final digest reaches `consume`.

use crate::{Algorithm, Pieces};
use std::hint::black_box;

#[inline(always)]
pub(crate) fn hash(algorithm: Algorithm, pieces: Pieces, mut consume: impl FnMut(&[u8])) {
    #[cfg(test)]
    crate::observe_call(algorithm.key());
    use sha1_checked::digest::Update as _;
    use sha2::Digest as _;
    match algorithm {
        Algorithm::Blake3 => {
            let mut h = blake3::Hasher::new();
            pieces(&mut |p| {
                h.update(p);
            });
            consume(h.finalize().as_bytes());
        }
        Algorithm::Blake3Rayon => {
            let mut h = blake3::Hasher::new();
            pieces(&mut |p| {
                h.update_rayon(p);
            });
            consume(h.finalize().as_bytes());
        }
        Algorithm::Blake3ServilSt | Algorithm::Blake3ServilMt => {
            let mut h = blake3_servil::convenience::Hasher::new(blake3_servil::Mode::Plain, None);
            pieces(&mut |p| {
                if algorithm == Algorithm::Blake3ServilSt {
                    #[cfg(test)]
                    crate::observe_call("update");
                    h.update(p);
                } else {
                    #[cfg(test)]
                    crate::observe_call("update_multithreaded");
                    blake3_servil::throughput::update_multithreaded(&mut h, p);
                }
            });
            consume(h.finalize().as_bytes());
        }
        Algorithm::Sha256 => {
            let mut h = crate::Sha256::new();
            pieces(&mut |p| h.update(p));
            consume(&h.finalize());
        }
        Algorithm::Sha256Ring => {
            let mut h = ring::digest::Context::new(&ring::digest::SHA256);
            pieces(&mut |p| h.update(p));
            consume(h.finish().as_ref());
        }
        Algorithm::Sha256CommonCrypto => consume(&crate::common_crypto::sha256_pieces(pieces)),
        Algorithm::Sha1Dc => {
            let mut h = sha1_checked::Sha1::new();
            pieces(&mut |p| h.update(p));
            consume(h.try_finalize().hash());
        }
        Algorithm::Sha3_256 => {
            let mut h = sha3::Sha3_256::new();
            pieces(&mut |p| sha3::Digest::update(&mut h, p));
            consume(&h.finalize());
        }
    }
}

/// Select each cold cell's API before the gap and clock reads, as the
/// one-shot cells do. Each monomorph has a constant contender.
pub(crate) fn call(algorithm: Algorithm) -> fn(&[u8], usize) {
    fn one<const A: usize>(input: &[u8], len: usize) {
        hash(
            Algorithm::ALL[A],
            &mut |update| update(black_box(&input[..len])),
            |d| {
                black_box(d);
            },
        );
    }
    macro_rules! calls {
        ($($a:literal)*) => {
            match Algorithm::ALL.iter().position(|&a| a == algorithm) {
                $(Some($a) => one::<$a>,)*
                _ => unreachable!("every contender has an incremental call"),
            }
        };
    }
    calls!(0 1 2 3 4 5 6 7 8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point, UseCase};

    // Published hashes of "abc", fixed independently of the adapter.
    fn abc(algorithm: Algorithm) -> &'static str {
        match algorithm {
            Algorithm::Blake3
            | Algorithm::Blake3Rayon
            | Algorithm::Blake3ServilSt
            | Algorithm::Blake3ServilMt => {
                "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
            }
            Algorithm::Sha256 | Algorithm::Sha256Ring | Algorithm::Sha256CommonCrypto => {
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            }
            Algorithm::Sha1Dc => "a9993e364706816aba3e25717850c26c9cd0d89d",
            Algorithm::Sha3_256 => {
                "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
            }
        }
    }

    #[test]
    fn fixed_vectors_cover_whole_and_fragmented_fresh_lifetimes() {
        blake3_servil::initialize_multithreaded(std::num::NonZeroUsize::new(2));
        for algorithm in Algorithm::ALL
            .into_iter()
            .filter(|a| a.availability().is_ok())
        {
            for chunks in [3, 1, 2, 3] {
                let mut delivered = 0;
                let mut results = Vec::new();
                hash(
                    algorithm,
                    &mut |update| {
                        for p in b"abc".chunks(chunks) {
                            delivered += p.len();
                            update(p);
                        }
                    },
                    |d| results.push(d.iter().map(|b| format!("{b:02x}")).collect::<String>()),
                );
                assert_eq!(delivered, 3);
                assert_eq!(
                    results,
                    [abc(algorithm)],
                    "{} chunks {chunks}",
                    algorithm.key()
                );
            }
        }
    }

    #[test]
    fn short_dispatch_consumes_every_digest_and_ignores_padding() {
        blake3_servil::initialize_multithreaded(std::num::NonZeroUsize::new(2));
        let (mut input, at) = crate::block_copy(b"abc");
        input[at + 3..].fill(99); // Whole-message length, rather than block length, reaches update.
        for algorithm in Algorithm::ALL
            .into_iter()
            .filter(|a| a.availability().is_ok())
        {
            for use_case in [UseCase::IncrementalMessage, UseCase::LentIncrementalMessage] {
                let point = Point {
                    use_case,
                    ..Point::one("", 3)
                };
                let mut results = Vec::new();
                crate::hash_batch(algorithm, &input[at..], point, 3, |d| {
                    results.push(d.iter().map(|b| format!("{b:02x}")).collect::<String>())
                });
                assert_eq!(
                    results,
                    vec![abc(algorithm); 3],
                    "{} {use_case:?}",
                    algorithm.key()
                );
            }
        }
    }

    #[test]
    fn each_axis_size_delivers_all_bytes_and_starts_a_fresh_hasher() {
        blake3_servil::initialize_multithreaded(std::num::NonZeroUsize::new(2));
        for (seed, len) in [64, 1024, 4096].into_iter().enumerate() {
            let (input, at) = crate::block_copy(&crate::make_input_seeded(len, seed as u64 + 3));
            for algorithm in Algorithm::ALL
                .into_iter()
                .filter(|a| a.availability().is_ok())
            {
                let mut expected = Vec::new();
                crate::hash_in_memory(algorithm, &input[at..], Point::one("", len), 1, |d| {
                    expected.push(d.to_vec())
                });
                for use_case in [UseCase::IncrementalMessage, UseCase::LentIncrementalMessage] {
                    let mut actual = Vec::new();
                    crate::hash_batch(
                        algorithm,
                        &input[at..],
                        Point {
                            use_case,
                            ..Point::one("", len)
                        },
                        3,
                        |d| actual.push(d.to_vec()),
                    );
                    assert_eq!(
                        actual,
                        vec![expected[0].clone(); 3],
                        "{} {len} {use_case:?}",
                        algorithm.key()
                    );
                }
            }
        }
    }

    #[test]
    fn cold_selection_and_nonstop_calls_use_fresh_incremental_lifetimes() {
        blake3_servil::initialize_multithreaded(std::num::NonZeroUsize::new(2));
        let input = crate::make_input(64);
        for algorithm in Algorithm::ALL
            .into_iter()
            .filter(|a| a.availability().is_ok())
        {
            let lifetime = match algorithm {
                Algorithm::Blake3ServilSt => vec![algorithm.key(), "update"],
                Algorithm::Blake3ServilMt => vec![algorithm.key(), "update_multithreaded"],
                _ => vec![algorithm.key()],
            };
            crate::CALLS.with(|c| c.borrow_mut().clear());
            let point = Point {
                use_case: UseCase::IncrementalMessage,
                ..Point::one("64 B", 64)
            };
            crate::take_sample(algorithm, &input, point, 2);
            assert_eq!(
                crate::CALLS.with(|c| c.borrow().clone()),
                lifetime.repeat(3),
                "one untimed warm-up and two complete lifetimes"
            );
            crate::CALLS.with(|c| c.borrow_mut().clear());
            let point = Point {
                use_case: UseCase::LentIncrementalMessage,
                ..point
            };
            let mut seen = 0;
            crate::hash_batch(algorithm, &input, point, 5, |_| seen += 1);
            assert_eq!(seen, 5);
            assert_eq!(
                crate::CALLS.with(|c| c.borrow().clone()),
                lifetime.repeat(5)
            );
        }
    }

    #[test]
    fn axes_and_selectors_hold_the_six_point_contract() {
        for (use_case, prefix, after_gap) in [
            (UseCase::IncrementalMessage, "incremental ", true),
            (UseCase::LentIncrementalMessage, "lent incremental ", false),
        ] {
            let points = &crate::POINTS[use_case.points()];
            assert_eq!(
                points.iter().map(|p| p.bytes).collect::<Vec<_>>(),
                [64, 1024, 4096]
            );
            assert_eq!(use_case.after_gap(), after_gap);
            assert_eq!(use_case.label_prefix(), prefix);
            for p in points {
                assert_eq!(p.messages, 1);
                assert_eq!(use_case.units(*p, 3), (3 * p.bytes) as u64);
                assert_eq!(
                    crate::POINTS[crate::point_named(&format!("{prefix}{}", p.label))].use_case,
                    use_case
                );
                for a in Algorithm::ALL {
                    assert!(a.takes_part(use_case));
                }
            }
        }
    }
}
