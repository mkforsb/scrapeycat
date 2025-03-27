use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use im::{vector, Vector};
use regex::Regex;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    effect::EffectInvocation,
    scrapelang::{
        lexer::lex,
        parser::{parse, ScrapeLangArgument, ScrapeLangInstruction},
        preprocessor::strip_comments,
    },
    scraper::{HttpDriver, Scraper},
    Error,
};

enum StepResult<H: HttpDriver> {
    UpdatedScraper(Scraper<H>),
    EffectInvocation(EffectInvocation),
    ScriptInvocation {
        name: String,
        args: Vec<String>,
        kwargs: HashMap<String, String>,
    },
    Get(String),
    Store {
        varname: String,
        value: Vector<String>,
    },
}

fn step<H: HttpDriver>(
    instruction: &ScrapeLangInstruction,
    scraper: &Scraper<H>,
    variables: &HashMap<String, Vector<String>>,
) -> Result<StepResult<H>, Error> {
    match instruction {
        ScrapeLangInstruction::Append { str } => Ok(StepResult::UpdatedScraper(
            scraper.append(&substitute_variables(str, variables)?),
        )),
        ScrapeLangInstruction::Clear => Ok(StepResult::UpdatedScraper(scraper.clear())),
        ScrapeLangInstruction::ClearHeaders => {
            Ok(StepResult::UpdatedScraper(scraper.clear_headers()))
        }
        ScrapeLangInstruction::Delete { regex } => Ok(StepResult::UpdatedScraper(
            scraper.delete(&substitute_variables(regex, variables)?)?,
        )),
        ScrapeLangInstruction::Drop { count } => {
            Ok(StepResult::UpdatedScraper(scraper.drop(*count)))
        }
        ScrapeLangInstruction::Extract { regex } => Ok(StepResult::UpdatedScraper(
            scraper.extract(&substitute_variables(regex, variables)?)?,
        )),
        ScrapeLangInstruction::Effect {
            effect_name,
            args,
            kwargs,
        } => {
            let args_subst = if !args.is_empty() {
                args.iter()
                    .map(|x| match x {
                        ScrapeLangArgument::String { str } => substitute_variables(str, variables),
                        ScrapeLangArgument::Identifier { name } => variables
                            .get(name)
                            .ok_or(Error::VariableNotFoundError(name.to_string()))
                            .map(|v| v.iter().cloned().collect::<Vec<_>>().join("")),
                    })
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                scraper.results().iter().cloned().collect::<Vec<_>>()
            };

            let kwargs_subst: HashMap<String, String> = kwargs
                .iter()
                .map(|(key, value)| match value {
                    ScrapeLangArgument::String { str } => {
                        Ok((key.clone(), substitute_variables(str, variables)?))
                    }
                    ScrapeLangArgument::Identifier { name } => Ok((
                        key.clone(),
                        variables
                            .get(name)
                            .ok_or(Error::VariableNotFoundError(name.to_string()))
                            .map(|v| v.iter().cloned().collect::<Vec<_>>().join(""))?,
                    )),
                })
                .collect::<Result<HashMap<String, String>, Error>>()?;

            Ok(StepResult::EffectInvocation(EffectInvocation::new(
                effect_name,
                args_subst,
                kwargs_subst,
            )))
        }
        ScrapeLangInstruction::First => Ok(StepResult::UpdatedScraper(scraper.first())),
        ScrapeLangInstruction::Get { url } => {
            Ok(StepResult::Get(substitute_variables(url, variables)?))
        }
        ScrapeLangInstruction::Header { key, value } => {
            Ok(StepResult::UpdatedScraper(scraper.set_header(
                substitute_variables(key, variables)?,
                substitute_variables(value, variables)?,
            )))
        }
        ScrapeLangInstruction::Load { varname } => {
            let mut new_results = scraper.results().clone();

            new_results.append(
                variables
                    .get(varname)
                    .ok_or(Error::VariableNotFoundError(varname.to_string()))?
                    .clone(),
            );

            Ok(StepResult::UpdatedScraper(
                scraper.clone().with_results(new_results),
            ))
        }
        ScrapeLangInstruction::Prepend { str } => Ok(StepResult::UpdatedScraper(
            scraper.prepend(&substitute_variables(str, variables)?),
        )),
        ScrapeLangInstruction::Store { varname } => Ok(StepResult::Store {
            varname: varname.to_string(),
            value: scraper.results().clone(),
        }),
        ScrapeLangInstruction::Run {
            job_name,
            args,
            kwargs,
        } => {
            let args_subst = if !args.is_empty() {
                args.iter()
                    .map(|x| match x {
                        ScrapeLangArgument::String { str } => substitute_variables(str, variables),
                        ScrapeLangArgument::Identifier { name } => variables
                            .get(name)
                            .ok_or(Error::VariableNotFoundError(name.to_string()))
                            .map(|v| v.iter().cloned().collect::<Vec<_>>().join("")),
                    })
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                scraper.results().iter().cloned().collect::<Vec<_>>()
            };

            let kwargs_subst: HashMap<String, String> = kwargs
                .iter()
                .map(|(key, value)| match value {
                    ScrapeLangArgument::String { str } => {
                        Ok((key.clone(), substitute_variables(str, variables)?))
                    }
                    ScrapeLangArgument::Identifier { name } => Ok((
                        key.clone(),
                        variables
                            .get(name)
                            .ok_or(Error::VariableNotFoundError(name.to_string()))
                            .map(|v| v.iter().cloned().collect::<Vec<_>>().join(""))?,
                    )),
                })
                .collect::<Result<HashMap<String, String>, Error>>()?;

            Ok(StepResult::ScriptInvocation {
                name: job_name.to_string(),
                args: args_subst,
                kwargs: kwargs_subst,
            })
        }
    }
}

pub type ScriptLoaderPointer = Arc<RwLock<dyn Fn(&str) -> Result<String, Error> + Send + Sync>>;

pub async fn run<H: HttpDriver>(
    script_name: &str,
    args: Vec<String>,
    kwargs: HashMap<String, String>,
    script_loader: ScriptLoaderPointer,
    effect_sender: UnboundedSender<EffectInvocation>,
) -> Result<Vector<String>, Error> {
    let script = {
        let locked_loader_fn = script_loader
            .read()
            .map_err(|_| Error::ScriptLoaderLockingError)?;

        locked_loader_fn(script_name)?

        // Lock dropped here
    };

    let code = strip_comments(&script);
    let tokens = lex(&code)?;
    let program = parse(&tokens)?;

    let mut variables = HashMap::<String, Vector<String>>::new();

    for (index, arg) in args.into_iter().enumerate() {
        variables.insert(format!("{}", index + 1), vector![arg]);
    }

    for (key, val) in kwargs {
        variables.insert(key, vector![val]);
    }

    let mut scraper = Scraper::<H>::new();

    for instruction in program {
        match step(&instruction, &scraper, &variables)? {
            StepResult::UpdatedScraper(updated_scraper) => scraper = updated_scraper,
            StepResult::EffectInvocation(effect_invocation) => {
                if let Err(e) = effect_sender.send(effect_invocation) {
                    eprintln!("{e}");
                }
            }
            StepResult::ScriptInvocation { name, args, kwargs } => {
                let mut new_results = scraper.results().clone();

                new_results.append(
                    Box::pin(run::<H>(
                        &name,
                        args,
                        kwargs,
                        script_loader.clone(),
                        effect_sender.clone(),
                    ))
                    .await?,
                );

                scraper = scraper.with_results(new_results);
            }
            StepResult::Get(url) => scraper = scraper.get(&url).await?,
            StepResult::Store { varname, value } => {
                variables.insert(varname, value);
            }
        }
    }

    Ok(scraper.results().clone())
}

fn substitute_variables(
    text: &str,
    variables: &HashMap<String, Vector<String>>,
) -> Result<String, Error> {
    let mut result = text.to_string();
    let mut delta: i32 = 0;
    let matcher = Regex::new("\\{(.+?)\\}").expect("Should be a valid regex");

    for matched in matcher.captures_iter(text) {
        let group = matched.get(1).unwrap();
        let varname = group.as_str().to_string();
        let matched_range = matched.get(0).unwrap().range();
        let old_len = result.len();

        result.replace_range(
            if delta >= 0 {
                (matched_range.start.saturating_sub(delta as usize))
                    ..(matched_range.end.saturating_sub(delta as usize))
            } else {
                (matched_range.start.saturating_add(-delta as usize))
                    ..(matched_range.end.saturating_add(-delta as usize))
            },
            variables
                .get(&varname)
                .ok_or(Error::VariableNotFoundError(varname))?
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("")
                .as_str(),
        );

        delta += (old_len as i32) - (result.len() as i32);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::scraper::NullHttpDriver;

    use super::*;

    #[test]
    fn test_substitute_variables_no_vars() {
        assert_eq!(substitute_variables("", &HashMap::new()).unwrap(), "");
        assert_eq!(
            substitute_variables("hello world", &HashMap::new()).unwrap(),
            "hello world"
        );
    }

    #[test]
    fn test_substitute_variables_missing_var() {
        assert!(substitute_variables("{x}", &HashMap::new())
            .is_err_and(|e| matches!(e, Error::VariableNotFoundError(_))));
    }

    #[test]
    fn test_substitute_variables_multiple() {
        let variables = HashMap::from([
            ("x1".to_string(), vector!["1".to_string()]), // result gets shorter
            ("x2".to_string(), vector!["2345".to_string()]), // result stays same length
            ("x3".to_string(), vector!["678912".to_string()]), // result gets longer
            ("$bar".to_string(), vector!["".to_string()]),
        ]);

        assert!(
            substitute_variables("{x1}{x2}{x3}", &variables).is_ok_and(|result| {
                assert_eq!(result, "12345678912");
                true
            })
        );

        assert!(
            substitute_variables("{x1} {x2} {x3}", &variables).is_ok_and(|result| {
                assert_eq!(result, "1 2345 678912");
                true
            })
        );

        assert!(
            substitute_variables("{x1} {x3} {x2}", &variables).is_ok_and(|result| {
                assert_eq!(result, "1 678912 2345");
                true
            })
        );

        assert!(
            substitute_variables("{x2} {x1} {x3}", &variables).is_ok_and(|result| {
                assert_eq!(result, "2345 1 678912");
                true
            })
        );

        assert!(
            substitute_variables("{x2} {x3} {x1}", &variables).is_ok_and(|result| {
                assert_eq!(result, "2345 678912 1");
                true
            })
        );

        assert!(
            substitute_variables("{x3} {x1} {x2}", &variables).is_ok_and(|result| {
                assert_eq!(result, "678912 1 2345");
                true
            })
        );

        assert!(
            substitute_variables("{x3} {x2} {x1}", &variables).is_ok_and(|result| {
                assert_eq!(result, "678912 2345 1");
                true
            })
        );

        assert!(
            substitute_variables("x1 {x1} foo {x2} bar {$bar} {x3} baz {x1}", &variables)
                .is_ok_and(|result| {
                    assert_eq!(result, "x1 1 foo 2345 bar  678912 baz 1");
                    true
                })
        );
    }

    #[test]
    fn test_results_as_implicit_args_for_effect() {
        let scraper = Scraper::<NullHttpDriver>::new()
            .with_results(vector!["foo".to_string(), "bar".to_string()]);

        assert!(step(
            &ScrapeLangInstruction::Effect {
                effect_name: "test".to_string(),
                args: vec![], // no args present, results should be used
                kwargs: HashMap::new()
            },
            &scraper,
            &HashMap::new()
        )
        .is_ok_and(|result| {
            matches!(
                result,
                StepResult::EffectInvocation(inv)
                    if inv.name() == "test" && inv.args() == &vec!["foo", "bar"]
            )
        }));
    }

    #[test]
    fn test_results_as_implicit_args_for_effect_with_explicit_args() {
        let scraper = Scraper::<NullHttpDriver>::new()
            .with_results(vector!["foo".to_string(), "bar".to_string()]);

        assert!(step(
            &ScrapeLangInstruction::Effect {
                effect_name: "test".to_string(),
                args: vec![ScrapeLangArgument::String {
                    str: "x".to_string() // explicit args present, should override results
                }],
                kwargs: HashMap::new()
            },
            &scraper,
            &HashMap::new()
        )
        .is_ok_and(|result| {
            matches!(
                result,
                StepResult::EffectInvocation(inv)
                    if inv.name() == "test" && inv.args() == &vec!["x"]
            )
        }));
    }
}
