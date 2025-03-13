use std::{
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
    time::Duration,
};

use chrono::{DateTime, Local};
use flagset::{flags, FlagSet};
use suite::{Job, Suite};
use tokio::sync::mpsc::{self, UnboundedReceiver};

pub mod config;
pub mod config_file;
pub mod cron;
pub mod suite;

use crate::{
    effect::{EffectInvocation, EffectOptions, EffectSignature},
    scrapelang::program::{run, ScriptLoaderPointer},
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

/// Trait for the clock of the main daemon loop in [run_forever].
pub trait Clock {
    /// Get the tick interval.
    ///
    /// The daemon will check for due jobs once per tick, but note that jobs are always
    /// scheduled at one-minute granularity.
    fn interval(&mut self) -> Duration;

    /// Check the clock.
    ///
    /// This method is called exactly once per interval.
    fn now(&mut self) -> Option<DateTime<Local>>;

    /// Peek at the clock to ensure we're not oversleeping.
    ///
    /// This method may be called multiple times per interval and/or in the middle of
    /// an interval. The distinction between [Clock::now] and [Clock::peek] is useful
    /// for creating different types of mock clocks in testing.
    fn peek(&mut self) -> Option<DateTime<Local>>;

    /// Sleep for some time.
    #[allow(async_fn_in_trait)]
    async fn sleep(&mut self, time: Duration);
}

/// The default local clock with a one-minute interval.
#[derive(Default)]
pub struct LocalMinuteIntervalClock;

impl Clock for LocalMinuteIntervalClock {
    fn interval(&mut self) -> Duration {
        Duration::from_secs(60)
    }

    fn now(&mut self) -> Option<DateTime<Local>> {
        Some(Local::now())
    }

    fn peek(&mut self) -> Option<DateTime<Local>> {
        Some(Local::now())
    }

    async fn sleep(&mut self, time: Duration) {
        tokio::time::sleep(time).await
    }
}

// TODO: it would be cool if the daemon could pick up changes to the config automatically
pub async fn run_forever(
    suites: Vec<Suite>,
    script_loader: ScriptLoaderPointer,
    effects: HashMap<String, EffectSignature>,
    mut clock: impl Clock,
) {
    let interval = clock.interval();

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
        let datetime_top = clock.now();

        if datetime_top.is_none() {
            break;
        }

        for (job, effect_tx, _) in &jobs {
            if job.is_due_at(datetime_top.expect("`datetime_top` cannot be None")) {
                let task_script_name = job.script_name().to_string();
                let task_args = job.args().clone();
                let task_kwargs = job.kwargs().clone();
                let task_effect_sender = effect_tx.clone();
                let task_script_loader = script_loader.clone();

                tokio::spawn(async move {
                    let _ = run(
                        &task_script_name,
                        task_args,
                        task_kwargs,
                        task_script_loader,
                        task_effect_sender,
                    )
                    .await;
                });
            }
        }

        clock.sleep(interval / 2).await;

        let datetime_middle = clock.peek();

        if datetime_middle.is_none() {
            break;
        }

        if Job::format_datetime(datetime_top.expect("`datetime_top` cannot be None"))
            == Job::format_datetime(datetime_middle.expect("`datetime_middle` cannot be None"))
        {
            clock.sleep(interval / 2).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        sync::{
            atomic::{AtomicU32, Ordering::SeqCst},
            Arc, RwLock,
        },
    };

    use chrono::TimeDelta;
    use flagset::FlagSet;

    use crate::{
        daemon::{cron::CronSpec, suite::Job},
        effect::{EffectArgs, EffectKwArgs},
        Error,
    };

    use super::*;

    fn script_loader(name_or_filename: &str) -> Result<String, Error> {
        fs::read_to_string(name_or_filename).map_err(|e| {
            eprintln!("error loading {name_or_filename}: {e}");
            e.into()
        })
    }

    /// A mock clock simulating a world where oversleeping never happens and thus
    /// every single time step is always considered.
    struct PerfectMockClock {
        timestamps: Vec<DateTime<Local>>,
        offset: usize,
    }

    impl Clock for PerfectMockClock {
        fn interval(&mut self) -> Duration {
            Duration::ZERO
        }

        fn now(&mut self) -> Option<DateTime<Local>> {
            self.offset += 1;
            self.timestamps.get(self.offset - 1).cloned()
        }

        fn peek(&mut self) -> Option<DateTime<Local>> {
            self.timestamps.get(self.offset - 1).cloned()
        }

        async fn sleep(&mut self, _time: Duration) {}
    }

    static TEST_PRINT_EACH_MINUTE_COUNT: AtomicU32 = AtomicU32::new(0);

    #[tokio::test]
    async fn test_print_each_minute() {
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

        TEST_PRINT_EACH_MINUTE_COUNT.swap(0, SeqCst);

        fn print(_: EffectArgs, _: EffectKwArgs, _: FlagSet<EffectOptions>) -> Option<Error> {
            TEST_PRINT_EACH_MINUTE_COUNT.fetch_add(1, SeqCst);
            None
        }

        let effects: HashMap<String, EffectSignature> =
            HashMap::from([("print".to_string(), print as EffectSignature)]);

        let t0 = Local::now();

        let clock = PerfectMockClock {
            timestamps: vec![t0, t0 + TimeDelta::minutes(1), t0 + TimeDelta::minutes(2)],
            offset: 0,
        };

        let task_handle = tokio::spawn(run_forever(
            vec![suite],
            Arc::new(RwLock::new(script_loader)),
            effects,
            clock,
        ));

        let _ = tokio::join!(task_handle);
        assert_eq!(TEST_PRINT_EACH_MINUTE_COUNT.load(SeqCst), 3);
    }

    static TEST_PRINT_EACH_MINUTE_DEDUP_COUNT: AtomicU32 = AtomicU32::new(0);

    #[tokio::test]
    async fn test_print_each_minute_dedup() {
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

        TEST_PRINT_EACH_MINUTE_DEDUP_COUNT.swap(0, SeqCst);

        fn print(_: EffectArgs, _: EffectKwArgs, _: FlagSet<EffectOptions>) -> Option<Error> {
            TEST_PRINT_EACH_MINUTE_DEDUP_COUNT.fetch_add(1, SeqCst);
            None
        }

        let effects: HashMap<String, EffectSignature> =
            HashMap::from([("print".to_string(), print as EffectSignature)]);

        let t0 = Local::now();

        let clock = PerfectMockClock {
            timestamps: vec![t0, t0 + TimeDelta::minutes(1), t0 + TimeDelta::minutes(2)],
            offset: 0,
        };

        let task_handle = tokio::spawn(run_forever(
            vec![suite],
            Arc::new(RwLock::new(script_loader)),
            effects,
            clock,
        ));

        let _ = tokio::join!(task_handle);
        assert_eq!(TEST_PRINT_EACH_MINUTE_DEDUP_COUNT.load(SeqCst), 1);
    }
}
