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
    part: &Part,
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
        let start = (part.slices[i].start as isize + shift) as usize;
        if start == 0 {
            scores[NEWLINE] += 1;
            // prefix_scores[0] += 1;
        } else {
            let ext = part.parent.words[i][start-1];
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
        let end = (part.slices[i].end as isize + shift) as usize;
        if end == part.parent.words[i].len() {
            scores[0] += 1;
        } else {
            let ext = part.parent.words[i][end];
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

fn score_part_shift(parts: &Parts, parti: usize, i: usize) -> Vec<([usize; NUM_SCORES], isize)> {
    let part = &parts[parti];
    let mut scores = vec![];

    let mut words: VecDeque<_> = part.get(i).iter().copied().collect();
    let prev = if parti > 0 { Some(&parts[parti-1]) } else { None };
    let prev_words = prev.map(|p| p.get(i)).into_iter().flatten().rev();
    let next = parts.get(parti+1);
    let next_words = next.map(|n| n.get(i)).into_iter().flatten();

    // no shift; more score if it is start or end of line
    let p = prev_words.clone().next().copied();
    let n = next_words.clone().next().copied();
    scores.push((score_words(part, p, &words, n, i, 0), 0));

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
            scores.push((score_words(part, p, &words, n, i, shift), shift));
        }
    }

    let mut words: VecDeque<_> = part.get(i).iter().copied().collect();
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
            scores.push((score_words(part, p, &words, n, i, shift), shift));
        }
    }

    scores
}

pub fn shift_parts(parts: &mut Vec<Part>) {
    // try to shift non matches around e.g. so that whitespace is at the ends

    if parts.len() < 2 {
        return
    }

    let mut insert_start = None;
    let mut insert_end = None;
    for i in 0..parts.len() {
        {
            let part = &parts[i];
            // must be one empty and one non empty
            if part.matches || part.is_empty(0) == part.is_empty(1) {
                continue
            }
        }

        let side = if parts[i].is_empty(0) { 1 } else { 0 };
        // prefer better score, less shifting, and shifting right
        let scores = score_part_shift(parts, i, side);
        let &(_score, shift) = scores.iter().max_by_key(|(score, shift)| (score, -shift.abs(), shift)).unwrap();

        if shift == 0 {
            continue
        }

        let (left, right) = parts.split_at_mut(i);
        let (part, right) = right.split_at_mut(1);
        let part = &mut part[0];

        let prev = left.last_mut().unwrap_or_else(|| {
            insert_start = Some(part.partition_from_start(0, 0, true).0);
            insert_start.as_mut().unwrap()
        });

        let next = right.first_mut().unwrap_or_else(|| {
            insert_end = Some(part.partition_from_end(0, 0, true).1);
            insert_end.as_mut().unwrap()
        });

        prev.slices = prev.shift_slice(0, shift);
        part.slices = part.shift_slice(shift, shift);
        next.slices = next.shift_slice(shift, 0);
    }

    if let Some(insert_start) = insert_start {
        parts.insert(0, insert_start);
    }
    parts.extend(insert_end);
}
