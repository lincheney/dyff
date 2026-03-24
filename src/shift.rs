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

fn score_words(
    parts: &[Part],
    prev: Option<Bytes>,
    words: &VecDeque<Bytes>,
    next: Option<Bytes>,
    i: usize,
    shift: isize,
) -> [usize; NUM_SCORES] {

    static PREFIXES: [(usize, &[u8]); 1] = [
        // (NEWLINE, b"\n"),
        // (WHITESPACE_PREFIX, b" \t"),
        // (OTHER_PREFIX, b"{"),
        // (GOOD_PREFIX, b",;"),
        (OTHER_PREFIX, b"{[("),
    ];
    static SUFFIXES: [(usize, &[u8]); 4] = [
        (NEWLINE, b"\n"),
        (WHITESPACE_SUFFIX, b" \t"),
        (GOOD_SUFFIX, b",;"),
        (OTHER_SUFFIX, b"}])"),
    ];

    let parent = parts[0].parent;
    let mut skip = 0;
    let mut scores = [0; NUM_SCORES];
    for &(ix, p) in &PREFIXES {
        let count = words.iter().skip(skip).take_while(|w| p.contains(&w[0])).count();
        skip += count;
        scores[ix] += count * 2;
    }

    let mut skip = 0;
    let mut done = false;
    while !done {
        let mut total = 0;
        for &(ix, p) in &SUFFIXES {
            let count = words.iter().rev().skip(skip).take_while(|w| p.contains(&w[0])).count();
            skip += count;
            scores[ix] += count * 2;
            total += count;
        }
        done = done || total == 0;
    }

    // check if this is at start of line
    if words[0] == b"\n" {
        scores[NEWLINE] += 1;
        // prefix_scores[0] += 1;
    } else {
        static EXT_PREFIXES: [(usize, &[u8]); 4] = [
            // (NEWLINE, b"\n"),
            (WHITESPACE_PREFIX, b" "),
            (OTHER_PREFIX, b"("),
            (OTHER_PREFIX, b"{"),
            (OTHER_PREFIX, b"["),
        ];
        let start = (parts[0].slices[i].start as isize + shift) as usize;
        if start == 0 {
            scores[NEWLINE] += 1;
            // prefix_scores[0] += 1;
        } else {
            let ext = parent.words[i][start-1];
            if ext == b"\n" {
                scores[NEWLINE] += 1;
                // prefix_scores[0] += 1;
            } else {
                for &(ix, p) in &EXT_PREFIXES {
                    if p == ext {
                        scores[ix] += 1;
                        break;
                    }
                }
            }
        }
    }

    // check if this is at end of line
    if *words.back().unwrap() == b"\n" {
        scores[NEWLINE] += 1;
    } else {
        static EXT_SUFFIXES: [(usize, &[u8]); 6] = [
            (NEWLINE, b"\n"),
            (WHITESPACE_SUFFIX, b" "),
            (GOOD_SUFFIX, b":"),
            (OTHER_SUFFIX, b")"),
            (OTHER_SUFFIX, b"}"),
            (OTHER_SUFFIX, b"]"),
        ];
        let end = (parts.last().unwrap().slices[i].end as isize + shift) as usize;
        if end == parent.words[i].len() {
            scores[0] += 1;
        } else {
            let ext = parent.words[i][end];
            for &(ix, s) in &EXT_SUFFIXES {
                if s == ext {
                    scores[ix] += 1;
                    break;
                }
            }
        }
    }

    // boost scores where they do not break a word
    if !(is_word(words[0][0]) && prev.is_some_and(|p| is_word(*p.last().unwrap()))) {
        scores[WORD_BREAK] += 1;
    }
    if !(is_word(*words.back().unwrap().last().unwrap()) && next.is_some_and(|n| is_word(n[0]))) {
        scores[WORD_BREAK] += 1;
    }

    scores
}

fn score_part_shift(parts: &Parts, range: Range<usize>, i: usize) -> Vec<([usize; NUM_SCORES], isize)> {
    let shiftable = &parts[range.clone()];
    let mut scores = vec![];

    let mut words: VecDeque<_> = shiftable.iter().flat_map(|p| p.get(i)).copied().collect();
    let prev = if range.start > 0 { Some(&parts[range.start-1]) } else { None };
    let prev_words = prev.map(|p| p.get(i)).into_iter().flatten().rev();
    let next = parts.get(range.end);
    let next_words = next.map(|n| n.get(i)).into_iter().flatten();

    // no shift; more score if it is start or end of line
    let p = prev_words.clone().next().copied();
    let n = next_words.clone().next().copied();
    scores.push((score_words(shiftable, p, &words, n, i, 0), 0));

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
            scores.push((score_words(shiftable, p, &words, n, i, shift), shift));
        }
    }

    let mut words: VecDeque<_> = shiftable.iter().flat_map(|p| p.get(i)).copied().collect();
    // try shift right ie move stuff at front to back
    if let Some(next) = next && next.matches {
        // let next_words = next_words.get(i);
        for (shift, &word) in next_words.clone().enumerate() {
            if word != words[0] {
                break
            }
            words.rotate_left(1);
            let p = next_words.clone().nth(shift).copied();
            let n = next_words.clone().nth(shift+1).copied();
            let shift = 1 + shift as isize;
            scores.push((score_words(shiftable, p, &words, n, i, shift), shift));
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

            for mut len in 1..parts.len()-i {
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
