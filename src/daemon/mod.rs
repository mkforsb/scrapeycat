use std::time::Duration;

use chrono::Local;

pub mod cron;
pub mod suite;

use suite::Suite;
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::{
    effect::{self, EffectInvocation, EffectOptions, EffectSignature},
    scrapelang::program::run,
    Error,
};

// TODO: implement dedup
async fn effects_handler(id: String, mut effects_receiver: UnboundedReceiver<EffectInvocation>) {
    loop {
        match effects_receiver.recv().await {
            Some(invocation) => {
                let effect_fn = match invocation.name() {
                    "print" => Some(effect::print as EffectSignature),
                    "notify" => Some(effect::notify as EffectSignature),
                    _ => None,
                };

                match effect_fn {
                    Some(function) => {
                        if let Some(error) = function(
                            invocation.args(),
                            invocation.kwargs(),
                            EffectOptions::default().into(),
                        ) {
                            eprintln!("{error}");
                        }
                    }
                    None => eprintln!("Unknown effect `{}` invoked from {id}", invocation.name()),
                }
            }
            None => return,
        }
    }
}

// TODO: implement dedup
// TODO: it would be cool if the daemon could pick up changes to the config automatically
pub async fn run_forever(suites: Vec<Suite>, script_loader: fn(&str) -> Result<String, Error>) {
    let jobs = suites
        .iter()
        .flat_map(|suite| {
            suite.jobs().enumerate().map(|(nth, job)| {
                let (tx, rx) = mpsc::unbounded_channel::<EffectInvocation>();
                (
                    job,
                    tx,
                    tokio::spawn(effects_handler(
                        format!("{}.{}-{}", suite.name(), job.script_name(), nth),
                        rx,
                    )),
                )
            })
        })
        .collect::<Vec<_>>();

    loop {
        let now = Local::now();

        for (job, effect_tx, _) in &jobs {
            if job.is_due_at(now) {
                let task_script_name = job.script_name().to_string();
                let task_args = job.args().clone();
                let task_kwargs = job.kwargs().clone();
                let task_effect_sender = effect_tx.clone();

                tokio::spawn(async move {
                    let _ = run(
                        &task_script_name,
                        task_args,
                        task_kwargs,
                        script_loader,
                        task_effect_sender,
                    )
                    .await;
                });
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::daemon::{cron::CronSpec, suite::Job};

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
        let suite = Suite::new(
            "default".to_string(),
            vec![Job::new(
                "default",
                "print",
                "* * * * *".parse::<CronSpec>().unwrap(),
                true,
            )
            .unwrap()],
        );

        run_forever(vec![suite], load_script).await;
    }
}
