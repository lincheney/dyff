macro_rules! regex {
    ($regex:literal . $method:ident($($arg:expr),*) ) => {{
        thread_local! {
            static RE: ::regex::bytes::Regex = ::regex::bytes::Regex::new($regex).unwrap();
        }
        RE.with(|r| r.$method($($arg),*))
    }};
}
