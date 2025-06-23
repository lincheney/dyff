use std::io::{BufRead, BufReader, BufWriter, Write, IsTerminal};
use std::process::{Command, Stdio, ExitCode};
use std::collections::HashMap;
use clap::Parser;
use anyhow::{Result};
use std::borrow::Cow;

mod hunk;
mod style;
mod block_maker;
mod part;
mod word_differ;
mod line_differ;
mod block;
mod types;
mod whitespace;
mod shift;
mod tokeniser;
#[macro_use]
mod regexes;
use hunk::Hunk;
use types::*;

fn strip_style<'a>(string: Bytes<'a>, replace: Bytes) -> std::borrow::Cow<'a, [u8]> {
    byte_regex!(r"\x1b\[[\d;]*m".replace_all(string, replace))
}

fn shell_quote<S: AsRef<str>>(val: S) -> String {
    let mut val = val.as_ref().replace('\'', "'\\''");
    val.insert(0, '\'');
    val.push('\'');
    val
}

#[derive(Clone, PartialEq, Debug, clap::ValueEnum)]
enum AutoChoices {
    Never,
    Auto,
    Always,
}

#[derive(Debug, clap::Parser)]
#[command(name = "diff")]
struct Cli {

    #[arg(long, value_enum, default_value_t = AutoChoices::Auto)]
    color: AutoChoices,

    #[arg(short = 'N', long = "no-line-numbers", action = clap::ArgAction::SetFalse)]
    line_numbers: bool,

    #[arg(short, long)]
    signs: bool,

    #[arg(short = 'I', long, value_enum, default_value_t = AutoChoices::Auto)]
    inline: AutoChoices,

    #[arg(long)]
    exact: bool,

    #[arg(short, long)]
    filter: Option<String>,

    /// use LABEL instead of file name and timestamp (can be repeated)
    #[arg(long)]
    label: Vec<String>,

    #[command(flatten)]
    style: StyleOpts,

    #[arg(allow_hyphen_values = true)]
    file1: Option<String>,
    #[arg(allow_hyphen_values = true)]
    file2: Option<String>,

    #[arg(allow_hyphen_values = true)]
    extras: Vec<String>,
}

#[derive(Debug, Clone, clap::Args)]
pub struct StyleOpts {
    #[arg(long, default_value_t = style::HEADER.into())]
    header: Cow<'static, str>,
    #[arg(long, default_value_t = style::COMMIT.into())]
    commit: Cow<'static, str>,
    #[arg(long, default_value_t = style::BACKGROUND.into())]
    background: Cow<'static, str>,
    #[arg(long, default_value_t = style::CONTEXT.into())]
    context: Cow<'static, str>,
    #[arg(long, default_value_t = style::LINENO.into())]
    lineno: Cow<'static, str>,
    #[arg(long, default_value_t = style::LINENO_DIFF.0.into())]
    lineno_left: Cow<'static, str>,
    #[arg(long, default_value_t = style::LINENO_DIFF.1.into())]
    lineno_right: Cow<'static, str>,
    #[arg(long, default_value_t = style::LINENO_BAR.into())]
    lineno_bar: Cow<'static, str>,

    #[arg(long, default_value_t = style::LINENO_OUR_BAR.into())]
    lineno_our_bar: Cow<'static, str>,
    #[arg(long, default_value_t = style::LINENO_THEIR_BAR.into())]
    lineno_their_bar: Cow<'static, str>,
    #[arg(long, default_value_t = style::LINENO_MERGE_BAR.into())]
    lineno_merge_bar: Cow<'static, str>,

    #[arg(long, default_value_t = style::FILENAME.2.into())]
    filename: Cow<'static, str>,
    #[arg(long, default_value_t = style::FILENAME.0.into())]
    filename_left: Cow<'static, str>,
    #[arg(long, default_value_t = style::FILENAME.1.into())]
    filename_right: Cow<'static, str>,
    #[arg(long, default_value_t = style::FILENAME_RENAME.into())]
    filename_rename: Cow<'static, str>,
    #[arg(long, default_value_t = style::FILENAME_HEADER.0.into())]
    filename_header_left: Cow<'static, str>,
    #[arg(long, default_value_t = style::FILENAME_HEADER.1.into())]
    filename_header_right: Cow<'static, str>,
    #[arg(long, default_value_t = style::FILENAME_SIGN.2.into())]
    filename_sign: Cow<'static, str>,
    #[arg(long, default_value_t = style::FILENAME_SIGN.0.into())]
    filename_sign_left: Cow<'static, str>,
    #[arg(long, default_value_t = style::FILENAME_SIGN.1.into())]
    filename_sign_right: Cow<'static, str>,
    #[arg(long, default_value_t = style::FILENAME_NON_MATCHING[0].into())]
    filename_non_matching_left: Cow<'static, str>,
    #[arg(long, default_value_t = style::FILENAME_NON_MATCHING[1].into())]
    filename_non_matching_right: Cow<'static, str>,

    #[arg(long, default_value_t = style::DIFF_MATCHING[0].into())]
    diff_matching_left: Cow<'static, str>,
    #[arg(long, default_value_t = style::DIFF_MATCHING[1].into())]
    diff_matching_right: Cow<'static, str>,

    #[arg(long, default_value_t = style::DIFF_NON_MATCHING[0].into())]
    diff_non_matching_left: Cow<'static, str>,
    #[arg(long, default_value_t = style::DIFF_NON_MATCHING[1].into())]
    diff_non_matching_right: Cow<'static, str>,

    #[arg(long, default_value_t = style::DIFF_INSERT[0].into())]
    diff_insert_left: Cow<'static, str>,
    #[arg(long, default_value_t = style::DIFF_INSERT[1].into())]
    diff_insert_right: Cow<'static, str>,

    #[arg(long, default_value_t = style::DIFF_MATCHING_INLINE.into())]
    diff_matching_inline: Cow<'static, str>,
    #[arg(long, default_value_t = style::DIFF_CONTEXT.into())]
    diff_context: Cow<'static, str>,
    #[arg(long, default_value_t = style::DIFF_TRAILING_WS.into())]
    diff_trailing_ws: Cow<'static, str>,
}

impl StyleOpts {
    fn insert_background(&mut self) {
        // insert the bg everywhere
        if !self.background.is_empty() {
            let mut replacement = "\x1b[0m".to_owned();
            replacement += &self.background;
            replacement += style::PAINT_RIGHT;
            replacement += "\x1b[";

            macro_rules! replace {
                ( $name:expr ) => {
                    let pat = "\x1b[0;";
                    if $name.contains(pat) {
                        $name = Cow::from($name.replace(pat, &replacement));
                    }
                }
            }

            replace!(self.header);
            replace!(self.commit);
            // replace!(self.background);
            replace!(self.context);
            replace!(self.lineno);
            replace!(self.lineno_left);
            replace!(self.lineno_right);
            replace!(self.lineno_bar);
            replace!(self.lineno_our_bar);
            replace!(self.lineno_their_bar);
            replace!(self.lineno_merge_bar);
            replace!(self.filename);
            replace!(self.filename_left);
            replace!(self.filename_right);
            replace!(self.filename_rename);
            replace!(self.filename_header_left);
            replace!(self.filename_header_right);
            replace!(self.filename_sign);
            replace!(self.filename_sign_left);
            replace!(self.filename_sign_right);
            replace!(self.filename_non_matching_left);
            replace!(self.filename_non_matching_right);
            replace!(self.diff_matching_left);
            replace!(self.diff_matching_right);
            replace!(self.diff_non_matching_left);
            replace!(self.diff_non_matching_right);
            replace!(self.diff_insert_left);
            replace!(self.diff_insert_right);
            replace!(self.diff_matching_inline);
            replace!(self.diff_context);
            replace!(self.diff_trailing_ws);

        }
    }

    fn print_background<T: std::io::Write>(&self, stdout: &mut BufWriter<T>) -> Result<()> {
        if !self.background.is_empty() {
            stdout.write_all(self.background.as_bytes())?;
            stdout.write_all(style::PAINT_RIGHT.as_bytes())?;
        }
        Ok(())
    }
}


fn _main() -> Result<ExitCode> {
    let mut args = Cli::parse();

    {
        fn not_flag<S: AsRef<str>>(x: S) -> bool {
            !x.as_ref().starts_with('-')
        }

        if args.file2.as_ref().map(not_flag) == Some(false) {
            args.extras.insert(0, args.file2.take().unwrap());
        }

        if args.file1.as_ref().map(not_flag) == Some(false) {
            args.extras.insert(0, args.file1.take().unwrap());
            args.file1 = args.file2.take();
        }

        if args.file1.is_none() {
            args.file1 = args.extras.iter().position(not_flag).map(|i| args.extras.remove(i));
        }
        if args.file2.is_none() {
            args.file2 = args.extras.iter().position(not_flag).map(|i| args.extras.remove(i));
        }
    }

    let stdout = std::io::stdout().lock();
    let is_tty = stdout.is_terminal();
    if !is_tty {
        if args.color == AutoChoices::Auto {
            args.color = AutoChoices::Never;
        }
        if args.inline == AutoChoices::Auto {
            args.inline = AutoChoices::Never;
        }
    }

    args.style.insert_background();
    let style = style::Style{
        line_numbers: args.line_numbers,
        signs: args.signs,
        inline: args.inline != AutoChoices::Never && !args.exact,

        diff_matching: [args.style.diff_matching_left.as_bytes(), args.style.diff_matching_right.as_bytes()],
        diff_matching_inline: args.style.diff_matching_inline.as_bytes(),
        diff_non_matching: [args.style.diff_non_matching_left.as_bytes(), args.style.diff_non_matching_right.as_bytes()],
        ..style::Style::default()
    };

    let command;
    let mut diff_proc = if let Some((file1, file2)) = args.file1.zip(args.file2) {
        let mut diff_args;

        if let Some(filter) = args.filter {
            if args.label.is_empty() {
                args.label.push(format!("{} | {}", file1, filter));
            }
            if args.label.len() < 2 {
                args.label.push(format!("{} | {}", file2, filter));
            }

            // shell quote
            let file1 = shell_quote(file1);
            let file2 = shell_quote(file2);
            let label1 = shell_quote(&args.label[0]);
            let label2 = shell_quote(&args.label[1]);
            let extras = args.extras.iter().map(shell_quote).collect::<Vec<_>>().join(" ");
            command = format!("diff {extras} --label {label1} --label {label2} <( < {file1} {filter} ) <( < {file2} {filter} ) ");

            diff_args = vec!["bash", "-c", command.as_str()];

        } else {
            for l in args.label {
                args.extras.push(format!("--label={}", l))
            }
            diff_args = vec!["diff"];
            diff_args.extend(args.extras.iter().map(|x| x.as_str()));
            diff_args.push(file1.as_ref());
            diff_args.push(file2.as_ref());
        }

        let diff_proc = Command::new(diff_args[0])
            .args(&diff_args[1..])
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()?;
        Some(diff_proc)

    } else {
        None
    };

    let mut proc_stdin = diff_proc.as_mut().map(|p| BufReader::new(p.stdout.take().unwrap()));
    let mut stdin = std::io::stdin().lock();

    let mut hunk: Option<Hunk> = None;
    let mut tokeniser = tokeniser::Tokeniser::new();
    let mut line_numbers = [0, 0];
    let mut unified = false;
    let mut merge_markers: Option<hunk::MergeMarkers> = None;
    let mut filename: Option<Vec<u8>> = None;
    let mut stdout = BufWriter::new(stdout);

    let mut diff_trailing_ws_pat = regex::escape(&args.style.diff_trailing_ws).into_bytes();
    diff_trailing_ws_pat.extend(b"$0");

    let mut buf = Vec::<u8>::new();
    let mut diff = false;
    let mut side = 0;
    loop {
        buf.clear();

        match proc_stdin.as_mut().map(|x| x.read_until(b'\n', &mut buf)).unwrap_or_else(|| stdin.read_until(b'\n', &mut buf)) {
            Ok(0) => break,
            x => x?,
        };
        diff = true;

        if args.color == AutoChoices::Never {
            stdout.write_all(&buf)?;
            continue
        }

        let stripped = strip_style(&buf, b"");

        if let Some(captures) = byte_regex!(r"^((?<header>@@ -(?<line_minus>\d+)(,\d+)? \+(?<line_plus>\d+)(,\d+)? @@)\s*)(?<context>.*)".captures(&stripped)) {
            unified = true;
            merge_markers = None;
            if let Some(mut hunk) = hunk {
                hunk.print(&mut stdout, &mut tokeniser, line_numbers, merge_markers.as_ref(), style, &args.style)?;
            }
            args.style.print_background(&mut stdout)?;
            stdout.write_all(args.style.header.as_bytes())?;
            stdout.write_all(&captures["header"])?;
            if !captures["context"].is_empty() {
                stdout.write_all(b" ")?;
                stdout.write_all(args.style.context.as_bytes())?;
                stdout.write_all(&captures["context"])?;
            }
            stdout.write_all(style::RESET)?;
            stdout.write_all(b"\n")?;
            hunk = Some(Hunk::new());
            line_numbers = [
                std::str::from_utf8(&captures["line_minus"])?.parse()?,
                std::str::from_utf8(&captures["line_plus"])?.parse()?,
            ];
            continue
        }

        if let Some(captures) = byte_regex!(r"^((?<header>@@@ -(?<our_line_minus>\d+)(,\d+)? -(?<their_line_minus>\d+)(,\d+)? \+(?<line_plus>\d+)(,\d+)? @@@)\s*)(?<context>.*)".captures(&stripped)) {
            unified = true;
            merge_markers = Some(HashMap::new());
            if let Some(mut hunk) = hunk {
                hunk.print(&mut stdout, &mut tokeniser, line_numbers, merge_markers.as_ref(), style, &args.style)?;
            }
            args.style.print_background(&mut stdout)?;
            stdout.write_all(args.style.header.as_bytes())?;
            stdout.write_all(&captures["header"])?;
            stdout.write_all(b" ")?;
            stdout.write_all(args.style.context.as_bytes())?;
            stdout.write_all(&captures["context"])?;
            stdout.write_all(style::RESET)?;
            stdout.write_all(b"\n")?;
            hunk = Some(Hunk::new());
            line_numbers = [
                std::str::from_utf8(&captures["our_line_minus"])?.parse()?,
                std::str::from_utf8(&captures["line_plus"])?.parse()?,
            ];
            continue
        }

        if let Some(captures) = byte_regex!(r"^(?<line_minus>\d+)(,\d+)?[acd](?<line_plus>\d+)(,\d+)?".captures(&stripped)) {
            unified = false;
            merge_markers = None;
            if let Some(mut hunk) = hunk {
                hunk.print(&mut stdout, &mut tokeniser, line_numbers, merge_markers.as_ref(), style, &args.style)?;
            }
            args.style.print_background(&mut stdout)?;
            stdout.write_all(args.style.header.as_bytes())?;
            stdout.write_all(&buf)?;
            stdout.write_all(style::RESET)?;
            hunk = Some(Hunk::new());
            line_numbers = [
                std::str::from_utf8(&captures["line_minus"])?.parse()?,
                std::str::from_utf8(&captures["line_plus"])?.parse()?,
            ];
            continue
        }


        if let Some(captures) =
            byte_regex!("^(?<header>diff( -r| --recursive)?) (?<filename1>[^\"\\s-][^\"\\s]+|\"(\\\\.|.)*\") (?<filename2>[^\"\\s]+|\"(\\\\.|.)*\")(?<trailer>.*)".captures(&stripped))
            .or_else(||
                byte_regex!("^(?<header>diff( --git| --cc)) (?<filename1>a/.*) (?<filename2>b/.*)(?<trailer>.*)".captures(&stripped))
            )
        {
            if let Some(mut hunk) = hunk {
                hunk.print(&mut stdout, &mut tokeniser, line_numbers, merge_markers.as_ref(), style, &args.style)?;
            }
            args.style.print_background(&mut stdout)?;
            stdout.write_all(style::DIFF_HEADER.as_bytes())?;
            stdout.write_all(&captures["header"])?;
            stdout.write_all(b" ")?;
            stdout.write_all(style::RESET)?;
            stdout.write_all(args.style.filename_header_left.as_bytes())?;
            stdout.write_all(&captures["filename1"])?;
            stdout.write_all(b" ")?;
            stdout.write_all(args.style.filename_header_right.as_bytes())?;
            stdout.write_all(&captures["filename2"])?;
            stdout.write_all(style::RESET)?;
            stdout.write_all(&captures["trailer"])?;
            stdout.write_all(b"\n")?;
            hunk = Some(Hunk::new());
            continue
        }

        if hunk.is_none() {
            if let Some(captures) = byte_regex!(r"^(?<sign>---|\+\+\+) ([ab]/)?(?<filename>[^\t]*)(?<trailer>\t.*)?".captures(&stripped)) {
                if &captures["sign"] == b"---" {
                    filename = Some(captures["filename"].to_owned());
                } else {
                    Hunk::print_filename(
                        &mut stdout,
                        &mut tokeniser,
                        filename.as_ref().map(|f| f.as_ref()),
                        Some(&captures["filename"]),
                        (&args.style.filename_sign_left, &args.style.filename_sign_right, &args.style.filename_sign),
                        style,
                        &args.style,
                    )?;
                }
                continue
            }

            args.style.print_background(&mut stdout)?;
            if byte_regex!(r"^commit [0-9a-f]+".is_match(&stripped)) {
                stdout.write_all(args.style.commit.as_bytes())?;
                stdout.write_all(&strip_style(&buf, format!("$0{}", args.style.commit).as_bytes()))?;
                stdout.write_all(style::RESET)?;
            } else {
                stdout.write_all(&buf)?;
            }
            continue
        }

        let h = hunk.as_mut().unwrap();

        if unified && merge_markers.is_some() {
            if let Some(captures) = byte_regex!(r"^(?<sign>[-+] | [-+]|[-+]{2})(?<line>.*\n)".captures(&stripped)) {
                let sign = &captures["sign"];
                side = if sign.contains(&b'+') { 1 } else { 0 };
                let lineno = line_numbers[side] + h.get(side).len();
                h.get_mut(side).push(captures["line"].to_owned());
                let bar = if sign[1] == b' ' {
                    &args.style.lineno_our_bar
                } else if sign[0] == b' ' {
                    &args.style.lineno_their_bar
                } else {
                    &args.style.lineno_merge_bar
                };
                merge_markers.as_mut().unwrap().insert((side, lineno), bar.to_string());
                continue
            }
        }

        if args.exact && stripped.starts_with(b" ") {
            h.print(&mut stdout, &mut tokeniser, line_numbers, merge_markers.as_ref(), style, &args.style)?;
            args.style.print_background(&mut stdout)?;
            if style.line_numbers {
                stdout.write_all(style::format_lineno(
                        line_numbers,
                        Some(&args.style.lineno), Some(&args.style.lineno),
                        None,
                ).as_ref())?;
            }
            if style.signs {
                stdout.write_all(style::SIGN[2])?;
            }
            stdout.write_all(args.style.diff_context.as_bytes())?;
            stdout.write_all(&byte_regex!(r"\s+\n".replace_all(&stripped[1..], &diff_trailing_ws_pat)))?;

            hunk = Some(Hunk::new());
            line_numbers[0] += 1;
            line_numbers[1] += 1;
            continue
        }


        if h.is_empty() {
            if let Some(captures) = byte_regex!(r"^rename (?<sign>to|from)[ \t](?<filename>.*\n)".captures(&stripped)) {
                if &captures["sign"] == b"from" {
                    filename = Some(captures["filename"].to_owned());
                } else {
                    Hunk::print_filename(
                        &mut stdout,
                        &mut tokeniser,
                        filename.as_ref().map(|f| f.as_ref()),
                        Some(&captures["filename"]),
                        ("rename from\t", "rename to\t", "rename from/to\t"),
                        style,
                        &args.style,
                    )?;
                }
                continue
            }
        }

        if &*stripped == b"\\ No newline at end of file\n" || &*stripped == b"\\ No newline at end of file" {
            if let Some(last_line) = h.get_mut(side).last_mut() {
                if last_line.ends_with(b"\n") {
                    last_line.pop();
                }
            }
            continue
        }

        if unified {
            if let Some(captures) = byte_regex!(r"^(?<sign>[-+])(?<line>.*\n)".captures(&stripped)) {
                side = if &captures["sign"] == b"+" { 1 } else { 0 };
                h.get_mut(side).push(captures["line"].to_owned());
                continue
            }
        }

        if !args.exact && unified && stripped.starts_with(b" ") {
            h.left.push(stripped[1..].to_owned());
            h.right.push(stripped[1..].to_owned());
            continue
        }

        if !unified {
            if *stripped == *b"---\n" {
                continue
            }

            if let Some(captures) = byte_regex!(r"^(?<sign>[<>]) (?<line>.*\n)".captures(&stripped)) {
                side = if &captures["sign"] == b">" { 1 } else { 0 };
                h.get_mut(side).push(captures["line"].to_owned());
                continue
            }
        }

        if &buf == b"\n" {
            h.print(&mut stdout, &mut tokeniser, line_numbers, merge_markers.as_ref(), style, &args.style)?;
            hunk = None;
            continue
        }

        h.print(&mut stdout, &mut tokeniser, line_numbers, merge_markers.as_ref(), style, &args.style)?;
        if byte_regex!("^index ".is_match(&stripped)) {
            args.style.print_background(&mut stdout)?;
            stdout.write_all(&strip_style(&buf, format!("$0{}", style::DIFF_HEADER).as_bytes()))?;
            hunk = None;
            continue
        }

        hunk = Some(Hunk::new());
        stdout.write_all(&stripped)?;
    }

    if let Some(mut hunk) = hunk {
        hunk.print(&mut stdout, &mut tokeniser, line_numbers, merge_markers.as_ref(), style, &args.style)?;
    }

    if let Some(mut diff_proc) = diff_proc {
        if let Some(code) = diff_proc.try_wait()?.and_then(|x| x.code()) {
            return if code <= u8::MAX as _ {
                Ok(ExitCode::from(code as u8))
            } else {
                Ok(ExitCode::FAILURE)
            }
        }
    }

    if diff {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn main() -> Result<ExitCode> {
    let result = _main();

    if let Err(e) = &result {
        if let Some(e) = e.downcast_ref::<std::io::Error>() {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                return Ok(ExitCode::from(141))
            }
        }
    }
    result
}
