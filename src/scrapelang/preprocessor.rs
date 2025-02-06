use regex::Regex;

pub fn strip_comments(text: &str) -> String {
    Regex::new("(?m)^\\s*#.*(\\r?\\n)?")
        .unwrap()
        .replace_all(text, "")
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_comments() {
        assert_eq!(strip_comments(""), "");
        assert_eq!(strip_comments("foo"), "foo");
        assert_eq!(strip_comments("#foo"), "");
        assert_eq!(strip_comments("   #foo"), "");
        assert_eq!(strip_comments("foo\n# bar\nbaz"), "foo\nbaz");
        assert_eq!(strip_comments("# foo\n bar\n # baz"), " bar\n");
    }
}
