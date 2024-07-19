use std::io::{BufRead, BufWriter, Write, IsTerminal};
use std::collections::HashMap;
use clap::Parser;
use regex::bytes::Regex;
use anyhow::{Result};

mod hunk;
mod style;
use hunk::Hunk;

const RESET: &[u8] = b"\x1b[0m";

fn strip_style<'a>(string: &'a [u8], replace: &[u8]) -> std::borrow::Cow<'a, [u8]> {
    let regex = Regex::new(r"\x1b\[[\d;]*m").unwrap();
    return regex.replace_all(string, replace);
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

    let mut stdout = std::io::stdout().lock();
    let mut stdin = std::io::stdin().lock();

    let is_tty = stdout.is_terminal();
    if !is_tty && args.color == ColorChoices::Auto {
        args.color = ColorChoices::Never;
    }

    let line_numbers = if args.line_numbers {
        style::LINENO_FORMAT
    } else {
        b""
    };
    let signs = if args.signs {
        style::SIGN
    } else {
        (b"" as _, b"" as _, b"" as _)
    };

    if let Some((file1, file2)) = args.file1.zip(args.file2) {
        if let Some(filter) = args.filter {
            if args.label.len() < 1 {
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
    let mut merge_markers: Option<HashMap<(usize, usize), Vec<u8>>> = None;
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
            stdout.write(&buf)?;
            continue
        }

        let stripped = strip_style(&buf, b"");

        let regex = Regex::new(r"((?<header>@@ -(?<line_minus>\d+)(,\d+)? \+(?<line_plus>\d+)(,\d+)? @@)\s*)(?<context>.*)").unwrap();
        if let Some(captures) = regex.captures(&stripped) {
            unified = true;
            merge_markers = None;
            // print_hunk(hunk, line_numbers, merge_markers)
            stdout.write(style::HEADER)?;
            stdout.write(&captures["header"])?;
            if captures["context"].len() > 0 {
                stdout.write(b" ")?;
                stdout.write(style::CONTEXT)?;
                stdout.write(&captures["context"])?;
            }
            stdout.write(RESET)?;
            stdout.write(b"\n")?;
            hunk = Some(Hunk::new());
            line_numbers = [
                std::str::from_utf8(&captures["line_minus"])?.parse()?,
                std::str::from_utf8(&captures["line_plus"])?.parse()?,
            ];
            continue
        }

        let regex = Regex::new(r"((?<header>@@@ -(?<our_line_minus>\d+)(,\d+)? -(?<their_line_minus>\d+)(,\d+)? \+(?<line_plus>\d+)(,\d+)? @@@)\s*)(?<context>.*)").unwrap();
        if let Some(captures) = regex.captures(&stripped) {
            unified = true;
            merge_markers = Some(HashMap::new());
            // print_hunk(hunk, line_numbers, merge_markers)
            stdout.write(style::HEADER)?;
            stdout.write(&captures["header"])?;
            stdout.write(b" ")?;
            stdout.write(style::CONTEXT)?;
            stdout.write(&captures["context"])?;
            stdout.write(RESET)?;
            stdout.write(b"\n")?;
            hunk = Some(Hunk::new());
            line_numbers = [
                std::str::from_utf8(&captures["our_line_minus"])?.parse()?,
                std::str::from_utf8(&captures["line_plus"])?.parse()?,
            ];
            continue
        }

        let regex = Regex::new(r"(?<line_minus>\d+)(,\d+)?[acd](?<line_plus>\d+)(,\d+)?$").unwrap();
        if let Some(captures) = regex.captures(&stripped) {
            unified = false;
            merge_markers = None;
            // print_hunk(hunk, line_numbers, merge_markers);
            stdout.write(style::HEADER)?;
            stdout.write(&buf)?;
            stdout.write(RESET)?;
            hunk = Some(Hunk::new());
            line_numbers = [
                std::str::from_utf8(&captures["line_minus"])?.parse()?,
                std::str::from_utf8(&captures["line_plus"])?.parse()?,
            ];
            continue
        }

        let regex = Regex::new("(?<header>diff( -r| --recursive| --git)?) (?<filename1>[^-\"\\s][^\"\\s]+|\"(\\\\.|.)*\") (?<filename2>[^\"\\s]+|\"(\\\\.|.)*\")(?<trailer>.*)$").unwrap();
        if let Some(captures) = regex.captures(&stripped) {
            // print_hunk(hunk, line_numbers, merge_markers)
            stdout.write(style::DIFF_HEADER.as_bytes())?;
            stdout.write(&captures["header"])?;
            stdout.write(b" ")?;
            stdout.write(RESET)?;
            stdout.write(style::FILENAME_HEADER.0)?;
            stdout.write(&captures["filename1"])?;
            stdout.write(b" ")?;
            stdout.write(style::FILENAME_HEADER.1)?;
            stdout.write(&captures["filename2"])?;
            stdout.write(RESET)?;
            stdout.write(&captures["trailer"])?;
            stdout.write(b"\n")?;
            hunk = Some(Hunk::new());
            continue
        }

        if hunk.is_none() {
            let regex = Regex::new(r"(?<sign>---|\+\+\+) ([ab]/)?(?<filename>.*?)(?<trailer>\t.*)?$").unwrap();
            if let Some(captures) = regex.captures(&stripped) {
                if &captures["sign"] == b"---" {
                    filename = Some(captures["filename"].to_owned());
                } else {
                    // print_filename(filename, match.group('filename'), STYLE['filename_sign'])
                }
                continue
            }

            let regex = Regex::new(r"commit [0-9a-f]+").unwrap();
            if let Some(captures) = regex.captures(&stripped) {
                stdout.write(&strip_style(&buf, format!("$0{}", style::COMMIT).as_bytes()))?;
            } else {
                stdout.write(&buf)?;
            }
            continue
        }

        let h = hunk.as_mut().unwrap();

        let regex = Regex::new(r"(?<sign>[-+] | [-+]|[-+]{2})(?<line>.*\n)").unwrap();
        if unified && merge_markers.is_some() {
            if let Some(captures) = regex.captures(&stripped) {
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
            // print_hunk(hunk, line_numbers, merge_markers)
            hunk = Some(Hunk::new());
            let regex = Regex::new(r"\s+\n").unwrap();
            let line = regex.replace_all(&stripped[1..], format!("{}$0", style::DIFF_TRAILING_WS).as_bytes());
            // stdout.write(format_lineno(*line_numbers, minus_style=STYLE['lineno'], plus_style=STYLE['lineno']) + STYLE['sign'][2] + RESET + STYLE['diff_context'] + line)
            line_numbers[0] += 1;
            line_numbers[1] += 1;
            continue
        }


        let regex = Regex::new(r"rename (?<sign>to|from) (?<filename>.*\n)").unwrap();
        if h.is_empty() {
            if let Some(captures) = regex.captures(&stripped) {
                if &captures["sign"] == b"from" {
                    filename = Some(captures["filename"].to_owned());
                } else {
                    // print_filename(filename, match.group('filename'), (b'rename from\t', b'rename to\t'), style={
                        // **STYLE,
                        // 'filename_header': STYLE['filename'],
                    // })
                }
                continue
            }
        }

        if *stripped == *b"\\ No newline at end of file\n" {
            // print_hunk(hunk, line_numbers, merge_markers)
            if !h.left.is_empty() {
                stdout.write(style::DIFF.0)?;
            }
            if !h.right.is_empty() {
                stdout.write(style::DIFF.1)?;
            }
            stdout.write(&buf)?;
            hunk = Some(Hunk::new());
            continue
        }

        if unified {
            let regex = Regex::new(r"(?<sign>[-+])(?<line>.*\n)").unwrap();
            if let Some(captures) = regex.captures(&stripped) {
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

            let regex = Regex::new(r"(?<sign>[<>]) (?<line>.*\n)").unwrap();
            if let Some(captures) = regex.captures(&stripped) {
                let side = if &captures["sign"] == b">" { 1 } else { 0 };
                h.get_mut(side).push(captures["line"].to_owned());
                continue
            }
        }

        if &buf == b"\n" {
            // print_hunk(hunk, line_numbers, merge_markers)
            hunk = None;
            continue
        }

        // print_hunk(hunk, line_numbers, merge_markers)
        let regex = Regex::new("index ").unwrap();
        if regex.is_match(&stripped) {
            stdout.write(&strip_style(&buf, format!("$0{}", style::DIFF_HEADER).as_bytes()))?;
            hunk = None;
            continue
        }

        hunk = Some(Hunk::new());
        stdout.write(&stripped)?;
    }

    // print_hunk(hunk, line_numbers, merge_markers)

    // if hasattr(proc, 'returncode'):
        // return proc.returncode
    // if line:
        // return 1

    Ok(())
}
