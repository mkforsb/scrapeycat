#![cfg(all(test, feature = "testutils"))]

use std::{
    collections::HashMap,
    env, fs,
    sync::{Arc, RwLock},
};

use libscrapeycat::{
    effect::EffectInvocation,
    scrapelang::program::run,
    testutils::{path_in_project_root, TestHttpDriver},
    Error,
};

/// The script loader for these tests loads `{name}.scrape` from
/// `/<projectroot>/tests/assets/scripts`
fn tests_script_loader(name: &str) -> Result<String, Error> {
    Ok(fs::read_to_string(path_in_project_root!(format!(
        "tests/assets/scripts/{name}.scrape"
    )))?)
}

/// The `test!("path/to/foo")' macro executes `path/to/foo.scrape` and verifies the result
/// against `path/to/foo.expect`
macro_rules! test {
    ($path:expr) => {{
        let (effect_sender, effect_receiver) =
            tokio::sync::mpsc::unbounded_channel::<EffectInvocation>();

        assert_eq!(
            format!(
                "{:#?}",
                run::<TestHttpDriver>(
                    $path,
                    vec![],
                    HashMap::new(),
                    Arc::new(RwLock::new(tests_script_loader)),
                    effect_sender,
                )
                .await
                .unwrap()
            )
            .trim(),
            fs::read_to_string(path_in_project_root!(format!(
                "tests/assets/scripts/{}.expect",
                $path
            )))
            .unwrap()
            .trim()
        );

        effect_receiver
    }};
}

#[tokio::test]
async fn test_bbc_science() {
    test!("bbc-science");
}

#[tokio::test]
async fn test_results_as_implicit_args_for_run() {
    test!("results-as-implicit-args-for-run");
}

#[tokio::test]
async fn test_explicit_args_override_results_as_implicit_args_for_run() {
    test!("explicit-args-override-results-as-implicit-args-for-run");
}

#[tokio::test]
async fn test_results_as_implicit_args_for_effect() {
    let mut effects = test!("results-as-implicit-args-for-effect");

    assert!(effects.recv().await.is_some_and(|inv| {
        assert_eq!(inv.name(), "test");
        assert_eq!(inv.args(), &vec!["x", "y", "z"]);
        true
    }));
}

#[tokio::test]
async fn test_explicit_args_override_results_as_implicit_args_for_effect() {
    let mut effects = test!("explicit-args-override-results-as-implicit-args-for-effect");

    assert!(effects.recv().await.is_some_and(|inv| {
        assert_eq!(inv.name(), "test");
        assert_eq!(inv.args(), &vec!["a", "b", "c"]);
        true
    }));
}

#[tokio::test]
async fn test_discard() {
    test!("discard");
}

#[tokio::test]
async fn test_retain() {
    test!("retain");
}
