use super::block_maker::BlockMaker;
use super::tokeniser::Token;
use std::io::{BufWriter, Write};
use std::cmp::{min};
use std::borrow::Cow;
use anyhow::{Result};
use super::part::Part;
use super::style;
use super::types::Bytes;
use super::whitespace::CheckAllWhitespace;

fn find_common_prefix_length<A: IntoIterator<Item=T>, B: IntoIterator<Item=T>, T: PartialEq>(a: A, b: B) -> usize {
    a.into_iter().zip(b).take_while(|(a, b)| a == b).count()
}

fn find_common_suffix_length<A, B, T>(a: A, b: B) -> usize
where T: PartialEq,
    A: IntoIterator<Item=T>,
    B: IntoIterator<Item=T>,
    A::IntoIter: DoubleEndedIterator,
    B::IntoIter: DoubleEndedIterator,
{
    a.into_iter().rev().zip(b.into_iter().rev()).take_while(|(a, b)| a == b).count()
}


#[derive(Debug, Default, Clone)]
pub struct Block<'a> {
    pub parts: Vec<Part<'a>>,
}

impl<'a> Block<'a> {
    const CUTOFF: f64 = 0.6;
    const SIMPLE_CUTOFF: f64 = 0.5;
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
            if self.is_both_empty() {
                return 1f64
            }
            return 0.
        }

        let matches: usize = self.parts.iter().filter(|p| p.matches).map(|p| min(MAXLEN, p.word_len(0))).sum();
        2. * matches as f64 / total as f64
    }

    fn squeeze_parts(&mut self) {
        // squeeze matches that are too small

        let mut parts = Vec::<Part>::new();
        let mut join = false;

        for (i, part) in self.parts.iter().enumerate() {
            if part.matches {

                let total_length = part.slices[0].len();

                // strip newlines
                let length: usize = part.get(0)
                    .iter()
                    .skip_while(|&&w| w == b"\n")
                    .take_while(|&&w| w != b"\n")
                    .map(|w| w.len())
                    .sum();

                if part.whole_line() || (total_length == 1 && part.get(0)[0] == b"\n") {

                // elif len(parts) >= 2 and any(parts[-1].is_empty(i) and not parts[-2].is_empty(i) for i in SIDES):
                    // // this is actually next to another part
                    // pass
                // elif i+2 < len(self.parts) and any(self.parts[i+1].is_empty(i) and not self.parts[i+2].is_empty(i) for i in SIDES):
                    // // this is actually next to another part
                    // pass
                } else if
                    (part.starts_line(0) && part.starts_line(1))
                    || (part.ends_line(0) && part.ends_line(1))
                    // don't sqeeze if we start/end a line but the prev/next is empty
                    || (part.starts_line(0) && i > 0 && self.parts.get(i-1).is_none_or(|p| p.is_empty(1)))
                    || (part.starts_line(1) && i > 0 && self.parts.get(i-1).is_none_or(|p| p.is_empty(0)))
                    || (part.ends_line(0) && self.parts.get(i+1).is_none_or(|p| p.is_empty(1)))
                    || (part.ends_line(1) && self.parts.get(i+1).is_none_or(|p| p.is_empty(0)))
                    || length == 0
                {
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
            parts.retain(|p| !p.is_both_empty());
            self.parts = parts;
        }
    }

    pub fn is_empty(&self, i: usize) -> bool {
        self.parts.iter().all(|p| p.is_empty(i))
    }

    pub fn is_both_empty(&self) -> bool {
        self.is_empty(0) && self.is_empty(1)
    }

    fn splits_to_multiline(&self) -> bool {
        if self.is_empty(0) || self.is_empty(1) {
            // one side is empty
            return false;
        }
        let splits = |i| self.parts[0].first_lineno(i) == self.parts.last().unwrap().last_lineno(i);
        splits(0) != splits(1)
    }

    fn split_at(&mut self, index: [usize; 2]) -> Block<'a> {
        let mut rhs = Block::default();
        self.parts.retain_mut(|part| {
            let (mut left, mut right) = part.partition(index[0], index[1], false);

            left.slices = [0, 1].map(|i| left.slices[i].start.min(index[i]) .. left.slices[i].end.min(index[i]));
            right.slices = [0, 1].map(|i| right.slices[i].start.max(index[i]) .. right.slices[i].end.max(index[i]));

            if !right.is_both_empty() {
                right.matches = right.tokens(0) == right.tokens(1);
                // if empty, make sure they are put in the right position
                if right.is_empty(0) {
                    let start = rhs.parts.last().map_or(index[0], |p| p.slices[0].end);
                    right.slices[0] = start .. start;
                } else if right.is_empty(1) {
                    let start = rhs.parts.last().map_or(index[1], |p| p.slices[1].end);
                    right.slices[1] = start .. start;
                }
                rhs.parts.push(right);
            }

            if !left.is_both_empty() {
                left.matches = left.tokens(0) == left.tokens(1);
                *part = left;
                true
            } else {
                false
            }
        });
        rhs
    }

    fn separate_newlines(&mut self) {
        // split out newlines so they are not merged together with other bits
        let mut i = 0;
        while i < self.parts.len() {
            let part = &mut self.parts[i];
            if !part.matches {
                // i don't have to check both at the same time
                // if the leading newline is split first,
                // we'll come back next look to check the rest
                match (part.tokens(0), part.tokens(1)) {
                    ([Token::NEWLINE, ..], [Token::NEWLINE, ..]) => {
                        let (mut nl, rest) = part.partition_from_start(1, 1, false);
                        nl.matches = true;
                        self.parts[i] = nl;
                        self.parts.insert(i+1, rest);
                    },
                    ([.., Token::NEWLINE], [.., Token::NEWLINE]) => {
                        let (rest, mut nl) = part.partition_from_end(1, 1, false);
                        nl.matches = true;
                        self.parts[i] = rest;
                        self.parts.insert(i+1, nl);
                    },
                    _ => (),
                }
            }
            i += 1;
        }
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
                prev.parts.extend(block.parts);
            } else {
                merged.push(prev);
                prev = block;
            }
        }

        merged.push(prev);
        merged
    }

    fn merge_adjacent_parts(&mut self) {
        let mut i = 1;
        while i < self.parts.len() {
            if self.parts[i-1].matches == self.parts[i].matches {
                let part = self.parts.remove(i);
                let prev = &mut self.parts[i - 1];
                prev.slices = [
                    prev.slices[0].start .. part.slices[0].end,
                    prev.slices[1].start .. part.slices[1].end,
                ];
            } else {
                i += 1;
            }
        }
    }

    fn merge_adjacent_blocks(blocks: Vec<Block>) -> Vec<Block> {

        // dirty way to check if the parts are all matching or all non matching
        // matches get 2, nonmatches get 1
        // when the bitwise OR them together,
        // if they all match the value is 2
        // if they all don't match the value is 1
        // if some match and some don't the value 3
        let match_value = |block: &Block| {
            block.parts.iter()
                .filter(|p| !(p.matches && p.tokens(0) == [Token::NEWLINE]))
                .map(|p| if p.matches { 2 } else { 1 })
                .reduce(|total, x| total | x)
        };

        let mut newblocks: Vec<Block> = vec![];
        for block in blocks {
            if !block.is_both_empty() {

                if let Some(prev) = newblocks.last_mut()
                    && let Some(value) = match_value(&block)
                    && let Some(prev_value) = match_value(&prev)
                    && value == prev_value
                    && value < 3
                {
                    prev.parts.extend(block.parts);
                } else {
                    newblocks.push(block);
                }
            }
        }
        newblocks
    }

    fn partition_blocks_on_score(blocks: Vec<Block>, cutoff: f64) -> Vec<Block> {
        // if score is too low, make the whole thing non matching

        let mut new = vec![];

        for mut block in blocks {
            // if this block only additions or only removals, then don't worry about the score
            let mut nonmatches = block.parts.iter().filter(|p| !p.matches);
            let score = block.score();
            if nonmatches.clone().any(|p| !p.is_empty(0))
                && nonmatches.any(|p| !p.is_empty(1))
                && 0. < score && score < cutoff
            {
                // low score

                // try to make new blocks with the best matching parts
                while let Some(best) = block.parts.iter().max_by_key(|p| (p.matches && !p.is_ascii_whitespace(0), p.word_len(0))) {
                    let parent = best.parent;

                    let starts = [0, 1].map(|i| best.parent.get_wordno(i, best.first_lineno(i)).max(block.parts[0].slices[i].start) );
                    let ends = [0, 1].map(|i| {
                        if best.slices[i].end == best.parent.words[i].len() {
                            usize::MAX
                        } else {
                            best.parent.get_wordno(i, best.last_lineno(i) + 1)
                        }.min(block.parts.last().unwrap().slices[i].end)
                    });

                    let mut newblock = block.split_at(starts);
                    let mut rest = newblock.split_at(ends);
                    block.parts.append(&mut rest.parts);

                    let score = newblock.score();
                    if 0. < score && score < cutoff {
                        // bad block, add the whole thing
                        let part = parent.make_part(false, starts[0]..ends[0], starts[1]..ends[1]);
                        newblock.parts.clear();
                        newblock.parts.push(part);
                        if score > Self::SIMPLE_CUTOFF {
                            // try to do a very simple diff for low scoring blocks
                            let rest = newblock.simple_match_common(false);
                            new.push(newblock);
                            new.extend(rest);
                        } else {
                            new.push(newblock);
                        }
                    } else {
                        newblock.merge_adjacent_parts();
                        newblock.separate_newlines();
                        if newblock.parts[0].matches && newblock.parts[0].is_space(0) {
                            newblock.parts[0].matches = false;
                        }

                        new.append(&mut newblock.split_block());
                    }
                }

            } else {
                if score == 0. && block.parts.len() > 1 {
                    // make it all non matching
                    // since there may be some matching whitespace we skipped over
                    for p in &mut block.parts {
                        if p.matches && !p.is_both_empty() && p.tokens(0) != &[Token::NEWLINE] {
                            p.matches = false;
                        }
                    }
                }
                new.push(block);
            }

        }

        new.sort_by_key(|block| (block.parts[0].slices[0].start, block.parts[0].slices[1].start));
        new
    }

    fn simple_match_common(&mut self, allow_space_only: bool) -> Option<Block<'a>> {
        // try to do a very simple diff for low scoring blocks

        if self.parts.is_empty() {
            return None;
        }

        // keep if one of these only 1 line
        let one_line = [0, 1].map(|i| self.parts[0].first_lineno(i) == self.parts.last().unwrap().last_lineno(i));
        if !one_line[0] && !one_line[1] {
            return None;
        }

        let mut block = None;

        // prefix
        if let Some(parti) = self.parts.iter().position(|p| !p.matches && !p.is_both_empty()) {
            let part = &self.parts[parti];

            // match indentation first
            let leading_space = [0, 1].map(|x| part.tokens(x).iter().take_while(|&&t| t == Token::SPACE).count());
            let (mut indent, rest) = part.partition_from_start(
                leading_space[0].saturating_sub(leading_space[1]),
                leading_space[1].saturating_sub(leading_space[0]),
                false,
            );

            // find common prefix
            let prefix = find_common_prefix_length(rest.tokens(0), rest.tokens(1));
            let (mut first, mut second) = rest.partition_from_start(prefix, prefix, false);
            first.matches = true;

            if allow_space_only || !first.is_ascii_whitespace(0) {
                // we only matched spaces
                if first.is_space(0) {
                    for i in [0, 1] {
                        let shift = indent.slices[i].len();
                        first.slices[i].start -= shift;
                        first.slices[i].end -= shift;
                        second.slices[i].start -= shift;
                        // indent will get shrunk to nothing
                        indent.slices[i].end -= shift;
                        debug_assert_eq!(indent.slices[i].start, indent.slices[i].end);
                    }
                }

                // if !second.inlineable() {
                    // try it out
                    let mut newblock = self.clone();
                    newblock.parts.splice(parti..=parti, [indent.clone(), first, second]);
                    newblock.squeeze_parts();
                    newblock.parts.retain(|p| !p.is_both_empty());

                    if newblock.parts.iter().filter(|p| p.matches).count() > self.parts.iter().filter(|p| p.matches).count() {
                        *self = newblock;

                        if parti == 0 && !one_line[0] && one_line[1] {
                            // we've matched a prefix but the lhs is multiline
                            // this is going to look weird
                            // so split the first line out
                            let split = [0, 1].map(|i| indent.parent.get_line_range(i, indent.first_lineno(i)).end);
                            block = Some(self.split_at([split[0], split[1]]));
                            self.separate_newlines();
                        }

                    }
                // }
            }
        }

        {
            let block = block.as_mut().unwrap_or(self);

            // suffix
            if let Some(parti) = block.parts.iter().rposition(|p| !p.matches && !p.is_both_empty()) {
                let part = &block.parts[parti];

                if part.single_line(0) && part.single_line(1) {

                    // find common suffix
                    let suffix = find_common_suffix_length(part.tokens(0), part.tokens(1));
                    let (first, mut second) = part.partition_from_end(suffix, suffix, false);
                    second.matches = true;

                    if !second.is_ascii_whitespace(0) {
                        if !first.inlineable() {
                            // try it out
                            let mut newblock = block.clone();
                            newblock.parts.splice(parti..=parti, [first, second]);
                            newblock.squeeze_parts();
                            newblock.parts.retain(|p| !p.is_both_empty());

                            if newblock.parts.iter().filter(|p| p.matches).count() > block.parts.iter().filter(|p| p.matches).count() {
                                *block = newblock;
                            }
                        }
                    }
                }
            }
        }

        block
    }

    fn rearrange_blocks(blocks: &mut [Self], forward: bool) {
        // rearrange blocks so that additions are with other additions

        let move_side = if forward { 0 } else { 1 };
        let block_len = blocks.len();

        let mut loop_fn = |i: usize, adji: usize| {
            let block = &blocks[i];

            if let Some(part) = &block.parts.last()
                && !part.matches
                && !part.is_empty(0)
                && !part.is_empty(1)
                // only one part, ignoring matching leading whitespace
                && (block.parts.len() == 1 || (block.parts.len() == 2 && block.parts[0].matches && block.parts[0].is_space(0)))
            {
                let adj_block = &blocks[adji];
                let changes: [usize; 2] = [0, 1].map(|side|
                    adj_block.parts.iter().filter(|p| !p.matches).map(|p| p.slices[side].len()).sum()
                );
                if changes[move_side] > changes[1 - move_side].pow(2) {

                    let prev = if forward {
                        &adj_block.parts[0]
                    } else {
                        adj_block.parts.last().unwrap()
                    };

                    // need to do this instead of part.slices in case there was a leading whitespace part
                    let slices = [0, 1].map(|side| {
                        block.parts[0].slices[side].start .. part.slices[side].end
                    });

                    // split into del and add part
                    let parts = [
                        part.parent.make_part(false, slices[0].clone(), prev.slices[1].end .. prev.slices[1].end),
                        part.parent.make_part(false, prev.slices[0].end .. prev.slices[0].end, slices[1].clone()),
                    ];

                    // keep this side
                    blocks[i].parts.splice(.., [parts[1 - move_side].clone()]);
                    // move this side into the adjacent block
                    if forward {
                        blocks[adji].parts.insert(0, parts[move_side].clone());
                    } else {
                        blocks[adji].parts.push(parts[move_side].clone());
                    }

                }
            }
        };

        if forward {
            (1 .. block_len).rev().for_each(|i| loop_fn(i-1, i));
        } else {
            (1 .. block_len).for_each(|i| loop_fn(i, i-1));
        }

    }

    fn last_non_empty(&self, i: usize) -> Option<&Part<'_>> {
        self.parts.iter().rev().find(|p| !p.is_empty(i))
    }

    fn last_lineno(&self, i: usize) -> Option<usize> {
        self.last_non_empty(i).map(|last| last.last_lineno(i))
    }

    pub fn set_block_maker(&mut self, parent: &'a BlockMaker) {
        for part in &mut self.parts {
            part.parent = parent;
        }
    }

    pub fn split_in_middle_of_word(
        &mut self,
        mut parent: Cow<'a, BlockMaker<'a>>,
        tokeniser: &mut crate::tokeniser::Tokeniser,
        mut shift: isize,
    ) -> (Cow<'a, BlockMaker<'a>>, isize) {

        let mut i = 0;
        while i < self.parts.len() {
            // realpart is the real one with the shifted slices
            let realpart = &mut self.parts[i];
            // but part is what we use for actually getting words
            // since it is still using the old slices and old parent
            let part = realpart.clone();
            realpart.slices = realpart.shift_slice(shift, shift);

            if !part.matches && !part.is_empty(0) && !part.is_empty(1) {
                let chars = [0, 1].map(|x| part.get(x).iter().flat_map(|x| x.iter()));
                let len = [0, 1].map(|x| chars[x].clone().count());

                let prefix = find_common_prefix_length(chars[0].clone(), chars[1].clone());
                let suffix = find_common_suffix_length(chars[0].clone(), chars[1].clone());
                let suffix = suffix.min(len[0] - prefix).min(len[1] - prefix);

                let matches_word = [0, 1].map(|x| suffix == part.get(x).last().unwrap().len());
                let match_side = if matches_word[0] { 0 } else { 1 };
                if suffix > 0
                    && (matches_word[0] != matches_word[1])
                    && part.get(match_side).len() == 1
                {
                    let mut after = realpart.clone();
                    after.slices[0].start = after.slices[0].end;
                    after.slices[1].start = after.slices[1].end;
                    after.matches = true;

                    for x in [0, 1] {
                        parent.to_mut().split_word(tokeniser, x, realpart.slices[x].end - 1, part.get(x).last().unwrap().len() - suffix);
                        after.slices[x].end += 1;
                    }
                    shift += 1;
                    self.parts.insert(i+1, after);
                    i += 1;
                }

                let realpart = &mut self.parts[i];
                let matches_word = [0, 1].map(|x| prefix == part.get(x)[0].len());
                let match_side = if matches_word[0] { 0 } else { 1 };
                if prefix > 0
                    && (matches_word[0] != matches_word[1])
                    && part.get(match_side).len() == 1
                {
                    let mut before = realpart.clone();
                    before.slices[0].end = before.slices[0].start;
                    before.slices[1].end = before.slices[1].start;
                    before.matches = true;

                    for x in [0, 1] {
                        parent.to_mut().split_word(tokeniser, x, realpart.slices[x].start, prefix);
                        realpart.slices[x].start += 1;
                        realpart.slices[x].end += 1;
                        before.slices[x].end += 1;
                    }
                    shift += 1;
                    self.parts.insert(i, before);
                    i += 1;
                }
            }
            i += 1;
        }

        self.parts.retain(|p| !p.is_both_empty());
        (parent, shift)
    }

    pub fn split_block(mut self) -> Vec<Self> {
        self.squeeze_parts();
        super::shift::shift_parts(&mut self.parts);

        let mut blocks = vec![];
        let mut block = Block::default();

        // group parts based on line numbers
        for mut part in self.parts {
            if part.is_both_empty() {
                continue
            }

            if !block.parts.is_empty() {
                let overlap = [0, 1].map(|x| block.last_lineno(x) == Some(part.first_lineno(x)));
                let newline = [0, 1].map(|x| part.tokens(x).first() == Some(&Token::NEWLINE));

                // either they don't overlap or only on a newline
                if (0..=1).all(|x| !overlap[x] || newline[x]) && (overlap[0] || overlap[1]) {
                    // move that newline back
                    let partition = [0, 1].map(|x| if newline[x] { 1 } else { 0 });
                    let (left, right) = part.partition_from_start(partition[0], partition[1], partition[0] == partition[1]);
                    block.parts.push(left);
                    part = right;
                }

                // different line
                if (0..=1).all(|x| block.last_lineno(x) != Some(part.first_lineno(x))) {
                    blocks.push(block);
                    block = Block::default();
                }
            }

            block.parts.push(part);
        }
        blocks.push(block);

        // match leading whitespace in each block
        // since it got treated as junk during the diff
        for block in &mut blocks {
            let first = &block.parts[0];
            if !first.matches {
                // find common prefix
                let prefix = find_common_prefix_length(first.tokens(0), first.tokens(1));
                if prefix != 0 {

                    let score = block.score();
                    // calculate the amount of indentation
                    let spaces = [0, 1].map(|x| {
                        block.parts.iter().flat_map(|p| p.tokens(x)).take_while(|&&t| t == Token::SPACE).count()
                    });
                    let actual_indent = spaces[1] as isize - spaces[0] as isize;

                    let (mut first, mut second) = first.partition_from_start(prefix, prefix, false);
                    // first part is matching
                    first.matches = true;
                    block.parts[0] = first;

                    let whitespace = [0, 1].map(|x| second.get(x).iter().take_while(|c| c.is_ascii_whitespace()).count());
                    if spaces[1] == 0 && whitespace[1] > 0 {
                        // probably tabs?
                        // split the whitespace out separately
                        let (ws, non_ws) = second.partition_from_start(whitespace[0], whitespace[1], false);
                        // insert an empty match to stop them getting merged back together
                        let (ws, mut empty) = ws.partition_from_end(0, 0, false);
                        empty.matches = true;
                        block.parts.splice(1..1, [ws, empty, non_ws]);
                        continue;
                    }

                    // if the score is too low dont bother trying to match indentation, it will look too messy
                    if score >= Block::CUTOFF {

                        // check changed indentation
                        let indented_side = if actual_indent > 0 { 1 } else { 0 };
                        // this is how much indentation the diff shows
                        let diff_indent = second.tokens(indented_side).iter().take_while(|&&t| t == Token::SPACE).count();
                        let mut diff_indent = diff_indent as isize * actual_indent.signum();
                        // how much is missing
                        let missing = actual_indent.abs() - diff_indent.abs();

                        if
                            // not enough indentation! can we take some from the next part?
                            missing > 0
                            // the next part starts with enough spaces
                            && let Some(next_part) = block.parts.get_mut(1)
                            && next_part.matches
                            && next_part.tokens(0).len() > missing as usize
                            && next_part.tokens(0)[..missing as usize].iter().all(|&t| t == Token::SPACE)
                        {
                            // let missing = missing as usize;
                            next_part.slices = next_part.shift_slice(missing as _, 0);
                            second.slices = second.shift_slice(0, missing as _);
                            diff_indent = actual_indent;
                        }

                        // check for indentation and split it out
                        // so it can be shifted to the start
                        // do indentation splitting only if there is more than one
                        if diff_indent.abs() > 0 {
                            let (ws, non_ws) = second.partition_from_start(0.max(-diff_indent) as usize, 0.max(diff_indent) as usize, false);
                            let (ws, mut empty) = ws.partition_from_end(0, 0, false);
                            // insert an empty match to stop them getting merged back together
                            empty.matches = true;
                            block.parts.splice(1..1, [ws, empty, non_ws]);
                            continue
                        }
                    }

                    block.parts.insert(1, second);

                }
            }
        }

        let mut blocks = Block::merge_blocks_on_score(blocks, Block::CUTOFF);

        for block in &mut blocks {
            super::shift::shift_parts(&mut block.parts);
            block.squeeze_parts();
        }

        // if score is too low, make the whole thing non matching
        let blocks = Block::partition_blocks_on_score(blocks, Block::CUTOFF);

        // merge again
        let mut blocks = Block::merge_blocks_on_score(blocks, Block::CUTOFF);
        for block in &mut blocks {
            // block.parts.retain(|p| !p.is_both_empty());
            block.merge_adjacent_parts();
        }

        let blocks = Block::merge_adjacent_blocks(blocks);

        let mut blocks: Vec<_> = blocks.into_iter().flat_map(|mut block| {
            // try to do a very simple diff for low scoring blocks
            let newblock = block.simple_match_common(true);
            [Some(block), newblock]
        }).flatten().collect();

        Block::rearrange_blocks(&mut blocks, false);
        Block::rearrange_blocks(&mut blocks, true);

        // remove empty ones
        for block in &mut blocks {
            // block.parts.retain(|p| !p.is_both_empty());
            block.merge_adjacent_parts();
        }

        blocks.retain(|b| !b.parts.is_empty());

        blocks
    }

    fn print_insert_marker<T: Write>(
        stdout: &mut BufWriter<T>,
        side: usize,
        word: Bytes,
        style: &style::Style,
        style_opts: &super::StyleOpts,
    ) -> Result<()> {

        // add an insertion marker
        let newline = word == b"\n";

        if newline {
            stdout.write_all(style::RESET)?;
        }
        stdout.write_all([&style_opts.diff_insert_left, &style_opts.diff_insert_right][side].as_bytes())?;
        if newline && style.newline_insert_markers {
            // need at least a space to draw the insert marker
            stdout.write_all(b" ")?;
            stdout.write_all(style::RESET)?;
        }
        // write only one char
        stdout.write_all(&word[0..1])?;
        Ok(())
    }

    pub fn print<
        T: Write,
        S: AsRef<str>,
        F: Fn([usize; 2], Option<usize>, Option<&str>, Option<&str>, Option<&str>)->S
    >(
        &self,
        stdout: &mut BufWriter<T>,
        merge_markers: Option<&super::hunk::MergeMarkers>,
        style: style::Style,
        style_opts: &super::StyleOpts,
        format_lineno: F,
    ) -> Result<()> {

        if self.parts.is_empty() {
            return Ok(())
        }
        let mut line_numbers = [self.parts[0].first_lineno(0), self.parts[0].first_lineno(1)];
        let max_lineno = self.parts.iter().flat_map(|p| [p.last_lineno(0), p.last_lineno(0)]).max().unwrap_or(0);
        let max_lineno_width = max_lineno.checked_ilog10().unwrap_or(0) as usize + 1;

        if !style.show_both && self.parts.iter().all(|p| p.matches || p.is_both_empty()) {
            // this is entirely matching

            let mut newline = true;
            for part in &self.parts {
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
                                Some(max_lineno_width),
                                Some(&style_opts.lineno), Some(&style_opts.lineno),
                                Some(&style_opts.lineno_bar),
                            ).as_ref().as_bytes())?;
                        }
                        if style.signs {
                            stdout.write_all(style::SIGN[2])?;
                        }
                        stdout.write_all(style_opts.diff_context.as_bytes())?;
                        newline = false;
                    }

                    let trailing_ws = words[last] == b"\n" && words[j..last].iter().all(|&w| w.is_ascii_whitespace());
                    if trailing_ws {
                        stdout.write_all(style_opts.diff_trailing_ws.as_bytes())?;
                    }
                    if *word == b"\n" {
                        stdout.write_all(style::RESET)?;
                    }
                    stdout.write_all(word)?;

                    if *word == b"\n" {
                        line_numbers[0] += 1;
                        line_numbers[1] += 1;
                        newline = true;
                    }
                }

            }

            if !newline {
                stdout.write_all(b"\n")?;
            }

            return Ok(())
        }

        let score = self.score();
        let inline = style.inline && (
            score > Block::CUTOFF || (
                self.parts.iter().all(|p| p.inlineable())
                // there must be some non whitespace matching part
                && self.parts.iter().any(|p| p.matches && (!p.is_ascii_whitespace(0) || !p.is_ascii_whitespace(1)))
            )
        );

        let outer_loop = if inline { 0..=0 } else { 0..=1 };
        for i in outer_loop {
            let mut newline = true;
            let mut insert = false;

            for part in &self.parts {
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

                let inner_loop: &[usize] = if !inline || part.matches {
                    &[i]
                // swap the order if it looks better that way
                } else if
                    !part.get(0).is_empty()
                    && !part.get(1).is_empty()
                    && !part.get(0)[0].is_ascii_whitespace()
                    && part.get(1)[0].is_ascii_whitespace()
                    && !part.tokens(1).contains(&Token::NEWLINE)
                {
                    &[1, 0]
                } else {
                    &[0, 1]
                };

                for &i in inner_loop {

                    let words = part.get(i);
                    if words.is_empty() {
                        stdout.write_all(style::RESET)?;
                        continue
                    }

                    // draw the other line number if we are inline
                    // AND the other side has non empty parts on same line
                    let other = 1 - i;
                    let other_is_empty = !self.parts.iter().any(|p| !p.is_empty(other) && p.first_lineno(other) <= line_numbers[other] && line_numbers[other] <= p.last_lineno(other));

                    stdout.write_all(highlight[i])?;
                    let last = words.len() - 1;
                    for (j, word) in words.iter().enumerate() {

                        if newline {
                            if style.line_numbers {
                                let mut lineno_args = line_numbers;

                                // draw the other line number if we are inline
                                // OR the other side has non empty parts on same line
                                if !inline || other_is_empty {
                                    lineno_args[other] = 0;
                                }

                                let bar_style = merge_markers.and_then(|m| m.get(&(i, line_numbers[i])).map(|x| x.as_ref())).or(Some(&*style_opts.lineno_bar));
                                stdout.write_all(format_lineno(
                                    lineno_args,
                                    Some(max_lineno_width),
                                    Some(&style_opts.lineno_left), Some(&style_opts.lineno_right),
                                    bar_style,
                                ).as_ref().as_bytes())?;
                            }
                            if style.signs {
                                stdout.write_all(style::SIGN[i])?;
                            }
                            stdout.write_all(highlight[i])?;

                            newline = false;
                        }

                        if *word == b"\n" {
                            line_numbers[i] += 1;
                            if inline && part.matches {
                                line_numbers[other] += 1;
                            }
                            newline = true;
                        }

                        let trailing_ws = words[last] == b"\n" && words[j..last].iter().all(|&w| w.is_ascii_whitespace());

                        if insert {
                            // add an insertion marker
                            if trailing_ws {
                                stdout.write_all(style_opts.diff_trailing_ws.as_bytes())?;
                            }
                            Self::print_insert_marker(stdout, i, word, &style, style_opts)?;
                            stdout.write_all(style::RESET)?;
                            if trailing_ws {
                                stdout.write_all(style_opts.diff_trailing_ws.as_bytes())?;
                            }
                            stdout.write_all(highlight[i])?;
                            // write the rest of the word
                            stdout.write_all(&word[1..])?;
                            insert = false;
                        } else {
                            if trailing_ws {
                                stdout.write_all(style_opts.diff_trailing_ws.as_bytes())?;
                            }
                            if *word == b"\n" {
                                if !part.matches && inline && !other_is_empty {
                                    stdout.write_all(style::RESET)?;
                                    stdout.write_all([&style_opts.diff_newline_left, &style_opts.diff_newline_right][i].as_bytes())?;
                                }
                                stdout.write_all(style::RESET)?;
                            }
                            stdout.write_all(word)?;
                        }
                    }
                }
            }

            if !newline {
                if insert {
                    Self::print_insert_marker(stdout, i, b"\n".into(), &style, style_opts)?;
                } else {
                    stdout.write_all(b"\n")?;
                }
            }

        }

        Ok(())
    }
}
