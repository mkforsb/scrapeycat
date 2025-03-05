use std::str::FromStr;

use winnow::Parser;

use crate::{
    util::boundedu8::{BoundedU8, BoundedU8RangeInclusive, UpperBoundedNonZeroU8},
    Error,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum CronSpecItem<const L: u8, const H: u8> {
    Any,
    AnyStepped(UpperBoundedNonZeroU8<H>),
    Single(BoundedU8<L, H>),
    SingleStepped(BoundedU8<L, H>, UpperBoundedNonZeroU8<H>),
    Range(BoundedU8RangeInclusive<L, H>),
    RangeStepped(BoundedU8RangeInclusive<L, H>, UpperBoundedNonZeroU8<H>),
}

impl<const L: u8, const H: u8> CronSpecItem<L, H> {
    pub fn to_regex_pattern(&self) -> String {
        match self {
            CronSpecItem::Any => "..".to_string(),
            CronSpecItem::AnyStepped(step) => (L..=H)
                .step_by(step.get() as usize)
                .map(|n| format!("{n:02}"))
                .collect::<Vec<_>>()
                .join("|"),
            CronSpecItem::Single(n) => format!("{:02}", n.get()),
            CronSpecItem::SingleStepped(n, step) => (n.get()..=H)
                .step_by(step.get() as usize)
                .map(|n| format!("{n:02}"))
                .collect::<Vec<_>>()
                .join("|"),
            CronSpecItem::Range(range) => range
                .get()
                .map(|n| format!("{n:02}"))
                .collect::<Vec<_>>()
                .join("|"),
            CronSpecItem::RangeStepped(range, step) => range
                .get()
                .step_by(step.get() as usize)
                .map(|n| format!("{n:02}"))
                .collect::<Vec<_>>()
                .join("|"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CronSpec {
    minute: Vec<CronSpecItem<0, 59>>,
    hour: Vec<CronSpecItem<0, 23>>,
    day_of_month: Vec<CronSpecItem<1, 31>>,
    month: Vec<CronSpecItem<1, 12>>,
    day_of_week: Vec<CronSpecItem<1, 7>>,
}

impl CronSpec {
    pub fn to_regex_pattern(&self) -> String {
        format!(
            "({})({})({})({})({})",
            self.minute
                .iter()
                .map(|x| x.to_regex_pattern())
                .collect::<Vec<_>>()
                .join("|"),
            self.hour
                .iter()
                .map(|x| x.to_regex_pattern())
                .collect::<Vec<_>>()
                .join("|"),
            self.day_of_month
                .iter()
                .map(|x| x.to_regex_pattern())
                .collect::<Vec<_>>()
                .join("|"),
            self.month
                .iter()
                .map(|x| x.to_regex_pattern())
                .collect::<Vec<_>>()
                .join("|"),
            self.day_of_week
                .iter()
                .map(|x| x.to_regex_pattern())
                .collect::<Vec<_>>()
                .join("|"),
        )
    }
}

impl FromStr for CronSpec {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse::parse_cronspec.parse(s).map_err(|e| {
            Error::ParseError(format!(
                r#"Invalid cron spec:
-------------------------------
{e}
-------------------------------
"#
            ))
        })
    }
}

mod parse {
    use winnow::{
        ascii::{digit1, multispace0, multispace1},
        combinator::{alt, cut_err, opt, peek},
        error::{AddContext, ContextError, ErrMode, ParserError, StrContext},
        stream::Stream,
        token::literal,
        ModalResult, Parser,
    };

    use super::{CronSpec, CronSpecItem};

    fn number<const L: u8, const H: u8>(
        label: &'static str,
    ) -> impl FnMut(&mut &str) -> ModalResult<u8> {
        move |input: &mut &str| -> ModalResult<u8> {
            digit1
                .parse_to::<u8>()
                .verify(|n| *n >= L && *n <= H)
                .context(StrContext::Label(label))
                .parse_next(input)
        }
    }

    fn nonzero_number<const L: u8, const H: u8>(
        label: &'static str,
    ) -> impl FnMut(&mut &str) -> ModalResult<u8> {
        move |input: &mut &str| -> ModalResult<u8> {
            digit1
                .parse_to::<u8>()
                .verify(|n| *n > 0 && *n >= L && *n <= H)
                .context(StrContext::Label(label))
                .parse_next(input)
        }
    }

    fn stepped<const L: u8, const H: u8>() -> impl FnMut(&mut &str) -> ModalResult<u8> {
        move |input: &mut &str| {
            ('/', nonzero_number::<L, H>("step"))
                .parse_next(input)
                .map(|(_, step)| step)
        }
    }

    fn any<const L: u8, const H: u8>(input: &mut &str) -> ModalResult<CronSpecItem<L, H>> {
        '*'.parse_next(input).map(|_| CronSpecItem::Any)
    }

    fn any_stepped<const L: u8, const H: u8>(input: &mut &str) -> ModalResult<CronSpecItem<L, H>> {
        if peek::<_, _, ContextError, _>("*/")
            .parse_next(input)
            .is_ok()
        {
            match cut_err((any::<L, H>, stepped::<L, H>())).parse_next(input) {
                Ok((_, step)) => Ok(CronSpecItem::AnyStepped(
                    step.try_into()
                        .expect("valid due to Parser::verify in stepped()"),
                )),
                Err(e) => Err(e),
            }
        } else {
            Err(ErrMode::Backtrack(ParserError::from_input(input)))
        }
    }

    fn single<const L: u8, const H: u8>(input: &mut &str) -> ModalResult<CronSpecItem<L, H>> {
        digit1
            .parse_to::<u8>()
            .verify(|n| *n >= L && *n <= H)
            .parse_next(input)
            .map(|n| CronSpecItem::Single(n.try_into().expect("valid due to Parser::verify")))
    }

    fn single_stepped<const L: u8, const H: u8>(
        input: &mut &str,
    ) -> ModalResult<CronSpecItem<L, H>> {
        if peek((digit1::<_, ContextError>, '/'))
            .parse_next(input)
            .is_ok()
        {
            cut_err((number::<L, H>("offset"), stepped::<L, H>()))
                .parse_next(input)
                .map(|(minute, step)| {
                    CronSpecItem::SingleStepped(
                        minute
                            .try_into()
                            .expect("valid due to Parser::verify in number()"),
                        step.try_into()
                            .expect("valid due to Parser::verify in nonzero_number()"),
                    )
                })
        } else {
            Err(ErrMode::Backtrack(ParserError::from_input(input)))
        }
    }

    fn range<const L: u8, const H: u8>(input: &mut &str) -> ModalResult<CronSpecItem<L, H>> {
        let orig_checkpoint = input.checkpoint();

        if peek((digit1::<_, ContextError>, '-'))
            .parse_next(input)
            .is_ok()
        {
            cut_err((
                number::<L, H>("range start"),
                '-',
                number::<L, H>("range end"),
            ))
            .parse_next(input)
            .and_then(|(start, _, end)| {
                Ok(CronSpecItem::Range((start..=end).try_into().map_err(
                    |_| {
                        Stream::reset(input, &orig_checkpoint);
                        ErrMode::Cut(ContextError::new().add_context(
                            input,
                            &orig_checkpoint,
                            StrContext::Label("range"),
                        ))
                    },
                )?))
            })
        } else {
            Err(ErrMode::Backtrack(ParserError::from_input(input)))
        }
    }

    fn range_stepped<const L: u8, const H: u8>(
        input: &mut &str,
    ) -> ModalResult<CronSpecItem<L, H>> {
        if peek((range::<L, H>, '/')).parse_next(input).is_ok() {
            cut_err((range::<L, H>, stepped::<L, H>()))
                .parse_next(input)
                .map(|(range, step)| match range {
                    CronSpecItem::Range(r) => CronSpecItem::RangeStepped(
                        r,
                        step.try_into()
                            .expect("valid due to Parser::verify in stepped()"),
                    ),
                    _ => panic!("impossible"),
                })
        } else {
            Err(ErrMode::Backtrack(ParserError::from_input(input)))
        }
    }

    fn cronspec_single_item<const L: u8, const H: u8>(
        input: &mut &str,
    ) -> ModalResult<CronSpecItem<L, H>> {
        alt((
            range_stepped,
            single_stepped,
            any_stepped,
            range,
            single,
            any,
        ))
        .parse_next(input)
    }

    fn cronspec_item<const L: u8, const H: u8>(
        input: &mut &str,
    ) -> ModalResult<Vec<CronSpecItem<L, H>>> {
        let mut result = vec![];

        match cronspec_single_item::<L, H>.parse_next(input) {
            Ok(item) => result.push(item),
            Err(e) => return Err(e),
        }

        while opt(literal(',')).parse_next(input)?.is_some() {
            match cronspec_single_item::<L, H>.parse_next(input) {
                Ok(item) => result.push(item),
                Err(e) => return Err(e),
            }
        }

        Ok(result)
    }

    pub fn parse_cronspec(input: &mut &str) -> ModalResult<CronSpec> {
        let (_, minute, _, hour, _, day_of_month, _, month, _, day_of_week, _) = (
            multispace0,
            cronspec_item.context(StrContext::Label("minute")),
            multispace1,
            cronspec_item.context(StrContext::Label("hour")),
            multispace1,
            cronspec_item.context(StrContext::Label("day of month")),
            multispace1,
            cronspec_item.context(StrContext::Label("month")),
            multispace1,
            cronspec_item.context(StrContext::Label("day of week")),
            multispace0,
        )
            .parse_next(input)?;

        Ok(CronSpec {
            minute,
            hour,
            day_of_month,
            month,
            day_of_week,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Bound;

    use bolero::{check, gen, TypeGenerator};
    use regex::Regex;

    use super::*;

    #[derive(Debug, Clone, TypeGenerator)]
    struct Single<const L: u8, const H: u8> {
        #[generator(gen::<u8>().with().bounds(L..=H))]
        value: u8,
    }

    #[derive(Debug, Clone)]
    struct Range<const L: u8, const H: u8> {
        start: u8,
        end: u8,
    }

    impl<const L: u8, const H: u8> TypeGenerator for Range<L, H> {
        fn generate<D: bolero::Driver>(driver: &mut D) -> Option<Self> {
            driver
                .gen_u8(Bound::Included(&L), Bound::Included(&H))
                .and_then(|start| {
                    driver
                        .gen_u8(Bound::Included(&start), Bound::Included(&H))
                        .map(|end| (start, end))
                })
                .map(|(start, end)| Range { start, end })
        }
    }

    #[derive(Debug, Clone, TypeGenerator)]
    struct Step<const H: u8> {
        #[generator(gen::<u8>().with().bounds(1..=H))]
        value: u8,
    }

    #[derive(Debug, Clone, TypeGenerator)]
    enum Item<const L: u8, const H: u8> {
        Any,
        AnyStepped(Step<H>),
        Single(Single<L, H>),
        SingleStepped(Single<L, H>, Step<H>),
        Range(Range<L, H>),
        RangeStepped(Range<L, H>, Step<H>),
    }

    impl<const L: u8, const H: u8> Item<L, H> {
        pub fn to_syntax(&self) -> String {
            match self {
                Item::Any => "*".to_string(),
                Item::AnyStepped(valid_step) => format!("*/{}", valid_step.value),
                Item::Single(valid_single) => format!("{}", valid_single.value),
                Item::SingleStepped(valid_single, valid_step) => {
                    format!("{}/{}", valid_single.value, valid_step.value)
                }
                Item::Range(valid_range) => {
                    format!("{}-{}", valid_range.start, valid_range.end)
                }
                Item::RangeStepped(valid_range, valid_step) => format!(
                    "{}-{}/{}",
                    valid_range.start, valid_range.end, valid_step.value
                ),
            }
        }
    }

    #[derive(Debug, Clone, TypeGenerator)]
    struct ValidSpec {
        #[generator(gen::<Vec<_>>().with().len(1..=3))]
        minute: Vec<Item<0, 59>>,

        #[generator(gen::<Vec<_>>().with().len(1..=3))]
        hour: Vec<Item<0, 23>>,

        #[generator(gen::<Vec<_>>().with().len(1..=3))]
        day_of_month: Vec<Item<1, 31>>,

        #[generator(gen::<Vec<_>>().with().len(1..=3))]
        month: Vec<Item<1, 12>>,

        #[generator(gen::<Vec<_>>().with().len(1..=3))]
        day_of_week: Vec<Item<1, 7>>,
    }

    impl ValidSpec {
        pub fn to_syntax(&self) -> String {
            format!(
                "{} {} {} {} {}",
                self.minute
                    .iter()
                    .map(|item| item.to_syntax())
                    .collect::<Vec<_>>()
                    .join(","),
                self.hour
                    .iter()
                    .map(|item| item.to_syntax())
                    .collect::<Vec<_>>()
                    .join(","),
                self.day_of_month
                    .iter()
                    .map(|item| item.to_syntax())
                    .collect::<Vec<_>>()
                    .join(","),
                self.month
                    .iter()
                    .map(|item| item.to_syntax())
                    .collect::<Vec<_>>()
                    .join(","),
                self.day_of_week
                    .iter()
                    .map(|item| item.to_syntax())
                    .collect::<Vec<_>>()
                    .join(","),
            )
        }
    }

    // unimplemented ideas for transforming valid syntax into still-valid syntax
    // - randomly grow/shrink block of whitespace
    // - randomly change "N-M" to "N" or "M"
    // - randomly change "N/M" to "*/M"
    // where _ is either whitespace or comma:
    // - randomly change "_N_" to "_N-N_" and the inverse
    // - randomly change "_N_" to "_*_"
    // - randomly change "_N_" to "_N/M_" for 1 <= M <= 7

    // unimplemented ideas for corrupting valid syntax into guaranteed invalid syntax
    // - insert/substitute invalid char anywhere
    // - randomly change "N-M" to "X-M" for X > 59 (invalid range start)
    // - randomly change "N-M" to "N-X" for X > 59 (invalid range end)
    // - randomly change "N-M" to "N-(N-1)" for N > 0 (reverse range)
    // - randomly change "," to ",,"
    // - randomly change "*" to "**", "*-", "-*", "*-*", "/*" or "*/"
    // - randomly change "*" to "N*" or "*N" for some number N

    #[test]
    fn test_arbitrary_valid() {
        check!()
            .with_generator(gen::<ValidSpec>())
            .with_max_len(1000)
            .for_each(|spec| {
                assert!(spec.to_syntax().parse::<CronSpec>().is_ok());
            });
    }

    #[test]
    fn test_arbitrary_valid_extra_whitespace() {
        check!()
            .with_generator(gen::<ValidSpec>())
            .with_max_len(1000)
            .for_each(|spec| {
                let syntax = spec.to_syntax();

                assert!(format!("{}   ", syntax).parse::<CronSpec>().is_ok());
                assert!(format!("  {}   ", syntax).parse::<CronSpec>().is_ok());
                assert!(format!("    {}", syntax).parse::<CronSpec>().is_ok());
                assert!(syntax.replace(" ", "  ").parse::<CronSpec>().is_ok());
            });
    }

    #[test]
    fn test_arbitrary_valid_corrupt_step() {
        check!()
            .with_generator(gen::<ValidSpec>())
            .with_max_len(1000)
            .for_each(|spec| {
                let syntax = spec.to_syntax();

                if syntax.contains("/") {
                    let replace = Regex::new("/\\d+").expect("valid regex");

                    assert!(replace
                        .replace_all(&syntax, "/0")
                        .to_string()
                        .parse::<CronSpec>()
                        .is_err());

                    assert!(replace
                        .replace_all(&syntax, "/999")
                        .to_string()
                        .parse::<CronSpec>()
                        .is_err());
                }
            });
    }

    #[test]
    fn test_parse_valid() {
        assert!("* * * * *".parse::<CronSpec>().is_ok_and(|result| {
            assert_eq!(result.minute, vec![CronSpecItem::Any]);
            assert_eq!(result.hour, vec![CronSpecItem::Any]);
            assert_eq!(result.day_of_month, vec![CronSpecItem::Any]);
            assert_eq!(result.month, vec![CronSpecItem::Any]);
            assert_eq!(result.day_of_week, vec![CronSpecItem::Any]);
            true
        }));

        assert!("*/3 1,2 3/4 5-6 1-5/2"
            .parse::<CronSpec>()
            .is_ok_and(|result| {
                assert_eq!(
                    result.minute,
                    vec![CronSpecItem::AnyStepped(3.try_into().unwrap())]
                );
                assert_eq!(
                    result.hour,
                    vec![
                        CronSpecItem::Single(1.try_into().unwrap()),
                        CronSpecItem::Single(2.try_into().unwrap())
                    ]
                );
                assert_eq!(
                    result.day_of_month,
                    vec![CronSpecItem::SingleStepped(
                        3.try_into().unwrap(),
                        4.try_into().unwrap()
                    )]
                );
                assert_eq!(
                    result.month,
                    vec![CronSpecItem::Range((5..=6).try_into().unwrap())]
                );
                assert_eq!(
                    result.day_of_week,
                    vec![CronSpecItem::RangeStepped(
                        (1..=5).try_into().unwrap(),
                        2.try_into().unwrap()
                    )]
                );
                true
            }));
    }

    #[test]
    fn test_parse_invalid_missing_spec() {
        assert!("".parse::<CronSpec>().is_err());
        assert!("*".parse::<CronSpec>().is_err());
        assert!("* *".parse::<CronSpec>().is_err());
        assert!("* * *".parse::<CronSpec>().is_err());
        assert!("* * * *".parse::<CronSpec>().is_err());
    }

    #[test]
    fn test_parse_invalid_minute() {
        assert!("60 * * * *".parse::<CronSpec>().is_err());
        assert!("100 * * * *".parse::<CronSpec>().is_err());
    }

    #[test]
    fn test_parse_invalid_hour() {
        assert!("* 24 * * *".parse::<CronSpec>().is_err());
        assert!("* 100 * * *".parse::<CronSpec>().is_err());
    }

    #[test]
    fn test_parse_invalid_day_of_month() {
        assert!("* * 0 * *".parse::<CronSpec>().is_err());
        assert!("* * 32 * *".parse::<CronSpec>().is_err());
        assert!("* * 100 * *".parse::<CronSpec>().is_err());
    }

    #[test]
    fn test_parse_invalid_month() {
        assert!("* * * 0 *".parse::<CronSpec>().is_err());
        assert!("* * * 13 *".parse::<CronSpec>().is_err());
        assert!("* * * 100 *".parse::<CronSpec>().is_err());
    }

    #[test]
    fn test_parse_invalid_day_of_week() {
        assert!("* * * * 0".parse::<CronSpec>().is_err());
        assert!("* * * * 8".parse::<CronSpec>().is_err());
        assert!("* * * * 100".parse::<CronSpec>().is_err());
    }

    #[test]
    fn test_parse_invalid_step() {
        assert!("*/ * * * *".parse::<CronSpec>().is_err());
        assert!("* 1/ * * *".parse::<CronSpec>().is_err());
        assert!("* * 2-3/ * *".parse::<CronSpec>().is_err());
        assert!("* * * */ *".parse::<CronSpec>().is_err());
        assert!("* * * * 4/".parse::<CronSpec>().is_err());

        assert!("*/0 * * * *".parse::<CronSpec>().is_err());
        assert!("* 1/0 * * *".parse::<CronSpec>().is_err());
        assert!("* * 2-3/0 * *".parse::<CronSpec>().is_err());
        assert!("* * * */0 *".parse::<CronSpec>().is_err());
        assert!("* * * * 4/0".parse::<CronSpec>().is_err());

        assert!("*/60 * * * *".parse::<CronSpec>().is_err());
        assert!("* 1/24 * * *".parse::<CronSpec>().is_err());
        assert!("* * 2-3/32 * *".parse::<CronSpec>().is_err());
        assert!("* * * */13 *".parse::<CronSpec>().is_err());
        assert!("* * * * 4/8".parse::<CronSpec>().is_err());
    }

    #[test]
    fn test_parse_invalid_range_start() {
        assert!("60-61 * * * *".parse::<CronSpec>().is_err());
        assert!("* 24-25 * * *".parse::<CronSpec>().is_err());
        assert!("* * 32-33 * *".parse::<CronSpec>().is_err());
        assert!("* * * 13-14 *".parse::<CronSpec>().is_err());
        assert!("* * * * 8-9".parse::<CronSpec>().is_err());
    }

    #[test]
    fn test_parse_invalid_range_end() {
        assert!("0-60 * * * *".parse::<CronSpec>().is_err());
        assert!("* 0-24 * * *".parse::<CronSpec>().is_err());
        assert!("* * 1-32 * *".parse::<CronSpec>().is_err());
        assert!("* * * 1-13 *".parse::<CronSpec>().is_err());
        assert!("* * * * 1-8".parse::<CronSpec>().is_err());
    }

    #[test]
    fn test_parse_invalid_reverse_range() {
        assert!("2-1 * * * *".parse::<CronSpec>().is_err());
        assert!("* 3-2 * * *".parse::<CronSpec>().is_err());
        assert!("* * 4-3 * *".parse::<CronSpec>().is_err());
        assert!("* * * 5-4 *".parse::<CronSpec>().is_err());
        assert!("* * * * 6-5".parse::<CronSpec>().is_err());
    }

    #[test]
    fn test_cronspec_to_regex() {
        assert!("* * * * *"
            .parse::<CronSpec>()
            .is_ok_and(|result| { result.to_regex_pattern() == "(..)(..)(..)(..)(..)" }));

        assert!("1,5 * * * *"
            .parse::<CronSpec>()
            .is_ok_and(|result| { result.to_regex_pattern() == "(01|05)(..)(..)(..)(..)" }));

        assert!("* 2-3 * * *"
            .parse::<CronSpec>()
            .is_ok_and(|result| { result.to_regex_pattern() == "(..)(02|03)(..)(..)(..)" }));

        assert!("* * 4/10 * *"
            .parse::<CronSpec>()
            .is_ok_and(|result| { result.to_regex_pattern() == "(..)(..)(04|14|24)(..)(..)" }));

        assert!("* * * 3-7/2 *"
            .parse::<CronSpec>()
            .is_ok_and(|result| { result.to_regex_pattern() == "(..)(..)(..)(03|05|07)(..)" }));

        assert!("* * * * */3"
            .parse::<CronSpec>()
            .is_ok_and(|result| { result.to_regex_pattern() == "(..)(..)(..)(..)(01|04|07)" }));

        assert!("2,7 4-6 10/5 2/4 */2"
            .parse::<CronSpec>()
            .is_ok_and(|result| {
                result.to_regex_pattern()
                    == "(02|07)(04|05|06)(10|15|20|25|30)(02|06|10)(01|03|05|07)"
            }));
    }
}
