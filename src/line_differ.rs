use std::ops::Range;
use std::collections::HashMap;
use super::block_maker::BlockMaker;

pub struct LineDiffer<'a> {
    parent: &'a BlockMaker<'a>,

    left: Vec<usize>,
    right: Vec<usize>,
    // line_index: HashMap<&'a [u8], usize>,
    b2j: HashMap<usize, Vec<usize>>,
}

impl<'a> LineDiffer<'a> {
    pub fn new(parent: &'a BlockMaker<'a>) -> Self {
        let mut b2j = HashMap::new();
        let mut line_index = HashMap::new();
        let mut sides = [vec![], vec![]];

        for i in 0..=1 {
            for (lineno, bounds) in parent.line_to_word[i].windows(2).enumerate() {
                let key = line_index.len();
                let start = bounds[0];
                let end = bounds[1];

                let words = &parent.words[i][start..end];
                let line = words.iter()
                    .flat_map(|m| m.as_bytes())
                    // .skip_while(|c| c.is_ascii_whitespace())
                    .copied()
                    .collect::<Vec<u8>>();

                let key = *line_index.entry(line).or_insert(key);
                sides[i].push(key);

                if i == 1 {
                    b2j.entry(key).or_insert_with(Vec::new).push(lineno);
                }
            }
        }

        let [left, right] = sides;
        Self{
            parent,
            b2j,
            left,
            right,
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
            let value = self.left[i];

            if let Some(j) = self.b2j.get(&value) {
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

                    newj2len.insert(j, k);

                    if k > bestsize {
                        besti = i + 1 - k;
                        bestj = j + 1 - k;
                        bestsize = k;
                    }
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
        let mut queue = vec![(0, self.left.len(), 0, self.right.len())];

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

