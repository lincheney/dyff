use regex::bytes::{Regex, Match};
use super::hunk::{Hunk};
use super::word_differ::WordDiffer;
use super::part::Part;
use super::block::Block;

#[derive(Debug)]
pub struct BlockMaker<'a> {
    line_numbers: [usize; 2],

    pub words: [Vec<Match<'a>>; 2],

    word_to_line: [Vec<usize>; 2],
    line_to_word: [Vec<usize>; 2],
}

impl<'a> BlockMaker<'a> {
    pub fn new(hunk: &'a Hunk, line_numbers: [usize; 2]) -> Self {
        let utf8_regex = "(?:[\\xc0-\\xdf][\\x80-\\xbf])|(?:[\\xe0-\\xef][\\x80-\\xbf][\\x80-\\xbf])|(?:[\\xf0-\\xf7][\\x80-\\xbf][\\x80-\\xbf][\\x80-\\xbf])";
        let regex = format!("{}{}{}", r"[A-Z]{2,}\d*|[A-Z][a-z0-9]*[a-z]|[a-z0-9]+[a-z]|\d+|\s|[-!=~+]=|(?:", utf8_regex, r")+|.|\n");
        let regex = Regex::new(&regex).unwrap();

        // make a mapping from word number to line number
        let mut words = [vec![], vec![]];
        let mut word_to_line = [vec![], vec![]];
        let mut line_to_word = [vec![], vec![]];

        for i in 0..=1 {
            line_to_word[i].push(0);
            let w = &mut words[i];
            for (lineno, line) in hunk.get(i).iter().enumerate() {
                let oldlen = w.len();
                w.extend(regex.find_iter(line));
                line_to_word[i].push(w.len());
                for _ in oldlen..w.len() {
                    word_to_line[i].push(lineno);
                }
            }
            word_to_line[i].push(*word_to_line[i].last().unwrap() + 1);
            line_to_word[i].push(w.len() + 1);
        }

        Self{
            words,
            line_numbers,
            word_to_line,
            line_to_word,
        }
    }

    pub fn get_lineno(&self, i: usize, wordno: usize) -> usize {
        self.word_to_line[i][wordno] + self.line_numbers[i]
    }

    pub fn get_wordno(&self, i: usize, lineno: usize) -> usize {
        self.line_to_word[i][lineno - self.line_numbers[i]]
    }

    pub fn make_block(&self) -> Block {
        let mut parts = vec![];
        let mut previ = 0;
        let mut prevj = 0;

        for part in WordDiffer::new(self).get_matching_blocks() {
            let i = part.slices[0].start;
            let j = part.slices[1].start;

            if previ < i || prevj < j {
                let part = Part{parent: self, matches: false, slices: [previ..i, prevj..j]};
                parts.extend(part.split().into_iter().flatten());
            }

            previ = part.slices[0].end;
            prevj = part.slices[1].end;
            parts.extend(part.split().into_iter().flatten());
        }

        let maxi = self.words[0].len();
        let maxj = self.words[1].len();
        if previ < maxi || prevj < maxj {
            let part = Part{parent: self, matches: false, slices: [previ..maxi, prevj..maxj]};
            parts.extend(part.split().into_iter().flatten());
        }

        // parts = [p for p in parts if not (p.is_empty(0) and p.is_empty(1))]:
        Block{parts}
    }

}
