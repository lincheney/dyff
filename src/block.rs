use std::io::{BufWriter, Write};
use anyhow::{Result};
use super::part::Part;
use super::style;
use super::regexes::regex;

pub struct Block<'a> {
    pub parts: Vec<Part<'a>>,
}

impl<'a> Block<'a> {
    pub fn split_block(self) -> Vec<Self> {
        vec![self]
    }

    pub fn print<
        T: Write,
        S: AsRef<str>,
        F: Fn(Option<usize>, Option<usize>, Option<&str>, Option<&str>, Option<&str>)->S
    >(
        &self,
        stdout: &mut BufWriter<T>,
        merge_markers: Option<&super::hunk::MergeMarkers>,
        style: style::Style,
        format_lineno: F,
    ) -> Result<()> {

        if !style.show_both && self.parts.iter().all(|p| p.matches || (p.is_empty(0) && p.is_empty(1))) {
            // this is entirely matching

            let mut lineno = 0;
            let mut newline = true;
            for part in self.parts.iter() {
                if !part.matches {
                    continue
                }

                for word in part.get(0) {
                    if newline {
                        if style.line_numbers {
                            stdout.write_all(format_lineno(
                                Some(part.first_lineno(0) + lineno), Some(part.first_lineno(1) + lineno),
                                Some(style::LINENO), Some(style::LINENO),
                                None,
                            ).as_ref().as_bytes())?;
                        }
                        if style.signs {
                            stdout.write_all(style::SIGN[2])?;
                        }
                        stdout.write_all(style::DIFF_CONTEXT)?;
                        newline = false;
                    }
                    stdout.write_all(&regex!(r"\s+\n".replace_all(word, style::DIFF_TRAILING_WS)))?;
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
            let mut newline = true;
            let mut insert = false;
            let mut lineno_args = [None, None];

            for part in self.parts.iter() {
                if part.is_empty(i) {
                    insert = true;
                    continue
                }

                if insert {
                    // add an insertion marker
                    stdout.write_all(style::DIFF_INSERT[i])?;
                    insert = false;
                }

                let highlight = if part.matches { style.diff_matching } else { style.diff_non_matching };
                stdout.write_all(highlight[i])?;
                for word in part.get(i) {

                    if newline {
                        let lineno = part.first_lineno(i) + lineno;

                        if style.line_numbers {
                            lineno_args[i] = Some(lineno);
                            let bar_style = merge_markers.and_then(|m| m.get(&(i, lineno)).map(|x| x.as_ref()));
                            stdout.write_all(format_lineno(
                                lineno_args[0], lineno_args[1],
                                None, None,
                                bar_style,
                            ).as_ref().as_bytes())?;
                        }
                        if style.signs {
                            stdout.write_all(style::SIGN[i])?;
                        }
                        stdout.write_all(highlight[i])?;

                        newline = false;
                    }

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
