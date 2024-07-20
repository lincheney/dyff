use std::collections::HashMap;
use std::io::{BufWriter, Write};
use anyhow::{Result};

type Line = Vec<Vec<u8>>;
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
        signs: bool,
    ) -> Result<()> {

        if !self.is_empty() {

            let maker = super::block_maker::BlockMaker::new(self, line_numbers);
            let blocks = maker.make_block().split_block();

            for block in blocks {
                block.print(stdout, merge_markers, signs)?;
                stdout.write_all(super::style::RESET)?;
                stdout.flush()?;
            }
        }
        Ok(())
    }

}
