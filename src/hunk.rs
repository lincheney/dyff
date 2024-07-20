use std::collections::HashMap;
use std::io::{BufWriter, Write};
use anyhow::{Result};
use super::style::{Style, FILENAME_HEADER, DIFF_NON_MATCHING};
use super::types::*;
use super::block_maker::BlockMaker;

pub type MergeMarkers = HashMap<(usize, usize), String>;

#[derive(Debug)]
pub struct Hunk {
    pub left: Line,
    pub right: Line,
}

impl Hunk {
    pub fn new() -> Self {
        Self{
            left: vec![],
            right: vec![],
        }
    }

    pub fn get(&self, i: usize) -> &Line {
        if i == 0 { &self.left } else { &self.right }
    }

    pub fn get_mut(&mut self, i: usize) -> &mut Line {
        if i == 0 { &mut self.left } else { &mut self.right }
    }

    pub fn is_empty(&self) -> bool {
        self.left.is_empty() && self.right.is_empty()
    }

    pub fn print<T: std::io::Write>(
        &mut self,
        stdout: &mut BufWriter<T>,
        line_numbers: [usize; 2],
        merge_markers: Option<&MergeMarkers>,
        style: Style,
    ) -> Result<()> {

        if !self.is_empty() {

            let maker = BlockMaker::new(self, line_numbers);
            let blocks = maker.make_block().split_block();

            for block in blocks {
                block.print(stdout, merge_markers, style, &super::style::format_lineno)?;
                stdout.write_all(super::style::RESET)?;
                stdout.flush()?;
            }
        }
        Ok(())
    }

    pub fn print_filename<'a, T: std::io::Write>(
        stdout: &mut BufWriter<T>,
        left: Option<Bytes>,
        right: Option<Bytes>,
        prefix: (&'a str, &'a str),
        style: Style,
    ) -> Result<()> {

        let mut hunk = Self::new();

        for (i, filename) in [left, right].iter().enumerate() {
            let mut filename = filename.unwrap_or(b"").to_owned();
            if !filename.ends_with(b"\n") {
                filename.push(b'\n');
            }
            hunk.get_mut(i).push(filename);
        }

        // add bold to it
        const BOLD: &[u8] = b"\x1b[1m";
        let mut diff_non_matching = [
            [0u8; DIFF_NON_MATCHING[0].len() + BOLD.len()],
            [0u8; DIFF_NON_MATCHING[1].len() + BOLD.len()],
        ];
        diff_non_matching[0][..DIFF_NON_MATCHING[0].len()].copy_from_slice(DIFF_NON_MATCHING[0]);
        diff_non_matching[1][..DIFF_NON_MATCHING[1].len()].copy_from_slice(DIFF_NON_MATCHING[1]);
        diff_non_matching[0][DIFF_NON_MATCHING[0].len()..].copy_from_slice(BOLD);
        diff_non_matching[1][DIFF_NON_MATCHING[1].len()..].copy_from_slice(BOLD);

        let style = Style{
            signs: false,
            line_numbers: false,
            show_both: true,
            diff_matching: [FILENAME_HEADER.0, FILENAME_HEADER.1],
            diff_non_matching: [&diff_non_matching[0], &diff_non_matching[1]],
            ..style
        };
        let maker = BlockMaker::new(&hunk, [0, 0]);
        let blocks = maker.make_block().split_block();
        for block in blocks {
            block.print(stdout, None, style, |num1: Option<usize>, _, _, _, _| -> &'a str {
                if num1.is_some() { &prefix.0 } else { &prefix.1 }
            })?;
        }
        Ok(())
    }

}
