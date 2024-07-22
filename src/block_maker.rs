use super::hunk::{Hunk};
use super::word_differ::WordDiffer;
use super::line_differ::LineDiffer;
use super::part::Part;
use super::block::Block;
use super::whitespace::CheckAllWhitespace;
use super::types::*;

#[derive(Debug)]
pub struct BlockMaker<'a> {
    line_numbers: [usize; 2],

    pub words: [Vec<Bytes<'a>>; 2],

    word_to_line: [Vec<usize>; 2],
    pub line_to_word: [Vec<usize>; 2],
}

impl<'a> BlockMaker<'a> {
    pub fn new(hunk: &'a Hunk, line_numbers: [usize; 2]) -> Self {
        // make a mapping from word number to line number
        let mut words = [vec![], vec![]];
        let mut word_to_line = [vec![], vec![]];
        let mut line_to_word = [vec![], vec![]];

        for i in 0..=1 {
            let w = &mut words[i];
            for (lineno, line) in hunk.get(i).iter().enumerate() {
                let oldlen = w.len();
                line_to_word[i].push(oldlen);
                super::regexes::regex!(
                    r"[A-Z][A-Z_]*[A-Z]\d*"
                    "|[A-Z][a-z0-9]*[a-z]"
                    "|[a-z0-9]+[a-z]"
                    r"|\d+"
                    r"|\s"
                    r"|[-!=~+]="
                    r"|(?:"
                        r"(?:[\xc0-\xdf][\x80-\xbf])"
                        r"|(?:[\xe0-\xef][\x80-\xbf][\x80-\xbf])"
                        r"|(?:[\xf0-\xf7][\x80-\xbf][\x80-\xbf][\x80-\xbf])"
                    r")+"
                    "|."
                    "|\n",
                    |r| { w.extend(r.find_iter(line).map(|m| m.as_bytes())) }
                );
                for _ in oldlen..w.len() {
                    word_to_line[i].push(lineno);
                }
            }
            word_to_line[i].push(line_to_word[i].len());
            line_to_word[i].push(w.len());
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

    fn get_line(&self, i: usize, lineno: usize) -> &[Bytes] {
        &self.words[i][self.get_wordno(i, lineno) .. self.get_wordno(i, lineno+1)]
    }

    pub fn make_part(&self, matches: bool, left: std::ops::Range<usize>, right: std::ops::Range<usize>) -> Part {
        Part{parent: self, matches, slices: [left, right]}
    }

    pub fn make_block(&self) -> Block {
        // diff by line first
        let mut ranges = vec![];
        let mut previ = 0;
        let mut prevj = 0;
        let maxi = self.words[0].len();
        let maxj = self.words[1].len();

        for (left, right) in LineDiffer::new(self).get_matching_blocks() {

            if previ < left.start && prevj < right.start && left.end < maxi && right.end < maxj {
                // these lines are in the middle
                // check if all the lines are merely indented
                let get_line = |i: usize, lineno: usize| {
                    let line = self.get_line(i, lineno);
                    let start = line.iter().position(|w| !w.is_ascii_whitespace()).unwrap_or(0);
                    &line[start..]
                };

                let start_line = self.get_lineno(0, left.start);
                let end_line = self.get_lineno(0, left.end);
                let line = get_line(0, start_line);

                if (start_line+1..end_line).all(|l| get_line(0, l) == line) {
                    let all_same = {
                        let prev_left = get_line(0, start_line-1);
                        let next_right = get_line(1, self.get_lineno(1, right.end));
                        prev_left == next_right && prev_left == line
                    } || {
                        let prev_right = get_line(1, self.get_lineno(1, right.start-1));
                        let next_left = get_line(0, end_line);
                        prev_right == next_left && prev_right == line
                    };

                    if all_same {
                        // diff for indentation instead
                        continue
                    }
                };
            }

            if previ < left.start || prevj < right.start {
                ranges.push((false, previ .. left.start, prevj .. right.start));
            }
            previ = left.end;
            prevj = right.end;
            ranges.push((true, left, right));
        }
        if previ < maxi || prevj < maxj {
            ranges.push((false, previ .. maxi, prevj .. maxj));
        }

        let mut parts = vec![];
        let mut differ = WordDiffer::new(self);

        for (matches, left, right) in ranges {
            if matches {
                // just one make part if it matches
                let part = self.make_part(true, left, right);
                parts.push(part);
                continue
            }

            let mut previ = left.start;
            let mut prevj = right.start;
            for part in differ.get_matching_blocks(left.start, left.end, right.start, right.end) {
                let i = part.slices[0].start;
                let j = part.slices[1].start;

                if previ < i || prevj < j {
                    let part = self.make_part(false, previ..i, prevj..j);
                    parts.extend(part.split().into_iter().flatten());
                }

                previ = part.slices[0].end;
                prevj = part.slices[1].end;
                parts.extend(part.split().into_iter().flatten());
            }

            if previ < left.end || prevj < right.end {
                let part = self.make_part(false, previ..left.end, prevj..right.end);
                parts.extend(part.split().into_iter().flatten());
            }
        }

        parts.retain(|p| !p.is_empty(0) || !p.is_empty(1));
        Block{parts}
    }

}
