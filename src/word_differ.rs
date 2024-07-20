use std::collections::HashMap;
use std::cmp::{min, max};
use super::block_maker::BlockMaker;
use super::part::Part;

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
            if !(line_start && word != b"\n" && word.iter().all(|c| c.is_ascii_whitespace())) {
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

    fn find_longest_match(
        &mut self,
        alo: usize,
        ahi: usize,
        blo: usize,
        bhi: usize,
    ) -> Option<(usize, usize, usize)> {

        let isjunk = |b: &[u8]| b != b"\n" && b.iter().all(|c| c.is_ascii_whitespace());

        let left = &self.parent.words[0];
        let right = &self.parent.words[1];

        let mut besti = alo;
        let mut bestj = blo;
        let mut bestsize = 0;
        let mut bestlen = 0;
        let mut bestline = usize::MAX;

        let mut mini = ahi;
        let mut minj = bhi;
        let mut maxi = 0;
        let mut maxj = 0;
        let mut bestcount = 0;

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


                    // prioritise more words, then longer words, then words on the expected line
                    let mut cmp = k.cmp(&bestsize);
                    if cmp.is_lt() {
                        continue
                    }

                    if cmp.is_gt() {
                        mini = ahi;
                        minj = bhi;
                        maxi = 0;
                        maxj = 0;
                        bestcount = 0;
                    }

                    let i = i + 1 - k;
                    let j = j + 1 - k;
                    mini = min(mini, i);
                    minj = min(minj, j);
                    maxi = max(maxi, i);
                    maxj = max(maxj, j);
                    bestcount += 1;

                    let l: usize = left[i .. i+k].iter().map(|w| w.len()).sum();
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
                    bestlen = l;
                    bestline = lineno_dist;
                }
            }

            std::mem::swap(&mut j2len, &mut newj2len);
        }

        if bestsize == 0 {
            return None
        }

        if bestcount > 1 {
            // this means there's multiple solutions
            if alo < mini && blo < minj {
                let m = self.find_longest_match(alo, mini, blo, minj);
                if m.is_some() {
                    return m
                }
            }
            if maxi < ahi && maxj < bhi {
                let m =self.find_longest_match(maxi, ahi, maxj, bhi);
                if m.is_some() {
                    return m
                }
            }
        }

        // match leading whitespace, up to start of line
        while
               besti > alo
            && bestj > blo
            && left[besti-1].as_bytes() == right[bestj-1].as_bytes()
            && (isjunk(left[besti-1].as_bytes()) || (
                   left[besti-1].as_bytes() == b"\n"
                && besti >= 2
                && left[besti-2].as_bytes() == b"\n"
                && bestj >= 2
                && right[bestj-2].as_bytes() == b"\n"
            ))
        {
            besti -= 1;
            bestj -= 1;
            bestsize += 1;
        }

        // match trailing whitespace, up to end of line
        while
               besti+bestsize < ahi
            && bestj+bestsize < bhi
            && left[besti+bestsize].as_bytes() == right[bestj+bestsize].as_bytes()
            && (
                (
                       left[besti+bestsize-1].as_bytes() != b"\n"
                    && isjunk(left[besti+bestsize].as_bytes())
                ) || left[besti+bestsize].as_bytes() == b"\n"
            )
        {
            bestsize += 1;
        }

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

                matching_blocks.push(Part{
                    parent: self.parent,
                    matches: true,
                    slices: [i..i+k, j..j+k],
                });
                if alo < i && blo < j {
                    queue.push((alo, i, blo, j));
                }
                if i+k < ahi && j+k < bhi {
                    queue.push((i+k, ahi, j+k, bhi));
                }
            }
        }
        matching_blocks.sort_by_key(|a| a.slices[0].start);
        matching_blocks
    }
}
