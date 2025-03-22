use anyhow::Result;
use tiktoken_rs::cl100k_base;

fn bpe_tokenize(text: &str) -> Result<Vec<String>> {
    let bpe = cl100k_base()?;
    bpe.split_by_token(text, true)
}

fn token_count(text: &str) -> Result<usize> {
    bpe_tokenize(text).map(|t| t.len())
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
}
