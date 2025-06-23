use super::types::*;

#[derive(Copy, Clone, Debug)]
pub struct Style<'a> {
    pub line_numbers: bool,
    pub signs: bool,
    pub show_both: bool,
    pub inline: bool,

    pub diff_matching: [Bytes<'a>; 2],
    pub diff_matching_inline: Bytes<'a>,
    pub diff_non_matching: [Bytes<'a>; 2],
}

impl std::default::Default for Style<'_> {
    fn default() -> Self {
        Self{
            line_numbers: true,
            signs: false,
            show_both: false,
            inline: false,
            diff_matching: [DIFF_MATCHING[0].as_bytes(), DIFF_MATCHING[1].as_bytes()],
            diff_matching_inline: DIFF_MATCHING_INLINE.as_bytes(),
            diff_non_matching: [DIFF_NON_MATCHING[0].as_bytes(), DIFF_NON_MATCHING[1].as_bytes()],
        }
    }
}

macro_rules! concat_bytes {
    (_, $A:expr, $B:expr) => {{
        const LEN: usize = $A.len() + $B.len();
        const fn combine(a: &'static [u8], b: &'static [u8]) -> [u8; LEN] {
            let out = [0u8; LEN];
            let out = copy_slice(a, out, 0);
            let out = copy_slice(b, out, a.len());
            out
        }
        const fn copy_slice(input: &[u8], mut output: [u8; LEN], offset: usize) -> [u8; LEN] {
            let mut index = 0;
            while index < input.len() {
                output[offset+index] = input[index];
                index += 1;
            }
            output
        }
        &combine($A, $B)
    }};

    ($A:expr, $B:expr, $($rest:expr),+) => {{
        concat_bytes!(concat_bytes!($A, $B), $($rest),*)
    }};
    ($A:expr, $B:expr) => {{
        concat_bytes!(_, $A, $B)
    }};
}

const fn bytes_to_str(bytes: &[u8]) -> &str {
    let Ok(result) = std::str::from_utf8(bytes) else {
        panic!(concat!("invalid utf8: ", stringify!(bytes)));
    };
    result
}

macro_rules! concat_str {
    ($($expr:expr),+) => {{
        bytes_to_str(concat_bytes!($($expr.as_bytes()),+))
    }};
}

pub const RESET: Bytes      = b"\x1b[0m";
pub const BOLD: &str        = "\x1b[1m";
pub const HEADER: &str      = "\x1b[0;36m";
pub const COMMIT: &str      = "\x1b[1;48;5;24m";
pub const CONTEXT: &str     = "\x1b[0;1;33;48;5;236m";
pub const DIFF_HEADER: &str = BOLD;
pub const SIGN: [Bytes; 3]  = [
    concat_bytes!(RESET, DIFF.0.as_bytes(), b"-"),
    concat_bytes!(RESET, DIFF.1.as_bytes(), b"+"),
    concat_bytes!(RESET, b" "),
];

pub const DIFF: (&str, &str)   = ("\x1b[0;31m", "\x1b[0;32m");

pub const LINENO: &str              = "\x1b[0;38;5;242m";
pub const LINENO_BAR: &str          = concat_str!(LINENO, "▏");
pub const LINENO_OUR_BAR: &str      = "\x1b[0;38;5;187m(";
pub const LINENO_THEIR_BAR: &str    = "\x1b[0;38;5;117m)";
pub const LINENO_MERGE_BAR: &str    = "\x1b[0;38;5;13;1m|";
pub const LINENO_DIFF: (&str, &str) = DIFF;

pub const FILENAME: (&str, &str, &str)        = (DIFF.0, DIFF.1, "");
const FILENAME_BG: &str                       = "\x1b[48;5;238m";
pub const FILENAME_RENAME: &str               = concat_str!("\x1b[0m", "\x1b[48;5;238m");
pub const FILENAME_HEADER: (&str, &str, &str) = (
    concat_str!(FILENAME.0, BOLD, FILENAME_BG),
    concat_str!(FILENAME.1, BOLD, FILENAME_BG),
    "",
);
pub const FILENAME_SIGN: (&str, &str, &str)   = (
    concat_str!(DIFF.0, FILENAME_BG, "\x1b[7m---\x1b[27m "),
    concat_str!(DIFF.1, FILENAME_BG, "\x1b[7m+++\x1b[27m "),
    concat_str!(        FILENAME_BG, "\x1b[7m###\x1b[27m "),
);
pub const FILENAME_NON_MATCHING: [&str; 2] = [
    concat_str!(DIFF_NON_MATCHING[0], FILENAME_BG, "\x1b[1m"),
    concat_str!(DIFF_NON_MATCHING[1], FILENAME_BG, "\x1b[1m"),
];

pub const DIFF_MATCHING: [&str; 2] = [
    "\x1b[0;38;2;220;190;210;48;2;35;20;20m",
    "\x1b[0;38;2;190;220;210;48;2;20;35;20m",
];
pub const DIFF_NON_MATCHING: [&str; 2] = [
    concat_str!(DIFF.0, "\x1b[1;48;2;80;30;30m"),
    concat_str!(DIFF.1, "\x1b[1;48;2;25;80;25m"),
];
pub const DIFF_INSERT: [&str; 2] = [
    "\x1b[4:3:58:5:10m",
    "\x1b[4:3;58;5;9m",
];
pub const DIFF_MATCHING_INLINE: &str = "\x1b[0;38;5;252m";
pub const DIFF_CONTEXT: &str = LINENO;
pub const DIFF_TRAILING_WS: &str = "\x1b[2;7m";

pub fn format_lineno(
    [num1, num2]: [usize; 2],
    left_style: Option<&str>,
    right_style: Option<&str>,
    bar_style: Option<&str>,
) -> String {
    let num1 = if num1 != 0 { Some(num1.to_string()) } else { None };
    let num2 = if num2 != 0 { Some(num2.to_string()) } else { None };

    format!(
        "{}{:<4}{}{}{:<4}{} ",
        left_style.unwrap_or(LINENO_DIFF.0),
        num1.as_ref().map(|n| n.as_ref()).unwrap_or(""),
        bar_style.unwrap_or(LINENO_BAR),
        right_style.unwrap_or(LINENO_DIFF.1),
        num2.as_ref().map(|n| n.as_ref()).unwrap_or(""),
        bar_style.unwrap_or(LINENO_BAR),
    )
}
