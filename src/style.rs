use super::types::*;

#[derive(Copy, Clone)]
pub struct Style<'a> {
    pub line_numbers: bool,
    pub signs: bool,
    pub show_both: bool,
    pub inline: bool,

    pub diff_matching: [Bytes<'a>; 2],
    pub diff_non_matching: [Bytes<'a>; 2],
}

impl<'a> std::default::Default for Style<'a> {
    fn default() -> Self {
        Self{
            line_numbers: true,
            signs: false,
            show_both: false,
            inline: false,
            diff_matching: DIFF_MATCHING,
            diff_non_matching: DIFF_NON_MATCHING,
        }
    }
}

pub const RESET: Bytes = b"\x1b[0m";
pub const HEADER: Bytes            = b"\x1b[0;36m";
pub const COMMIT: &str            = "\x1b[1;48;5;24m";
pub const CONTEXT: Bytes           = b"\x1b[0;1;33;48;5;236m";
pub const DIFF_HEADER: &str       = "\x1b[0;1m";
pub const FILENAME_HEADER: (Bytes, Bytes, Bytes)   = (b"\x1b[0;1;31;48;5;238m", b"\x1b[0;1;32;48;5;238m", b"");
pub const FILENAME: (Bytes, Bytes, Bytes)          = (b"\x1b[0;31m", b"\x1b[0;32m", b"");
pub const DIFF: (Bytes, Bytes)              = (b"\x1b[0;31m", b"\x1b[0;32m");
// pub const DIFF_MATCHING =     (b"\x1b[0;38;2;250;100;100m", b"\x1b[0;38;2;100;250;100m");
pub const DIFF_MATCHING: [Bytes; 2]     = [b"\x1b[0;38;2;220;190;210;48;2;35;20;20m", b"\x1b[0;38;2;190;220;210;48;2;20;35;20m"];
// pub const DIFF_MATCHING =     (b"\x1b[0;38;5;225m", b"\x1b[0;38;5;195m");
// pub const DIFF_MATCHING =     (b"\x1b[0m", b"\x1b[0m");
// pub const DIFF_NON_MATCHING = (b"\x1b[0;48;2;200;80;80;38;5;235m", b"\x1b[0;48;2;80;200;80;38;5;235m");
pub const DIFF_NON_MATCHING: [Bytes; 2] = [b"\x1b[0;1;31;48;2;80;30;30m", b"\x1b[0;1;32;48;2;25;80;25m"];
// pub const DIFF_NON_MATCHING =     (b"\x1b[0;48;2;60;0;0m", b"\x1b[0;48;2;0;60;0m");
// pub const DIFF_NON_MATCHING = (b"\x1b[0;41;38;5;235m", b"\x1b[0;42;38;5;235m");
pub const DIFF_TRAILING_WS: Bytes  = b"\x1b[2;7m$0";
pub const DIFF_INSERT: [Bytes; 2]       = [b"\x1b[4:3:58:5:10m", b"\x1b[4:3;58;5;9m"];
// pub const DIFF_INSERT =       (b"\x1b[0;7;2;48;5;211;38;5;52m", b"\x1b[0;7;2;48;5;158;38;5;22m");
pub const DIFF_CONTEXT: Bytes      = b"\x1b[0;38;5;242m";
pub const SIGN: [Bytes; 3]              = [b"-", b"+", RESET];
pub const FILENAME_SIGN: (&str, &str)     = ("\x1b[0;31;48;5;238;7m---\x1b[27m ", "\x1b[0;32;48;5;238;7m+++\x1b[27m ");
pub const LINENO: &str            = "\x1b[0;38;5;242m";
pub const LINENO_BAR: &str        = "\x1b[0;38;5;242m▏";
pub const LINENO_OUR_BAR: &str    = "\x1b[0;38;5;187m(";
pub const LINENO_THEIR_BAR: &str  = "\x1b[0;38;5;117m)";
pub const LINENO_MERGE_BAR: &str  = "\x1b[0;38;5;13;1m|";
pub const LINENO_DIFF: (&str, &str)       = ("\x1b[0;31m", "\x1b[0;32m");

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
