use std::collections::HashMap;

use crate::{
    scrapelang::lexer::{ScrapeLangToken, TextPosition},
    Error,
};

fn unescape(text: &str) -> String {
    let mut result: Vec<char> = vec![];
    let mut escaped = false;

    for char in text.chars() {
        if escaped {
            escaped = false;
            // TODO: special chars e.g \n
            result.push(char);
        } else if char == '\\' {
            escaped = true;
        } else {
            result.push(char);
        }
    }

    result.into_iter().collect::<String>()
}

#[derive(Debug, PartialEq, Eq)]
pub enum ScrapeLangArgument {
    String { str: String },
    Identifier { name: String },
}

#[derive(Debug, PartialEq, Eq)]
pub enum ScrapeLangInstruction {
    Append {
        str: String,
    },
    Clear,
    ClearHeaders,
    Delete {
        regex: String,
    },
    Discard {
        regex: String,
    },
    Drop {
        count: usize,
    },
    Effect {
        effect_name: String,
        args: Vec<ScrapeLangArgument>,
        kwargs: HashMap<String, ScrapeLangArgument>,
    },
    Extract {
        regex: String,
    },
    First,
    Get {
        url: String,
    },
    Header {
        key: String,
        value: String,
    },
    Load {
        varname: String,
    },
    Prepend {
        str: String,
    },
    Retain {
        regex: String,
    },
    Run {
        job_name: String,
        args: Vec<ScrapeLangArgument>,
        kwargs: HashMap<String, ScrapeLangArgument>,
    },
    Store {
        varname: String,
    },
}

macro_rules! try_parse {
    ($tokens:ident, $pos:ident, $variant:ident, $name:expr, $matched:expr) => {
        match $tokens.get(0) {
            Some(ScrapeLangToken::$variant { pos_after, .. }) => ($matched)($tokens, *pos_after),
            Some(tok) => Err(Error::ParseError(format!(
                "Expected `{}` but found `{}` at line {} column {}",
                $name,
                tok.name(),
                tok.pos().row,
                tok.pos().col
            ))),
            None => Err(Error::ParseError(format!(
                "Unexpected EOF at line {}",
                $pos.row
            ))),
        }
    };
}

type ParseResult = Result<(ScrapeLangInstruction, usize), Error>;

struct CallArgs {
    args: Vec<ScrapeLangArgument>,
    kwargs: HashMap<String, ScrapeLangArgument>,
    num_tokens: usize,
    pos_after: TextPosition,
}

impl<'a, 'b> ScrapeLangInstruction
where
    'a: 'b,
{
    fn separator(
        token: Option<&'b ScrapeLangToken<'a>>,
        pos: TextPosition,
    ) -> Result<TextPosition, Error> {
        match token {
            Some(ScrapeLangToken::Whitespace { pos_after, .. }) => Ok(*pos_after),
            Some(tok) => Err(Error::ParseError(format!(
                "Syntax error, unexpected `{}` at line {} column {}",
                tok.name(),
                tok.pos().row,
                tok.pos().col
            ))),
            None => Err(Error::ParseError(format!(
                "Unexpected EOF at line {}",
                pos.row
            ))),
        }
    }

    fn statement_terminator(token: Option<&'b ScrapeLangToken<'a>>) -> Result<(), Error> {
        match token {
            None | Some(ScrapeLangToken::Whitespace { .. }) => Ok(()),
            Some(tok) => Err(Error::ParseError(format!(
                "Syntax error, unexpected `{}` at line {} column {}",
                tok.name(),
                tok.pos().row,
                tok.pos().col
            ))),
        }
    }

    fn string(
        token: Option<&'b ScrapeLangToken<'a>>,
        pos: TextPosition,
    ) -> Result<(&'a str, TextPosition), Error> {
        match token {
            Some(ScrapeLangToken::String { str, pos_after, .. }) => Ok((str, *pos_after)),
            Some(tok) => Err(Error::ParseError(format!(
                "Expected `String` but found `{}` at line {} column {}",
                tok.name(),
                tok.pos().row,
                tok.pos().col
            ))),
            None => Err(Error::ParseError(format!(
                "Unexpected EOF at line {}",
                pos.row
            ))),
        }
    }

    fn number(
        token: Option<&'b ScrapeLangToken<'a>>,
        pos: TextPosition,
    ) -> Result<(usize, TextPosition), Error> {
        match token {
            Some(ScrapeLangToken::Number {
                value, pos_after, ..
            }) => Ok((str::parse(value)?, *pos_after)),
            Some(tok) => Err(Error::ParseError(format!(
                "Expected `Number` but found `{}` at line {} column {}",
                tok.name(),
                tok.pos().row,
                tok.pos().col
            ))),
            None => Err(Error::ParseError(format!(
                "Unexpected EOF at line {}",
                pos.row
            ))),
        }
    }

    fn identifier(
        token: Option<&'b ScrapeLangToken<'a>>,
        pos: TextPosition,
    ) -> Result<(&'a str, TextPosition), Error> {
        match token {
            Some(ScrapeLangToken::Identifier {
                name, pos_after, ..
            }) => Ok((name, *pos_after)),
            Some(tok) => Err(Error::ParseError(format!(
                "Expected `Identifier` but found `{}` at line {} column {}",
                tok.name(),
                tok.pos().row,
                tok.pos().col
            ))),
            None => Err(Error::ParseError(format!(
                "Unexpected EOF at line {}",
                pos.row
            ))),
        }
    }

    fn call_args(
        maybe_tokens: Option<&'b [ScrapeLangToken<'a>]>,
        pos: TextPosition,
    ) -> Result<CallArgs, Error> {
        let mut result = CallArgs {
            args: vec![],
            kwargs: HashMap::new(),
            num_tokens: 0,
            pos_after: pos,
        };

        let tokens_no_ws = maybe_tokens.map(|tokens| {
            tokens
                .iter()
                .filter(|tok| tok.name() != "Whitespace")
                .collect::<Vec<_>>()
        });

        if tokens_no_ws.as_ref().is_some_and(|tokens| {
            tokens
                .first()
                .is_some_and(|tok| tok.name() == "LeftParenthesis")
        }) {
            let tokens = tokens_no_ws.unwrap();
            let last_token: Option<&ScrapeLangToken>;
            let mut index = 1;
            let mut need_comma = false;

            loop {
                match tokens.get(index) {
                    Some(ScrapeLangToken::String { pos_after, str, .. }) if !need_comma => {
                        result
                            .args
                            .push(ScrapeLangArgument::String { str: unescape(str) });
                        result.pos_after = *pos_after;
                        need_comma = true;
                        index += 1;
                    }
                    Some(ScrapeLangToken::Identifier {
                        pos_after, name, ..
                    }) if !need_comma => {
                        if tokens
                            .get(index + 1)
                            .is_some_and(|tok| tok.name() == "Equals")
                        {
                            match tokens.get(index + 2) {
                                Some(ScrapeLangToken::String { pos_after, str, .. }) => {
                                    result.kwargs.insert(
                                        name.to_string(),
                                        ScrapeLangArgument::String { str: unescape(str) },
                                    );
                                    result.pos_after = *pos_after;
                                    need_comma = true;
                                    index += 3;
                                }
                                Some(ScrapeLangToken::Identifier {
                                    pos_after,
                                    name: name2,
                                    ..
                                }) => {
                                    result.kwargs.insert(
                                        name.to_string(),
                                        ScrapeLangArgument::Identifier {
                                            name: name2.to_string(),
                                        },
                                    );
                                    result.pos_after = *pos_after;
                                    need_comma = true;
                                    index += 3;
                                }
                                Some(tok) => {
                                    return Err(Error::ParseError(format!(
                                        "Unexpected `{}` at line {} column {}",
                                        tok.name(),
                                        tok.pos().row,
                                        tok.pos().col
                                    )))
                                }
                                None => {
                                    return Err(Error::ParseError(format!(
                                        "Unexpected EOF at line {}",
                                        pos_after.row
                                    )))
                                }
                            };
                        } else {
                            result.args.push(ScrapeLangArgument::Identifier {
                                name: name.to_string(),
                            });
                            result.pos_after = *pos_after;
                            need_comma = true;
                            index += 1;
                        }
                    }
                    Some(ScrapeLangToken::Comma { pos_after, .. }) if need_comma => {
                        result.pos_after = *pos_after;
                        need_comma = false;
                        index += 1;
                    }
                    Some(tok @ ScrapeLangToken::RightParenthesis { pos_after, .. }) => {
                        result.num_tokens += 1;
                        result.pos_after = *pos_after;
                        last_token = Some(*tok);
                        break;
                    }
                    Some(tok) => {
                        return Err(Error::ParseError(format!(
                            "Unexpected `{}` at line {} column {}",
                            tok.name(),
                            tok.pos().row,
                            tok.pos().col
                        )))
                    }
                    None => {
                        return Err(Error::ParseError(format!(
                            "Unexpected EOF at line {}",
                            result.pos_after.row
                        )))
                    }
                }
            }

            if let Some(token) = last_token {
                let actual_pos = maybe_tokens
                    .unwrap()
                    .iter()
                    .enumerate()
                    .find(|(_, x)| std::ptr::eq(token, *x))
                    .unwrap();
                result.num_tokens = actual_pos.0 + 1;
            }
        }

        Ok(result)
    }

    pub fn parse_append(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Append,
            "Append",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (text, _) = Self::string(tokens.get(2), pos_after)?;
                Self::statement_terminator(tokens.get(3))?;
                Ok((
                    ScrapeLangInstruction::Append {
                        str: unescape(text),
                    },
                    3,
                ))
            }
        )
    }

    pub fn parse_clear(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Clear,
            "Clear",
            |tokens: &'b [ScrapeLangToken<'a>], _: TextPosition| {
                Self::statement_terminator(tokens.get(1))?;
                Ok((ScrapeLangInstruction::Clear, 1))
            }
        )
    }

    pub fn parse_clear_headers(
        tokens: &'b [ScrapeLangToken<'a>],
        pos: TextPosition,
    ) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            ClearHeaders,
            "ClearHeaders",
            |tokens: &'b [ScrapeLangToken<'a>], _: TextPosition| {
                Self::statement_terminator(tokens.get(1))?;
                Ok((ScrapeLangInstruction::ClearHeaders, 1))
            }
        )
    }

    pub fn parse_delete(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Delete,
            "Delete",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (text, _) = Self::string(tokens.get(2), pos_after)?;
                Self::statement_terminator(tokens.get(3))?;
                Ok((
                    ScrapeLangInstruction::Delete {
                        regex: unescape(text),
                    },
                    3,
                ))
            }
        )
    }

    pub fn parse_discard(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Discard,
            "Discard",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (text, _) = Self::string(tokens.get(2), pos_after)?;
                Self::statement_terminator(tokens.get(3))?;
                Ok((
                    ScrapeLangInstruction::Discard {
                        regex: unescape(text),
                    },
                    3,
                ))
            }
        )
    }

    pub fn parse_drop(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Drop,
            "Drop",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (number, _) = Self::number(tokens.get(2), pos_after)?;
                Self::statement_terminator(tokens.get(3))?;
                Ok((ScrapeLangInstruction::Drop { count: number }, 3))
            }
        )
    }

    pub fn parse_effect(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Effect,
            "Effect",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (name, pos_after) = Self::identifier(tokens.get(2), pos_after)?;
                let call_args = Self::call_args(tokens.get(3..), pos_after)?;
                Self::statement_terminator(tokens.get(3 + call_args.num_tokens))?;
                Ok((
                    ScrapeLangInstruction::Effect {
                        effect_name: name.to_string(),
                        args: call_args.args,
                        kwargs: call_args.kwargs,
                    },
                    3 + call_args.num_tokens,
                ))
            }
        )
    }

    pub fn parse_extract(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Extract,
            "Extract",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (text, _) = Self::string(tokens.get(2), pos_after)?;
                Self::statement_terminator(tokens.get(3))?;
                Ok((
                    ScrapeLangInstruction::Extract {
                        regex: unescape(text),
                    },
                    3,
                ))
            }
        )
    }

    pub fn parse_first(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            First,
            "First",
            |tokens: &'b [ScrapeLangToken<'a>], _: TextPosition| {
                Self::statement_terminator(tokens.get(1))?;
                Ok((ScrapeLangInstruction::First, 1))
            }
        )
    }

    pub fn parse_get(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Get,
            "Get",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (text, _) = Self::string(tokens.get(2), pos_after)?;
                Self::statement_terminator(tokens.get(3))?;
                Ok((
                    ScrapeLangInstruction::Get {
                        url: unescape(text),
                    },
                    3,
                ))
            }
        )
    }

    pub fn parse_header(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Header,
            "Header",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (key, pos_after) = Self::string(tokens.get(2), pos_after)?;
                let pos_after = Self::separator(tokens.get(3), pos_after)?;
                let (value, _) = Self::string(tokens.get(4), pos_after)?;
                Self::statement_terminator(tokens.get(5))?;
                Ok((
                    ScrapeLangInstruction::Header {
                        key: unescape(key),
                        value: unescape(value),
                    },
                    5,
                ))
            }
        )
    }

    pub fn parse_load(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Load,
            "Load",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (name, _) = Self::identifier(tokens.get(2), pos_after)?;
                Self::statement_terminator(tokens.get(3))?;
                Ok((
                    ScrapeLangInstruction::Load {
                        varname: name.to_string(),
                    },
                    3,
                ))
            }
        )
    }

    pub fn parse_prepend(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Prepend,
            "Prepend",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (text, _) = Self::string(tokens.get(2), pos_after)?;
                Self::statement_terminator(tokens.get(3))?;
                Ok((
                    ScrapeLangInstruction::Prepend {
                        str: unescape(text),
                    },
                    3,
                ))
            }
        )
    }

    pub fn parse_retain(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Retain,
            "Retain",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (text, _) = Self::string(tokens.get(2), pos_after)?;
                Self::statement_terminator(tokens.get(3))?;
                Ok((
                    ScrapeLangInstruction::Retain {
                        regex: unescape(text),
                    },
                    3,
                ))
            }
        )
    }

    pub fn parse_run(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Run,
            "Run",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (name, pos_after) = Self::identifier(tokens.get(2), pos_after)?;
                let call_args = Self::call_args(tokens.get(3..), pos_after)?;
                Self::statement_terminator(tokens.get(3 + call_args.num_tokens))?;
                Ok((
                    ScrapeLangInstruction::Run {
                        job_name: name.to_string(),
                        args: call_args.args,
                        kwargs: call_args.kwargs,
                    },
                    3 + call_args.num_tokens,
                ))
            }
        )
    }

    pub fn parse_store(tokens: &'b [ScrapeLangToken<'a>], pos: TextPosition) -> ParseResult {
        try_parse!(
            tokens,
            pos,
            Store,
            "Store",
            |tokens: &'b [ScrapeLangToken<'a>], pos_after: TextPosition| {
                let pos_after = Self::separator(tokens.get(1), pos_after)?;
                let (name, _) = Self::identifier(tokens.get(2), pos_after)?;
                Self::statement_terminator(tokens.get(3))?;
                Ok((
                    ScrapeLangInstruction::Store {
                        varname: name.to_string(),
                    },
                    3,
                ))
            }
        )
    }
}

pub fn parse<'a, 'b>(tokens: &'b [ScrapeLangToken<'a>]) -> Result<Vec<ScrapeLangInstruction>, Error>
where
    'a: 'b,
{
    let mut tokens_ws_dedup = tokens.to_vec();
    tokens_ws_dedup.dedup_by(|a, b| a.name() == "Whitespace" && b.name() == "Whitespace");

    let mut rest = tokens_ws_dedup.as_slice();
    let mut result = vec![];

    while !rest.is_empty() {
        while let Some(ScrapeLangToken::Whitespace { .. }) = rest.first() {
            rest = &rest[1..];
        }

        if rest.is_empty() {
            break;
        }

        match rest.first() {
            Some(ScrapeLangToken::Append { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_append(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Clear { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_clear(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::ClearHeaders { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_clear_headers(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Delete { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_delete(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Discard { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_discard(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Drop { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_drop(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Effect { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_effect(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Extract { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_extract(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::First { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_first(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Get { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_get(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Header { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_header(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Load { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_load(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Prepend { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_prepend(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Retain { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_retain(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Run { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_run(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(ScrapeLangToken::Store { pos, .. }) => {
                let (instr, num_toks) = ScrapeLangInstruction::parse_store(rest, *pos)?;
                result.push(instr);
                rest = &rest[num_toks..];
            }
            Some(tok) => {
                return Err(Error::ParseError(format!(
                    "Syntax error, unexpected `{}` at line {} column {}",
                    tok.name(),
                    tok.pos().row,
                    tok.pos().col
                )))
            }
            None => todo!(),
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos_after(startrow: usize, startcol: usize, text: &str) -> TextPosition {
        let mut result = TextPosition {
            row: startrow,
            col: startcol,
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

    fn tokenseq(spec: &[&'static str]) -> Vec<ScrapeLangToken<'static>> {
        let mut result = vec![];
        let mut pos = TextPosition { row: 1, col: 1 };

        macro_rules! simple {
            ($variant:ident, $text:expr) => {{
                result.push(ScrapeLangToken::$variant {
                    pos,
                    pos_after: pos_after(pos.row, pos.col, $text),
                });
                pos = pos_after(pos.row, pos.col, $text);
            }};
        }

        for str in spec {
            let stuff = str.splitn(2, " ").collect::<Vec<_>>();

            #[allow(clippy::get_first)]
            match (stuff.get(0), stuff.get(1)) {
                (Some(&"append"), _) => simple!(Append, "append"),
                (Some(&"clear"), _) => simple!(Clear, "clear"),
                (Some(&"clearheaders"), _) => simple!(ClearHeaders, "clearheaders"),
                (Some(&"delete"), _) => simple!(Delete, "delete"),
                (Some(&"discard"), _) => simple!(Discard, "discard"),
                (Some(&"drop"), _) => simple!(Drop, "drop"),
                (Some(&"effect"), _) => simple!(Effect, "effect"),
                (Some(&"extract"), _) => simple!(Extract, "extract"),
                (Some(&"first"), _) => simple!(First, "first"),
                (Some(&"get"), _) => simple!(Get, "get"),
                (Some(&"header"), _) => simple!(Header, "header"),
                (Some(&"load"), _) => simple!(Load, "load"),
                (Some(&"prepend"), _) => simple!(Prepend, "prepend"),
                (Some(&"retain"), _) => simple!(Retain, "retain"),
                (Some(&"run"), _) => simple!(Run, "run"),
                (Some(&"store"), _) => simple!(Store, "store"),
                (Some(&"space"), _) => simple!(Whitespace, " "),
                (Some(&"("), _) => simple!(LeftParenthesis, "("),
                (Some(&")"), _) => simple!(RightParenthesis, ")"),
                (Some(&","), _) => simple!(Comma, ","),
                (Some(&"="), _) => simple!(Equals, "="),
                (Some(&"string"), Some(str)) => {
                    result.push(ScrapeLangToken::String {
                        pos,
                        pos_after: pos_after(pos.row, pos.col, &format!("\"{}\"", str)),
                        str,
                    });
                    pos = pos_after(pos.row, pos.col, &format!("\"{}\"", str));
                }
                (Some(&"number"), Some(value)) => {
                    result.push(ScrapeLangToken::Number {
                        pos,
                        pos_after: pos_after(pos.row, pos.col, str),
                        value,
                    });
                    pos = pos_after(pos.row, pos.col, str);
                }
                (Some(&"ident"), Some(name)) => {
                    result.push(ScrapeLangToken::Identifier {
                        pos,
                        pos_after: pos_after(pos.row, pos.col, str),
                        name,
                    });
                    pos = pos_after(pos.row, pos.col, str);
                }
                (a, b) => panic!("dont know what to do with {:?}, {:?}", a, b),
            }
        }

        result
    }

    #[test]
    pub fn test_parse_append() {
        assert!(
            parse(tokenseq(&["append", "space", "string hello"]).as_slice()).is_ok_and(|result| {
                assert_eq!(
                    result[0],
                    ScrapeLangInstruction::Append {
                        str: "hello".to_string()
                    }
                );
                true
            })
        );
    }

    #[test]
    pub fn test_parse_clear() {
        assert!(parse(tokenseq(&["clear"]).as_slice()).is_ok_and(|result| {
            assert_eq!(result[0], ScrapeLangInstruction::Clear);
            true
        }));
    }

    #[test]
    pub fn test_parse_clear_headers() {
        assert!(
            parse(tokenseq(&["clearheaders"]).as_slice()).is_ok_and(|result| {
                assert_eq!(result[0], ScrapeLangInstruction::ClearHeaders);
                true
            })
        );
    }

    #[test]
    pub fn test_parse_delete() {
        assert!(
            parse(tokenseq(&["delete", "space", "string [a-z]+"]).as_slice()).is_ok_and(|result| {
                assert_eq!(
                    result[0],
                    ScrapeLangInstruction::Delete {
                        regex: "[a-z]+".to_string()
                    }
                );
                true
            })
        );
    }

    #[test]
    pub fn test_parse_discard() {
        assert!(
            parse(tokenseq(&["discard", "space", "string unwanted"]).as_slice()).is_ok_and(
                |result| {
                    assert!(matches!(
                        &result[0],
                        ScrapeLangInstruction::Discard { regex } if regex == "unwanted"));
                    true
                }
            )
        );
    }

    #[test]
    pub fn test_parse_drop() {
        assert!(
            parse(tokenseq(&["drop", "space", "number 2"]).as_slice()).is_ok_and(|result| {
                assert_eq!(result[0], ScrapeLangInstruction::Drop { count: 2 });
                true
            })
        );
    }

    #[test]
    pub fn test_parse_effect() {
        assert!(
            parse(tokenseq(&["effect", "space", "ident notify"]).as_slice()).is_ok_and(|result| {
                assert_eq!(
                    result[0],
                    ScrapeLangInstruction::Effect {
                        effect_name: "notify".to_string(),
                        args: vec![],
                        kwargs: HashMap::new(),
                    }
                );
                true
            })
        );

        assert!(
            parse(tokenseq(&["effect", "space", "ident notify", "(", ")"]).as_slice()).is_ok_and(
                |result| {
                    assert_eq!(
                        result[0],
                        ScrapeLangInstruction::Effect {
                            effect_name: "notify".to_string(),
                            args: vec![],
                            kwargs: HashMap::new(),
                        }
                    );
                    true
                }
            )
        );

        assert!(parse(
            tokenseq(&[
                "effect",
                "space",
                "ident notify",
                "(",
                "ident $x",
                ",",
                "ident $y",
                ")"
            ])
            .as_slice()
        )
        .is_ok_and(|result| {
            assert_eq!(
                result[0],
                ScrapeLangInstruction::Effect {
                    effect_name: "notify".to_string(),
                    args: vec![
                        ScrapeLangArgument::Identifier {
                            name: "$x".to_string()
                        },
                        ScrapeLangArgument::Identifier {
                            name: "$y".to_string()
                        },
                    ],
                    kwargs: HashMap::new(),
                }
            );
            true
        }));

        assert!(parse(
            tokenseq(&[
                "effect",
                "space",
                "ident notify",
                "(",
                "ident $x",
                ",",
                "ident $y",
                ")"
            ])
            .as_slice()
        )
        .is_ok_and(|result| {
            assert_eq!(
                result[0],
                ScrapeLangInstruction::Effect {
                    effect_name: "notify".to_string(),
                    args: vec![
                        ScrapeLangArgument::Identifier {
                            name: "$x".to_string()
                        },
                        ScrapeLangArgument::Identifier {
                            name: "$y".to_string()
                        },
                    ],
                    kwargs: HashMap::new(),
                }
            );
            true
        }));

        assert!(parse(
            tokenseq(&[
                "effect",
                "space",
                "ident notify",
                "(",
                "ident $x",
                ",",
                "ident foo",
                "=",
                "string bar",
                ",",
                "ident $y",
                ")"
            ])
            .as_slice()
        )
        .is_ok_and(|result| {
            assert_eq!(
                result[0],
                ScrapeLangInstruction::Effect {
                    effect_name: "notify".to_string(),
                    args: vec![
                        ScrapeLangArgument::Identifier {
                            name: "$x".to_string()
                        },
                        ScrapeLangArgument::Identifier {
                            name: "$y".to_string()
                        },
                    ],
                    kwargs: HashMap::from_iter([(
                        "foo".to_string(),
                        ScrapeLangArgument::String {
                            str: "bar".to_string()
                        }
                    )]),
                }
            );
            true
        }));

        assert!(parse(
            tokenseq(&[
                "effect",
                "space",
                "ident notify",
                "space",
                "(",
                "ident $x",
                "space",
                ",",
                "ident foo",
                "space",
                "=",
                "space",
                "string bar",
                ",",
                "ident $y",
                ")"
            ])
            .as_slice()
        )
        .is_ok_and(|result| {
            assert_eq!(
                result[0],
                ScrapeLangInstruction::Effect {
                    effect_name: "notify".to_string(),
                    args: vec![
                        ScrapeLangArgument::Identifier {
                            name: "$x".to_string()
                        },
                        ScrapeLangArgument::Identifier {
                            name: "$y".to_string()
                        },
                    ],
                    kwargs: HashMap::from_iter([(
                        "foo".to_string(),
                        ScrapeLangArgument::String {
                            str: "bar".to_string()
                        }
                    )]),
                }
            );
            true
        }));
    }

    #[test]
    pub fn test_parse_extract() {
        assert!(
            parse(tokenseq(&["extract", "space", "string \\\\w{3}?;"]).as_slice()).is_ok_and(
                |result| {
                    assert_eq!(
                        result[0],
                        ScrapeLangInstruction::Extract {
                            regex: "\\w{3}?;".to_string(),
                        }
                    );
                    true
                }
            )
        );
    }

    #[test]
    pub fn test_parse_first() {
        assert!(parse(tokenseq(&["first"]).as_slice()).is_ok_and(|result| {
            assert_eq!(result[0], ScrapeLangInstruction::First);
            true
        }));
    }

    #[test]
    pub fn test_parse_get() {
        assert!(
            parse(tokenseq(&["get", "space", "string https://www.rust-lang.org/"]).as_slice())
                .is_ok_and(|result| {
                    assert_eq!(
                        result[0],
                        ScrapeLangInstruction::Get {
                            url: "https://www.rust-lang.org/".to_string(),
                        }
                    );
                    true
                })
        );
    }

    #[test]
    pub fn test_parse_header() {
        assert!(parse(
            tokenseq(&[
                "header",
                "space",
                "string User-Agent",
                "space",
                "string Chromium"
            ])
            .as_slice()
        )
        .is_ok_and(|result| {
            assert_eq!(
                result[0],
                ScrapeLangInstruction::Header {
                    key: "User-Agent".to_string(),
                    value: "Chromium".to_string(),
                }
            );
            true
        }));
    }

    #[test]
    pub fn test_parse_load() {
        assert!(
            parse(tokenseq(&["load", "space", "ident $x"]).as_slice()).is_ok_and(|result| {
                assert_eq!(
                    result[0],
                    ScrapeLangInstruction::Load {
                        varname: "$x".to_string()
                    }
                );
                true
            })
        );
    }

    #[test]
    pub fn test_parse_prepend() {
        assert!(
            parse(tokenseq(&["prepend", "space", "string foo bar baz"]).as_slice()).is_ok_and(
                |result| {
                    assert_eq!(
                        result[0],
                        ScrapeLangInstruction::Prepend {
                            str: "foo bar baz".to_string()
                        }
                    );
                    true
                }
            )
        );
    }

    #[test]
    pub fn test_parse_retain() {
        assert!(
            parse(tokenseq(&["retain", "space", "string wanted"]).as_slice()).is_ok_and(|result| {
                assert!(matches!(
                    &result[0],
                    ScrapeLangInstruction::Retain { regex } if regex == "wanted"));
                true
            })
        );
    }

    #[test]
    pub fn test_parse_run() {
        assert!(
            parse(tokenseq(&["run", "space", "ident word-of-the-day"]).as_slice()).is_ok_and(
                |result| {
                    assert_eq!(
                        result[0],
                        ScrapeLangInstruction::Run {
                            job_name: "word-of-the-day".to_string(),
                            args: vec![],
                            kwargs: HashMap::new(),
                        }
                    );
                    true
                }
            )
        );
    }

    #[test]
    pub fn test_parse_store() {
        assert!(
            parse(tokenseq(&["store", "space", "ident $y"]).as_slice()).is_ok_and(|result| {
                assert_eq!(
                    result[0],
                    ScrapeLangInstruction::Store {
                        varname: "$y".to_string()
                    }
                );
                true
            })
        );
    }
}
