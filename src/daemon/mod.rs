use std::{collections::HashMap, time::Duration};

use chrono::Local;

pub mod cron;
pub mod suite;

use suite::Suite;

use crate::{effect::EffectSignature, scrapelang::program::run, Error};

// TODO: implement dedup
// TODO: it would be cool if the daemon could pick up changes to the config automatically
pub async fn run_forever(
    suites: Vec<Suite>,
    script_loader: fn(&str) -> Result<String, Error>,
    effects: HashMap<String, EffectSignature>,
) {
    loop {
        let now = Local::now();

        for suite in suites.iter() {
            for job in suite.jobs() {
                if job.is_due_at(now) {
                    let task_script_name = job.script_name().to_string();
                    let task_args = job.args().clone();
                    let task_kwargs = job.kwargs().clone();
                    let task_effects = effects.clone();

                    tokio::spawn(async move {
                        let _ = run(
                            &task_script_name,
                            task_args,
                            task_kwargs,
                            script_loader,
                            task_effects,
                        )
                        .await;
                    });
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        daemon::{cron::CronSpec, suite::Job},
        effect::{self, EffectSignature},
    };

    use super::*;

    fn load_script(name_or_filename: &str) -> Result<String, Error> {
        fs::read_to_string(name_or_filename)
            .or_else(|_| fs::read_to_string(format!("{name_or_filename}.scrape")))
            .or_else(|_| fs::read_to_string(format!("./scripts/{name_or_filename}")))
            .or_else(|_| fs::read_to_string(format!("./scripts/{name_or_filename}.scrape")))
            .map_err(|e| e.into())
    }

    #[ignore]
    #[tokio::test]
    async fn test_print_once_per_second() {
        let effects: HashMap<String, EffectSignature> = HashMap::from_iter(
            ([
                ("print".to_string(), effect::print as EffectSignature),
                ("notify".to_string(), effect::notify),
            ])
            .into_iter(),
        );

        let suite = Suite::new(vec![Job::new(
            "print",
            "* * * * *".parse::<CronSpec>().unwrap(),
            true,
        )
        .unwrap()]);

        run_forever(vec![suite], load_script, effects).await;
    }
}
