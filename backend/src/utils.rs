use anyhow::Result;
use tiktoken_rs::cl100k_base;

/// Nanoseconds at 1 millisecond
pub const NANOS_IN_MILLIS: u64 = 1_000_000;

/// Tokenize string from given string, using bpe cl100k.
fn bpe_tokenize(text: &str) -> Result<Vec<String>> {
    let bpe = cl100k_base()?;
    bpe.split_by_token(text, true)
}

/// Count token size from given string, using bpe cl100k.
fn token_count(text: &str) -> Result<usize> {
    bpe_tokenize(text).map(|t| t.len())
}

/// Gets current timestamp inside a canister, in milliseconds since the epoch (1970-01-01)
pub fn timestamp() -> u64 {
    ic_cdk::api::time() / NANOS_IN_MILLIS
}

#[cfg(test)]
pub mod mock_timestamp {
    use std::cell::Cell;

    thread_local! {
        static TIMESTAMP: Cell<u64> = Cell::new(0);
    }

    pub fn timestamp() -> u64 {
        let ts = TIMESTAMP.get();
        TIMESTAMP.with(|c| c.set(ts + 1));
        ts
    }

    pub fn reset_to(time: u64) {
        TIMESTAMP.with(|c| c.set(time));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_count_valid() {
        let tokens = token_count("This is a test      with spaces").unwrap();
        assert_eq!(tokens, 7);
    }

    #[test]
    fn tokenize_valid() {
        let tokens = bpe_tokenize("This is a test      with spaces").unwrap();
        assert_eq!(
            tokens,
            vec!["This", " is", " a", " test", "     ", " with", " spaces"]
        );
    }

    #[test]
    #[should_panic]
    fn timestamp_canister_only() {
        let _ = timestamp();
    }
}
