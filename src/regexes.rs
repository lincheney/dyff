macro_rules! regex {
    ($($regex:literal)+ . $method:ident($($arg:expr),*) ) => {{
        thread_local! {
            static RE: ::regex::bytes::Regex = ::regex::bytes::Regex::new(concat!($($regex),*)).unwrap();
        }
        RE.with(|r| r.$method($($arg),*))
    }};

    ($($regex:literal)+, |$name:ident| $body:tt ) => {{
        thread_local! {
            static RE: ::regex::bytes::Regex = ::regex::bytes::Regex::new(concat!($($regex),*)).unwrap();
        }
        RE.with(|$name| $body)
    }};
}

pub(crate) use regex;
