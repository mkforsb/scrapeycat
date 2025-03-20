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
    scraper::{ReqwestHttpDriver, Scraper},
    Error,
};

pub type ScriptLoaderPointer = Arc<RwLock<dyn Fn(&str) -> Result<String, Error> + Send + Sync>>;

pub async fn run(
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

    let mut scraper = Scraper::<ReqwestHttpDriver>::new();

    for instruction in program {
        match instruction {
            ScrapeLangInstruction::Append { str } => {
                scraper = scraper.append(&substitute_variables(&str, &variables)?)
            }
            ScrapeLangInstruction::Clear => scraper = scraper.clear(),
            ScrapeLangInstruction::Delete { regex } => {
                scraper = scraper.delete(&substitute_variables(&regex, &variables)?)?
            }
            ScrapeLangInstruction::Drop { count } => scraper = scraper.drop(count),
            ScrapeLangInstruction::Extract { regex } => {
                scraper = scraper.extract(&substitute_variables(&regex, &variables)?)?
            }
            ScrapeLangInstruction::Effect {
                effect_name,
                args,
                kwargs,
            } => {
                // TODO: use results as args if args are empty
                let args_subst = args
                    .iter()
                    .map(|x| match x {
                        ScrapeLangArgument::String { str } => substitute_variables(str, &variables),
                        ScrapeLangArgument::Identifier { name } => variables
                            .get(name)
                            .ok_or(Error::VariableNotFoundError(name.to_string()))
                            .map(|v| v.iter().cloned().collect::<Vec<_>>().join("")),
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let kwargs_subst: HashMap<String, String> = kwargs
                    .into_iter()
                    .map(|(key, value)| match value {
                        ScrapeLangArgument::String { str } => {
                            Ok((key, substitute_variables(&str, &variables)?))
                        }
                        ScrapeLangArgument::Identifier { name } => Ok((
                            key,
                            variables
                                .get(&name)
                                .ok_or(Error::VariableNotFoundError(name.to_string()))
                                .map(|v| v.iter().cloned().collect::<Vec<_>>().join(""))?,
                        )),
                    })
                    .collect::<Result<HashMap<String, String>, Error>>()?;

                if let Err(e) =
                    effect_sender.send(EffectInvocation::new(effect_name, args_subst, kwargs_subst))
                {
                    eprintln!("{e}");
                }
            }
            ScrapeLangInstruction::First => scraper = scraper.first(),
            ScrapeLangInstruction::Get { url } => {
                scraper = scraper
                    .get(&substitute_variables(&url, &variables)?)
                    .await?
            }
            ScrapeLangInstruction::Load { varname } => {
                let mut new_results = scraper.results().clone();

                new_results.append(
                    variables
                        .get(&varname)
                        .ok_or(Error::VariableNotFoundError(varname.to_string()))?
                        .clone(),
                );

                scraper = scraper.with_results(new_results);
            }
            ScrapeLangInstruction::Prepend { str } => {
                scraper = scraper.prepend(&substitute_variables(&str, &variables)?)
            }
            ScrapeLangInstruction::Store { varname } => {
                variables.insert(varname, scraper.results().clone());
            }
            ScrapeLangInstruction::Run {
                job_name,
                args,
                kwargs,
            } => {
                // TODO: use results as args if args are empty
                let args_subst = args
                    .iter()
                    .map(|x| match x {
                        ScrapeLangArgument::String { str } => substitute_variables(str, &variables),
                        ScrapeLangArgument::Identifier { name } => variables
                            .get(name)
                            .ok_or(Error::VariableNotFoundError(name.to_string()))
                            .map(|v| v.iter().cloned().collect::<Vec<_>>().join("")),
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let kwargs_subst: HashMap<String, String> = kwargs
                    .into_iter()
                    .map(|(key, value)| match value {
                        ScrapeLangArgument::String { str } => {
                            Ok((key, substitute_variables(&str, &variables)?))
                        }
                        ScrapeLangArgument::Identifier { name } => Ok((
                            key,
                            variables
                                .get(&name)
                                .ok_or(Error::VariableNotFoundError(name.to_string()))
                                .map(|v| v.iter().cloned().collect::<Vec<_>>().join(""))?,
                        )),
                    })
                    .collect::<Result<HashMap<String, String>, Error>>()?;

                let mut new_results = scraper.results().clone();
                new_results.append(
                    Box::pin(run(
                        &job_name,
                        args_subst,
                        kwargs_subst,
                        script_loader.clone(),
                        effect_sender.clone(),
                    ))
                    .await?,
                );

                scraper = scraper.with_results(new_results);
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
    let matcher = Regex::new("\\{(.+?)\\}").expect("Should be a valid regex");

    for matched in matcher.captures_iter(text) {
        let group = matched.get(1).unwrap();
        let varname = group.as_str().to_string();

        result.replace_range(
            matched.get(0).unwrap().range(),
            variables
                .get(&varname)
                .ok_or(Error::VariableNotFoundError(varname))?
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("")
                .as_str(),
        );
    }

    Ok(result)
}
