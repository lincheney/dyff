use std::io::{BufRead, BufWriter, Write, IsTerminal};
use std::collections::HashMap;
use clap::Parser;
use anyhow::{Result};

mod hunk;
mod style;
mod block_maker;
mod part;
mod word_differ;
mod block;
mod types;
#[macro_use]
mod regexes;
use hunk::Hunk;

fn strip_style<'a>(string: &'a [u8], replace: &[u8]) -> std::borrow::Cow<'a, [u8]> {
    regex!(r"\x1b\[[\d;]*m".replace_all(string, replace))
}

#[derive(Clone, PartialEq, Debug, clap::ValueEnum)]
enum ColorChoices {
    Never,
    Auto,
    Always,
}

#[derive(Debug, clap::Parser)]
#[command(name = "diff")]
struct Cli {

    #[arg(long, value_enum, default_value_t = ColorChoices::Always)]
    color: ColorChoices,

    #[arg(short = 'N', long = "no-line-numbers", action = clap::ArgAction::SetFalse)]
    line_numbers: bool,

    #[arg(short, long)]
    signs: bool,

    #[arg(long)]
    exact: bool,

    #[arg(short, long)]
    filter: Option<String>,

    /// use LABEL instead of file name and timestamp (can be repeated)
    #[arg(long)]
    label: Vec<String>,

    file1: Option<String>,
    file2: Option<String>,

    #[arg(allow_hyphen_values = true)]
    extras: Vec<String>,
}

fn main() -> Result<()> {
    let mut args = Cli::parse();

    let stdout = std::io::stdout().lock();
    let mut stdin = std::io::stdin().lock();

    let is_tty = stdout.is_terminal();
    if !is_tty && args.color == ColorChoices::Auto {
        args.color = ColorChoices::Never;
    }

    let style = style::Style{
        line_numbers: args.line_numbers,
        signs: args.signs,
        ..style::Style::default()
    };

    if let Some((file1, file2)) = args.file1.zip(args.file2) {
        if let Some(filter) = args.filter {
            if args.label.is_empty() {
                args.label.push(format!("{} | {}", file1, filter));
            }
            if args.label.len() < 2 {
                args.label.push(format!("{} | {}", file2, filter));
            }
            // diff_args = ' '.join(map(shlex.quote, [
                // 'diff', *extras,
                // '--label', args.label[0],
                // '--label', args.label[1],
            // ]))
            // diff_args = ['bash', '-c', f'{diff_args} <(set -- {shlex.quote(args.file1)}; cat "$1" | {args.filter}) <(set -- {shlex.quote(args.file2)}; cat "$1" | {args.filter})']
        } else {
            for l in args.label {
                args.extras.push(format!("--label={}", l))
            }
            // diff_args = ['diff', *extras, args.file1, args.file2]
        }

        // diff_proc = subprocess.Popen(diff_args, stdout=subprocess.PIPE, close_fds=False)
    } else {
        // diff_proc = contextlib.nullcontext(SimpleNamespace(stdout=sys.stdin.buffer))
    }

    let mut hunk: Option<Hunk> = None;
    let mut line_numbers = [0, 0];
    let mut unified = false;
    let mut merge_markers: Option<hunk::MergeMarkers> = None;
    let mut filename: Option<Vec<u8>> = None;
    let mut stdout = BufWriter::new(stdout);

    let mut buf = Vec::<u8>::new();
    loop {
        buf.clear();
        match stdin.read_until(b'\n', &mut buf) {
            Ok(0) => break,
            x => x?,
        };

        if args.color == ColorChoices::Never {
            stdout.write_all(&buf)?;
            continue
        }

        let stripped = strip_style(&buf, b"");

        if let Some(captures) = regex!(r"^((?<header>@@ -(?<line_minus>\d+)(,\d+)? \+(?<line_plus>\d+)(,\d+)? @@)\s*)(?<context>.*)".captures(&stripped)) {
            unified = true;
            merge_markers = None;
            if let Some(mut hunk) = hunk {
                hunk.print(&mut stdout, line_numbers, merge_markers.as_ref(), style)?;
            }
            stdout.write_all(style::HEADER)?;
            stdout.write_all(&captures["header"])?;
            if !captures["context"].is_empty() {
                stdout.write_all(b" ")?;
                stdout.write_all(style::CONTEXT)?;
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

        if let Some(captures) = regex!(r"^((?<header>@@@ -(?<our_line_minus>\d+)(,\d+)? -(?<their_line_minus>\d+)(,\d+)? \+(?<line_plus>\d+)(,\d+)? @@@)\s*)(?<context>.*)".captures(&stripped)) {
            unified = true;
            merge_markers = Some(HashMap::new());
            if let Some(mut hunk) = hunk {
                hunk.print(&mut stdout, line_numbers, merge_markers.as_ref(), style)?;
            }
            stdout.write_all(style::HEADER)?;
            stdout.write_all(&captures["header"])?;
            stdout.write_all(b" ")?;
            stdout.write_all(style::CONTEXT)?;
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

        if let Some(captures) = regex!(r"^(?<line_minus>\d+)(,\d+)?[acd](?<line_plus>\d+)(,\d+)?$".captures(&stripped)) {
            unified = false;
            merge_markers = None;
            if let Some(mut hunk) = hunk {
                hunk.print(&mut stdout, line_numbers, merge_markers.as_ref(), style)?;
            }
            stdout.write_all(style::HEADER)?;
            stdout.write_all(&buf)?;
            stdout.write_all(style::RESET)?;
            hunk = Some(Hunk::new());
            line_numbers = [
                std::str::from_utf8(&captures["line_minus"])?.parse()?,
                std::str::from_utf8(&captures["line_plus"])?.parse()?,
            ];
            continue
        }

        if let Some(captures) = regex!("^(?<header>diff( -r| --recursive| --git)?) (?<filename1>[^\"\\s-][^\"\\s]+|\"(\\\\.|.)*\") (?<filename2>[^\"\\s]+|\"(\\\\.|.)*\")(?<trailer>.*)".captures(&stripped)) {
            if let Some(mut hunk) = hunk {
                hunk.print(&mut stdout, line_numbers, merge_markers.as_ref(), style)?;
            }
            stdout.write_all(style::DIFF_HEADER.as_bytes())?;
            stdout.write_all(&captures["header"])?;
            stdout.write_all(b" ")?;
            stdout.write_all(style::RESET)?;
            stdout.write_all(style::FILENAME_HEADER.0)?;
            stdout.write_all(&captures["filename1"])?;
            stdout.write_all(b" ")?;
            stdout.write_all(style::FILENAME_HEADER.1)?;
            stdout.write_all(&captures["filename2"])?;
            stdout.write_all(style::RESET)?;
            stdout.write_all(&captures["trailer"])?;
            stdout.write_all(b"\n")?;
            hunk = Some(Hunk::new());
            continue
        }

        if hunk.is_none() {
            if let Some(captures) = regex!(r"^(?<sign>---|\+\+\+) ([ab]/)?(?<filename>[^\t]*)(?<trailer>\t.*)?".captures(&stripped)) {
                if &captures["sign"] == b"---" {
                    filename = Some(captures["filename"].to_owned());
                } else {
                    Hunk::print_filename(
                        &mut stdout,
                        filename.as_ref().map(|f| f.as_ref()),
                        Some(&captures["filename"]),
                        style::FILENAME_SIGN,
                        style,
                    )?;
                }
                continue
            }

            if regex!(r"^commit [0-9a-f]+".is_match(&stripped)) {
                stdout.write_all(style::COMMIT.as_bytes())?;
                stdout.write_all(&strip_style(&buf, format!("$0{}", style::COMMIT).as_bytes()))?;
                stdout.write_all(style::RESET)?;
            } else {
                stdout.write_all(&buf)?;
            }
            continue
        }

        let h = hunk.as_mut().unwrap();

        if unified && merge_markers.is_some() {
            if let Some(captures) = regex!(r"^(?<sign>[-+] | [-+]|[-+]{2})(?<line>.*\n)".captures(&stripped)) {
                let sign = &captures["sign"];
                let side = if sign.contains(&b'+') { 1 } else { 0 };
                let lineno = line_numbers[side] + h.get(side).len();
                h.get_mut(side).push(captures["line"].to_owned());
                let bar = if sign[1] == b' ' {
                    style::LINENO_OUR_BAR
                } else if sign[0] == b' ' {
                    style::LINENO_THEIR_BAR
                } else {
                    style::LINENO_MERGE_BAR
                };
                merge_markers.as_mut().unwrap().insert((side, lineno), bar.into());
                continue
            }
        }

        if args.exact && stripped.starts_with(b" ") {
            h.print(&mut stdout, line_numbers, merge_markers.as_ref(), style)?;
            stdout.write_all(style::format_lineno(
                Some(line_numbers[0]), Some(line_numbers[1]),
                Some(style::LINENO), Some(style::LINENO),
                None,
            ).as_ref())?;
            if style.signs {
                stdout.write_all(style::SIGN[2])?;
            }
            stdout.write_all(style::RESET)?;
            stdout.write_all(&regex!(r"\s+\n".replace_all(&stripped[1..], style::DIFF_TRAILING_WS)))?;

            hunk = Some(Hunk::new());
            line_numbers[0] += 1;
            line_numbers[1] += 1;
            continue
        }


        if h.is_empty() {
            if let Some(captures) = regex!(r"^rename (?<sign>to|from) (?<filename>.*\n)".captures(&stripped)) {
                if &captures["sign"] == b"from" {
                    filename = Some(captures["filename"].to_owned());
                } else {
                    Hunk::print_filename(
                        &mut stdout,
                        filename.as_ref().map(|f| f.as_ref()),
                        Some(&captures["filename"]),
                        ("rename from\t", "rename to\t"),
                        style,
                    )?;
                    // 'filename_header': STYLE['filename'],
                }
                continue
            }
        }

        if *stripped == *b"\\ No newline at end of file\n" {
            h.print(&mut stdout, line_numbers, merge_markers.as_ref(), style)?;
            if !h.left.is_empty() {
                stdout.write_all(style::DIFF.0)?;
            }
            if !h.right.is_empty() {
                stdout.write_all(style::DIFF.1)?;
            }
            stdout.write_all(&buf)?;
            hunk = Some(Hunk::new());
            continue
        }

        if unified {
            if let Some(captures) = regex!(r"^(?<sign>[-+])(?<line>.*\n)".captures(&stripped)) {
                let side = if &captures["sign"] == b"+" { 1 } else { 0 };
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

            if let Some(captures) = regex!(r"^(?<sign>[<>]) (?<line>.*\n)".captures(&stripped)) {
                let side = if &captures["sign"] == b">" { 1 } else { 0 };
                h.get_mut(side).push(captures["line"].to_owned());
                continue
            }
        }

        if &buf == b"\n" {
            h.print(&mut stdout, line_numbers, merge_markers.as_ref(), style)?;
            hunk = None;
            continue
        }

        h.print(&mut stdout, line_numbers, merge_markers.as_ref(), style)?;
        if regex!("^index ".is_match(&stripped)) {
            stdout.write_all(&strip_style(&buf, format!("$0{}", style::DIFF_HEADER).as_bytes()))?;
            hunk = None;
            continue
        }

        hunk = Some(Hunk::new());
        stdout.write_all(&stripped)?;
    }

    if let Some(mut hunk) = hunk {
        hunk.print(&mut stdout, line_numbers, merge_markers.as_ref(), style)?;
    }

    // if hasattr(proc, 'returncode'):
        // return proc.returncode
    // if line:
        // return 1

    Ok(())
}
