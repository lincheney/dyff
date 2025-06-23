macro_rules! regex {
    ($path:path, $($regex:literal)+ . $method:ident($($arg:expr),*) ) => {{
        use $path as Regex;
        thread_local! {
            static RE: Regex = Regex::new(concat!($($regex),*)).unwrap();
        }
        RE.with(|r| r.$method($($arg),*))
    }};

    ($path:path, $($regex:literal)+, |$name:ident| $body:tt ) => {{
        use $path as Regex;
        thread_local! {
            static RE: Regex = Regex::new(concat!($($regex),*)).unwrap();
        }
        RE.with(|$name| $body)
    }};

    ($($regex:literal)+ . $method:ident($($arg:expr),*) ) => {{
        crate::regexes::regex!(::regex::Regex, $($regex)+ . $method($($arg),*) )
    }};

    ($($regex:literal)+, |$name:ident| $body:tt ) => {{
        crate::regexes::regex!(::regex::Regex, $($regex)+, |$name| $body )
    }};
}

macro_rules! byte_regex {
    ($($regex:literal)+ . $method:ident($($arg:expr),*) ) => {{
        crate::regexes::regex!(::regex::bytes::Regex, $($regex)+ . $method($($arg),*) )
    }};

    ($($regex:literal)+, |$name:ident| $body:tt ) => {{
        crate::regexes::regex!(::regex::bytes::Regex, $($regex)+, |$name| $body )
    }};
}

pub(crate) use {byte_regex, regex};
