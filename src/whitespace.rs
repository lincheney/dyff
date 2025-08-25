pub trait CheckAllWhitespace {
    fn is_ascii_whitespace(&self) -> bool;
}

impl<T: AsRef<[u8]>> CheckAllWhitespace for T {
    fn is_ascii_whitespace(&self) -> bool {
        self.as_ref().iter().all(|c| c.is_ascii_whitespace())
    }
}
