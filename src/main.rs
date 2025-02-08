use std::{collections::HashMap, env, fs};

use scrapeycat::{
    effect::{self, EffectInvocation},
    scrapelang::program::run,
    Error,
};
use tokio::sync::mpsc;

fn load_script(name_or_filename: &str) -> Result<String, Error> {
    fs::read_to_string(name_or_filename)
        .or_else(|_| fs::read_to_string(format!("{name_or_filename}.scrape")))
        .or_else(|_| fs::read_to_string(format!("./scripts/{name_or_filename}")))
        .or_else(|_| fs::read_to_string(format!("./scripts/{name_or_filename}.scrape")))
        .map_err(|e| e.into())
}

#[tokio::main]
async fn main() {
    let args = env::args().collect::<Vec<_>>();

    let (effects_sender, effects_receiver) = mpsc::unbounded_channel::<EffectInvocation>();
    let effects_runner_task = tokio::spawn(effect::default_effects_runner_task(effects_receiver));

    match args.get(1) {
        Some(script_name_or_filename) => {
            let args = env::args().skip(2).collect::<Vec<String>>();

            println!(
                "{:#?}",
                run(
                    script_name_or_filename,
                    args,
                    HashMap::new(),
                    load_script,
                    effects_sender,
                )
                .await
            );

            let _ = tokio::join!(effects_runner_task);
        }
        None => {
            println!(
                "usage: {} path/to/script",
                args.first().unwrap_or(&"scrapeycat".to_string())
            );
        }
    }
}
