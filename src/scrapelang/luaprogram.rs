use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use im::Vector;
use mlua::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    effect::EffectInvocation,
    scraper::{HttpDriver, Scraper},
    Error,
};

pub type ScriptLoaderPointer = Arc<RwLock<dyn Fn(&str) -> Result<String, Error> + Send + Sync>>;

pub async fn run<H: HttpDriver + 'static>(
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

    let lua = Lua::new();
    lua.load_std_libs(LuaStdLib::ALL_SAFE)
        .map_err(|e| Error::LuaError(e.to_string()))?;

    lua.globals().set("args", args).unwrap();
    lua.globals().set("kwargs", kwargs).unwrap();

    let scraper = lua.create_any_userdata(Scraper::<H>::new()).unwrap();
    lua.globals().set("_scraper", scraper).unwrap();

    macro_rules! get_scraper {
        ($lua:ident) => {
            $lua.globals()
                .get::<LuaAnyUserData>("_scraper")
                .unwrap()
                .borrow_mut::<Scraper<H>>()
                .unwrap()
        };
    }

    let append = lua
        .create_function(|lua: &Lua, text: String| {
            let mut scraper = get_scraper!(lua);
            *scraper = scraper.append(&text);
            Ok(())
        })
        .unwrap();

    let clear = lua
        .create_function(|lua: &Lua, ()| {
            let mut scraper = get_scraper!(lua);
            *scraper = scraper.clear();
            Ok(())
        })
        .unwrap();

    let clearheaders = lua
        .create_function(|lua: &Lua, ()| {
            let mut scraper = get_scraper!(lua);
            *scraper = scraper.clear_headers();
            Ok(())
        })
        .unwrap();

    let delete = lua
        .create_function(|lua: &Lua, pattern: String| {
            let mut scraper = get_scraper!(lua);
            *scraper = scraper.delete(&pattern).unwrap();
            Ok(())
        })
        .unwrap();

    let discard = lua
        .create_function(|lua: &Lua, pattern: String| {
            let mut scraper = get_scraper!(lua);
            *scraper = scraper.discard(&pattern).unwrap();
            Ok(())
        })
        .unwrap();

    let drop = lua
        .create_function(|lua: &Lua, n: usize| {
            let mut scraper = get_scraper!(lua);
            *scraper = Scraper::<H>::drop(&scraper, n);
            Ok(())
        })
        .unwrap();

    let effect_sender_for_effect = effect_sender.clone();

    let effect = lua
        .create_function(move |lua: &Lua, (name, args_table): (String, LuaTable)| {
            let scraper = get_scraper!(lua);
            let mut args: Vec<String> = vec![];

            for i in 1..100 {
                if let Ok(value) = args_table.get(i) {
                    args.push(value);
                }
            }

            if args.len() == 0 {
                args.extend(scraper.results().iter().cloned());
            }

            let mut kwargs: HashMap<String, String> = HashMap::new();

            for pair in args_table.pairs::<String, String>() {
                if let Ok((key, value)) = pair {
                    if !key.chars().all(|ch| ch.is_digit(10)) {
                        kwargs.insert(key, value);
                    }
                }
            }

            effect_sender_for_effect
                .send(EffectInvocation::new(name, args, kwargs))
                .unwrap();

            Ok(())
        })
        .unwrap();

    let effect_sender_for_run = effect_sender.clone();
    let script_loader_for_run = script_loader.clone();

    let runfn = lua
        .create_async_function(
            move |lua: Lua, (name, args_table): (String, Option<LuaTable>)| {
                let effect_sender_for_run_inner = effect_sender_for_run.clone();
                let script_loader_for_run_inner = script_loader_for_run.clone();

                async move {
                    let mut scraper = get_scraper!(lua);
                    let mut args: Vec<String> = vec![];
                    let mut kwargs: HashMap<String, String> = HashMap::new();

                    if let Some(args_table) = args_table {
                        for i in 1..100 {
                            if let Ok(value) = args_table.get(i) {
                                args.push(value);
                            }
                        }

                        for pair in args_table.pairs::<String, String>() {
                            if let Ok((key, value)) = pair {
                                if !key.chars().all(|ch| ch.is_digit(10)) {
                                    kwargs.insert(key, value);
                                }
                            }
                        }
                    }

                    if args.len() == 0 {
                        args.extend(scraper.results().iter().cloned());
                    }

                    let mut new_results = scraper.results().clone();

                    new_results.append(
                        Box::pin(run::<H>(
                            &name,
                            args,
                            kwargs,
                            script_loader_for_run_inner,
                            effect_sender_for_run_inner,
                        ))
                        .await
                        .unwrap(),
                    );

                    *scraper = scraper.clone().with_results(new_results);
                    Ok(())
                }
            },
        )
        .unwrap();

    let extract = lua
        .create_function(|lua: &Lua, pattern: String| {
            let mut scraper = get_scraper!(lua);
            *scraper = scraper.extract(&pattern).unwrap();
            Ok(())
        })
        .unwrap();

    let first = lua
        .create_function(|lua: &Lua, ()| {
            let mut scraper = get_scraper!(lua);
            *scraper = scraper.first();
            Ok(())
        })
        .unwrap();

    let header = lua
        .create_function(|lua: &Lua, (name, value): (String, String)| {
            let mut scraper = get_scraper!(lua);
            *scraper = scraper.set_header(name, value);
            Ok(())
        })
        .unwrap();

    let get = lua
        .create_async_function(|lua: Lua, url: String| async move {
            let mut scraper = get_scraper!(lua);
            *scraper = scraper.get(&url).await.unwrap();

            Ok(())
        })
        .unwrap();

    let results = lua
        .create_function(|lua: &Lua, ()| {
            let scraper = get_scraper!(lua);
            let strings = scraper.results().iter().cloned().collect::<Vec<_>>();
            Ok(strings)
        })
        .unwrap();

    let set_results = lua
        .create_function(|lua: &Lua, results: LuaTable| {
            let mut scraper = get_scraper!(lua);
            *scraper = scraper.clone().with_results(Vector::from_iter(
                results.sequence_values().map(|x| x.unwrap()),
            ));
            Ok(())
        })
        .unwrap();

    let prepend = lua
        .create_function(|lua: &Lua, text: String| {
            let mut scraper = get_scraper!(lua);
            *scraper = scraper.prepend(&text);
            Ok(())
        })
        .unwrap();

    let retain = lua
        .create_function(|lua: &Lua, pattern: String| {
            let mut scraper = get_scraper!(lua);
            *scraper = scraper.retain(&pattern).unwrap();
            Ok(())
        })
        .unwrap();

    lua.globals().set("append", append).unwrap();
    lua.globals().set("clear", clear).unwrap();
    lua.globals().set("clearheaders", clearheaders).unwrap();
    lua.globals().set("delete", delete).unwrap();
    lua.globals().set("discard", discard).unwrap();
    lua.globals().set("drop", drop).unwrap();
    lua.globals().set("effect", effect).unwrap();
    lua.globals().set("extract", extract).unwrap();
    lua.globals().set("first", first).unwrap();
    lua.globals().set("header", header).unwrap();
    lua.globals().set("get", get).unwrap();
    lua.globals().set("prepend", prepend).unwrap();
    lua.globals().set("results", results).unwrap();
    lua.globals().set("set_results", set_results).unwrap();
    lua.globals().set("retain", retain).unwrap();
    lua.globals().set("run", runfn).unwrap();

    lua.load(lua_code).exec_async().await.unwrap();

    Ok(get_scraper!(lua).results().clone())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tokio::sync::mpsc;

    use crate::scraper::ReqwestHttpDriver;

    use super::*;

    #[tokio::test]
    async fn foo() {
        let script_name = "/tmp/script.txt";
        let args = vec![];
        let kwargs = HashMap::new();
        let script_loader = Arc::new(RwLock::new(|filename: &str| {
            Ok(fs::read_to_string(filename).unwrap())
        }));

        let (effect_tx, mut effect_rx) = mpsc::unbounded_channel::<EffectInvocation>();

        dbg!(
            run::<ReqwestHttpDriver>(script_name, args, kwargs, script_loader, effect_tx)
                .await
                .unwrap()
        );

        while let Some(message) = effect_rx.recv().await {
            dbg!(message);
        }
    }
}
