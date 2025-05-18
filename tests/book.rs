//! This module implements testing machinery for code examples in the mdbook (/book).
//!
//! Notes on the implementation:
//!
//! 1. All files in /book/src with extension .md are scanned.
//!
//! 2. The scan searches for an HTML comment starting with "<!-- test", followed by a JSON object
//!    (delimited by curly braces) defining a test spec, followed by the closing of the HTML
//!    comment with "-->". Once found, the scanner will pick the first following markdown Lua code
//!    block (or panic if there is none) as the code to be associated with the test spec. The
//!    intended way to define a test looks something like the following:
//!
//!    ````
//!    <!-- test {
//!      json spec
//!    } -->
//!    ```lua
//!    code example to be tested
//!    ```
//!    ````
//!
//! 3. The test spec is given as a JSON object according to the following schema:
//!    
//!    ```
//!    interface Spec {
//!      input?: string,          // text to return for `get(url)` for any `url`
//!      preamble?: string,       // script text to prepend to code example script
//!      postamble?: string,      // script text to append to code example script
//!      args?: string[],         // positional arguments to pass to script
//!      kwargs?: {               // keyword arguments / named variables to pass to script
//!        (key: string,)*          // zero or more
//!      },
//!      expect: {                // expectations
//!        output?: string[],       // expected final output
//!        effects?: [              // expected sequence of effect invocations
//!          ({
//!            name: string,        // name of effect
//!            args?: string[],     // positional arguments to effect
//!            kwargs?: {           // keyword arguments to effect
//!              (key: string,)*      // zero or more
//!            }
//!          },)*                   // zero or more
//!        ],
//!        headers?: string[],      // expected sequence of stringified request headers
//!      }
//!    }
//!    ```
//!
//! 4. Stringified request headers are collected for each request, and the stringification will
//!    transform each key-value header map into a sorted list formatted as a single string of the
//!    form "Header1: Value, Header2: Value ...", i.e joined by the string ", ".
//!
//!    For example, if the request headers were {"User-Agent": "Firefox", "Accept-Encoding": "*"},
//!    the stringified headers will be "Accept-Encoding: *, User-Agent: Firefox".
#![cfg(all(test, feature = "testutils"))]

use std::{
    cell::RefCell,
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
    postamble: Option<String>,
    args: Option<Vec<String>>,
    kwargs: Option<HashMap<String, String>>,
    expect: TestExpectSpec,
}

#[derive(Debug, Clone, Deserialize)]
struct TestExpectSpec {
    output: Option<Vec<String>>,
    effects: Option<Vec<Effect>>,
    headers: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
struct StringifiedHeaders(String);

impl StringifiedHeaders {
    pub fn new(headers: &HttpHeaders<'_>) -> Self {
        match headers {
            HttpHeaders::NoHeaders => StringifiedHeaders("".to_string()),
            HttpHeaders::Headers(hash_map) => {
                let mut headers = hash_map
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<_>>();
                headers.sort();
                StringifiedHeaders(headers.join(", "))
            }
        }
    }
}

impl PartialEq<String> for StringifiedHeaders {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

#[derive(Debug)]
struct TestState {
    script: String,
    input: String,
    headers_seen: Vec<StringifiedHeaders>,
}

impl TestState {
    pub fn new(script: String, input: String, headers_seen: Vec<StringifiedHeaders>) -> Self {
        TestState {
            script,
            input,
            headers_seen,
        }
    }
}

thread_local! {
    static TEST_STATE: RefCell<Option<TestState>> = RefCell::new(None);
}

fn script_loader(_name: &str) -> Result<String, Error> {
    Ok(TEST_STATE.with(|state| state.borrow().as_ref().unwrap().script.clone()))
}

#[derive(Clone)]
struct BookTestHttpDriver;

impl HttpDriver for BookTestHttpDriver {
    async fn get(_url: &str, headers: HttpHeaders<'_>) -> Result<String, Error> {
        TEST_STATE.with(|state| {
            state
                .borrow_mut()
                .as_mut()
                .unwrap()
                .headers_seen
                .push(StringifiedHeaders::new(&headers))
        });

        Ok(TEST_STATE.with(|state| state.borrow().as_ref().unwrap().input.clone()))
    }
}

/// Book test runner
async fn run_test(script: String, spec: TestSpec) {
    TEST_STATE.replace(Some(TestState::new(
        script,
        spec.input.unwrap_or("".to_string()),
        vec![],
    )));

    let (effect_sender, mut effect_receiver) = unbounded_channel::<EffectInvocation>();

    let result = run::<BookTestHttpDriver>(
        "",
        spec.args.unwrap_or(vec![]),
        spec.kwargs.unwrap_or(HashMap::new()),
        Arc::new(RwLock::new(script_loader)),
        effect_sender,
    )
    .await
    .unwrap();

    if let Some(output) = spec.expect.output {
        assert_eq!(result.into_iter().collect::<Vec<_>>(), output);
    }

    if let Some(effects) = spec.expect.effects {
        for effect in effects {
            assert_eq!(effect, effect_receiver.recv().await.unwrap().into());
        }

        effect_receiver.close();
        assert!(effect_receiver.recv().await.is_none());
    }

    if let Some(headers) = spec.expect.headers {
        assert_eq!(
            TEST_STATE.with(|state| state.borrow().as_ref().unwrap().headers_seen.clone()),
            headers,
        );
    }
}

/// Book test main entry point, implements the scanner
#[tokio::test]
async fn test_book() {
    let xamble_templates = HashMap::from([
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
        eprint!("{:?} ", source.path());

        let text = read_to_string(source.path()).unwrap();
        let mut num_tests = 0;

        for matched in tests.captures_iter(&text) {
            num_tests += 1;

            let spec = serde_json::from_str::<TestSpec>(matched.get(1).unwrap().as_str()).unwrap();
            let end = matched.get(0).unwrap().end();

            let mut script = code_blocks
                .captures_at(&text, end)
                .unwrap()
                .get(1)
                .unwrap()
                .as_str()
                .to_string();

            if let Some(ref text) = spec.preamble {
                script = format!(
                    "{}\n{script}\n",
                    if text.starts_with("template:") {
                        xamble_templates
                            .get(text.strip_prefix("template:").unwrap().trim())
                            .expect("An existing template name should be given")
                            .to_string()
                    } else {
                        text.clone()
                    }
                )
            }

            if let Some(ref text) = spec.postamble {
                script = format!(
                    "{script}\n{}\n",
                    if text.starts_with("template:") {
                        xamble_templates
                            .get(text.strip_prefix("template:").unwrap().trim())
                            .expect("An existing template name should be given")
                            .to_string()
                    } else {
                        text.clone()
                    }
                )
            }

            run_test(script, spec).await;
        }

        eprintln!("{num_tests}");
    }
}

/// Tests for the book test runner itself
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_test_effects_ignored() {
        let script = r#"
            effect("print", { "hello", "world" })
            effect("print", { "goodbye", "world" })
        "#
        .to_string();

        let spec = TestSpec {
            input: None,
            preamble: None,
            postamble: None,
            args: None,
            kwargs: None,
            expect: TestExpectSpec {
                output: None,
                effects: None,
                headers: None,
            },
        };

        run_test(script, spec).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_run_test_extraneous_effect() {
        let script = r#"
            effect("print", { "hello", "world" })
            effect("print", { "goodbye", "world" })
        "#
        .to_string();

        let spec = TestSpec {
            input: None,
            preamble: None,
            postamble: None,
            args: None,
            kwargs: None,
            expect: TestExpectSpec {
                output: None,
                effects: Some(vec![Effect {
                    name: "print".to_string(),
                    args: Some(vec!["hello".to_string(), "world".to_string()]),
                    kwargs: None,
                }]),
                headers: None,
            },
        };

        run_test(script, spec).await;
    }

    #[tokio::test]
    async fn test_run_test_effects_match() {
        let script = r#"
            effect("print", { "hello", "world" })
            effect("print", { "goodbye", "world" })
        "#
        .to_string();

        let spec = TestSpec {
            input: None,
            preamble: None,
            postamble: None,
            args: None,
            kwargs: None,
            expect: TestExpectSpec {
                output: None,
                effects: Some(vec![
                    Effect {
                        name: "print".to_string(),
                        args: Some(vec!["hello".to_string(), "world".to_string()]),
                        kwargs: None,
                    },
                    Effect {
                        name: "print".to_string(),
                        args: Some(vec!["goodbye".to_string(), "world".to_string()]),
                        kwargs: None,
                    },
                ]),
                headers: None,
            },
        };

        run_test(script, spec).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_run_test_effect_mismatch() {
        let script = r#"
            effect("print", { "hello", "world" })
            effect("print", { "goodbye", "world" })
        "#
        .to_string();

        let spec = TestSpec {
            input: None,
            preamble: None,
            postamble: None,
            args: None,
            kwargs: None,
            expect: TestExpectSpec {
                output: None,
                effects: Some(vec![
                    Effect {
                        name: "print".to_string(),
                        args: Some(vec!["hello".to_string(), "world".to_string()]),
                        kwargs: None,
                    },
                    Effect {
                        name: "print".to_string(),
                        args: Some(vec!["adios".to_string(), "world".to_string()]),
                        kwargs: None,
                    },
                ]),
                headers: None,
            },
        };

        run_test(script, spec).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_run_test_effect_missing() {
        let script = r#"
            effect("print", { "hello", "world" })
            effect("print", { "goodbye", "world" })
        "#
        .to_string();

        let spec = TestSpec {
            input: None,
            preamble: None,
            postamble: None,
            args: None,
            kwargs: None,
            expect: TestExpectSpec {
                output: None,
                effects: Some(vec![
                    Effect {
                        name: "print".to_string(),
                        args: Some(vec!["hello".to_string(), "world".to_string()]),
                        kwargs: None,
                    },
                    Effect {
                        name: "print".to_string(),
                        args: Some(vec!["goodbye".to_string(), "world".to_string()]),
                        kwargs: None,
                    },
                    Effect {
                        name: "print".to_string(),
                        args: Some(vec!["fin".to_string()]),
                        kwargs: None,
                    },
                ]),
                headers: None,
            },
        };

        run_test(script, spec).await;
    }
}
