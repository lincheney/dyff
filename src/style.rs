type Bytes<'a> = &'a [u8];

pub const RESET: Bytes = b"\x1b[0m";
pub const HEADER: Bytes            = b"\x1b[0;36m";
pub const COMMIT: &str            = "\x1b[1;48;5;24m";
pub const CONTEXT: Bytes           = b"\x1b[0;1;33;48;5;236m";
pub const DIFF_HEADER: &str       = "\x1b[0;1m";
pub const FILENAME_HEADER: (Bytes, Bytes, Bytes)   = (b"\x1b[0;1;31;48;5;238m", b"\x1b[0;1;32;48;5;238m", b"");
pub const FILENAME: (Bytes, Bytes, Bytes)          = (b"\x1b[0;31m", b"\x1b[0;32m", b"");
pub const DIFF: (Bytes, Bytes)              = (b"\x1b[0;31m", b"\x1b[0;32m");
// pub const DIFF_MATCHING =     (b"\x1b[0;38;2;250;100;100m", b"\x1b[0;38;2;100;250;100m");
pub const DIFF_MATCHING: (Bytes, Bytes)     = (b"\x1b[0;38;2;220;190;210;48;2;35;20;20m", b"\x1b[0;38;2;190;220;210;48;2;20;35;20m");
// pub const DIFF_MATCHING =     (b"\x1b[0;38;5;225m", b"\x1b[0;38;5;195m");
// pub const DIFF_MATCHING =     (b"\x1b[0m", b"\x1b[0m");
// pub const DIFF_NON_MATCHING = (b"\x1b[0;48;2;200;80;80;38;5;235m", b"\x1b[0;48;2;80;200;80;38;5;235m");
pub const DIFF_NON_MATCHING: (Bytes, Bytes) = (b"\x1b[0;1;31;48;2;80;30;30m", b"\x1b[0;1;32;48;2;25;80;25m");
// pub const DIFF_NON_MATCHING =     (b"\x1b[0;48;2;60;0;0m", b"\x1b[0;48;2;0;60;0m");
// pub const DIFF_NON_MATCHING = (b"\x1b[0;41;38;5;235m", b"\x1b[0;42;38;5;235m");
pub const DIFF_TRAILING_WS: &str  = "\x1b[2;7m";
pub const DIFF_INSERT: (Bytes, Bytes)       = (b"\x1b[4:3:58:5:10m", b"\x1b[4:3;58;5;9m");
// pub const DIFF_INSERT =       (b"\x1b[0;7;2;48;5;211;38;5;52m", b"\x1b[0;7;2;48;5;158;38;5;22m");
pub const DIFF_CONTEXT: Bytes      = b"\x1b[0;38;5;242m";
pub const SIGN: [Bytes; 3]              = [b"-", b"+", RESET];
pub const FILENAME_SIGN: (Bytes, Bytes)     = (b"\x1b[0;31;48;5;238;7m---\x1b[27m ", b"\x1b[0;32;48;5;238;7m+++\x1b[27m ");
pub const LINENO: Bytes            = b"\x1b[0;38;5;242m";
pub const LINENO_BAR: &str        = "\x1b[0;38;5;242m▏";
pub const LINENO_OUR_BAR: &str    = "\x1b[0;38;5;187m(";
pub const LINENO_THEIR_BAR: &str  = "\x1b[0;38;5;117m)";
pub const LINENO_MERGE_BAR: &str  = "\x1b[0;38;5;13;1m|";
pub const LINENO_DIFF: (&str, &str)       = ("\x1b[0;31m", "\x1b[0;32m");

pub fn format_lineno(num1: Option<usize>, num2: Option<usize>) -> String {
    let num1 = num1.map(|n| n.to_string());
    let num2 = num2.map(|n| n.to_string());
    format!(
        "{}{:<4}{}{}{:<4}{} ",
        LINENO_DIFF.0,
        num1.as_ref().map(|n| n.as_ref()).unwrap_or(""),
        LINENO_BAR,
        LINENO_DIFF.1,
        num2.as_ref().map(|n| n.as_ref()).unwrap_or(""),
        LINENO_BAR,
    )
}
