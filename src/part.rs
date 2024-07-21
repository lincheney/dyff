use std::cmp::{max};

fn both_sides<T, F: FnMut(usize)->T>(mut f: F) -> [T; 2] {
    [f(0), f(1)]
}

#[derive(Clone)]
pub struct Part<'a> {
    pub parent: &'a super::block_maker::BlockMaker<'a>,
    pub matches: bool,

    pub slices: [std::ops::Range<usize>; 2],
}

impl<'a> Part<'a> {

    pub fn get(&self, i: usize) -> impl DoubleEndedIterator<Item=&[u8]> + ExactSizeIterator {
        self.parent.words[i][self.slices[i].clone()].iter().map(|x| x.as_bytes())
    }

    fn get_non_whitespace(&self, i: usize) -> impl Iterator<Item=&[u8]> {
        self.get(i).filter(|word| !word.iter().all(|c| c.is_ascii_whitespace()))
    }

    pub fn first_lineno(&self, i: usize) -> usize {
        self.parent.get_lineno(i, self.slices[i].start)
    }

    pub fn last_lineno(&self, i: usize) -> usize {
        self.parent.get_lineno(i, max(self.slices[i].start + 1, self.slices[i].end) - 1)
    }

    pub fn single_line(&self, i: usize) -> bool {
        self.first_lineno(i) == self.last_lineno(i)
    }

    pub fn starts_line(&self, i: usize) -> bool {
        self.slices[i].start == self.parent.get_wordno(i, self.first_lineno(i))
    }

    pub fn ends_line(&self, i: usize) -> bool {
        self.slices[i].end == self.parent.words[i].len()
        || self.slices[i].end == self.parent.get_wordno(i, self.last_lineno(i) + 1)
    }

    pub fn whole_line(&self) -> bool {
        self.starts_line(0) && self.starts_line(1) && self.ends_line(0) && self.ends_line(1)
    }

    fn char_len(&self, i: usize) -> usize {
        // score whitespace lower
        self.get_non_whitespace(i).map(|word| word.len()).sum()
    }

    pub fn word_len(&self, i: usize) -> usize {
        // score whitespace lower
        self.get_non_whitespace(i).count()
    }

    pub fn partition(&self, a: usize, b: usize) -> (Self, Self) {
        let a = a.clamp(self.slices[0].start, self.slices[0].end);
        let b = b.clamp(self.slices[1].start, self.slices[1].end);
        let first = [self.slices[0].start .. a, self.slices[1].start .. b];
        let last = [a .. self.slices[0].end, b .. self.slices[1].end];

        (
            Self{parent: self.parent, matches: self.matches, slices: first},
            Self{parent: self.parent, matches: self.matches, slices: last},
        )
    }

    pub fn partition_from_start(&self, a: usize, b: usize) -> (Self, Self) {
        self.partition(self.slices[0].start + a, self.slices[1].start + b)
    }

    pub fn partition_from_end(&self, a: usize, b: usize) -> (Self, Self) {
        self.partition(self.slices[0].end - a, self.slices[1].end - b)
    }

    pub fn is_empty(&self, i: usize) -> bool {
        self.slices[i].is_empty()
    }

    pub fn shift_slice(&self, a: isize, b: isize) -> [std::ops::Range<usize>; 2] {
        [
            (self.slices[0].start as isize + a) as usize .. (self.slices[0].end as isize + b) as usize,
            (self.slices[1].start as isize + a) as usize .. (self.slices[1].end as isize + b) as usize,
        ]
    }

    fn splitable(&self, i: usize) -> bool {
        self.is_empty(i)
        || self.last_lineno(i) > self.first_lineno(i) // spans at least 2 lines
        || (self.starts_line(i) && self.ends_line(i)) // spans at least 1 lines and first or last line is whole line
    }

    pub fn split(self) -> [Option<Self>; 3] {
        if self.whole_line() {
            return [Some(self), None, None];
        }

        let mut starts_line = both_sides(|i| self.starts_line(i));
        let mut ends_line = both_sides(|i| self.ends_line(i));
        // if matches, they must both start/end line
        if self.matches && (!starts_line[0] || !starts_line[1]) {
            starts_line = [false, false];
        }
        if self.matches && (!ends_line[0] || !ends_line[1]) {
            ends_line = [false, false];
        }

        let mut prefix_pivot = both_sides(|i|
            if starts_line[i] {
                self.slices[i].start
            } else {
                self.parent.get_wordno(i, self.first_lineno(i) + 1)
            }
        );
        let mut suffix_pivot = both_sides(|i|
            if ends_line[i] {
                self.slices[i].end
            } else {
                self.parent.get_wordno(i, self.last_lineno(i))
            }
        );

        if self.splitable(0) && self.splitable(1) {
            // partition at end of first line
            let (first, second) = self.partition(prefix_pivot[0], prefix_pivot[1]);
            // partition at start of last line
            let (second, third) = second.partition(suffix_pivot[0], suffix_pivot[1]);
            return [Some(first), Some(second), Some(third)]
        }

        for i in 0..=1 {
            if !self.matches && self.splitable(i) && !self.is_empty(i) {
                // one is multiline, other is not

                let other = 1 - i;
                // the other takes up its whole line
                if starts_line[other] && ends_line[other] {
                    prefix_pivot[other] = self.slices[other].start;
                    suffix_pivot[other] = self.slices[other].end;
                    // partition at end of first line
                    let (first, second) = self.partition(prefix_pivot[0], prefix_pivot[1]);
                    // partition at start of last line
                    let (second, third) = second.partition(suffix_pivot[0], suffix_pivot[1]);
                    return [Some(first), Some(second), Some(third)]
                }

                // TODO spans part one and another line
                if self.first_lineno(other) != self.last_lineno(other) {
                    continue
                }

                let (first, second) = if starts_line[i] && ends_line[i] {
                    // whole line -> separate
                    if i == 0 {
                        self.partition(self.slices[0].end, self.slices[1].start)
                    } else {
                        self.partition(self.slices[0].start, self.slices[1].end)
                    }
                } else if (starts_line[other] && !ends_line[i]) || starts_line[i] {
                    // matches up with end
                    suffix_pivot[other] = self.slices[other].start;
                    self.partition(suffix_pivot[0], suffix_pivot[1])
                } else {
                    // otherwise matches with start
                    prefix_pivot[other] = self.slices[other].end;
                    self.partition(prefix_pivot[0], prefix_pivot[1])
                };
                return [Some(first), Some(second), None]
            }
        }

        [Some(self), None, None]
    }
}

impl<'a> std::fmt::Debug for Part<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "(\"{}\", \"{}\")",
            std::str::from_utf8(&self.get(0).collect::<Vec<_>>().concat().iter().flat_map(|c| std::ascii::escape_default(*c)).collect::<Vec<_>>()).unwrap(),
            std::str::from_utf8(&self.get(1).collect::<Vec<_>>().concat().iter().flat_map(|c| std::ascii::escape_default(*c)).collect::<Vec<_>>()).unwrap(),
        )
    }
}
