use std::{
    collections::HashMap,
    fs,
    sync::{Arc, RwLock},
};

use clap::Parser;
use regex::Regex;
use tokio::sync::mpsc;

use scrapeycat::{
    daemon::{self, config_file::ConfigFile},
    effect::{self, EffectInvocation, EffectSignature},
    scrapelang::program::run,
    Error,
};

#[derive(Debug, Parser)]
enum Cli {
    Run {
        script: String,

        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    Daemon {
        config: String,
    },
}

fn load_script(name_or_filename: &str) -> Result<String, Error> {
    fs::read_to_string(name_or_filename)
        .or_else(|_| fs::read_to_string(format!("{name_or_filename}.scrape")))
        .or_else(|_| fs::read_to_string(format!("./scripts/{name_or_filename}")))
        .or_else(|_| fs::read_to_string(format!("./scripts/{name_or_filename}.scrape")))
        .map_err(|e| e.into())
}

fn split_posargs_and_kwargs(args: Vec<String>) -> (Vec<String>, HashMap<String, String>) {
    let identifier = Regex::new("^[A-Za-z_$.-][A-Za-z0-9_$.-]*").expect("Should be a valid regex");

    let mut posargs = Vec::new();
    let mut kwargs = HashMap::new();

    for val in args {
        if identifier.is_match(&val) && val.contains('=') {
            let (key, val) = val.split_at(val.find('=').expect("Should exist due to `contains`"));
            kwargs.insert(key.to_string(), val[1..].to_string());
        } else {
            posargs.push(val);
        }
    }

    (posargs, kwargs)
}

#[tokio::main]
async fn main() {
    match Cli::parse() {
        Cli::Run { script, args } => {
            let (effects_sender, effects_receiver) = mpsc::unbounded_channel::<EffectInvocation>();
            let effects_runner_task =
                tokio::spawn(effect::default_effects_runner_task(effects_receiver));

            let (posargs, kwargs) = split_posargs_and_kwargs(args);

            match run(
                &script,
                posargs,
                kwargs,
                Arc::new(RwLock::new(load_script)),
                effects_sender,
            )
            .await
            {
                Ok(results) => println!("{results:#?}"),
                Err(e) => eprintln!("{e}"),
            }

            let _ = tokio::join!(effects_runner_task);
        }

        Cli::Daemon { config } => match ConfigFile::config_from_file(&config) {
            Ok(config) => {
                daemon::run_config(
                    config,
                    HashMap::from([
                        ("print".to_string(), effect::print as EffectSignature),
                        ("notify".to_string(), effect::notify as EffectSignature),
                    ]),
                )
                .await;
            }
            Err(e) => eprintln!("{e}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_posargs_and_kwargs() {
        macro_rules! args {
            ($($($val:expr),+)?) => {
                Vec::<String>::from([
                    $($(($val.into())),+)?
                ])
            };
        }

        macro_rules! map {
            ($($($key:expr => $val:expr),+)?) => {
                HashMap::<String, String>::from([
                    $($(($key.into(), $val.into())),+)?
                ])
            };
        }

        assert_eq!(split_posargs_and_kwargs(args![]), (vec![], map![]));
        assert_eq!(split_posargs_and_kwargs(args!["a"]), (args!["a"], map![]));

        assert_eq!(
            split_posargs_and_kwargs(args!["b=c"]),
            (args![], map!["b" => "c"])
        );

        assert_eq!(
            split_posargs_and_kwargs(args!["a", "b=c"]),
            (args!["a"], map!["b" => "c"])
        );

        assert_eq!(
            split_posargs_and_kwargs(args!["a", "b=c", "dee", "ee=eff"]),
            (args!["a", "dee"], map!["b" => "c", "ee" => "eff"])
        );

        assert_eq!(
            split_posargs_and_kwargs(args!["a", "b=c", "dee", "ee=eff", "=gee"]),
            (args!["a", "dee", "=gee"], map!["b" => "c", "ee" => "eff"])
        );

        assert_eq!(
            split_posargs_and_kwargs(args!["a", "b=c", "dee", "ee=eff", "=gee", "1=2"]),
            (
                args!["a", "dee", "=gee", "1=2"],
                map!["b" => "c", "ee" => "eff"]
            )
        );
    }
}
