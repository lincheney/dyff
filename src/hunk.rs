use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use anyhow::{Result};
use super::style::Style;
use super::types::{Line, Bytes};
use super::block_maker::BlockMaker;

const NO_NEWLINE: &[u8] = b"\\ No newline at end of file\n";

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
            let mut blocks = maker.make_block(tokeniser).split_block();

            // we need to do this here since we are modifying the block maker
            let mut maker = Cow::Borrowed(&maker);
            let mut shift = 0;
            for block in &mut blocks {
                (maker, shift) = block.split_in_middle_of_word(maker, tokeniser, shift);
            }
            if matches!(maker, Cow::Owned(_)) {
                for block in &mut blocks {
                    block.set_block_maker(&maker);
                }
            }

            for block in &blocks {
                block.print(stdout, merge_markers, style, style_opts, super::style::format_lineno)?;
                stdout.flush()?;
            }

            let has_newline = [0, 1].map(|i| {
                blocks.iter().flat_map(|b| &b.parts).rfind(|p| !p.is_empty(i)).is_none_or(|p| p.ends_with_newline(i))
            });
            // print the no newline message
            match has_newline {
                [true, true] => (),
                [false, false] => {
                    if style.inline {
                        stdout.write_all(style_opts.diff_context.as_bytes())?;
                        stdout.write_all(NO_NEWLINE)?;
                    } else {
                        stdout.write_all(style.diff_matching[0])?;
                        stdout.write_all(NO_NEWLINE)?;
                        stdout.write_all(style.diff_matching[1])?;
                        stdout.write_all(NO_NEWLINE)?;
                    }
                },
                _ => {
                    stdout.write_all(style.diff_non_matching[if has_newline[1] { 0 } else { 1 }])?;
                    stdout.write_all(NO_NEWLINE)?;
                },
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
            let mut filename = filename.unwrap_or(b"".into()).to_owned();
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
            diff_matching: [(*style_opts.filename_header_left).into(), (*style_opts.filename_header_right).into()],
            diff_matching_inline: (*style_opts.filename_rename).into(),
            diff_non_matching: [(*style_opts.filename_non_matching_left).into(), (*style_opts.filename_non_matching_right).into()],
            ..style
        };
        let maker = BlockMaker::new(&hunk, [1, 1], tokeniser);
        let blocks = maker.make_block(tokeniser).split_block();
        for block in blocks {
            block.print(stdout, None, style, style_opts, |num: [usize; 2], _, _, _, _| -> &'a str {
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
