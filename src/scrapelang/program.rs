use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, RwLock},
};

use im::{vector, Vector};
use log::error;
use mlua::prelude::*;
use regex::Regex;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    effect::EffectInvocation,
    scraper::{HttpDriver, Scraper},
    Error,
};

fn substitute_variables(
    text: &str,
    variables: &HashMap<String, Vector<String>>,
) -> Result<String, Error> {
    let mut result = text.to_string();
    let mut delta: i32 = 0;
    let matcher = Regex::new("\\{(.+?)\\}").expect("Should be a valid regex");

    for matched in matcher.captures_iter(text) {
        let group = matched.get(1).expect("Group 1 should exist");
        let varname = group.as_str().to_string();
        let matched_range = matched.get(0).expect("Group 0 should always exist").range();
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

impl From<mlua::Error> for Error {
    fn from(value: mlua::Error) -> Self {
        Error::LuaError(value.to_string())
    }
}

impl From<Error> for mlua::Error {
    fn from(value: Error) -> Self {
        value.into_lua_err()
    }
}

struct LuaScraperState<H: HttpDriver + 'static> {
    scraper: Scraper<H>,
    variables: HashMap<String, Vector<String>>,
}

impl<H: HttpDriver + 'static> LuaScraperState<H> {
    pub fn new() -> Self {
        LuaScraperState {
            scraper: Scraper::new(),
            variables: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct InterruptedError;

impl std::fmt::Display for InterruptedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Interrupted")
    }
}

impl std::error::Error for InterruptedError {}

#[inline(always)]
fn get_state<H: HttpDriver + 'static>(
    lua: &Lua,
) -> Result<mlua::AppDataRefMut<'_, LuaScraperState<H>>, Error> {
    lua.app_data_mut::<LuaScraperState<H>>()
        .ok_or(Error::LuaError(
            "Cannot access lua scraper state".to_string(),
        ))
}

fn create_lua_context<H: HttpDriver + Send + Sync + 'static>(
    args: Vec<String>,
    kwargs: HashMap<String, String>,
    effect_sender: UnboundedSender<EffectInvocation>,
    script_loader: ScriptLoaderPointer,
) -> Result<Lua, Error> {
    let mut state = LuaScraperState::<H>::new();

    for (index, arg) in args.into_iter().enumerate() {
        state
            .variables
            .insert(format!("{}", index + 1), vector![arg]);
    }

    for (key, val) in kwargs {
        state.variables.insert(key, vector![val]);
    }

    let lua = Lua::new();

    lua.load_std_libs(LuaStdLib::ALL_SAFE)?;
    lua.set_app_data(state);

    lua.globals().set(
        "abortIfEmpty",
        lua.create_function(|lua: &Lua, ()| {
            let state = get_state::<H>(lua)?;

            if state.scraper.results().is_empty() {
                Err(LuaError::ExternalError(Arc::new(InterruptedError {})))
            } else {
                Ok(())
            }
        })?,
    )?;

    lua.globals().set(
        "append",
        lua.create_function(|lua: &Lua, text: String| {
            let mut state = get_state::<H>(lua)?;

            state.scraper = state
                .scraper
                .append(&substitute_variables(&text, &state.variables)?);

            Ok(())
        })?,
    )?;

    lua.globals().set(
        "apply",
        lua.create_function(|lua: &Lua, f: LuaFunction| {
            // We don't want to hold a borrow to the state while applying the function
            let results = {
                let state = get_state::<H>(lua)?;
                state.scraper.results().iter().cloned().collect::<Vec<_>>()
            };

            let applied = f.call::<Vec<String>>(results)?;
            let mut state = get_state::<H>(lua)?;

            state.scraper = state.scraper.clone().with_results(Vector::from(applied));
            Ok(())
        })?,
    )?;

    lua.globals().set(
        "clear",
        lua.create_function(|lua: &Lua, ()| {
            let mut state = get_state::<H>(lua)?;

            state.scraper = state.scraper.clear();
            Ok(())
        })?,
    )?;

    lua.globals().set(
        "clearHeaders",
        lua.create_function(|lua: &Lua, ()| {
            let mut state = get_state::<H>(lua)?;

            state.scraper = state.scraper.clear_headers();
            Ok(())
        })?,
    )?;

    lua.globals().set(
        "delete",
        lua.create_function(|lua: &Lua, pattern: String| {
            let mut state = get_state::<H>(lua)?;

            state.scraper = state
                .scraper
                .delete(&substitute_variables(&pattern, &state.variables)?)?;

            Ok(())
        })?,
    )?;

    lua.globals().set(
        "discard",
        lua.create_function(|lua: &Lua, pattern: String| {
            let mut state = get_state::<H>(lua)?;

            state.scraper = state
                .scraper
                .discard(&substitute_variables(&pattern, &state.variables)?)?;

            Ok(())
        })?,
    )?;

    lua.globals().set(
        "drop",
        lua.create_function(|lua: &Lua, n: usize| {
            let mut state = get_state::<H>(lua)?;

            state.scraper = state.scraper.drop(n);
            Ok(())
        })?,
    )?;

    let effect_sender_for_effect_fn = UnboundedSender::clone(&effect_sender);

    lua.globals().set(
        "effect",
        lua.create_function(
            move |lua: &Lua, (name, args_table): (String, Option<LuaTable>)| {
                let state = get_state::<H>(lua)?;
                let mut args: Vec<String> = vec![];
                let mut kwargs: HashMap<String, String> = HashMap::new();

                if let Some(args_table) = args_table {
                    for i in 1..100 {
                        if let Ok(value) = args_table.get::<String>(i) {
                            args.push(substitute_variables(&value, &state.variables)?);
                        }
                    }

                    for (key, value) in args_table.pairs::<String, String>().flatten() {
                        if !key.chars().all(|ch| ch.is_ascii_digit()) {
                            kwargs.insert(key, substitute_variables(&value, &state.variables)?);
                        }
                    }
                }

                if args.is_empty() {
                    args.extend(state.scraper.results().iter().cloned());
                }

                match effect_sender_for_effect_fn.send(EffectInvocation::new(name, args, kwargs)) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e.into_lua_err()),
                }
            },
        )?,
    )?;

    lua.globals().set(
        "extract",
        lua.create_function(|lua: &Lua, pattern: String| {
            let mut state = get_state::<H>(lua)?;

            state.scraper = state
                .scraper
                .extract(&substitute_variables(&pattern, &state.variables)?)?;

            Ok(())
        })?,
    )?;

    lua.globals().set(
        "first",
        lua.create_function(|lua: &Lua, ()| {
            let mut state = get_state::<H>(lua)?;

            state.scraper = state.scraper.first();
            Ok(())
        })?,
    )?;

    lua.globals().set(
        "get",
        lua.create_async_function(|lua: Lua, url: String| async move {
            let (scraper, url_subst) = {
                let state = get_state::<H>(&lua)?;
                (
                    state.scraper.clone(),
                    &substitute_variables(&url, &state.variables)?,
                )
            };

            let updated_scraper = scraper.get(url_subst).await?;

            let mut state = get_state::<H>(&lua)?;
            state.scraper = updated_scraper;

            Ok(())
        })?,
    )?;

    lua.globals().set(
        "header",
        lua.create_function(|lua: &Lua, (key, value): (String, String)| {
            let mut state = get_state::<H>(lua)?;

            state.scraper = state
                .scraper
                .set_header(key, substitute_variables(&value, &state.variables)?);

            Ok(())
        })?,
    )?;

    lua.globals().set(
        "list",
        lua.create_function(|lua: &Lua, name: String| {
            get_state::<H>(lua)?
                .variables
                .get(&name)
                .map(|v| v.iter().cloned().collect::<Vec<_>>())
                .ok_or_else(|| {
                    error!("variable `{name}` not found");
                    Error::LuaError(format!("variable `{name}` not found")).into_lua_err()
                })
        })?,
    )?;

    lua.globals().set(
        "load",
        lua.create_function(|lua: &Lua, name: String| {
            let mut state = get_state::<H>(lua)?;
            let mut results = state.scraper.results().clone();

            let stored = state.variables.get(&name).ok_or_else(|| {
                error!("variable `{name}` not found");
                Error::LuaError(format!("variable `{name}` not found")).into_lua_err()
            })?;

            results.extend(stored.iter().cloned());
            state.scraper = state.scraper.clone().with_results(results);
            Ok(())
        })?,
    )?;

    lua.globals().set(
        "map",
        lua.create_function(|lua: &Lua, f: LuaFunction| {
            // We don't want to hold a borrow to the state while applying the function
            let results = {
                let state = get_state::<H>(lua)?;
                state.scraper.results().clone()
            };

            let mapped = Vector::from(
                results
                    .into_iter()
                    .map(|s| f.call::<String>(s))
                    .collect::<Result<Vec<_>, mlua::Error>>()?,
            );

            let mut state = get_state::<H>(lua)?;

            state.scraper = state.scraper.clone().with_results(mapped);
            Ok(())
        })?,
    )?;

    lua.globals().set(
        "prepend",
        lua.create_function(|lua: &Lua, text: String| {
            let mut state = get_state::<H>(lua)?;

            state.scraper = state
                .scraper
                .prepend(&substitute_variables(&text, &state.variables)?);

            Ok(())
        })?,
    )?;

    lua.globals().set(
        "retain",
        lua.create_function(|lua: &Lua, pattern: String| {
            let mut state = get_state::<H>(lua)?;

            state.scraper = state
                .scraper
                .retain(&substitute_variables(&pattern, &state.variables)?)?;

            Ok(())
        })?,
    )?;

    let effect_sender_for_run_fn = UnboundedSender::clone(&effect_sender);
    let script_loader_for_run_fn = Arc::clone(&script_loader);

    lua.globals().set(
        "run",
        lua.create_async_function(
            move |lua: Lua, (name, args_table): (String, Option<LuaTable>)| {
                let effect_sender_inner = UnboundedSender::clone(&effect_sender_for_run_fn);
                let script_loader_inner = Arc::clone(&script_loader_for_run_fn);

                async move {
                    let (args, kwargs, mut new_results) = {
                        let state = get_state::<H>(&lua)?;
                        let mut args: Vec<String> = vec![];
                        let mut kwargs: HashMap<String, String> = HashMap::new();

                        if let Some(args_table) = args_table {
                            for i in 1..100 {
                                if let Ok(value) = args_table.get::<String>(i) {
                                    args.push(substitute_variables(&value, &state.variables)?);
                                }
                            }

                            for (key, value) in args_table.pairs::<String, String>().flatten() {
                                if !key.chars().all(|ch| ch.is_ascii_digit()) {
                                    kwargs.insert(
                                        key,
                                        substitute_variables(&value, &state.variables)?,
                                    );
                                }
                            }
                        }

                        if args.is_empty() {
                            args.extend(state.scraper.results().iter().cloned());
                        }

                        (args, kwargs, state.scraper.results().clone())
                    };

                    let inner_results = Box::pin(run::<H>(
                        &name,
                        args,
                        kwargs,
                        script_loader_inner,
                        effect_sender_inner,
                    ))
                    .await;

                    match inner_results {
                        Ok(results) => {
                            new_results.append(results);

                            let mut state = get_state::<H>(&lua)?;
                            state.scraper = state.scraper.clone().with_results(new_results);

                            Ok(())
                        }
                        Err(e) => Err(e.into_lua_err()),
                    }
                }
            },
        )?,
    )?;

    lua.globals().set(
        "store",
        lua.create_function(|lua: &Lua, name: String| {
            let mut state = get_state::<H>(lua)?;
            let results = state.scraper.results().clone();

            state.variables.insert(name, results);
            Ok(())
        })?,
    )?;

    lua.globals().set(
        "var",
        lua.create_function(|lua: &Lua, name: String| {
            get_state::<H>(lua)?
                .variables
                .get(&name)
                .map(|v| v.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" "))
                .ok_or_else(|| {
                    error!("variable `{name}` not found");
                    Error::LuaError(format!("variable `{name}` not found")).into_lua_err()
                })
        })?,
    )?;

    Ok(lua)
}

fn is_interruption(error: &LuaError) -> bool {
    if let LuaError::CallbackError { cause, .. } = error {
        if let LuaError::ExternalError(inner_error) = cause.deref() {
            return inner_error.downcast_ref::<InterruptedError>().is_some();
        }
    }

    false
}

pub type ScriptLoaderPointer = Arc<RwLock<dyn Fn(&str) -> Result<String, Error> + Send + Sync>>;

pub async fn run<H: HttpDriver + Send + Sync + 'static>(
    script_name: &str,
    args: Vec<String>,
    kwargs: HashMap<String, String>,
    script_loader: ScriptLoaderPointer,
    effect_sender: UnboundedSender<EffectInvocation>,
) -> Result<Vector<String>, Error> {
    let lua_code = {
        let locked_loader_fn = script_loader
            .read()
            .map_err(|_| Error::ScriptLoaderLockingError)?;

        locked_loader_fn(script_name)?

        // Lock dropped here
    };

    let lua = create_lua_context::<H>(args, kwargs, effect_sender, script_loader)?;

    if let Err(e) = lua.load(lua_code).exec_async().await {
        if !is_interruption(&e) {
            return Err(e.into());
        }
    }

    Ok({
        // Workaround for "temporary dropped while borrowed"
        let results = get_state::<H>(&lua)?.scraper.results().clone();
        results
    })
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc::unbounded_channel;

    use crate::{
        scraper::NullHttpDriver,
        testutils::{HeaderTestHttpDriver, TestHttpDriver},
    };

    use super::*;

    macro_rules! results {
        ($($str:expr),*) => {
            vector![$($str.to_string()),*]
        };
    }

    macro_rules! lua_call {
        ($lua:ident, $fname:expr, $args:expr => $ret:ty) => {
            $lua.globals()
                .get::<LuaFunction>($fname)
                .unwrap()
                .call::<$ret>($args)
                .unwrap()
        };
    }

    macro_rules! lua_run_async {
        ($lua:ident, $script:expr) => {
            $lua.load($script).exec_async().await
        };
    }

    fn null_script_loader_inner(_name: &str) -> Result<String, Error> {
        Err(Error::JobNotFoundError)
    }

    fn null_script_loader() -> ScriptLoaderPointer {
        Arc::new(RwLock::new(null_script_loader_inner))
    }

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
            ("x1".to_string(), results!["1"]),      // Result gets shorter
            ("x2".to_string(), results!["2345"]),   // Result stays same length
            ("x3".to_string(), results!["678912"]), // Result gets longer
            ("$bar".to_string(), results![""]),
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
    fn test_create_lua_context_get_and_set_state() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<NullHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        {
            let mut state = get_state::<NullHttpDriver>(&lua).unwrap();

            state.scraper = state.scraper.clone().with_results(results!["hello"]);

            state
                .variables
                .insert("test".to_string(), results!["world"]);
        }

        let state = get_state::<NullHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["hello"]);
        assert_eq!(state.variables.get("test"), Some(&results!["world"]));
    }

    #[tokio::test]
    async fn test_lua_abort_if_empty() {
        let (effect_tx, mut effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                abortIfEmpty()
                effect("print", { "hello" })
                get("string://test")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results![]);

        effect_rx.close();

        assert!(effect_rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn test_lua_append() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://hello")
                append(" world")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["hello world"]);
    }

    #[tokio::test]
    async fn test_lua_append_using_variables() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://world!!")
                store("varname")
                clear()
                get("string://hello")
                append(" {varname}")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["hello world!!"]);
    }

    #[tokio::test]
    async fn test_lua_apply() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                function process(results)
                    table.insert(results, "a")
                    table.insert(results, "b")
                    return results
                end

                get("string://hello")
                apply(process)
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["hello", "a", "b"]);
    }

    #[tokio::test]
    async fn test_lua_apply_using_variables_in_applied_fn() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                function process(results)
                    table.insert(results, var("varname"))
                    return results
                end

                get("string://hello")
                store("varname")
                clear()
                get("string://world")
                apply(process)
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["world", "hello"]);
    }

    #[tokio::test]
    async fn test_lua_clear() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://hello")
                clear()
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results![]);
    }

    #[tokio::test]
    async fn test_lua_clearheaders() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua = create_lua_context::<HeaderTestHttpDriver>(
            vec![],
            HashMap::new(),
            effect_tx,
            script_loader,
        )
        .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                header("User-Agent", "Mozilla/Firefox")
                clearHeaders()
                get("")
            "#
        );

        let state = get_state::<HeaderTestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["Headers({})"]);
    }

    #[tokio::test]
    async fn test_lua_delete() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://123-456")
                get("string://84-9851-858-44")
                get("string://786---858-4")
                delete("-")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(
            state.scraper.results(),
            &results!["123456", "84985185844", "7868584"]
        );
    }

    #[tokio::test]
    async fn test_lua_delete_using_variables() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://-")
                store("varname")
                clear()
                get("string://123-456")
                get("string://84-9851-858-44")
                get("string://786---858-4")
                delete("{varname}4")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(
            state.scraper.results(),
            &results!["12356", "84-9851-8584", "786---858"]
        );
    }

    #[tokio::test]
    async fn test_lua_discard() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://123-456")
                get("string://84-9851-858-44")
                get("string://786---858-4")
                discard("858")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["123-456"]);
    }

    #[tokio::test]
    async fn test_lua_discard_using_variables() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://-")
                store("varname")
                clear()
                get("string://123-456")
                get("string://84-9851-858-44")
                get("string://786---858-4")
                discard("{varname}{varname}858")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(
            state.scraper.results(),
            &results!["123-456", "84-9851-858-44"]
        );
    }

    #[tokio::test]
    async fn test_lua_drop() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://123-456")
                get("string://84-9851-858-44")
                get("string://786---858-4")
                drop(2)
            "#
        );

        {
            let state = get_state::<TestHttpDriver>(&lua).unwrap();
            assert_eq!(state.scraper.results(), &results!["786---858-4"]);
        }

        lua_call!(lua, "drop", 200 => ());

        let state = get_state::<TestHttpDriver>(&lua).unwrap();
        assert_eq!(state.scraper.results(), &results![]);
    }

    #[tokio::test]
    async fn test_lua_effect() {
        let (effect_tx, mut effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"effect("notify", {"hello", "world", mode="default"})"#
        );

        assert!(effect_rx.recv().await.is_some_and(|invocation| {
            assert_eq!(invocation.name(), "notify");
            assert_eq!(
                invocation.args(),
                &vec!["hello".to_string(), "world".to_string()]
            );
            assert_eq!(
                invocation.kwargs().get("mode"),
                Some(&"default".to_string())
            );
            true
        }));
    }

    #[tokio::test]
    async fn test_lua_effect_using_variables() {
        let (effect_tx, mut effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://variabilitious")
                store("varname")
                effect("notify", {"hello", "{varname}", "world", mode="{varname}"})
            "#
        );

        assert!(effect_rx.recv().await.is_some_and(|invocation| {
            assert_eq!(invocation.name(), "notify");
            assert_eq!(
                invocation.args(),
                &vec![
                    "hello".to_string(),
                    "variabilitious".to_string(),
                    "world".to_string()
                ]
            );
            assert_eq!(
                invocation.kwargs().get("mode"),
                Some(&"variabilitious".to_string())
            );
            true
        }));
    }

    #[tokio::test]
    async fn test_lua_extract() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://123-456")
                get("string://84-9851-858-44")
                get("string://786---858-4")
                extract("-(4.?)")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["45", "44", "4"]);
    }

    #[tokio::test]
    async fn test_lua_extract_using_variables() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://-(4.?)")
                store("varname")
                clear()
                get("string://123-456")
                get("string://84-9851-858-44")
                get("string://786---858-4")
                extract("{varname}")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["45", "44", "4"]);
    }

    #[tokio::test]
    async fn test_lua_first() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://123-456")
                get("string://84-9851-858-44")
                get("string://786---858-4")
                first()
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["123-456"]);
    }

    #[tokio::test]
    async fn test_lua_get() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(lua, r#"get("string://hello")"#);

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["hello"]);
    }

    #[tokio::test]
    async fn test_lua_get_using_variables() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://foobar")
                store("myvar")
                clear()
                get("string://{myvar}")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["foobar"]);
    }

    #[tokio::test]
    async fn test_lua_header() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua = create_lua_context::<HeaderTestHttpDriver>(
            vec![],
            HashMap::new(),
            effect_tx,
            script_loader,
        )
        .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                header("User-Agent", "Mozilla/Firefox")
                get("")
            "#
        );

        {
            let state = get_state::<HeaderTestHttpDriver>(&lua).unwrap();

            assert_eq!(
                state.scraper.results(),
                &results!["Headers({\"User-Agent\": \"Mozilla/Firefox\"})"]
            );
        }

        let _ = lua_run_async!(
            lua,
            r#"
                clear()
                header("Accept-Encoding", "gzip")
                get("")
            "#
        );

        let state = get_state::<HeaderTestHttpDriver>(&lua).unwrap();

        assert_eq!(
            state.scraper.results(),
            &results![r#"Headers({"Accept-Encoding": "gzip", "User-Agent": "Mozilla/Firefox"})"#]
        );
    }

    #[tokio::test]
    async fn test_lua_header_using_variables() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua = create_lua_context::<HeaderTestHttpDriver>(
            vec![],
            HashMap::new(),
            effect_tx,
            script_loader,
        )
        .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                header("Test", "123")
                get("")
                store("$MyVariable")
                clear()
                clearHeaders()
                header("pre{$MyVariable}post", "aff{$MyVariable}suff")
                get("")
            "#
        );

        let state = get_state::<HeaderTestHttpDriver>(&lua).unwrap();

        assert_eq!(
            state.scraper.results(),
            // Variable substitution only occurs for the value
            &results![r#"Headers({"pre{$MyVariable}post": "affHeaders({"Test": "123"})suff"})"#]
        );
    }

    #[tokio::test]
    async fn test_lua_list() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://hello")
                get("string://world")
                store("myVariable")
            "#
        );

        let my_variable = lua_call!(lua, "list", "myVariable" => Vec<String>);

        assert_eq!(my_variable, vec!["hello", "world"]);
    }

    #[tokio::test]
    async fn test_lua_list_missing() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        assert!(lua_run_async!(
            lua,
            r#"
                local x = list("foo")
            "#
        )
        .is_err());
    }

    #[tokio::test]
    async fn test_lua_load() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://hello")
                store("myVariable")
                clear()
                load("myVariable")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["hello"]);
    }

    #[tokio::test]
    #[should_panic]
    async fn test_lua_load_does_not_do_variable_substitution() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        lua_run_async!(
            lua,
            r#"
                get("string://hello")
                store("myVariable")
                clear()
                load("{myVariable}") -- variable `{myVariable}` not found!
            "#
        )
        .unwrap();
    }

    #[tokio::test]
    async fn test_lua_map() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        lua.load(
            r#"
                get("string://mapme")
                get("string://mapmetoo")
                map(function(x)
                    return "(" .. x .. ")!"
                end)
            "#,
        )
        .exec()
        .unwrap();

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(
            state.scraper.results(),
            &results!["(mapme)!", "(mapmetoo)!"]
        );
    }

    #[tokio::test]
    async fn test_lua_map_using_variables_in_applied_fn() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://foo")
                store("myvar")
                clear()
                get("string://mapme")
                get("string://mapmetoo")
                map(function(x)
                    return var("myvar") .. x .. "!"
                end)
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(
            state.scraper.results(),
            &results!["foomapme!", "foomapmetoo!"]
        );
    }

    #[tokio::test]
    async fn test_lua_prepend() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://world")
                prepend("hello ")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["hello world"]);
    }

    #[tokio::test]
    async fn test_lua_prepend_using_variables() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://hello")
                store("myvar")
                clear()
                get("string://world")
                prepend("{myvar} ")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.scraper.results(), &results!["hello world"]);
    }

    #[tokio::test]
    async fn test_lua_retain() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://123-456")
                get("string://84-9851-858-44")
                get("string://786---858-4")
                retain("858")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(
            state.scraper.results(),
            &results!["84-9851-858-44", "786---858-4"]
        );
    }

    #[tokio::test]
    async fn test_lua_retain_using_variables() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://5")
                store("myvar")
                clear()
                get("string://123-456")
                get("string://84-9851-858-44")
                get("string://786---858-4")
                retain("8{myvar}8")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(
            state.scraper.results(),
            &results!["84-9851-858-44", "786---858-4"]
        );
    }

    #[tokio::test]
    async fn test_lua_run() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();

        let script_loader = Arc::new(RwLock::new(|name: &str| {
            if name == "test123" {
                Ok(r#"get("string://bazinga")"#.to_string())
            } else {
                Err(Error::JobNotFoundError)
            }
        }));

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(lua, r#"run("test123")"#);

        let state = get_state::<TestHttpDriver>(&lua).unwrap();
        assert_eq!(state.scraper.results(), &results!["bazinga"]);
    }

    #[tokio::test]
    async fn test_lua_run_using_variables() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();

        let script_loader = Arc::new(RwLock::new(|name: &str| {
            if name == "{myvar}" {
                Ok(r#"get("string://bazinga {1} {2} {limit}")"#.to_string())
            } else {
                Err(Error::JobNotFoundError)
            }
        }));

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://foobar")
                store("myvar")
                clear()
                run("{myvar}", {"hello", "{myvar}", limit="_{myvar}_"})
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();
        assert_eq!(
            state.scraper.results(),
            &results!["bazinga hello foobar _foobar_"]
        );
    }

    #[tokio::test]
    async fn test_lua_store() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://hello")
                store("myVariable")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(state.variables.get("myVariable"), Some(&results!["hello"]));
    }

    #[tokio::test]
    async fn test_lua_store_does_not_do_variable_substitution() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://hello")
                store("{myVariable}")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();

        assert_eq!(
            state.variables.get("{myVariable}"),
            Some(&results!["hello"])
        );
    }

    #[tokio::test]
    async fn test_lua_var() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://hello")
                get("string://world")
                store("myVariable")
            "#
        );

        let my_variable = lua_call!(lua, "var", "myVariable" => String);

        assert_eq!(my_variable, "hello world");
    }

    #[tokio::test]
    async fn test_lua_var_missing() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        assert!(lua_run_async!(
            lua,
            r#"
                local x = var("foo")
            "#
        )
        .is_err());
    }

    #[tokio::test]
    async fn test_lua_var_does_not_do_variable_substitution() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://hello")
                store("{myVariable}")
            "#
        );

        let my_variable = lua_call!(lua, "var", "{myVariable}" => String);

        assert_eq!(my_variable, "hello");
    }

    #[tokio::test]
    async fn test_results_as_implicit_args_for_effect() {
        let (effect_tx, mut effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://hello world")
                extract("\\S+")
                effect("notify", {mode="default"})
            "#
        );

        assert!(effect_rx.recv().await.is_some_and(|invocation| {
            assert_eq!(invocation.name(), "notify");
            assert_eq!(
                invocation.args(),
                &vec!["hello".to_string(), "world".to_string()]
            );
            assert_eq!(
                invocation.kwargs().get("mode"),
                Some(&"default".to_string())
            );
            true
        }));
    }

    #[tokio::test]
    async fn test_results_as_implicit_args_for_effect_with_explicit_args() {
        let (effect_tx, mut effect_rx) = unbounded_channel::<EffectInvocation>();
        let script_loader = null_script_loader();

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://hello world")
                extract("\\S+")
                effect("notify", {"foo", "bar", "baz", mode="default"})
            "#
        );

        assert!(effect_rx.recv().await.is_some_and(|invocation| {
            assert_eq!(invocation.name(), "notify");
            assert_eq!(
                invocation.args(),
                &vec!["foo".to_string(), "bar".to_string(), "baz".to_string()]
            );
            assert_eq!(
                invocation.kwargs().get("mode"),
                Some(&"default".to_string())
            );
            true
        }));
    }

    #[tokio::test]
    async fn test_results_as_implicit_args_for_run() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();

        let script_loader = Arc::new(RwLock::new(|name: &str| {
            if name == "test123" {
                Ok(r#"get("string://{2} {3} {1}")"#.to_string())
            } else {
                Err(Error::JobNotFoundError)
            }
        }));

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://foo bar baz")
                extract("\\S+")
                run("test123")
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();
        assert_eq!(
            state.scraper.results(),
            &results!["foo", "bar", "baz", "bar baz foo"]
        );
    }

    #[tokio::test]
    async fn test_results_as_implicit_args_for_run_with_explicit_args() {
        let (effect_tx, _effect_rx) = unbounded_channel::<EffectInvocation>();

        let script_loader = Arc::new(RwLock::new(|name: &str| {
            if name == "test123" {
                Ok(r#"get("string://{2} {3} {1}")"#.to_string())
            } else {
                Err(Error::JobNotFoundError)
            }
        }));

        let lua =
            create_lua_context::<TestHttpDriver>(vec![], HashMap::new(), effect_tx, script_loader)
                .unwrap();

        let _ = lua_run_async!(
            lua,
            r#"
                get("string://foo bar baz")
                extract("\\S+")
                run("test123", {"a", "b", "c"})
            "#
        );

        let state = get_state::<TestHttpDriver>(&lua).unwrap();
        assert_eq!(
            state.scraper.results(),
            &results!["foo", "bar", "baz", "b c a"]
        );
    }

    #[tokio::test]
    async fn test_run() {
        let (effect_tx, mut effect_rx) = unbounded_channel::<EffectInvocation>();

        let script_loader = Arc::new(RwLock::new(|name: &str| {
            if name == "first" {
                Ok(r#"
                        run("second", {"{1}", "{tag}"})
                        effect("notify", {title="Result"})
                    "#
                .to_string())
            } else if name == "second" {
                Ok(r#"
                        get("string://{2} {1}")
                    "#
                .to_string())
            } else {
                Err(Error::JobNotFoundError)
            }
        }));

        let results = run::<TestHttpDriver>(
            "first",
            vec!["hello".to_string()],
            HashMap::from([("tag".to_string(), "1.0".to_string())]),
            script_loader,
            effect_tx,
        )
        .await
        .unwrap();

        assert_eq!(results, results!["1.0 hello"]);

        assert!(effect_rx.recv().await.is_some_and(|invocation| {
            assert_eq!(invocation.name(), "notify");
            assert_eq!(invocation.args(), &vec!["1.0 hello".to_string()]);
            assert_eq!(
                invocation.kwargs(),
                &HashMap::from([("title".to_string(), "Result".to_string())])
            );
            true
        }));
    }
}
