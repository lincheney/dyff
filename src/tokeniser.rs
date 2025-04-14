use std::collections::HashMap;
use crate::types::Word;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Token(pub usize);

impl Token {
    pub const NEWLINE: Self = Self(0);
    pub const SPACE: Self = Self(1);
    pub const TAB: Self = Self(2);
    pub const FORM_FEED: Self = Self(3);
    pub const CARRIAGE_RETURN: Self = Self(4);

    pub fn is_ascii_whitespace(self) -> bool {
        matches!(self, Self::TAB | Self::SPACE | Self::FORM_FEED | Self::CARRIAGE_RETURN)
    }
}

#[derive(Debug)]
pub struct Tokeniser {
    mapping: HashMap<Word, Token>,
}

impl Tokeniser {
    pub fn new() -> Tokeniser {
        let mut tokeniser = Tokeniser {
            mapping: HashMap::new()
        };
        tokeniser.mapping.insert(b"\n".into(), Token::NEWLINE);
        tokeniser.mapping.insert(b" ".into(), Token::SPACE);
        tokeniser.mapping.insert(b"\t".into(), Token::TAB);
        tokeniser.mapping.insert(b"\x0c".into(), Token::FORM_FEED);
        tokeniser.mapping.insert(b"\r".into(), Token::CARRIAGE_RETURN);
        tokeniser
    }

    pub fn max_token(&self) -> Token {
        Token(self.mapping.len())
    }

    pub fn map(&mut self, word: &[u8]) -> Token {
        if let Some(t) = self.mapping.get(word) {
            *t
        } else {
            let t = self.max_token();
            self.mapping.insert(word.into(), t);
            t
        }
    }

}
