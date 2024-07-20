use std::io::{BufWriter, Write};
use anyhow::{Result};
use super::part::Part;
use super::style;

pub struct Block<'a> {
    pub parts: Vec<Part<'a>>,
}

impl<'a> Block<'a> {
    pub fn split_block(self) -> Vec<Self> {
        vec![self]
    }

    pub fn print<T: Write>(
        &self,
        stdout: &mut BufWriter<T>,
        merge_markers: Option<&super::hunk::MergeMarkers>,
        // format_lineno=format_lineno,
        signs: bool,
    ) -> Result<()> {

        if self.parts.iter().all(|p| p.matches || (p.is_empty(0) && p.is_empty(1))) {
            // this is entirely matching

            let mut lineno = 0;
            let mut newline = false;
            for part in self.parts.iter() {
                if !part.matches {
                    continue
                }

                if newline {
                    stdout.write_all(style::format_lineno(
                        Some(part.first_lineno(0) + lineno),
                        Some(part.first_lineno(1) + lineno),
                    ).as_bytes())?;
                    if signs {
                        stdout.write_all(style::SIGN[2])?;
                    }
                    stdout.write_all(style::DIFF_CONTEXT)?;
                    newline = false;
                }

                for word in part.get(0) {
                    // line = re.sub(rb'(\s+\n)', style['diff_trailing_ws'].replace(b'\\', b'\\\\') + rb'\1', line)
                    stdout.write_all(word)?;
                    if word == b"\n" {
                        lineno += 1;
                        newline = true;
                    }
                }

            }
            return Ok(())
        }

        for i in 0..=1 {
            let mut lineno = 0;
            let mut newline = false;
            let mut insert = false;
            let mut lineno_args = [None, None];

            for part in self.parts.iter() {
                if part.is_empty(i) {
                    insert = true;
                    continue
                }

                if newline {
                    // let lineno = [part.first_lineno(0) + lineno, part.first_lineno(1) + lineno];
                    // stdout.write_all(format_lineno(*lineno_args, minus_style=style['lineno'], plus_style=style['lineno']) + style['sign'][2]);
                    lineno_args[i] = Some(part.first_lineno(i) + lineno);
                    stdout.write_all(style::format_lineno(
                        lineno_args[0],
                        lineno_args[1],
                    ).as_bytes())?;
                    if signs {
                        stdout.write_all(style::SIGN[i])?;
                    }
                    let bar_style = style::LINENO_BAR;
                    let bar_style = merge_markers.and_then(|m| m.get(&(i, lineno)).map(|x| x.as_ref())).unwrap_or(style::LINENO_BAR);
                    newline = false;
                }

                // if insert && !lines[0].is_empty() {
                    // // add an insertion marker
                    // // if lines[0][0] == b'\n'[0]:
                        // // lines[0] = b' ' + lines[0]
                    // lines[0] = style['diff_insert'][i] + lines[0][0:1] + highlight + lines[0][1:];
                // }
                insert = false;

                let highlight = if part.matches { style::DIFF_MATCHING } else { style::DIFF_NON_MATCHING };
                let highlight = if i == 0 { highlight.0 } else { highlight.1 };
                stdout.write_all(highlight)?;
                for word in part.get(i) {
                    // line = re.sub(rb'(\s+\n)', style['diff_trailing_ws'].replace(b'\\', b'\\\\') + rb'\1', line)
                    stdout.write_all(word)?;
                    if word == b"\n" {
                        lineno += 1;
                        newline = true;
                    }
                }
            }
        }

        Ok(())
    }
}
