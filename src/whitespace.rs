pub trait CheckAllWhitespace {
    fn is_ascii_whitespace(&self) -> bool;
}

impl CheckAllWhitespace for &[u8] {
    fn is_ascii_whitespace(&self) -> bool {
        self.iter().all(|c| c.is_ascii_whitespace())
    }
}
