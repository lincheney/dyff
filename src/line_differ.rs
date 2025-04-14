use std::ops::Range;
use std::collections::HashMap;
use super::block_maker::BlockMaker;

pub struct LineDiffer<'a> {
    parent: &'a BlockMaker<'a>,

    // line_index: HashMap<&'a [u8], usize>,
    b2j: Vec<Vec<usize>>,
}

impl<'a> LineDiffer<'a> {
    pub fn new(parent: &'a BlockMaker<'a>) -> Self {
        let mut b2j = vec![];
        b2j.resize_with(parent.tokeniser.max_token().0, Vec::new);

        for (lineno, tok) in parent.line_tokens[1].iter().enumerate() {
            b2j[tok.0].push(lineno);
        }

        Self{
            parent,
            b2j,
        }
    }

    fn find_longest_match(
        &mut self,
        alo: usize,
        ahi: usize,
        blo: usize,
        bhi: usize,
    ) -> Option<(usize, usize, usize)> {

        let mut besti = alo;
        let mut bestj = blo;
        let mut bestsize = 0;

        let mut j2len = HashMap::<usize, usize>::new();
        let mut newj2len = HashMap::<usize, usize>::new();

        for i in alo..ahi {
            // look at all instances of a[i] in b; note that because
            // b2j has no junk keys, the loop is skipped if a[i] is junk
            newj2len.clear();
            let value = self.parent.line_tokens[0][i];

            let j = &self.b2j[value.0];
            for &j in j.iter().skip_while(|&&j| j < blo).take_while(|&&j| j < bhi) {
                // a[i] matches b[j]
                let k = if j == 0 { 1 } else { j2len.get(&(j-1)).unwrap_or(&0) + 1 };

                newj2len.insert(j, k);

                if k > bestsize {
                    besti = i + 1 - k;
                    bestj = j + 1 - k;
                    bestsize = k;
                }
            }

            std::mem::swap(&mut j2len, &mut newj2len);
        }

        if bestsize <= 1 {
            return None
        }

        Some((besti, bestj, bestsize))
    }

    pub fn get_matching_blocks(&mut self) -> Vec<(Range<usize>, Range<usize>)> {
        let mut queue = vec![(0, self.parent.line_tokens[0].len(), 0, self.parent.line_tokens[1].len())];

        let mut matching_blocks = vec![];
        while let Some((alo, ahi, blo, bhi)) = queue.pop() {
            if let Some((i, j, k)) = self.find_longest_match(alo, ahi, blo, bhi) {
                // a[alo:i] vs b[blo:j] unknown
                // a[i:i+k] same as b[j:j+k]
                // a[i+k:ahi] vs b[j+k:bhi] unknown

                matching_blocks.push((
                    self.parent.line_to_word[0][i] .. self.parent.line_to_word[0][i+k],
                    self.parent.line_to_word[1][j] .. self.parent.line_to_word[1][j+k],
                ));

                if alo < i && blo < j {
                    queue.push((alo, i, blo, j));
                }
                if i+k < ahi && j+k < bhi {
                    queue.push((i+k, ahi, j+k, bhi));
                }
            }
        }
        matching_blocks.sort_by_key(|(a, _b)| a.start);
        matching_blocks
    }
}

