use std::collections::HashMap;
use std::cmp::{min, max};
use super::block_maker::BlockMaker;
use super::part::Part;

fn is_whitespace(b: &[u8]) -> bool {
    b.iter().all(|c| c.is_ascii_whitespace())
}

fn isjunk(b: &[u8]) -> bool {
    b != b"\n" && is_whitespace(b)
}

pub struct WordDiffer<'a> {
    parent: &'a BlockMaker<'a>,

    b2j: HashMap<&'a [u8], Vec<usize>>,

    matched_lines: HashMap<(usize, usize), usize>,
}

impl<'a> WordDiffer<'a> {
    pub fn new(parent: &'a BlockMaker<'a>) -> Self {
        let mut b2j = HashMap::new();
        let matched_lines = HashMap::new();

        let mut line_start = true;
        for (i, word) in parent.words[1].iter().enumerate() {
            let word = word.as_bytes();
            // whitespace at start is 'junk' as it is usually just indentation
            if !(line_start && word != b"\n" && is_whitespace(word)) {
                b2j.entry(word).or_insert_with(Vec::new).push(i);
                line_start = word == b"\n"
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
        let left = &self.parent.words[0];
        let right = &self.parent.words[1];

        // match leading whitespace, up to start of line
        while
               i > alo
            && j > blo
            && left[i-1].as_bytes() == right[j-1].as_bytes()
            && (isjunk(left[i-1].as_bytes()) || (
                   left[i-1].as_bytes() == b"\n"
                && i >= 2
                && left[i-2].as_bytes() == b"\n"
                && j >= 2
                && right[j-2].as_bytes() == b"\n"
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
            && left[i+k].as_bytes() == right[j+k].as_bytes()
            && (
                left[i+k].as_bytes() == b"\n"
                || (
                       left[i+k-1].as_bytes() != b"\n"
                    && isjunk(left[i+k].as_bytes())
                )
            )
        {
            k += 1;
        }

        (i, j, k)
    }

    fn find_longest_match(
        &mut self,
        alo: usize,
        ahi: usize,
        blo: usize,
        bhi: usize,
    ) -> Option<(usize, usize, usize)> {

        let left = &self.parent.words[0];
        let right = &self.parent.words[1];

        let mut besti = alo;
        let mut bestj = blo;
        let mut bestsize = 0;
        let mut best_non_ws = 0;
        let mut bestlen = 0;
        let mut bestline = usize::MAX;

        let mut mini = ahi;
        let mut minj = bhi;
        let mut maxi = 0;
        let mut maxj = 0;

        let first_line_a = self.parent.get_lineno(0, alo);
        let first_line_b = self.parent.get_lineno(1, blo);
        // let last_line_a = self.parent.get_lineno(0, ahi-1);
        // let last_line_b = self.parent.get_lineno(1, bhi-1);

        // let line_diff = first_line_a as isize - first_line_b as isize;
        // let single_line_a = first_line_a == last_line_a;
        // let single_line_b = first_line_b == last_line_b;

        let mut j2len = HashMap::<usize, usize>::new();
        let mut newj2len = HashMap::<usize, usize>::new();

        for i in alo..ahi {
            // look at all instances of a[i] in b; note that because
            // b2j has no junk keys, the loop is skipped if a[i] is junk
            newj2len.clear();
            let word = left[i].as_bytes();
            let lineno_a = self.parent.get_lineno(0, i);
            let expected_lineno_b = self.matched_lines.get(&(0, lineno_a));

            if let Some(j) = self.b2j.get(word) {
                for j in j.iter() {
                    let j = *j;
                    // a[i] matches b[j]
                    if j < blo {
                        continue
                    }
                    if j >= bhi {
                        break
                    }
                    let k = if j == 0 { 1 } else { j2len.get(&(j-1)).unwrap_or(&0) + 1 };

                    // do not allow matches to start with a newline
                    if word != b"\n" {
                        newj2len.insert(j, k);
                    }

                    // don't match whitespace (but allow matching beyond it later)
                    if isjunk(word) {
                        continue
                    }

                    let i = i + 1 - k;
                    let j = j + 1 - k;
                    let leading_ws = right[j..j+k].iter().take_while(|m| is_whitespace(m.as_bytes())).count();
                    let trailing_ws = right[j..j+k].iter().rev().take_while(|m| is_whitespace(m.as_bytes())).count();
                    let trailing_ws = min(k - leading_ws, trailing_ws);
                    let non_ws = k - leading_ws - trailing_ws;

                    // prioritise more words, then longer words, then words on the expected line
                    let mut cmp = non_ws.cmp(&best_non_ws);
                    if cmp.is_lt() {
                        continue
                    }

                    // aggregate based on num non whitespace words
                    if cmp.is_gt() {
                        mini = ahi;
                        minj = bhi;
                        maxi = 0;
                        maxj = 0;
                    }

                    mini = min(mini, i);
                    minj = min(minj, j);
                    maxi = max(maxi, i+k);
                    maxj = max(maxj, j+k);

                    let l: usize = left[i+leading_ws .. i+k-trailing_ws].iter().map(|w| w.len()).sum();
                    cmp = cmp.then(l.cmp(&bestlen));
                    if cmp.is_lt() {
                        continue
                    }

                    let lineno_b = self.parent.get_lineno(1, j);
                    // compare the expected line b or a depending on which one has previously been matched
                    let lineno_dist = if let Some(expected_lineno_b) = expected_lineno_b {
                        expected_lineno_b.abs_diff(lineno_b)
                    } else if let Some(expected_lineno_a) = self.matched_lines.get(&(1, lineno_b)) {
                        expected_lineno_a.abs_diff(lineno_a)
                    } else {
                        lineno_a.abs_diff(lineno_b + first_line_a - first_line_b)
                    };

                    cmp = cmp.then(bestline.cmp(&lineno_dist));
                    if cmp.is_lt() {
                        continue
                    }

                    besti = i;
                    bestj = j;
                    bestsize = k;
                    best_non_ws = non_ws;
                    bestlen = l;
                    bestline = lineno_dist;
                }
            }

            std::mem::swap(&mut j2len, &mut newj2len);
        }

        if bestsize == 0 {
            return None
        }

        // more than one "best" match
        // try find matches elsewhere first
        // they may populate self.matched_lines which helps us narrow down which is better
        if maxi != mini + bestsize {
            // this means there's multiple solutions
            if alo < mini && blo < minj {
                if let Some(m) = self.find_longest_match(alo, mini, blo, minj) {
                    // we didn't just match a newline
                    if !(m.2 == 1 && left[m.0].as_bytes() == b"\n") {
                        return Some(m)
                    }
                }
            }
            if maxi < ahi && maxj < bhi {
                if let Some(m) = self.find_longest_match(maxi, ahi, maxj, bhi) {
                    if !(m.2 == 1 && left[m.0].as_bytes() == b"\n") {
                        return Some(m)
                    }
                }
            }
        }

        let (besti, bestj, bestsize) = self.extend_match((besti, bestj, bestsize), alo, ahi, blo, bhi);

        let left_line = self.parent.get_lineno(0, besti);
        let right_line = self.parent.get_lineno(1, bestj);
        self.matched_lines.entry((0, left_line)).or_insert(right_line);
        self.matched_lines.entry((1, right_line)).or_insert(left_line);

        Some((besti, bestj, bestsize))
    }

    pub fn get_matching_blocks(&mut self, alo: usize, ahi: usize, blo: usize, bhi: usize) -> Vec<Part<'a>> {
        // self.matched_lines.clear();
        // let mut queue = vec![(0, self.parent.words[0].len(), 0, self.parent.words[1].len())];
        let mut queue = vec![(alo, ahi, blo, bhi)];

        let mut matching_blocks = vec![];
        while let Some((alo, ahi, blo, bhi)) = queue.pop() {
            if let Some((i, j, k)) = self.find_longest_match(alo, ahi, blo, bhi) {
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

            if let Some(next) = matching_blocks.get_mut(i+1) {
                if block.0 + block.2 == next.0 && block.1 + block.2 == next.1 {
                    block.2 += next.2;
                    next.0 += next.2;
                    next.1 += next.2;
                    next.2 = 0;
                }
            }

            parts.push(self.parent.make_part(true, block.0..block.0+block.2, block.1..block.1+block.2));
        }

        parts
    }
}
