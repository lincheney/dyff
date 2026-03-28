use std::ops::Range;
use super::part::Part;
use std::collections::VecDeque;
use super::types::Bytes;

const WORD_BREAK: usize = 0;
const NEWLINE: usize = 1;
const GOOD_SUFFIX: usize = 2;
// const GOOD_PREFIX: usize = 3;
const WHITESPACE_SUFFIX: usize = 4;
const WHITESPACE_PREFIX: usize = 5;
const OTHER_SUFFIX: usize = 6;
const OTHER_PREFIX: usize = 7;
const NUM_SCORES: usize = 8;
type Parts<'a> = Vec<Part<'a>>;

fn is_word(x: u8) -> bool {
    x == b'_' || (!x.is_ascii_control() && !x.is_ascii_punctuation() && !x.is_ascii_whitespace())
}

fn add_scores<const N: usize>(mut a: [usize; N], b: [usize; N]) -> [usize; N] {
    for (x, y) in a.iter_mut().zip(&b) {
        *x += y;
    }
    a
}

fn words_from_parts<'a>(parts: &'a [Part], side: usize) -> impl DoubleEndedIterator<Item=Bytes<'a>> {
    parts.iter().flat_map(move |p| p.get(side)).copied()
}

const fn make_const_array<const N: usize>(values: &[&'static [u8]]) -> [&'static [u8]; N] {
    let mut result = [b"" as _; N];
    let mut i = 0;
    while i < values.len() {
        result[i] = values[i];
        i += 1;
    }
    result
}

fn score_words_prefix(
    first_part: &Part,
    words: &VecDeque<Bytes>,
    prev_word: Option<Bytes>,
    side: usize,
    shift: isize,
) -> [usize; NUM_SCORES] {
    const PREFIXES: [(usize, [&[u8]; 3]); 1] = [
        // (NEWLINE, b"\n"),
        // (WHITESPACE_PREFIX, b" \t"),
        // (OTHER_PREFIX, b"{"),
        // (GOOD_PREFIX, b",;"),
        (OTHER_PREFIX, make_const_array(&[b"{", b"[", b"("])),
    ];

    let parent = first_part.parent;
    let mut skip = 0;
    let mut scores = [0; NUM_SCORES];
    for &(ix, p) in &PREFIXES {
        let count = words.iter().skip(skip).take_while(|w| p.contains(&w.as_ref())).count();
        skip += count;
        scores[ix] += count * 2;
    }

    // check if this is at start of line
    let start = (first_part.slices[side].start as isize + shift) as usize;
    let newline_prefix = (parent.words[side][0] != b"\n").then_some(&b"\n".into()).into_iter()
        .chain(&parent.words[side][..start])
        .chain(words.front().filter(|&&x| x == b"\n"))
        .rev()
        .take_while(|&&x| x == b"\n") // treat start of string as a newline
        .count();
    scores[NEWLINE] += newline_prefix;

    if newline_prefix == 0 && start > 0 {
        static EXT_PREFIXES: [(usize, &[u8]); 4] = [
            // (NEWLINE, b"\n"),
            (WHITESPACE_PREFIX, b" "),
            (OTHER_PREFIX, b"("),
            (OTHER_PREFIX, b"{"),
            (OTHER_PREFIX, b"["),
        ];
        let ext = parent.words[side][start-1];
        for &(ix, p) in &EXT_PREFIXES {
            if p == ext {
                scores[ix] += 1;
                break;
            }
        }
    }

    // boost scores where they do not break a word
    if !(is_word(words[0][0]) && prev_word.is_some_and(|p| is_word(*p.last().unwrap()))) {
        scores[WORD_BREAK] += 1;
    }

    scores
}

fn score_words_suffix(
    last_part: &Part,
    words: &VecDeque<Bytes>,
    next_word: Option<Bytes>,
    side: usize,
    shift: isize,
) -> [usize; NUM_SCORES] {

    const SUFFIXES: [(usize, [&[u8]; 4]); 4] = [
        (NEWLINE, make_const_array(&[b"\n"])),
        (WHITESPACE_SUFFIX, make_const_array(&[b" ", b"\t"])),
        (GOOD_SUFFIX, make_const_array(&[b",", b";"])),
        (OTHER_SUFFIX, make_const_array(&[b"}", b"]", b")"])),
    ];
    const WORD_SUFFIXES: [&[u8]; 1] = [
        b"break",
    ];

    let parent = last_part.parent;
    let mut skip = 0;
    let mut scores = [0; NUM_SCORES];

    let mut done = false;
    while !done {
        let mut total = 0;
        for &(ix, p) in &SUFFIXES {
            let count = words.iter().rev().skip(skip).take_while(|w| p.contains(&w.as_ref())).count();
            skip += count;
            scores[ix] += count * 2;
            total += count;
        }

        if skip < words.len() {
            for suffix in &WORD_SUFFIXES {
                let mut chars = words.iter()
                    .rev() // iter from the end
                    .skip(skip) // skip the last words
                    .enumerate()
                    .flat_map(|(i, x)| x.iter().rev().map(move |c| (i, c))); // get chars in reverse

                let this_suffix = chars.by_ref().take(suffix.len()).map(|(_, c)| c);

                // compare with the suffix in reverse
                // and make sure there is a word break
                if this_suffix.eq(suffix.iter().rev()) {
                    let (count, next_char) = chars.next().unzip();
                    if !matches!(next_char, Some(b'a'..=b'z' | b'_' | b'A'..=b'Z' | b'0'..=b'9')) {
                        skip += count.unwrap_or(words.len() - skip);
                        scores[GOOD_SUFFIX] += 1;
                        break;
                    }
                }
            }
        }

        done = done || total == 0;
    }

    // check if this is at end of line
    let end = (last_part.slices[side].end as isize + shift) as usize;
    let newline_suffix = words.back().filter(|&&x| x == b"\n").into_iter()
        .chain(&parent.words[side][end..])
        .chain((parent.words[side].last().unwrap() != &b"\n").then_some(&b"\n".into())) // treat end of string as a newline
        .take_while(|&&x| x == b"\n")
        .count();
    scores[NEWLINE] += newline_suffix;

    // check for other suffixes
    if newline_suffix == 0 && let Some(&ext) = parent.words[side].get(end) {
        static EXT_SUFFIXES: [(usize, &[u8]); 5] = [
            // (NEWLINE, b"\n"),
            (WHITESPACE_SUFFIX, b" "),
            (GOOD_SUFFIX, b":"),
            (OTHER_SUFFIX, b")"),
            (OTHER_SUFFIX, b"}"),
            (OTHER_SUFFIX, b"]"),
        ];
        for &(ix, s) in &EXT_SUFFIXES {
            if s == ext {
                scores[ix] += 1;
                break;
            }
        }
    }

    if !(is_word(*words.back().unwrap().last().unwrap()) && next_word.is_some_and(|n| is_word(n[0]))) {
        scores[WORD_BREAK] += 1;
    }

    scores
}

fn score_words(
    parts: &[Part],
    words: &VecDeque<Bytes>,
    prev_word: Option<Bytes>,
    next_word: Option<Bytes>,
    side: usize,
    shift: isize,
) -> [usize; NUM_SCORES] {
    let prefix_scores = score_words_prefix(&parts[0], words, prev_word, side, shift);
    let suffix_scores = score_words_suffix(parts.last().unwrap(), words, next_word, side, shift);
    add_scores(prefix_scores, suffix_scores)
}

fn score_part_shift(parts: &Parts, range: Range<usize>, side: usize) -> Vec<([usize; NUM_SCORES], isize)> {
    let shiftable = &parts[range.clone()];
    let mut scores = vec![];

    let mut words: VecDeque<_> = words_from_parts(shiftable, side).collect();
    let prev = if range.start > 0 { Some(&parts[range.start-1]) } else { None };
    let prev_words = prev.map(|p| p.get(side)).into_iter().flatten().rev();
    let next = parts.get(range.end);
    let next_words = next.map(|n| n.get(side)).into_iter().flatten();

    // no shift; more score if it is start or end of line
    let p = prev_words.clone().next().copied();
    let n = next_words.clone().next().copied();
    scores.push((score_words(shiftable, &words, p, n, side, 0), 0));

    // try shift left ie move stuff at back to front
    if let Some(prev) = prev && prev.matches {
        for (shift, word) in prev_words.clone().enumerate() {
            if word != words.back().unwrap() {
                break
            }
            words.rotate_right(1);
            let p = prev_words.clone().nth(shift+1).copied();
            let n = prev_words.clone().nth(shift).copied();
            let shift = -(1 + shift as isize);

            let score = if p.is_none() && range.start > 1 && !parts[range.start-2].matches {
                // this joins onto another part on the left
                let suffix_scores = score_words_suffix(shiftable.last().unwrap(), &words, n, side, shift);

                let leftmost = parts[..range.start-1].iter().rev().take_while(|p| !p.matches).count();
                let leftmost = range.start - 1 - leftmost;
                for word in words_from_parts(&parts[leftmost .. range.start - 1], side).rev() {
                    words.push_front(word);
                }
                let p = if leftmost > 1 {
                    parts[leftmost-1].get(side).last().copied()
                } else {
                    None
                };
                let mut prefix_scores = score_words_prefix(&parts[leftmost], &words, p, side, 0);
                prefix_scores[OTHER_PREFIX] += 1;
                add_scores(prefix_scores, suffix_scores)
            } else {
                score_words(shiftable, &words, p, n, side, shift)
            };

            scores.push((score, shift));
        }
    }

    let mut words: VecDeque<_> = words_from_parts(shiftable, side).collect();
    // try shift right ie move stuff at front to back
    if let Some(next) = next && next.matches {
        // let next_words = next_words.get(side);
        for (shift, &word) in next_words.clone().enumerate() {
            if word != words[0] {
                break
            }
            words.rotate_left(1);
            let p = next_words.clone().nth(shift).copied();
            let n = next_words.clone().nth(shift+1).copied();
            let shift = 1 + shift as isize;

            let score = if n.is_none() && range.end < parts.len()-1 && !parts[range.end+1].matches {
                // this joins onto another part on the right
                let prefix_scores = score_words_prefix(&shiftable[0], &words, p, side, shift);

                let rightmost = parts[range.end+1..].iter().take_while(|p| !p.matches).count();
                let rightmost = range.end + rightmost;
                for word in words_from_parts(&parts[range.end + 1 .. rightmost + 1], side) {
                    words.push_back(word);
                }
                let n = if rightmost + 1 < parts.len() {
                    parts[rightmost + 1].get(side).last().copied()
                } else {
                    None
                };
                let mut suffix_scores = score_words_suffix(&parts[rightmost], &words, n, side, 0);
                suffix_scores[OTHER_SUFFIX] += 1;
                add_scores(prefix_scores, suffix_scores)
            } else {
                score_words(shiftable, &words, p, n, side, shift)
            };

            scores.push((score, shift));
        }
    }

    scores
}

pub fn shift_parts(parts: &mut Vec<Part>) {
    // try to shift non matches around e.g. so that whitespace is at the ends

    if parts.len() < 2 {
        return
    }

    let mut i = 0;
    while i < parts.len() {

        if let Some(side) = parts[i].shiftable_side() {

            for mut len in 1..=parts.len()-i {
                if parts[i+len-1].shiftable_side() != Some(side) {
                    // not shiftable or wrong side
                    break
                }

                // prefer better score, less shifting, and shifting right
                let scores = score_part_shift(parts, i..i+len, side);
                let &(_score, mut shift) = scores.iter().max_by_key(|(score, shift)| (score, -shift.abs(), shift)).unwrap();

                if shift == 0 {
                    // try shifting one part at the same time
                    continue
                }

                let nonside_range = parts[i].shift_slice(shift, shift)[1 - side].clone();

                // shift right
                if shift > 0 {
                    if i == 0 {
                        // need to add an extra one
                        parts.insert(i, parts[i].partition_from_start(0, 0, true).0);
                        i += 1;
                    }

                    // remove any parts that get shifted out of existence
                    while len > 1 && let part_len = parts[i].get(side).len() as isize && part_len < shift {
                        parts.remove(i);
                        shift -= part_len;
                        len -= 1;
                    }
                }
                let start = &mut parts[i].slices[side].start;
                *start = (*start as isize + shift) as usize;
                // shift the prev part
                if i > 0 {
                    parts[i-1].slices = parts[i-1].shift_slice(0, shift);
                }

                // shift left
                if shift < 0 {
                    if i+len == parts.len() {
                        // need to add an extra one
                        parts.push(parts[i+len-1].partition_from_end(0, 0, true).1);
                    }

                    // remove any parts that get shifted out of existence
                    while len > 1 && let part_len = parts[i+len].get(side).len() as isize && part_len < -shift {
                        parts.remove(i+len);
                        shift += part_len;
                        len -= 1;
                    }
                }
                let end = &mut parts[i+len-1].slices[side].end;
                *end = (*end as isize + shift) as usize;
                // shift the next part
                if i + len < parts.len() {
                    parts[i+len].slices = parts[i+len].shift_slice(shift, 0);
                }

                for part in &mut parts[i..i+len] {
                    part.slices[1 - side] = nonside_range.clone();
                }

                break
            }
        }

        i += 1;
    }

}
