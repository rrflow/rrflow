//! Stable SHA-256 identity behind one kernel-owned incremental interface.
//!
//! The locked RustCrypto primitive uses runtime CPU detection with a portable
//! fallback. Upstream state never escapes this wrapper; storage, receipts and
//! adapters keep the same byte-level identity and public API.

use sha2::Digest as _;

/// Incremental SHA-256 state for authenticating bounded I/O.
///
/// Callers cannot inspect or serialize a partial digest and accidentally treat
/// it as durable evidence. Lazy initialization preserves const construction.
#[derive(Debug, Clone)]
pub struct Sha256 {
    state: Option<sha2::Sha256>,
    length: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    pub const fn new() -> Self {
        Self {
            state: None,
            length: 0,
        }
    }

    pub fn update(&mut self, input: &[u8]) {
        self.length = self
            .length
            .checked_add(input.len() as u64)
            .expect("SHA-256 input length exceeds u64");
        self.state.get_or_insert_default().update(input);
    }

    pub fn finalize(self) -> [u8; 32] {
        self.state.unwrap_or_default().finalize().into()
    }

    pub fn finalize_hex(self) -> String {
        hex(self.finalize())
    }
}

/// SHA-256 of `input`.
pub fn sha256(input: &[u8]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(input);
    digest.finalize()
}

pub fn sha256_hex(input: &[u8]) -> String {
    hex(sha256(input))
}

fn hex(digest: [u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_vectors_cover_padding_and_incremental_boundaries() {
        // Generated independently with Python hashlib.sha256, not this wrapper.
        for (length, expected) in [
            (
                0,
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                1,
                "ca358758f6d27e6cf45272937977a748fd88391db679ceda7dc7bf1f005ee879",
            ),
            (
                55,
                "8aa994584139d128848eeebc4e815639ba5ab6e6e39574195a63ac4f14f7c43b",
            ),
            (
                56,
                "ad574708f75c044c9b85de64cb568ee7711ff4f36448c6242f053ba8f6cc2b63",
            ),
            (
                63,
                "280ed3e8ff1df845b2e7dfe6ac6cee817bef20e783cc65abc41b818b4d2fe076",
            ),
            (
                64,
                "c6ab9724ade5b6a7a1edfffb12f3aa9181351355af8fd08c919952ad211339dd",
            ),
            (
                65,
                "788367c73c7ddf4c53f65e68cc0d943e6227ab55b0e78ba63ace822b1c6301c0",
            ),
            (
                127,
                "192409cd280e14b743642ad1343fbd3e82d9305de72c078117745a679210cc3d",
            ),
            (
                128,
                "cc548ca2dec1f6fe4f58b2e27aa9c7521607df1130d140b55a4dad0665302356",
            ),
            (
                129,
                "81e89a7b2911aaa7795f9e3d4910cb47d6cd2b00d83b8399481527261a1a7519",
            ),
            (
                4097,
                "bb37fb3430fb3bc003f70512ee0189aec84b029b221885c8f16280d1b935483e",
            ),
        ] {
            let input = (0..length).map(|n| (n * 31 + 7) as u8).collect::<Vec<_>>();
            assert_eq!(sha256_hex(&input), expected, "one-shot length {length}");
            for chunk in [1, 7, 55, 56, 63, 64, 65, 127, 128, 4096] {
                let mut stream = Sha256::new();
                for bytes in input.chunks(chunk) {
                    stream.update(bytes);
                    stream.update(&[]);
                }
                assert_eq!(
                    stream.finalize_hex(),
                    expected,
                    "length {length}, chunk {chunk}"
                );
            }
        }
        assert_eq!(
            sha256_hex(&vec![b'a'; 1_000_000]),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn const_initialization_and_cloned_streams_preserve_independent_state() {
        const EMPTY: Sha256 = Sha256::new();
        assert_eq!(
            EMPTY.finalize_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let mut first = EMPTY;
        first.update(&[0xa5; 55]);
        let mut second = first.clone();
        first.update(b"left");
        second.update(b"right");
        assert_eq!(
            first.finalize_hex(),
            "63be0d41dc727923d3374c60f89b4e3bc6340e3dd0e95fedc48e66128aaa189e"
        );
        assert_eq!(
            second.finalize_hex(),
            "e916bf82f45a6c13f3121ea2b4b792b64765448adfbc3c5ebf7316f662ae4548"
        );
    }

    #[test]
    #[ignore = "manual bounded release throughput measurement, not capacity qualification"]
    fn bounded_sha256_throughput() {
        let input = vec![0xa5; 32 * 1024 * 1024];
        let started = std::time::Instant::now();
        for _ in 0..16 {
            let hash = sha256_hex(std::hint::black_box(&input));
            assert_eq!(
                std::hint::black_box(hash.as_str()),
                "edeadec8f638055689d5be63b4bcf2654fb64bf91fb6651e9a924f052a9c7db0"
            );
        }
        println!(
            "digest_throughput bytes={} elapsed_ns={} target={} qualification=false",
            input.len() * 16,
            started.elapsed().as_nanos(),
            std::env::consts::ARCH
        );
    }

    #[test]
    #[should_panic(expected = "SHA-256 input length exceeds u64")]
    fn input_length_overflow_remains_rejected() {
        let mut stream = Sha256::new();
        stream.length = u64::MAX;
        stream.update(&[1]);
    }

    #[test]
    fn matches_published_sha256_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let mut incremental = Sha256::new();
        incremental.update(b"a");
        incremental.update(b"");
        incremental.update(b"bc");
        assert_eq!(
            incremental.finalize_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        let input = (0..4097)
            .map(|index| (index * 31 + 7) as u8)
            .collect::<Vec<_>>();
        let expected = sha256_hex(&input);
        for chunk_size in 1..=130 {
            let mut streamed = Sha256::new();
            for chunk in input.chunks(chunk_size) {
                streamed.update(chunk);
            }
            assert_eq!(streamed.finalize_hex(), expected, "chunk size {chunk_size}");
        }
    }
}
