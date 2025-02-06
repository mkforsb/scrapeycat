use std::{collections::HashMap, env, fs};

use scrapeycat::{
    effect::{self, EffectSignature},
    scrapelang::program::run,
    Error,
};

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

    let effects: HashMap<String, EffectSignature> = HashMap::from_iter(
        ([
            ("print".to_string(), effect::print as EffectSignature),
            ("notify".to_string(), effect::notify),
        ])
        .into_iter(),
    );

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
                    effects
                )
                .await
            );
        }
        None => {
            println!(
                "usage: {} path/to/script",
                args.first().unwrap_or(&"scrapeycat".to_string())
            );
        }
    }
}
