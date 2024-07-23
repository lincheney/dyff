use super::part::Part;
use std::collections::VecDeque;
use super::types::*;

const NUM_SCORES: usize = 4;
type Parts<'a> = Vec<Part<'a>>;

fn score_words(part: &Part, words: &VecDeque<Bytes>, i: usize, shift: isize) -> [[usize; NUM_SCORES]; 2] {
    static PREFIXES: [(usize, Bytes); 3] = [
        (0, b"\n"),
        (1, b" \t"),
        // (1, b"{"),
        // (1, b",;"),
        (2, b"{[("),
    ];
    static SUFFIXES: [(usize, Bytes); 6] = [
        (0, b"\n"),
        (1, b" \t"),
        (2, b"}"),
        (1, b" \t"),
        (2, b",;"),
        (2, b"}])"),
    ];

    let mut skip = 0;
    let mut prefix_scores = [0; NUM_SCORES];
    for &(ix, p) in PREFIXES.iter() {
        let count = words.iter().skip(skip).take_while(|w| p.contains(&w[0])).count();
        skip += count;
        prefix_scores[ix] += count;
    }

    let mut skip = 0;
    let mut suffix_scores = [0; NUM_SCORES];
    for &(ix, p) in SUFFIXES.iter() {
        let count = words.iter().rev().skip(skip).take_while(|w| p.contains(&w[0])).count();
        skip += count;
        suffix_scores[ix] += count;
    }


    if words[0] != b"\n" && words.back().unwrap() != b"\n" {
        // check if this is at start of line
        let start = (part.slices[i].start as isize + shift) as usize;
        if start == 0 || part.parent.words[i][start-1] == b"\n" {
            prefix_scores[0] += 1;
        }

        // check if this is at end of line
        let end = (part.slices[i].end as isize + shift) as usize;
        if end == part.parent.words[i].len() || part.parent.words[i][end] == b"\n" {
            suffix_scores[0] += 1;
        }
    }

    // prefer suffix scores
    [suffix_scores, prefix_scores]
}

fn score_part_shift(parts: &Parts, parti: usize, i: usize) -> Vec<([[usize; NUM_SCORES]; 2], isize)> {
    let part = &parts[parti];
    let mut scores = vec![];

    let mut words: VecDeque<_> = part.get(i).iter().copied().collect();
    // no shift; more score if it is start or end of line
    scores.push((score_words(part, &words, i, 0), 0));

    // try shift left ie move stuff at back to front
    if parti > 0 && parts[parti-1].matches {
        let prev_words = parts[parti-1].get(i);
        for (shift, word) in prev_words.iter().rev().enumerate() {
            if word != words.back().unwrap() {
                break
            }
            words.rotate_right(1);
            let shift = -(1 + shift as isize);
            scores.push((score_words(part, &words, i, shift), shift));
        }
    }

    let mut words: VecDeque<_> = part.get(i).iter().copied().collect();
    // try shift right ie move stuff at front to back
    if let Some(next_words) = parts.get(parti+1) {
        if next_words.matches {
            let next_words = next_words.get(i);
            for (shift, &word) in next_words.iter().enumerate() {
                if word != words[0] {
                    break
                }
                words.rotate_left(1);
                let shift = 1 + shift as isize;
                scores.push((score_words(part, &words, i, shift), shift));
            }
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
