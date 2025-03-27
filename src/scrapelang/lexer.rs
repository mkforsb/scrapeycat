use regex::Regex;

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrapeLangToken<'a> {
    Append {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Clear {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    ClearHeaders {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Comma {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Delete {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Discard {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Drop {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Effect {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Equals {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Extract {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    First {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Get {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Header {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Identifier {
        pos: TextPosition,
        pos_after: TextPosition,
        name: &'a str,
    },
    LeftParenthesis {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Load {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Number {
        pos: TextPosition,
        pos_after: TextPosition,
        value: &'a str,
    },
    Prepend {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Retain {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    RightParenthesis {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Run {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    Store {
        pos: TextPosition,
        pos_after: TextPosition,
    },
    String {
        pos: TextPosition,
        pos_after: TextPosition,
        str: &'a str,
    },
    Whitespace {
        pos: TextPosition,
        pos_after: TextPosition,
    },
}

impl ScrapeLangToken<'_> {
    pub fn name(&self) -> &'static str {
        match self {
            ScrapeLangToken::Append { .. } => "Append",
            ScrapeLangToken::Clear { .. } => "Clear",
            ScrapeLangToken::ClearHeaders { .. } => "ClearHeaders",
            ScrapeLangToken::Comma { .. } => "Comma",
            ScrapeLangToken::Delete { .. } => "Delete",
            ScrapeLangToken::Discard { .. } => "Discard",
            ScrapeLangToken::Drop { .. } => "Drop",
            ScrapeLangToken::Effect { .. } => "Effect",
            ScrapeLangToken::Equals { .. } => "Equals",
            ScrapeLangToken::Extract { .. } => "Extract",
            ScrapeLangToken::First { .. } => "First",
            ScrapeLangToken::Get { .. } => "Get",
            ScrapeLangToken::Header { .. } => "Header",
            ScrapeLangToken::Identifier { .. } => "Identifier",
            ScrapeLangToken::LeftParenthesis { .. } => "LeftParenthesis",
            ScrapeLangToken::Load { .. } => "Load",
            ScrapeLangToken::Number { .. } => "Number",
            ScrapeLangToken::Prepend { .. } => "Prepend",
            ScrapeLangToken::Retain { .. } => "Retain",
            ScrapeLangToken::RightParenthesis { .. } => "RightParenthesis",
            ScrapeLangToken::Run { .. } => "Run",
            ScrapeLangToken::Store { .. } => "Store",
            ScrapeLangToken::String { .. } => "String",
            ScrapeLangToken::Whitespace { .. } => "Whitespace",
        }
    }

    pub fn pos(&self) -> TextPosition {
        match self {
            ScrapeLangToken::Append { pos, .. } => *pos,
            ScrapeLangToken::Clear { pos, .. } => *pos,
            ScrapeLangToken::ClearHeaders { pos, .. } => *pos,
            ScrapeLangToken::Comma { pos, .. } => *pos,
            ScrapeLangToken::Delete { pos, .. } => *pos,
            ScrapeLangToken::Discard { pos, .. } => *pos,
            ScrapeLangToken::Drop { pos, .. } => *pos,
            ScrapeLangToken::Effect { pos, .. } => *pos,
            ScrapeLangToken::Equals { pos, .. } => *pos,
            ScrapeLangToken::Extract { pos, .. } => *pos,
            ScrapeLangToken::First { pos, .. } => *pos,
            ScrapeLangToken::Get { pos, .. } => *pos,
            ScrapeLangToken::Header { pos, .. } => *pos,
            ScrapeLangToken::Identifier { pos, .. } => *pos,
            ScrapeLangToken::LeftParenthesis { pos, .. } => *pos,
            ScrapeLangToken::Load { pos, .. } => *pos,
            ScrapeLangToken::Number { pos, .. } => *pos,
            ScrapeLangToken::Prepend { pos, .. } => *pos,
            ScrapeLangToken::Retain { pos, .. } => *pos,
            ScrapeLangToken::RightParenthesis { pos, .. } => *pos,
            ScrapeLangToken::Run { pos, .. } => *pos,
            ScrapeLangToken::Store { pos, .. } => *pos,
            ScrapeLangToken::String { pos, .. } => *pos,
            ScrapeLangToken::Whitespace { pos, .. } => *pos,
        }
    }

    pub fn pos_after(&self) -> TextPosition {
        match self {
            ScrapeLangToken::Append { pos_after, .. } => *pos_after,
            ScrapeLangToken::Clear { pos_after, .. } => *pos_after,
            ScrapeLangToken::ClearHeaders { pos_after, .. } => *pos_after,
            ScrapeLangToken::Comma { pos_after, .. } => *pos_after,
            ScrapeLangToken::Delete { pos_after, .. } => *pos_after,
            ScrapeLangToken::Discard { pos_after, .. } => *pos_after,
            ScrapeLangToken::Drop { pos_after, .. } => *pos_after,
            ScrapeLangToken::Effect { pos_after, .. } => *pos_after,
            ScrapeLangToken::Equals { pos_after, .. } => *pos_after,
            ScrapeLangToken::Extract { pos_after, .. } => *pos_after,
            ScrapeLangToken::First { pos_after, .. } => *pos_after,
            ScrapeLangToken::Get { pos_after, .. } => *pos_after,
            ScrapeLangToken::Header { pos_after, .. } => *pos_after,
            ScrapeLangToken::Identifier { pos_after, .. } => *pos_after,
            ScrapeLangToken::LeftParenthesis { pos_after, .. } => *pos_after,
            ScrapeLangToken::Load { pos_after, .. } => *pos_after,
            ScrapeLangToken::Number { pos_after, .. } => *pos_after,
            ScrapeLangToken::Prepend { pos_after, .. } => *pos_after,
            ScrapeLangToken::Retain { pos_after, .. } => *pos_after,
            ScrapeLangToken::RightParenthesis { pos_after, .. } => *pos_after,
            ScrapeLangToken::Run { pos_after, .. } => *pos_after,
            ScrapeLangToken::Store { pos_after, .. } => *pos_after,
            ScrapeLangToken::String { pos_after, .. } => *pos_after,
            ScrapeLangToken::Whitespace { pos_after, .. } => *pos_after,
        }
    }
}

fn text_position_after(start_pos: &TextPosition, text: &str) -> TextPosition {
    let mut result = TextPosition {
        row: start_pos.row,
        col: start_pos.col,
    };

    for char in text.chars() {
        if char == '\n' {
            result.row += 1;
            result.col = 1;
        } else {
            result.col += 1;
        }
    }

    result
}

pub fn lex(text: &str) -> Result<Vec<ScrapeLangToken>, Error> {
    #[derive(Debug)]
    struct MatchResult<'a> {
        matched: &'a str,
        token: ScrapeLangToken<'a>,
    }

    struct Matcher {
        #[allow(clippy::type_complexity)]
        try_match: Box<dyn Fn(&str, TextPosition) -> Option<Result<MatchResult, Error>>>,
    }

    let keyword_append = Regex::new("^append").expect("Should be a valid regex");
    let keyword_clear = Regex::new("^clear").expect("Should be a valid regex");
    let keyword_clearheaders = Regex::new("^clearheaders").expect("Should be a valid regex");
    let keyword_delete = Regex::new("^delete").expect("Should be a valid regex");
    let keyword_discard = Regex::new("^discard").expect("Should be a valid regex");
    let keyword_drop = Regex::new("^drop").expect("Should be a valid regex");
    let keyword_effect = Regex::new("^effect").expect("Should be a valid regex");
    let keyword_extract = Regex::new("^extract").expect("Should be a valid regex");
    let keyword_first = Regex::new("^first").expect("Should be a valid regex");
    let keyword_get = Regex::new("^get").expect("Should be a valid regex");
    let keyword_header = Regex::new("^header").expect("Should be a valid regex");
    let keyword_load = Regex::new("^load").expect("Should be a valid regex");
    let keyword_prepend = Regex::new("^prepend").expect("Should be a valid regex");
    let keyword_retain = Regex::new("^retain").expect("Should be a valid regex");
    let keyword_run = Regex::new("^run").expect("Should be a valid regex");
    let keyword_store = Regex::new("^store").expect("Should be a valid regex");

    let spaces_and_tabs = Regex::new("^[ \\t]+").expect("Should be a valid regex");
    let newline = Regex::new("^\\r?\\n").expect("Should be a valid regex");
    let left_paren = Regex::new("^\\(").expect("Should be a valid regex");
    let right_paren = Regex::new("^\\)").expect("Should be a valid regex");
    let comma = Regex::new("^,").expect("Should be a valid regex");
    let equals = Regex::new("^=").expect("Should be a valid regex");

    let number = Regex::new("^[1-9][0-9]*").expect("Should be a valid regex");
    let identifier = Regex::new("^[A-Za-z_$.-][A-Za-z0-9_$.-]*").expect("Should be a valid regex");

    macro_rules! simple_matcher {
        ($regex:ident, $token:ident) => {
            Matcher {
                try_match: Box::new(move |text, pos| {
                    $regex.find(text).map(|m| {
                        Ok(MatchResult {
                            matched: &text[m.range()],
                            token: ScrapeLangToken::$token {
                                pos,
                                pos_after: TextPosition {
                                    row: pos.row,
                                    col: pos.col + text[m.range()].chars().count(),
                                },
                            },
                        })
                    })
                }),
            }
        };
        ($regex:ident, $token:ident, $value:ident) => {
            Matcher {
                try_match: Box::new(move |text, pos| {
                    $regex.find(text).map(|m| {
                        Ok(MatchResult {
                            matched: &text[m.range()],
                            token: ScrapeLangToken::$token {
                                pos,
                                pos_after: TextPosition {
                                    row: pos.row,
                                    col: pos.col + text[m.range()].chars().count(),
                                },
                                $value: &text[m.range()],
                            },
                        })
                    })
                }),
            }
        };
    }

    struct CharDelimitedRangeMatcher {
        open: char,
        close: char,
        name: String,

        #[allow(clippy::type_complexity)]
        // token: (self.result)(text, pos, num_bytes, num_chars),
        result: Box<dyn Fn(&str, TextPosition, usize, usize) -> ScrapeLangToken>,
    }

    impl<'a> CharDelimitedRangeMatcher {
        pub fn try_match(
            &self,
            text: &'a str,
            pos: TextPosition,
        ) -> Option<Result<MatchResult<'a>, Error>> {
            if text.starts_with(self.open) {
                let mut done = false;
                let mut escaped = false;
                let mut num_chars = 1;
                let mut num_bytes = 1;

                for char in text.chars().skip(1) {
                    if escaped {
                        escaped = false;
                    } else if char == self.close {
                        done = true;
                        num_chars += 1;
                        num_bytes += 1;
                        break;
                    } else if char == '\\' {
                        escaped = true;
                    }

                    num_chars += 1;
                    num_bytes += char.len_utf8();
                }

                if !done {
                    return Some(Err(Error::ParseError(format!(
                        "Unterminated {} at line {}, column {}",
                        self.name,
                        pos.row,
                        pos.col + num_chars
                    ))));
                }

                Some(Ok(MatchResult {
                    matched: &text[..num_bytes],
                    token: (self.result)(text, pos, num_bytes, num_chars),
                }))
            } else {
                None
            }
        }
    }

    let string_matcher = CharDelimitedRangeMatcher {
        open: '"',
        close: '"',
        name: "String".to_string(),
        result: Box::new(|text, pos, num_bytes, _| ScrapeLangToken::String {
            pos,
            pos_after: text_position_after(&pos, &text[..num_bytes]),
            str: &text[1..(num_bytes - 1)],
        }),
    };

    // Ordering is significant here (keywords first).
    let matchers = [
        simple_matcher!(keyword_append, Append),
        simple_matcher!(keyword_clear, Clear),
        simple_matcher!(keyword_clearheaders, ClearHeaders),
        simple_matcher!(keyword_delete, Delete),
        simple_matcher!(keyword_discard, Discard),
        simple_matcher!(keyword_drop, Drop),
        simple_matcher!(keyword_effect, Effect),
        simple_matcher!(keyword_extract, Extract),
        simple_matcher!(keyword_first, First),
        simple_matcher!(keyword_get, Get),
        simple_matcher!(keyword_header, Header),
        simple_matcher!(keyword_load, Load),
        simple_matcher!(keyword_prepend, Prepend),
        simple_matcher!(keyword_retain, Retain),
        simple_matcher!(keyword_run, Run),
        simple_matcher!(keyword_store, Store),
        simple_matcher!(comma, Comma),
        simple_matcher!(equals, Equals),
        simple_matcher!(identifier, Identifier, name),
        simple_matcher!(left_paren, LeftParenthesis),
        simple_matcher!(number, Number, value),
        simple_matcher!(right_paren, RightParenthesis),
        Matcher {
            try_match: Box::new(move |text, pos| string_matcher.try_match(text, pos)),
        },
        Matcher {
            // Whitespace that doesn't alter the row position.
            try_match: Box::new(move |text, pos| {
                spaces_and_tabs.find(text).map(|m| {
                    Ok(MatchResult {
                        matched: &text[m.range()],
                        token: ScrapeLangToken::Whitespace {
                            pos,
                            pos_after: TextPosition {
                                row: pos.row,
                                col: pos.col + text[m.range()].chars().count(),
                            },
                        },
                    })
                })
            }),
        },
        Matcher {
            // Newline
            try_match: Box::new(move |text, pos| {
                newline.find(text).map(|m| {
                    Ok(MatchResult {
                        matched: &text[m.range()],
                        token: ScrapeLangToken::Whitespace {
                            pos,
                            pos_after: TextPosition {
                                row: pos.row + 1,
                                col: 1,
                            },
                        },
                    })
                })
            }),
        },
    ];

    let mut result = Vec::new();
    let mut rest = text;
    let mut pos = TextPosition { row: 1, col: 1 };

    while !rest.is_empty() {
        let mut matches = matchers
            .iter()
            .filter_map(|m| (m.try_match)(rest, pos))
            .collect::<Result<Vec<_>, _>>()?;

        if matches.is_empty() {
            return Err(Error::ParseError(format!(
                "Syntax error at line {} column {}",
                pos.row, pos.col
            )));
        }

        // A stable sort is required here.
        // TODO: make this less fragile by explicitly prioritizing keywords when
        //       two or more matchers match an equal amount of text
        matches.sort_by(|a, b| b.matched.len().cmp(&a.matched.len()));

        let matched = matches.into_iter().next().expect("`matches` is nonempty");

        rest = &rest[matched.matched.len()..];
        pos = matched.token.pos_after();

        result.push(matched.token);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Lex, strip whitespace, grab .name()
    fn lex_no_ws(text: &str) -> Result<Vec<ScrapeLangToken<'_>>, Error> {
        lex(text).map(|x| {
            x.into_iter()
                .filter_map(|x| match x {
                    ScrapeLangToken::Whitespace { .. } => None,
                    token => Some(token),
                })
                .collect()
        })
    }

    fn lex_no_ws_names(text: &str) -> Vec<&'static str> {
        lex_no_ws(text).unwrap().iter().map(|x| x.name()).collect()
    }

    #[test]
    fn test_lex_empty() {
        assert_eq!(lex("").unwrap(), vec![]);
    }

    #[test]
    fn test_lex_append() {
        assert_eq!(lex_no_ws_names("append"), vec!["Append"]);
        assert_eq!(lex_no_ws_names("  append   "), vec!["Append"]);

        assert_eq!(lex_no_ws_names("appendx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("appendappend"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_append_string() {
        assert!(lex_no_ws("append \"text\"").is_ok_and(|result| {
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].name(), "Append");
            assert_eq!(result[0].pos(), TextPosition { row: 1, col: 1 });
            assert_eq!(result[0].pos_after(), TextPosition { row: 1, col: 7 });
            assert!(matches!(
                result[1],
                ScrapeLangToken::String {
                    pos: TextPosition { row: 1, col: 8 },
                    pos_after: TextPosition { row: 1, col: 14 },
                    str: "text",
                }
            ));
            true
        }))
    }

    #[test]
    fn test_lex_clear() {
        assert_eq!(lex_no_ws_names("clear"), vec!["Clear"]);
        assert_eq!(lex_no_ws_names("  clear   "), vec!["Clear"]);

        assert_eq!(lex_no_ws_names("clearx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("clearclear"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_clearheaders() {
        assert_eq!(lex_no_ws_names("clearheaders"), vec!["ClearHeaders"]);
        assert_eq!(lex_no_ws_names("  clearheaders   "), vec!["ClearHeaders"]);

        assert_eq!(lex_no_ws_names("clearheadersx"), vec!["Identifier"]);
        assert_eq!(
            lex_no_ws_names("clearheadersclearheaders"),
            vec!["Identifier"]
        );
    }

    #[test]
    fn test_lex_comma() {
        assert_eq!(lex_no_ws_names(","), vec!["Comma"]);
        assert_eq!(lex_no_ws_names("  ,   "), vec!["Comma"]);
    }

    #[test]
    fn test_lex_delete() {
        assert_eq!(lex_no_ws_names("delete"), vec!["Delete"]);
        assert_eq!(lex_no_ws_names("  delete   "), vec!["Delete"]);

        assert_eq!(lex_no_ws_names("deletex"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("deletedelete"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_delete_string() {
        assert!(lex_no_ws("delete \" foo \"").is_ok_and(|result| {
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].name(), "Delete");
            assert_eq!(result[0].pos(), TextPosition { row: 1, col: 1 });
            assert!(matches!(
                result[1],
                ScrapeLangToken::String {
                    pos: TextPosition { row: 1, col: 8 },
                    pos_after: TextPosition { row: 1, col: 15 },
                    str: " foo ",
                }
            ));
            true
        }))
    }

    #[test]
    fn test_lex_discard() {
        assert_eq!(lex_no_ws_names("discard"), vec!["Discard"]);
        assert_eq!(lex_no_ws_names("  discard   "), vec!["Discard"]);

        assert_eq!(lex_no_ws_names("discardx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("discarddiscard"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_discard_pattern() {
        assert!(lex_no_ws("discard \"donotwant\"").is_ok_and(|result| {
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].name(), "Discard");
            assert_eq!(result[0].pos(), TextPosition { row: 1, col: 1 });
            assert!(matches!(
                result[1],
                ScrapeLangToken::String {
                    pos: TextPosition { row: 1, col: 9 },
                    pos_after: TextPosition { row: 1, col: 20 },
                    str: "donotwant",
                }
            ));
            true
        }))
    }

    #[test]
    fn test_lex_drop() {
        assert_eq!(lex_no_ws_names("drop"), vec!["Drop"]);
        assert_eq!(lex_no_ws_names("  drop   "), vec!["Drop"]);

        assert_eq!(lex_no_ws_names("dropx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("dropdrop"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_drop_count() {
        assert!(lex_no_ws("drop 8").is_ok_and(|result| {
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].name(), "Drop");
            assert_eq!(result[0].pos(), TextPosition { row: 1, col: 1 });
            assert!(matches!(
                result[1],
                ScrapeLangToken::Number {
                    pos: TextPosition { row: 1, col: 6 },
                    pos_after: TextPosition { row: 1, col: 7 },
                    value: "8",
                }
            ));
            true
        }))
    }

    #[test]
    fn test_lex_effect() {
        assert_eq!(lex_no_ws_names("effect"), vec!["Effect"]);
        assert_eq!(lex_no_ws_names("  effect   "), vec!["Effect"]);

        assert_eq!(lex_no_ws_names("effectx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("effecteffect"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_effect_call() {
        assert!(
            lex(r#"effect notify(summary="New Message", body=$content)"#).is_ok_and(|_| {
                // TODO: verify the result
                true
            })
        );
    }

    #[test]
    fn test_lex_run() {
        assert_eq!(lex_no_ws_names("run"), vec!["Run"]);
        assert_eq!(lex_no_ws_names("  run   "), vec!["Run"]);

        assert_eq!(lex_no_ws_names("runx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("runrun"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_run_call() {
        assert!(lex_no_ws(r#"run jobname"#).is_ok_and(|result| {
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].name(), "Run");
            assert!(matches!(
                result[1],
                ScrapeLangToken::Identifier {
                    pos: TextPosition { row: 1, col: 5 },
                    pos_after: TextPosition { row: 1, col: 12 },
                    name: "jobname",
                }
            ));
            true
        }));
    }

    #[test]
    fn test_lex_equals() {
        assert_eq!(lex_no_ws_names("="), vec!["Equals"]);
        assert_eq!(lex_no_ws_names("  =   "), vec!["Equals"]);
    }

    #[test]
    fn test_lex_extract() {
        assert_eq!(lex_no_ws_names("extract"), vec!["Extract"]);
        assert_eq!(lex_no_ws_names("  extract   "), vec!["Extract"]);

        assert_eq!(lex_no_ws_names("extractx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("extractextract"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_extract_pattern() {
        assert!(lex_no_ws("extract \"some.+?pattern\"").is_ok_and(|result| {
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].name(), "Extract");
            assert_eq!(result[0].pos(), TextPosition { row: 1, col: 1 });
            assert!(matches!(
                result[1],
                ScrapeLangToken::String {
                    pos: TextPosition { row: 1, col: 9 },
                    pos_after: TextPosition { row: 1, col: 25 },
                    str: "some.+?pattern",
                }
            ));
            true
        }))
    }

    #[test]
    fn test_lex_first() {
        assert_eq!(lex_no_ws_names("first"), vec!["First"]);
        assert_eq!(lex_no_ws_names("  first   "), vec!["First"]);

        assert_eq!(lex_no_ws_names("firstx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("firstfirst"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_get() {
        assert_eq!(lex_no_ws_names("get"), vec!["Get"]);
        assert_eq!(lex_no_ws_names("  get   "), vec!["Get"]);

        assert_eq!(lex_no_ws_names("getx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("getget"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_get_url() {
        assert!(
            lex_no_ws("get \"https://www.rust-lang.org/\"").is_ok_and(|result| {
                assert_eq!(result.len(), 2);
                assert_eq!(result[0].name(), "Get");
                assert_eq!(result[0].pos(), TextPosition { row: 1, col: 1 });
                assert!(matches!(
                    result[1],
                    ScrapeLangToken::String {
                        pos: TextPosition { row: 1, col: 5 },
                        pos_after: TextPosition { row: 1, col: 33 },
                        str: "https://www.rust-lang.org/",
                    }
                ));
                true
            })
        )
    }

    #[test]
    fn test_lex_header() {
        assert_eq!(lex_no_ws_names("header"), vec!["Header"]);
        assert_eq!(lex_no_ws_names("  header   "), vec!["Header"]);

        assert_eq!(lex_no_ws_names("headerx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("headerheader"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_header_spec() {
        assert!(
            lex_no_ws("header \"User-Agent\" \"Firefox\"").is_ok_and(|result| {
                assert_eq!(result.len(), 3);
                assert_eq!(result[0].name(), "Header");
                assert_eq!(result[0].pos(), TextPosition { row: 1, col: 1 });
                assert!(matches!(
                    result[1],
                    ScrapeLangToken::String {
                        pos: TextPosition { row: 1, col: 8 },
                        pos_after: TextPosition { row: 1, col: 20 },
                        str: "User-Agent",
                    }
                ));
                assert!(matches!(
                    result[2],
                    ScrapeLangToken::String {
                        pos: TextPosition { row: 1, col: 21 },
                        pos_after: TextPosition { row: 1, col: 30 },
                        str: "Firefox",
                    }
                ));
                true
            })
        )
    }

    #[test]
    fn test_lex_identifier() {
        assert!(matches!(
            lex("k9").unwrap().first().unwrap(),
            ScrapeLangToken::Identifier { name: "k9", .. }
        ));

        assert!(matches!(
            lex("_9000grimblo").unwrap().first().unwrap(),
            ScrapeLangToken::Identifier {
                name: "_9000grimblo",
                ..
            }
        ));

        assert!(matches!(
            lex("$FullPage").unwrap().first().unwrap(),
            ScrapeLangToken::Identifier {
                name: "$FullPage",
                ..
            }
        ));

        assert!(matches!(
            lex("$0").unwrap().first().unwrap(),
            ScrapeLangToken::Identifier { name: "$0", .. }
        ));
    }

    #[test]
    fn test_lex_left_paren() {
        assert_eq!(lex_no_ws_names("("), vec!["LeftParenthesis"]);
        assert_eq!(lex_no_ws_names("  (   "), vec!["LeftParenthesis"]);
    }

    #[test]
    fn test_lex_right_paren() {
        assert_eq!(lex_no_ws_names(")"), vec!["RightParenthesis"]);
        assert_eq!(lex_no_ws_names("  )   "), vec!["RightParenthesis"]);
    }

    #[test]
    fn test_lex_load() {
        assert_eq!(lex_no_ws_names("load"), vec!["Load"]);
        assert_eq!(lex_no_ws_names("  load   "), vec!["Load"]);

        assert_eq!(lex_no_ws_names("loadx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("loadload"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_load_ident() {
        assert!(lex_no_ws("load $foo").is_ok_and(|result| {
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].name(), "Load");
            assert_eq!(result[0].pos(), TextPosition { row: 1, col: 1 });
            assert!(matches!(
                result[1],
                ScrapeLangToken::Identifier {
                    pos: TextPosition { row: 1, col: 6 },
                    pos_after: TextPosition { row: 1, col: 10 },
                    name: "$foo",
                }
            ));
            true
        }))
    }

    #[test]
    fn test_lex_store() {
        assert_eq!(lex_no_ws_names("store"), vec!["Store"]);
        assert_eq!(lex_no_ws_names("  store   "), vec!["Store"]);

        assert_eq!(lex_no_ws_names("storex"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("storestore"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_store_ident() {
        assert!(lex_no_ws("store $foo").is_ok_and(|result| {
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].name(), "Store");
            assert_eq!(result[0].pos(), TextPosition { row: 1, col: 1 });
            assert!(matches!(
                result[1],
                ScrapeLangToken::Identifier {
                    pos: TextPosition { row: 1, col: 7 },
                    pos_after: TextPosition { row: 1, col: 11 },
                    name: "$foo",
                }
            ));
            true
        }))
    }

    #[test]
    fn test_lex_number() {
        assert!(lex("123").is_ok_and(|result| {
            assert!(matches!(
                result[0],
                ScrapeLangToken::Number { value: "123", .. }
            ));
            true
        }));

        assert!(lex("0123").is_err());
    }

    #[test]
    fn test_lex_prepend() {
        assert_eq!(lex_no_ws_names("prepend"), vec!["Prepend"]);
        assert_eq!(lex_no_ws_names("  prepend   "), vec!["Prepend"]);

        assert_eq!(lex_no_ws_names("prependx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("prependprepend"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_prepend_string() {
        assert!(lex_no_ws("prepend \"text\"").is_ok_and(|result| {
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].name(), "Prepend");
            assert_eq!(result[0].pos(), TextPosition { row: 1, col: 1 });
            assert!(matches!(
                result[1],
                ScrapeLangToken::String {
                    pos: TextPosition { row: 1, col: 9 },
                    pos_after: TextPosition { row: 1, col: 15 },
                    str: "text",
                }
            ));
            true
        }))
    }

    #[test]
    fn test_lex_retain() {
        assert_eq!(lex_no_ws_names("retain"), vec!["Retain"]);
        assert_eq!(lex_no_ws_names("  retain   "), vec!["Retain"]);

        assert_eq!(lex_no_ws_names("retainx"), vec!["Identifier"]);
        assert_eq!(lex_no_ws_names("retainretain"), vec!["Identifier"]);
    }

    #[test]
    fn test_lex_retain_pattern() {
        assert!(lex_no_ws("retain \"dowant\"").is_ok_and(|result| {
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].name(), "Retain");
            assert_eq!(result[0].pos(), TextPosition { row: 1, col: 1 });
            assert!(matches!(
                result[1],
                ScrapeLangToken::String {
                    pos: TextPosition { row: 1, col: 8 },
                    pos_after: TextPosition { row: 1, col: 16 },
                    str: "dowant",
                }
            ));
            true
        }))
    }

    #[test]
    fn test_lex_string() {
        assert!(matches!(
            lex("\"hello world\"").unwrap().first().unwrap(),
            ScrapeLangToken::String {
                str: "hello world",
                ..
            }
        ));

        assert!(matches!(
            lex("\"hello \\\"world\\\"\"").unwrap().first().unwrap(),
            ScrapeLangToken::String {
                str: "hello \\\"world\\\"",
                ..
            }
        ));

        assert!(lex("\"hello world").is_err());
    }

    #[test]
    fn test_lex_string_multiline() {
        assert!(matches!(
            lex("\"hello\nworld\"").unwrap().first().unwrap(),
            ScrapeLangToken::String {
                pos_after: TextPosition { row: 2, col: 7 },
                str: "hello\nworld",
                ..
            }
        ));

        assert!(matches!(
            lex("\"hello\n\nwor\nld\"").unwrap().first().unwrap(),
            ScrapeLangToken::String {
                pos_after: TextPosition { row: 4, col: 4 },
                str: "hello\n\nwor\nld",
                ..
            }
        ));
    }

    #[test]
    fn test_lex_whitespace() {
        assert!(lex("\na\n  b").is_ok_and(|result| {
            assert_eq!(result.len(), 5);
            assert_eq!(result[0].name(), "Whitespace");
            true
        }));
    }
}
