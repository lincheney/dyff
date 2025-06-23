use std::collections::HashMap;
use std::io::{BufWriter, Write};
use anyhow::{Result};
use super::style::Style;
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
        tokeniser: &mut super::tokeniser::Tokeniser,
        line_numbers: [usize; 2],
        merge_markers: Option<&MergeMarkers>,
        style: Style,
        style_opts: &super::StyleOpts,
    ) -> Result<()> {

        if !self.is_empty() {

            let maker = BlockMaker::new(self, line_numbers, tokeniser);
            let blocks = maker.make_block().split_block();

            let len = blocks.len();
            let last = [0, 1].map(|i| {
                blocks.iter().enumerate().rfind(|(_i, b)| !b.is_empty(i)).map(|(i, _b)| i).unwrap_or(len)
            });

            for (i, block) in blocks.iter().enumerate() {
                block.print(stdout, merge_markers, style, style_opts, i == last[0] || i == last[1], super::style::format_lineno)?;
                stdout.flush()?;
            }
        }
        Ok(())
    }

    pub fn print_filename<'a, T: std::io::Write>(
        stdout: &mut BufWriter<T>,
        tokeniser: &mut super::tokeniser::Tokeniser,
        left: Option<Bytes>,
        right: Option<Bytes>,
        prefix: (&'a str, &'a str, &'a str),
        // suffix: (&'a str, &'a str),
        style: Style,
        style_opts: &super::StyleOpts,
    ) -> Result<()> {

        let mut hunk = Self::new();

        for (i, filename) in [left, right].iter().enumerate() {
            let mut filename = filename.unwrap_or(b"").to_owned();
            if !filename.ends_with(b"\n") {
                filename.push(b'\n');
            }
            hunk.get_mut(i).push(filename);
        }

        let style = Style{
            signs: false,
            line_numbers: true,
            show_both: true,
            // inline: false,
            diff_matching: [style_opts.filename_header_left.as_bytes(), style_opts.filename_header_right.as_bytes()],
            diff_matching_inline: style_opts.filename_rename.as_bytes(),
            diff_non_matching: [style_opts.filename_non_matching_left.as_bytes(), style_opts.filename_non_matching_right.as_bytes()],
            ..style
        };
        let maker = BlockMaker::new(&hunk, [1, 1], tokeniser);
        let blocks = maker.make_block().split_block();
        for block in blocks {
            block.print(stdout, None, style, style_opts, false, |num: [usize; 2], _, _, _| -> &'a str {
                match num {
                    [_, 0] => prefix.0,
                    [0, _] => prefix.1,
                    [_, _] => prefix.2,
                }
            })?;
        }
        Ok(())
    }

}
