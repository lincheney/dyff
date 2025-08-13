use std::collections::HashMap;
use std::cmp::min;
use super::block_maker::BlockMaker;
use super::part::Part;
use super::tokeniser::Token;

fn isjunk(tok: Token) -> bool {
    tok.is_ascii_whitespace()
}

pub struct WordDiffer<'a> {
    parent: &'a BlockMaker<'a>,

    b2j: Vec<Vec<usize>>,

    matched_lines: HashMap<(usize, usize), usize>,
}

#[derive(Clone, Copy)]
struct DiffMatch {
    left: usize,
    right: usize,
    length: usize,
    lineno_dist: usize,
    lineno_dist_strong: bool,
    non_ws_length: usize,
    char_length: usize,
}

impl DiffMatch {
    fn sort_key(&self) -> (usize, isize) {
        (self.lineno_dist, -(self.char_length as isize))
    }
}

impl<'a> WordDiffer<'a> {
    pub fn new(parent: &'a BlockMaker<'a>) -> Self {
        let mut b2j = vec![];
        b2j.resize_with(parent.tokeniser.max_token().0, Vec::new);
        let matched_lines = HashMap::new();

        let mut line_start = true;
        for (i, &tok) in parent.tokens[1].iter().enumerate() {
            // whitespace at start is 'junk' as it is usually just indentation
            if !(line_start && tok.is_ascii_whitespace()) {
                b2j[tok.0].push(i);
                line_start = tok == Token::NEWLINE
            }
        }

        Self{
            parent,
            b2j,
            matched_lines,
        }
    }

    fn extend_match(
        &self,
        (mut i, mut j, mut k): (usize, usize, usize),
        alo: usize,
        ahi: usize,
        blo: usize,
        bhi: usize,
    ) -> (usize, usize, usize) {
        let left = &self.parent.tokens[0];
        let right = &self.parent.tokens[1];

        // match leading whitespace, up to start of line
        while
               i > alo
            && j > blo
            && left[i-1] == right[j-1]
            && (isjunk(left[i-1]) || (
                   left[i-1] == Token::NEWLINE
                && i >= 2
                && left[i-2] == Token::NEWLINE
                && j >= 2
                && right[j-2] == Token::NEWLINE
            ))
        {
            i -= 1;
            j -= 1;
            k += 1;
        }

        // match trailing whitespace, up to end of line
        while
               i+k < ahi
            && j+k < bhi
            && left[i+k] == right[j+k]
            && (
                left[i+k] == Token::NEWLINE
                || (
                       left[i+k-1] != Token::NEWLINE
                    && isjunk(left[i+k])
                )
            )
        {
            k += 1;
        }

        (i, j, k)
    }

    fn handle_multiple_matches(
        &mut self,
        matches: &mut Vec<DiffMatch>,
        alo: usize,
        ahi: usize,
        blo: usize,
        bhi: usize,
        write: bool,
    ) -> Option<DiffMatch> {

        let left = &self.parent.words[0];

        let mut left_map = HashMap::new();
        let mut right_map = HashMap::new();
        for m in matches.iter() {
            *left_map.entry(m.left).or_insert(0) += 1;
            *right_map.entry(m.right).or_insert(0) += 1;
        }

        // use any match that doesn't overlap with each other
        let non_overlapping: Vec<_> = matches
            .iter()
            .filter(|m| *left_map.get(&m.left).unwrap() == 1 && *right_map.get(&m.right).unwrap() == 1)
            .copied()
            .collect();
        if !non_overlapping.is_empty() {
            *matches = non_overlapping;
            return None;
        }

        // use any with exact lineno match
        let exact_lineno: Vec<_> = matches
            .iter()
            .filter(|m| m.lineno_dist_strong && m.lineno_dist == 0)
            .copied()
            .collect();
        if !exact_lineno.is_empty() {
            *matches = exact_lineno;
            return None;
        }

        let mini = matches.iter().map(|x| x.left).min().unwrap();
        let maxi = matches.iter().map(|x| x.left + x.length).max().unwrap();
        let minj = matches.iter().map(|x| x.right).min().unwrap();
        let maxj = matches.iter().map(|x| x.right + x.length).max().unwrap();

        // if the left/right has only a single match
        // exclude that part and re-search over all of the other side
        let left_single = left_map.len() == 1;
        let right_single = right_map.len() == 1;
        if left_single || right_single {

            let pivot = matches[0].left;
            let (mini, maxi) = if left_single { (pivot, pivot) } else { (mini, maxi) };
            let pivot = matches[0].right;
            let (minj, maxj) = if left_single { (pivot, pivot) } else { (minj, maxj) };

            let left_iter = std::iter::once(alo).chain(matches.iter().map(|m| m.left+m.length));
            let left_iter = left_iter.zip(matches.iter().map(|m| m.left).chain(std::iter::once(ahi)));

            let mut new_matches = vec![];
            for (alo, ahi) in left_iter {
                if alo >= ahi {
                    continue
                }

                let right_iter = std::iter::once(blo).chain(matches.iter().map(|m| m.right+m.length));
                let right_iter = right_iter.zip(matches.iter().map(|m| m.right).chain(std::iter::once(bhi)));

                for (blo, bhi) in right_iter {
                    if blo >= bhi {
                        continue
                    }

                    if let Some(m) = self.find_longest_match(alo, ahi, blo, bhi, false)
                    && ((m.left < maxi && m.right < maxj) || (m.left > mini && m.right > minj))
                    // we didn't just match a newline
                    && !(m.length == 1 && left[m.left] == b"\n") {
                        new_matches.push(m);
                    }
                }
            }

            if !new_matches.is_empty() {
                let best_non_ws = new_matches.iter().map(|m| m.non_ws_length).max().unwrap();
                new_matches.retain(|m| m.non_ws_length == best_non_ws);
                *matches = new_matches;
                return None;
            }

        }

        // otherwise check in any non-overlapping region
        for (alo, ahi, blo, bhi) in [(alo, mini, blo, minj), (maxi, ahi, maxj, bhi)] {
            if alo >= ahi || blo >= bhi {
                continue
            }
            if let Some(m) = self.find_longest_match(alo, mini, blo, minj, write) {
                // we didn't just match a newline
                if !(m.length == 1 && left[m.left] == b"\n") {
                    return Some(m)
                }
            }
        }

        None
    }

    fn find_longest_match(
        &mut self,
        alo: usize,
        ahi: usize,
        blo: usize,
        bhi: usize,
        write: bool,
    ) -> Option<DiffMatch> {

        let left = &self.parent.tokens[0];
        let right = &self.parent.tokens[1];
        let left_words = &self.parent.words[0];
        // let right_words = &self.parent.words[1];

        let mut best_non_ws = 0;

        let mut matches = vec![];

        let first_line_a = self.parent.get_lineno(0, alo);
        let first_line_b = self.parent.get_lineno(1, blo);
        // let last_line_a = self.parent.get_lineno(0, ahi-1);
        // let last_line_b = self.parent.get_lineno(1, bhi-1);

        // let line_diff = first_line_a as isize - first_line_b as isize;
        // let single_line_a = first_line_a == last_line_a;
        // let single_line_b = first_line_b == last_line_b;

        let mut j2len = vec![0; self.parent.words[1].len()];
        let mut newj2len = vec![0; self.parent.words[1].len()];

        for (i, &tok) in left[alo..ahi].iter().enumerate() {
            // look at all instances of a[i] in b; note that because
            // b2j has no junk keys, the loop is skipped if a[i] is junk
            newj2len.fill(0);
            let lineno_a = self.parent.get_lineno(0, i);
            let expected_lineno_b = self.matched_lines.get(&(0, lineno_a));

            let j = &self.b2j[tok.0];
            let junk = isjunk(tok);

            for &j in j.iter().skip_while(|&&j| j < blo).take_while(|&&j| j < bhi) {
                // a[i] matches b[j]
                let k = if j == 0 { 1 } else { j2len[j-1] + 1};

                // do not allow matches to start with a newline
                if tok != Token::NEWLINE {
                    newj2len[j] = k;
                }
                // don't match whitespace (but allow matching beyond it later)
                if junk {
                    continue
                }

                let i = i + 1 - k;
                let j = j + 1 - k;
                let leading_ws = right[j..j+k].iter().take_while(|m| isjunk(**m)).count();
                let trailing_ws = right[j..j+k].iter().rev().take_while(|m| isjunk(**m)).count();
                let trailing_ws = min(k - leading_ws, trailing_ws);
                let non_ws_length = k - leading_ws - trailing_ws;

                // prioritise more words, then longer words, then words on the expected line
                let cmp = non_ws_length.cmp(&best_non_ws);
                if cmp.is_lt() {
                    continue
                }

                // aggregate based on num non whitespace words
                if cmp.is_gt() {
                    matches.clear();
                }

                let lineno_b = self.parent.get_lineno(1, j);
                // compare the expected line b or a depending on which one has previously been matched
                let (lineno_dist, lineno_dist_strong) = if let Some(expected_lineno_b) = expected_lineno_b {
                    (expected_lineno_b.abs_diff(lineno_b), true)
                } else if let Some(expected_lineno_a) = self.matched_lines.get(&(1, lineno_b)) {
                    (expected_lineno_a.abs_diff(lineno_a), true)
                } else {
                    (lineno_a.abs_diff(lineno_b + first_line_a - first_line_b), false)
                };

                best_non_ws = non_ws_length;
                matches.push(DiffMatch{
                    // parent: self,
                    left: i,
                    right: j,
                    length: k,
                    lineno_dist,
                    lineno_dist_strong,
                    non_ws_length,
                    char_length: left_words[i+leading_ws .. i+k-trailing_ws].iter().map(|w| w.len()).sum(),
                });
            }

            std::mem::swap(&mut j2len, &mut newj2len);
        }

        if matches.is_empty() {
            return None
        }

        // more than one "best" match
        // try find matches elsewhere first
        // they may populate self.matched_lines which helps us narrow down which is better
        if matches.len() > 1 {
            let m = self.handle_multiple_matches(&mut matches, alo, ahi, blo, bhi, write);
            if m.is_some() {
                return m
            }
        }

        let best = matches.into_iter().min_by_key(|m| m.sort_key()).unwrap();
        if write {
            let left_line = self.parent.get_lineno(0, best.left);
            let right_line = self.parent.get_lineno(1, best.right);
            self.matched_lines.entry((0, left_line)).or_insert(right_line);
            self.matched_lines.entry((1, right_line)).or_insert(left_line);
        }
        Some(best)
    }

    pub fn get_matching_blocks(&mut self, alo: usize, ahi: usize, blo: usize, bhi: usize) -> Vec<Part<'a>> {
        // self.matched_lines.clear();
        // let mut queue = vec![(0, self.parent.words[0].len(), 0, self.parent.words[1].len())];
        let mut queue = vec![(alo, ahi, blo, bhi)];

        let mut matching_blocks = vec![];
        while let Some((alo, ahi, blo, bhi)) = queue.pop() {
            if let Some(m) = self.find_longest_match(alo, ahi, blo, bhi, true) {
                let (i, j, k) = self.extend_match((m.left, m.right, m.length), alo, ahi, blo, bhi);
                // a[alo:i] vs b[blo:j] unknown
                // a[i:i+k] same as b[j:j+k]
                // a[i+k:ahi] vs b[j+k:bhi] unknown

                matching_blocks.push((i, j, k));
                if alo < i && blo < j {
                    queue.push((alo, i, blo, j));
                }
                if i+k < ahi && j+k < bhi {
                    queue.push((i+k, ahi, j+k, bhi));
                }
            }
        }
        matching_blocks.sort_by_key(|a| a.0);

        let mut parts = vec![];
        for i in 0..matching_blocks.len() {
            if matching_blocks[i].2 == 0 {
                continue
            }

            let prev = if i > 0 { matching_blocks[i-1] } else { (alo, blo, 0) };
            let next = matching_blocks.get(i+1).copied().unwrap_or((ahi, bhi, 0));

            let mut block = self.extend_match(
                matching_blocks[i],
                prev.0 + prev.2, next.0,
                prev.1 + prev.2, next.1,
            );

            if let Some(next) = matching_blocks.get_mut(i+1)
            && block.0 + block.2 == next.0 && block.1 + block.2 == next.1 {
                block.2 += next.2;
                next.0 += next.2;
                next.1 += next.2;
                next.2 = 0;
            }

            parts.push(self.parent.make_part(true, block.0..block.0+block.2, block.1..block.1+block.2));
        }

        parts
    }
}
