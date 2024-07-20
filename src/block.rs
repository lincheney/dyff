use std::io::{BufWriter, Write};
use std::cmp::min;
use std::collections::VecDeque;
use anyhow::{Result};
use super::part::Part;
use super::style;
use super::regexes::regex;
use super::types::*;

fn find_common_prefix_length<
    'a,
    A: Iterator<Item=&'a [u8]>,
    B: Iterator<Item=&'a [u8]>,
>(a: A, b: B) -> usize {
    a.zip(b).take_while(|(a, b)| a == b).count()
}

fn find_common_suffix_length<
    'a,
    A: DoubleEndedIterator<Item=Bytes<'a>> + ExactSizeIterator,
    B: DoubleEndedIterator<Item=Bytes<'a>> + ExactSizeIterator,
>(a: A, b: B) -> usize {

    a.zip(b).rev().take_while(|(a, b)| a == b).count()
}


pub struct Block<'a> {
    pub parts: Vec<Part<'a>>,
}

impl<'a> Block<'a> {
    const CUTOFF: f64 = 0.6;
    const MIN_SIZE_EOL: usize = 2;
    const MIN_SIZE: usize = 7;

    fn perfect(&self) -> bool {
        self.parts.iter().all(|p| p.matches && p.whole_line())
    }

    fn score(&self) -> f64 {
        // limit the effect of very long blocks
        const maxlen: usize = 10;
        let total: usize = self.parts.iter().map(|p| min(maxlen, p.word_len(0)) + min(maxlen, p.word_len(1))).sum();
        if total == 0 {
            return 1f64
        }

        let matches: usize = self.parts.iter().filter(|p| p.matches).map(|p| min(maxlen, p.word_len(0))).sum();
        2. * matches as f64 / total as f64
    }

    fn squeeze_parts(&mut self) {
        // squeeze matches that are too small

        let mut parts = Vec::<Part>::new();
        let mut join = false;

        for (_i, part) in self.parts.iter().enumerate() {
            if part.matches {

                let total_length = part.slices[0].len();

                // strip newlines
                let leading_nl = part.get(0).take_while(|w| w == b"\n").count();
                let trailing_nl = part.get(0).rev().take_while(|w| w == b"\n").count();
                let trailing_nl = min(trailing_nl, total_length - leading_nl);
                let length: usize = part.get(0)
                    .skip(leading_nl)
                    .take(total_length - trailing_nl - leading_nl)
                    .map(|w| w.len())
                    .sum();

                if part.whole_line() || (total_length == 1 && part.get(0).next().unwrap() == b"\n") {

                // elif len(parts) >= 2 and any(parts[-1].is_empty(i) and not parts[-2].is_empty(i) for i in SIDES):
                    // // this is actually next to another part
                    // pass
                // elif i+2 < len(self.parts) and any(self.parts[i+1].is_empty(i) and not self.parts[i+2].is_empty(i) for i in SIDES):
                    // // this is actually next to another part
                    // pass
                } else if part.starts_line(0) || part.starts_line(1) || part.ends_line(0) || part.ends_line(1) {
                    join = false;
                    // join = length < min_size_eol
                } else {
                    join = length < Block::MIN_SIZE;
                }

                if !join {
                    parts.push(part.clone());
                } else if parts.is_empty() {
                    // insert a placeholder
                    let mut part = part.partition(part.slices[0].start, part.slices[1].start).0;
                    part.matches = false;
                    parts.push(part);
                }
            }

            // join if requested or adjacent non matches
            if !parts.is_empty() && (join || (!part.matches && !parts.last().unwrap().matches)) {
                let mut last = parts.pop().unwrap();
                last.slices = [
                    last.slices[0].start .. part.slices[0].end,
                    last.slices[1].start .. part.slices[1].end,
                ];
                parts.extend(last.split().into_iter().flatten());
            } else if !part.matches {
                parts.push(part.clone());
            }

            join = false;
        }

        if parts.len() != self.parts.len() {
            parts.retain(|p| !p.is_empty(0) || !p.is_empty(1));
            self.parts = parts;
        }
    }

    fn is_empty(&self, i: usize) -> bool {
        self.parts.iter().all(|p| p.is_empty(i))
    }

    fn splits_to_multiline(&self) -> bool {
        if self.is_empty(0) || self.is_empty(1) {
            // one side is empty
            return false;
        }
        let splits = |i| self.parts[0].first_lineno(i) == self.parts.last().unwrap().last_lineno(i);
        splits(0) != splits(1)
    }

    fn merge_blocks_on_score(mut blocks: Vec<Block>, cutoff: f64) -> Vec<Block> {
        // merge adjacent blocks if they are both good matches or both bad matches
        let mut drain  = blocks.drain(..);
        let mut prev = drain.next().unwrap();
        let mut merged = vec![];

        for block in drain {
            let prev_perfect = prev.perfect();
            let new_perfect = block.perfect();

            let mut merge = if prev_perfect || new_perfect {
                // check if both perfect
                prev_perfect == new_perfect
            } else {
                // check if they're both good or both bad
                (prev.score() < cutoff) == (block.score() < cutoff)
            };

            // do not merge blocks where one side is single line and the other is multiline
            if merge && (prev.splits_to_multiline() && prev.score() > 0.) || (block.splits_to_multiline() && block.score() > 0.) {
                merge = false;
            }

            if merge {
                prev.parts.extend(block.parts)
            } else {
                merged.push(prev);
                prev = block;
            }
        }

        merged.push(prev);
        merged
    }

    fn last_non_empty(&self, i: usize) -> Option<&Part> {
        self.parts.iter().rev().find(|p| !p.is_empty(i))
    }

    fn score_words(&self, words: &VecDeque<&[u8]>, parti: usize, i: usize, shift: isize) -> [[usize; 4]; 2] {
        static prefixes: [&[u8]; 4] = [
            b"\n",
            b" \t",
            b",;",
            b"{[(",
        ];
        static suffixes: [&[u8]; 4] = [
            b"\n",
            b" \t",
            b",;",
            b"}])",
        ];

        let mut suffix_scores = [0; 4];
        let mut prefix_scores = [0; 4];

        let mut skip = 0;
        for (i, p) in prefixes.iter().enumerate() {
            let count = words.iter().skip(skip).take_while(|w| p.contains(&w[0])).count();
            skip += count;
            prefix_scores[i] += count;
        }

        let mut skip = 0;
        for (i, p) in suffixes.iter().enumerate() {
            let count = words.iter().rev().skip(skip).take_while(|w| p.contains(&w[0])).count();
            skip += count;
            suffix_scores[i] += count;
        }

        let part = &self.parts[parti];

        // check if this is at start of line
        let start = (part.slices[i].start as isize + shift - 1) as usize;
        if start == 0 || part.parent.words[i][start].as_bytes() == b"\n" {
            prefix_scores[0] += 1;
        }

        // check if this is at end of line
        let end = (part.slices[i].end as isize + shift) as usize;
        if part.parent.words[i].get(end).map(|s| s.as_bytes() == b"\n") == Some(true) {
            suffix_scores[0] += 1;
        }

        // prefer suffix scores
        [suffix_scores, prefix_scores]
    }

    fn score_part_shift(&self, parti: usize, i: usize) -> Vec<([[usize; 4]; 2], isize)> {
        let part = &self.parts[parti];
        let mut scores = vec![];

        let mut words = part.get(i).collect::<VecDeque<&[u8]>>();
        // no shift; more score if it is start or end of line
        // let mut iter = std::iter::once((self.score_words(&words, parti, i, 0), 0));
        scores.push((self.score_words(&words, parti, i, 0), 0));

        // try shift left ie move stuff at back to front
        if parti > 0 {
            let prev_words = self.parts[parti-1].get(i);
            for (shift, word) in prev_words.rev().enumerate() {
                if word != *words.back().unwrap() {
                    break
                }
                words.rotate_right(1);
                let shift = -(1 + shift as isize);
                scores.push((self.score_words(&words, parti, i, shift), shift));
            }
        }

        let mut words = part.get(i).collect::<VecDeque<&[u8]>>();
        // try shift right ie move stuff at front to back
        if let Some(next_words) = self.parts.get(parti+1) {
            let next_words = next_words.get(i);
            for (shift, word) in next_words.enumerate() {
                if word != words[0] {
                    break
                }
                words.rotate_left(1);
                let shift = 1 + shift as isize;
                scores.push((self.score_words(&words, parti, i, shift), shift));
            }
        }

        scores
    }

    fn shift_parts(&mut self) {
        // try to shift non matches around e.g. so that whitespace is at the ends
        let mut insert_start = None;
        let mut insert_end = None;
        for i in 0..self.parts.len() {
            {
                let part = &self.parts[i];
                if part.matches || (!part.is_empty(0) && !part.is_empty(1)) {
                    continue
                }
            }

            let side = if self.parts[i].is_empty(0) { 1 } else { 0 };
            // prefer better score, less shifting, and shifting right
            let scores = self.score_part_shift(i, side);
            let (_score, shift) = scores.iter().max_by_key(|(score, shift)| (score, -shift.abs(), shift)).unwrap();

            if *shift == 0 {
                continue
            }

            let (left, right) = self.parts.split_at_mut(i);
            let (mid, right) = right.split_at_mut(1);
            let part = &mut mid[0];

            let prev = left.last_mut().unwrap_or_else(|| {
                let mut prev = part.partition(part.slices[0].start, part.slices[1].start).0;
                prev.matches = true;
                insert_start = Some(prev);
                insert_start.as_mut().unwrap()
            });
            prev.slices = [
                prev.slices[0].start .. (part.slices[0].start as isize + shift) as usize,
                prev.slices[1].start .. (part.slices[1].start as isize + shift) as usize,
            ];

            let next = right.first_mut().unwrap_or_else(|| {
                let mut next = part.partition(part.slices[0].end, part.slices[1].end).1;
                next.matches = true;
                insert_end = Some(next);
                insert_end.as_mut().unwrap()
            });
            next.slices = [
                (part.slices[0].end as isize + shift) as usize .. next.slices[0].end,
                (part.slices[1].end as isize + shift) as usize .. next.slices[1].end,
            ];

            part.slices = [
                prev.slices[0].end .. next.slices[0].start,
                prev.slices[1].end .. next.slices[1].start,
            ];
        }

        if let Some(insert_start) = insert_start {
            self.parts.insert(0, insert_start);
        }
        if let Some(insert_end) = insert_end {
            self.parts.push(insert_end);
        }
    }

    pub fn split_block(mut self) -> Vec<Self> {
        self.squeeze_parts();

        let mut blocks = vec![Block{parts: vec![]}];

        // group parts based on line numbers
        for part in self.parts {
            let block = &blocks.last().unwrap();

            let join = if block.parts.is_empty() {
                true
            } else if block.last_non_empty(0).map(|last| part.first_lineno(0) == last.last_lineno(0)).unwrap_or(false) {
                true
            } else if block.last_non_empty(1).map(|last| part.first_lineno(1) == last.last_lineno(1)).unwrap_or(false) {
                true
            } else {
                false
            };

            if !join {
                // different line
                blocks.push(Block{parts: vec![]});
            }
            blocks.last_mut().unwrap().parts.push(part);
        }

        // match leading whitespace in each block
        // since it got treated as junk during the diff
        for block in blocks.iter_mut() {
            let first = &block.parts[0];
            if !first.matches {
                // find common prefix
                let prefix = find_common_prefix_length(first.get(0), first.get(1));
                if prefix != 0 {
                    let (mut first, second) = first.partition(first.slices[0].start + prefix, first.slices[1].start + prefix);
                    first.matches = true;
                    block.parts[0] = first;
                    block.parts.insert(1, second);
                }
            }
        }

        let mut blocks = Block::merge_blocks_on_score(blocks, Block::CUTOFF);

        for block in blocks.iter_mut() {
            block.shift_parts();
            block.squeeze_parts();
        }

        // if score is too low, make the whole thing non matching
        for block in blocks.iter_mut() {
            if block.score() < Block::CUTOFF {
                let first = &block.parts[0];
                let part = Part{
                    parent: first.parent,
                    matches: false,
                    slices: [
                        first.slices[0].start .. block.parts.last().unwrap().slices[0].end,
                        first.slices[1].start .. block.parts.last().unwrap().slices[1].end,
                    ],
                };

                // find common prefix
                let prefix = find_common_prefix_length(part.get(0), part.get(1));
                let (mut first, second) = part.partition(part.slices[0].start + prefix, part.slices[1].start + prefix);
                first.matches = true;

                // find common suffix
                let suffix = if second.single_line(0) && second.single_line(1) {
                    find_common_suffix_length(second.get(0), second.get(1))
                } else {
                    0
                };
                let (second, mut third) = second.partition(second.slices[0].end - suffix, second.slices[1].end - suffix);
                third.matches = true;

                block.parts.clear();
                block.parts.extend_from_slice(&[first, second, third]);
                block.squeeze_parts();
                block.parts.retain(|p| !p.is_empty(0) || p.is_empty(1));
                // nothing matches
                if block.parts.iter().all(|p| !p.matches) {
                    block.parts.clear();
                    block.parts.push(part);
                }
            }
        }

        // merge again
        let blocks = Block::merge_blocks_on_score(blocks, Block::CUTOFF);

        blocks
    }

    pub fn print<
        T: Write,
        S: AsRef<str>,
        F: Fn(Option<usize>, Option<usize>, Option<&str>, Option<&str>, Option<&str>)->S
    >(
        &self,
        stdout: &mut BufWriter<T>,
        merge_markers: Option<&super::hunk::MergeMarkers>,
        style: style::Style,
        format_lineno: F,
    ) -> Result<()> {

        if !style.show_both && self.parts.iter().all(|p| p.matches || (p.is_empty(0) && p.is_empty(1))) {
            // this is entirely matching

            let mut lineno = 0;
            let mut newline = true;
            for part in self.parts.iter() {
                if !part.matches {
                    continue
                }

                for word in part.get(0) {
                    if newline {
                        if style.line_numbers {
                            stdout.write_all(format_lineno(
                                Some(part.first_lineno(0) + lineno), Some(part.first_lineno(1) + lineno),
                                Some(style::LINENO), Some(style::LINENO),
                                None,
                            ).as_ref().as_bytes())?;
                        }
                        if style.signs {
                            stdout.write_all(style::SIGN[2])?;
                        }
                        stdout.write_all(style::DIFF_CONTEXT)?;
                        newline = false;
                    }
                    stdout.write_all(&regex!(r"\s+\n".replace_all(word, style::DIFF_TRAILING_WS)))?;
                    if word == b"\n" {
                        lineno += 1;
                        newline = true;
                    }
                }

            }
            return Ok(())
        }

        for i in 0..=1 {
            let mut lineno = 0;
            let mut newline = true;
            let mut insert = false;
            let mut lineno_args = [None, None];

            for part in self.parts.iter() {
                if part.is_empty(i) {
                    insert = true;
                    continue
                }

                if insert {
                    // add an insertion marker
                    stdout.write_all(style::DIFF_INSERT[i])?;
                    insert = false;
                }

                let highlight = if part.matches { style.diff_matching } else { style.diff_non_matching };
                stdout.write_all(highlight[i])?;
                for word in part.get(i) {

                    if newline {
                        let lineno = part.first_lineno(i) + lineno;

                        if style.line_numbers {
                            lineno_args[i] = Some(lineno);
                            let bar_style = merge_markers.and_then(|m| m.get(&(i, lineno)).map(|x| x.as_ref()));
                            stdout.write_all(format_lineno(
                                lineno_args[0], lineno_args[1],
                                None, None,
                                bar_style,
                            ).as_ref().as_bytes())?;
                        }
                        if style.signs {
                            stdout.write_all(style::SIGN[i])?;
                        }
                        stdout.write_all(highlight[i])?;

                        newline = false;
                    }

                    // line = re.sub(rb'(\s+\n)', style['diff_trailing_ws'].replace(b'\\', b'\\\\') + rb'\1', line)
                    stdout.write_all(word)?;
                    if word == b"\n" {
                        lineno += 1;
                        newline = true;
                    }
                }
            }
        }

        Ok(())
    }
}
