#![cfg(all(test, feature = "testutils"))]

use std::{
    cell::Cell,
    collections::HashMap,
    env,
    fs::{read_dir, read_to_string},
    sync::{Arc, RwLock},
};

use regex::Regex;
use serde::Deserialize;
use tokio::sync::mpsc::unbounded_channel;

use libscrapeycat::{
    effect::EffectInvocation,
    scrapelang::program::run,
    scraper::{HttpDriver, HttpHeaders},
    testutils::path_in_project_root,
    Error,
};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct Effect {
    name: String,
    args: Option<Vec<String>>,
    kwargs: Option<HashMap<String, String>>,
}

impl From<EffectInvocation> for Effect {
    fn from(value: EffectInvocation) -> Self {
        let args = if value.args().is_empty() {
            None
        } else {
            Some(value.args().clone())
        };

        let kwargs = if value.kwargs().is_empty() {
            None
        } else {
            Some(value.kwargs().clone())
        };

        Effect {
            name: value.name().to_string(),
            args,
            kwargs,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct TestSpec {
    input: Option<String>,
    preamble: Option<String>,
    args: Option<Vec<String>>,
    kwargs: Option<HashMap<String, String>>,
    output: Option<Vec<String>>,
    effects: Option<Vec<Effect>>,
}

thread_local! {
    static SCRIPT: Cell<Option<String>> = Cell::new(None);
    static INPUT: Cell<Option<String>> = Cell::new(None);
}

fn script_loader(_name: &str) -> Result<String, Error> {
    SCRIPT
        .take()
        .ok_or(Error::ScriptNotFoundError("No script".to_string()))
}

#[derive(Clone)]
struct BookTestHttpDriver;

impl HttpDriver for BookTestHttpDriver {
    async fn get(_url: &str, _headers: HttpHeaders<'_>) -> Result<String, Error> {
        INPUT
            .take()
            .ok_or(Error::HTTPDriverError("No input".to_string()))
    }
}

#[tokio::test]
async fn test_book() {
    let preamble_templates = HashMap::from([
        ("get", "get(\"\")\n"),
        ("get-and-split-by-newline", "get(\"\")\nextract(\".+\")\n"),
    ]);

    let tests = Regex::new("(?s)<!-- test (\\{.+?\\}) -->").unwrap();
    let code_blocks = Regex::new("(?s)```lua(.+?)```").unwrap();

    for source in read_dir(path_in_project_root!("book/src"))
        .unwrap()
        .map(|x| x.unwrap())
        .filter(|x| x.path().is_file() && x.path().extension().unwrap() == "md")
    {
        let text = read_to_string(source.path()).unwrap();
        let mut num_tests = 0;

        for matched in tests.captures_iter(&text) {
            num_tests += 1;

            let test = serde_json::from_str::<TestSpec>(matched.get(1).unwrap().as_str()).unwrap();
            let end = matched.get(0).unwrap().end();

            let code = code_blocks
                .captures_at(&text, end)
                .unwrap()
                .get(1)
                .unwrap()
                .as_str()
                .to_string();

            SCRIPT.set(Some(format!(
                "{}\n{code}",
                if let Some(text) = test.preamble {
                    if text.starts_with("template:") {
                        preamble_templates
                            .get(text.strip_prefix("template:").unwrap().trim())
                            .expect("an existing template name should be given")
                            .to_string()
                    } else {
                        text
                    }
                } else {
                    "".to_string()
                }
            )));

            INPUT.set(test.input);

            let (effect_sender, mut effect_receiver) = unbounded_channel::<EffectInvocation>();

            let result = run::<BookTestHttpDriver>(
                "",
                test.args.unwrap_or(vec![]),
                test.kwargs.unwrap_or(HashMap::new()),
                Arc::new(RwLock::new(script_loader)),
                effect_sender,
            )
            .await
            .unwrap();

            if let Some(output) = test.output {
                assert_eq!(result.into_iter().collect::<Vec<_>>(), output);
            }

            if let Some(effects) = test.effects {
                for effect in effects {
                    assert_eq!(effect, effect_receiver.recv().await.unwrap().into());
                }
            }
        }

        eprintln!("{source:?}: {num_tests}");
    }
}
