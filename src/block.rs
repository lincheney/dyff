use std::io::{BufWriter, Write};
use std::cmp::{min};
use anyhow::{Result};
use super::part::Part;
use super::style;
use super::types::*;
use super::whitespace::CheckAllWhitespace;

fn find_common_prefix_length(a: &[Bytes], b: &[Bytes]) -> usize {
    a.iter().zip(b).take_while(|(a, b)| a == b).count()
}

fn find_common_suffix_length(a: &[Bytes], b: &[Bytes]) -> usize {
    a.iter().rev().zip(b.iter().rev()).take_while(|(a, b)| a == b).count()
}


#[derive(Debug)]
pub struct Block<'a> {
    pub parts: Vec<Part<'a>>,
}

impl<'a> Block<'a> {
    const CUTOFF: f64 = 0.6;
    const _MIN_SIZE_EOL: usize = 2;
    const MIN_SIZE: usize = 7;

    fn perfect(&self) -> bool {
        self.parts.iter().all(|p| p.matches && p.whole_line())
    }

    fn score(&self) -> f64 {
        // limit the effect of very long blocks
        const MAXLEN: usize = 10;
        let total: usize = self.parts.iter().map(|p| min(MAXLEN, p.word_len(0)) + min(MAXLEN, p.word_len(1))).sum();
        if total == 0 {
            return 1f64
        }

        let matches: usize = self.parts.iter().filter(|p| p.matches).map(|p| min(MAXLEN, p.word_len(0))).sum();
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
                let length: usize = part.get(0)
                    .iter()
                    .skip_while(|&w| w == b"\n")
                    .take_while(|&w| w != b"\n")
                    .map(|w| w.len())
                    .sum();

                if part.whole_line() || (total_length == 1 && part.get(0)[0] == b"\n") {

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
                    parts.push(part.partition_from_start(0, 0, false).0);
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
            let prev_score = prev.score();
            let new_score = block.score();

            let mut merge = if prev_perfect || new_perfect {
                // check if both perfect
                prev_perfect == new_perfect
            } else if prev_score == 0. || new_score == 0. {
                // check if both terrible
                prev_score == new_score
            } else {
                // check if they're both good or both bad
                (prev_score < cutoff) == (new_score < cutoff)
            };

            // do not merge blocks where one side is single line and the other is multiline
            if merge && (prev.splits_to_multiline() && prev_score > 0.) || (block.splits_to_multiline() && new_score > 0.) {
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

    pub fn split_block(mut self) -> Vec<Self> {
        self.squeeze_parts();
        super::shift::shift_parts(&mut self.parts);

        let mut blocks = vec![Block{parts: vec![]}];

        // group parts based on line numbers
        for part in self.parts {
            if part.is_empty(0) && part.is_empty(1) {
                continue
            }

            let block = &blocks.last().unwrap();

            if !block.parts.is_empty()
                && block.last_non_empty(0).map(|last| last.last_lineno(0)) != Some(part.first_lineno(0))
                && block.last_non_empty(1).map(|last| last.last_lineno(1)) != Some(part.first_lineno(1))
            {
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
                    let (mut first, second) = first.partition_from_start(prefix, prefix, false);
                    first.matches = true;
                    block.parts[0] = first;
                    block.parts.insert(1, second);
                }
            }
        }

        let mut blocks = Block::merge_blocks_on_score(blocks, Block::CUTOFF);

        for block in blocks.iter_mut() {
            super::shift::shift_parts(&mut block.parts);
            block.squeeze_parts();
        }

        // if score is too low, make the whole thing non matching
        for block in blocks.iter_mut() {
            let score = block.score();
            if 0. < score && score < Block::CUTOFF {
                let first = &block.parts[0];
                let part = Part{
                    parent: first.parent,
                    matches: false,
                    slices: [
                        first.slices[0].start .. block.parts.last().unwrap().slices[0].end,
                        first.slices[1].start .. block.parts.last().unwrap().slices[1].end,
                    ],
                };

                block.parts.clear();
                block.parts.push(part);
            }
        }

        // merge again
        let mut blocks = Block::merge_blocks_on_score(blocks, Block::CUTOFF);

        for block in blocks.iter_mut() {
            // try to do a very simple diff for low scoring blocks

            if block.parts.len() == 1 && block.score() == 0. {
                let part = &block.parts[0];

                // find common prefix
                let prefix = find_common_prefix_length(part.get(0), part.get(1));
                let (first, second) = part.partition_from_start(prefix, prefix, true);

                // find common suffix
                let suffix = if second.single_line(0) && second.single_line(1) {
                    find_common_suffix_length(second.get(0), second.get(1))
                } else {
                    0
                };
                let (mut second, third) = second.partition_from_end(suffix, suffix, true);
                second.matches = false;

                // matching common prefix/suffix looks weird when score is low and inlined
                if second.is_empty(0) || second.is_empty(1) || !second.inlineable() {
                    // try it out
                    let old_parts = std::mem::replace(&mut block.parts, vec![first, second, third]);
                    block.squeeze_parts();
                    block.parts.retain(|p| !p.is_empty(0) || !p.is_empty(1));

                    // nothing matches, go back to the way it was before
                    if block.parts.iter().all(|p| !p.matches) {
                        block.parts = old_parts;
                    }
                }

            }
        }

        // remove empty ones
        for block in blocks.iter_mut() {
            block.parts.retain(|p| !p.is_empty(0) || !p.is_empty(1));
        }

        blocks
    }

    pub fn print<
        T: Write,
        S: AsRef<str>,
        F: Fn([usize; 2], Option<&str>, Option<&str>, Option<&str>)->S
    >(
        &self,
        stdout: &mut BufWriter<T>,
        merge_markers: Option<&super::hunk::MergeMarkers>,
        style: style::Style,
        format_lineno: F,
    ) -> Result<()> {

        if self.parts.is_empty() {
            return Ok(())
        }
        let mut line_numbers = [self.parts[0].first_lineno(0), self.parts[0].first_lineno(1)];

        if !style.show_both && self.parts.iter().all(|p| p.matches || (p.is_empty(0) && p.is_empty(1))) {
            // this is entirely matching

            let mut newline = true;
            for part in self.parts.iter() {
                if !part.matches {
                    continue
                }

                let words = part.get(0);
                let last = words.len() - 1;
                for (j, word) in words.iter().enumerate() {
                    if newline {
                        if style.line_numbers {
                            stdout.write_all(format_lineno(
                                line_numbers,
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

                    let trailing_ws = words[last] == b"\n" && words[j..last].iter().all(|&w| w.is_ascii_whitespace());
                    if trailing_ws {
                        stdout.write_all(style::DIFF_TRAILING_WS)?;
                    }
                    stdout.write_all(word)?;

                    if word == b"\n" {
                        line_numbers[0] += 1;
                        line_numbers[1] += 1;
                        newline = true;
                    }
                }

            }
            return Ok(())
        }

        let score = self.score();
        let inline = style.inline && (score > Block::CUTOFF || self.parts.iter().all(|p| p.inlineable()));
        // let inline = style.inline && self.parts.iter().all(|p| p.inlineable());

        let outer_loop = if inline { 0..=0 } else { 0..=1 };
        for i in outer_loop {
            let mut newline = true;
            let mut insert = false;

            for part in self.parts.iter() {
                if !inline && part.is_empty(i) {
                    insert = score > 0.;
                    continue
                }

                let highlight = if !part.matches {
                    style.diff_non_matching
                } else if inline {
                    [style.diff_matching_inline, style.diff_matching_inline]
                } else {
                    style.diff_matching
                };

                let inner_loop = if inline && !part.matches { 0..=1 } else { i..=i };
                for i in inner_loop {
                    stdout.write_all(highlight[i])?;

                    let words = part.get(i);
                    if words.is_empty() {
                        continue
                    }

                    let last = words.len() - 1;
                    for (j, word) in words.iter().enumerate() {

                        if newline {
                            if style.line_numbers {
                                let mut lineno_args = line_numbers;

                                // draw the other line number if we are inline
                                // OR the other side has non empty parts on same line
                                let other = 1 - i;
                                if !inline || !self.parts.iter().any(|p| !p.is_empty(other) && p.first_lineno(other) <= line_numbers[other] && line_numbers[other] <= p.last_lineno(other)) {
                                    lineno_args[1-i] = 0;
                                }

                                let bar_style = merge_markers.and_then(|m| m.get(&(i, line_numbers[i])).map(|x| x.as_ref()));
                                stdout.write_all(format_lineno(
                                    lineno_args,
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

                        if word == b"\n" {
                            line_numbers[i] += 1;
                            if inline && part.matches {
                                line_numbers[1-i] += 1;
                            }
                            newline = true;
                        }

                        let trailing_ws = words[last] == b"\n" && words[j..last].iter().all(|&w| w.is_ascii_whitespace());

                        if insert {
                            // add an insertion marker
                            // write only one char
                            stdout.write_all(style::DIFF_INSERT[i])?;
                            if trailing_ws {
                                stdout.write_all(style::DIFF_TRAILING_WS)?;
                            }
                            stdout.write_all(&word[0..1])?;
                            if trailing_ws {
                                stdout.write_all(style::DIFF_TRAILING_WS)?;
                            }
                            stdout.write_all(highlight[i])?;
                            stdout.write_all(&word[1..])?;
                            insert = false;
                        } else {
                            if trailing_ws {
                                stdout.write_all(style::DIFF_TRAILING_WS)?;
                            }
                            stdout.write_all(word)?;
                        }
                    }
                }
            }

        }

        Ok(())
    }
}
