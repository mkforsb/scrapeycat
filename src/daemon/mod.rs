use std::{
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
    time::Duration,
};

use chrono::Local;

pub mod cron;
pub mod suite;

use flagset::{flags, FlagSet};
use suite::Suite;
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::{
    effect::{EffectInvocation, EffectOptions, EffectSignature},
    scrapelang::program::run,
    Error,
};

flags! {
    #[derive(Default)]
    enum EffectsHandlerOptions: u32 {
        #[default]
        Default = 0,

        Deduplicate = 1,
    }
}

async fn effects_handler(
    id: String,
    mut effects_receiver: UnboundedReceiver<EffectInvocation>,
    effects: HashMap<String, EffectSignature>,
    options: FlagSet<EffectsHandlerOptions>,
) {
    let mut dedup_seen: HashSet<u64> = HashSet::new();

    loop {
        match effects_receiver.recv().await {
            Some(invocation) => {
                if options.contains(EffectsHandlerOptions::Deduplicate) {
                    let mut hasher = DefaultHasher::new();
                    invocation.hash(&mut hasher);

                    let invocation_hash = hasher.finish();

                    if dedup_seen.contains(&invocation_hash) {
                        continue;
                    }

                    dedup_seen.insert(invocation_hash);
                }

                match effects.get(invocation.name()) {
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

// TODO: it would be cool if the daemon could pick up changes to the config automatically
pub async fn run_forever(
    suites: Vec<Suite>,
    script_loader: fn(&str) -> Result<String, Error>,
    effects: HashMap<String, EffectSignature>,
) {
    let jobs = suites
        .iter()
        .flat_map(|suite| {
            suite.jobs().enumerate().map(|(nth, job)| {
                let mut options: FlagSet<_> = EffectsHandlerOptions::Default.into();

                if job.is_dedup() {
                    options |= EffectsHandlerOptions::Deduplicate;
                }

                let (tx, rx) = mpsc::unbounded_channel::<EffectInvocation>();
                (
                    job,
                    tx,
                    tokio::spawn(effects_handler(
                        format!("{}.{}-{}", suite.name(), job.script_name(), nth),
                        rx,
                        effects.clone(),
                        options,
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
    use std::{
        env, fs,
        sync::atomic::{AtomicU32, Ordering::SeqCst},
    };

    use flagset::FlagSet;

    use crate::{
        daemon::{cron::CronSpec, suite::Job},
        effect::{EffectArgs, EffectKwArgs},
    };

    use super::*;

    fn load_script(name_or_filename: &str) -> Result<String, Error> {
        fs::read_to_string(name_or_filename).map_err(|e| {
            eprintln!("error loading {name_or_filename}: {e}");
            e.into()
        })
    }

    static TEST_PRINT_ONCE_PER_SECOND_COUNT: AtomicU32 = AtomicU32::new(0);

    #[tokio::test]
    async fn test_print_once_per_second() {
        let suite = Suite::new(
            "default".to_string(),
            vec![Job::new(
                "default",
                format!(
                    "{}/scripts/print.scrape",
                    env::var("CARGO_MANIFEST_DIR").unwrap()
                ),
                None,
                None,
                "* * * * *".parse::<CronSpec>().unwrap(),
                false,
            )
            .unwrap()],
        );

        TEST_PRINT_ONCE_PER_SECOND_COUNT.swap(0, SeqCst);

        fn print(_: EffectArgs, _: EffectKwArgs, _: FlagSet<EffectOptions>) -> Option<Error> {
            TEST_PRINT_ONCE_PER_SECOND_COUNT.fetch_add(1, SeqCst);
            None
        }

        let effects: HashMap<String, EffectSignature> =
            HashMap::from([("print".to_string(), print as EffectSignature)]);

        let task_handle = tokio::spawn(run_forever(vec![suite], load_script, effects));

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert_eq!(TEST_PRINT_ONCE_PER_SECOND_COUNT.load(SeqCst), 1);

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        assert_eq!(TEST_PRINT_ONCE_PER_SECOND_COUNT.load(SeqCst), 2);

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        assert_eq!(TEST_PRINT_ONCE_PER_SECOND_COUNT.load(SeqCst), 3);

        task_handle.abort();
    }

    static TEST_PRINT_ONCE_PER_SECOND_DEDUP_COUNT: AtomicU32 = AtomicU32::new(0);

    #[tokio::test]
    async fn test_print_once_per_second_dedup() {
        let suite = Suite::new(
            "default".to_string(),
            vec![Job::new(
                "default",
                format!(
                    "{}/scripts/print.scrape",
                    env::var("CARGO_MANIFEST_DIR").unwrap()
                ),
                None,
                None,
                "* * * * *".parse::<CronSpec>().unwrap(),
                true,
            )
            .unwrap()],
        );

        TEST_PRINT_ONCE_PER_SECOND_DEDUP_COUNT.swap(0, SeqCst);

        fn print(_: EffectArgs, _: EffectKwArgs, _: FlagSet<EffectOptions>) -> Option<Error> {
            TEST_PRINT_ONCE_PER_SECOND_DEDUP_COUNT.fetch_add(1, SeqCst);
            None
        }

        let effects: HashMap<String, EffectSignature> =
            HashMap::from([("print".to_string(), print as EffectSignature)]);

        let task_handle = tokio::spawn(run_forever(vec![suite], load_script, effects));

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert_eq!(TEST_PRINT_ONCE_PER_SECOND_DEDUP_COUNT.load(SeqCst), 1);

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        assert_eq!(TEST_PRINT_ONCE_PER_SECOND_DEDUP_COUNT.load(SeqCst), 1);

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        assert_eq!(TEST_PRINT_ONCE_PER_SECOND_DEDUP_COUNT.load(SeqCst), 1);

        task_handle.abort();
    }
}
