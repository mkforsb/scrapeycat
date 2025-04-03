use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::Duration,
};

use bolero::{check, r#gen};
use im::vector;
use libscrapeycat::{
    effect::EffectInvocation,
    scrapelang::program::run,
    scraper::{HttpDriver, HttpHeaders},
    Error,
};
use regex::Regex;
use tokio::{sync::mpsc::unbounded_channel, time::sleep};

#[derive(Debug, Clone)]
struct StressTestHttpDriver;

impl HttpDriver for StressTestHttpDriver {
    /// This driver receives `get("X,Y")` where X and Y are numbers, and returns the string X
    /// after sleeping for Y milliseconds.
    async fn get(url: &str, _headers: HttpHeaders<'_>) -> Result<String, Error> {
        let captures = Regex::new("^(\\d+),(\\d+)").unwrap().captures(url).unwrap();

        let result = captures.get(1).unwrap().as_str().to_string();
        let sleep_duration_millis = captures.get(2).unwrap().as_str().parse::<u64>().unwrap();

        sleep(Duration::from_millis(sleep_duration_millis)).await;
        Ok(result)
    }
}

/// This test spawns a large number of scraper tasks (calling `run`) where each task is assigned a
/// random number used to induce `sleep` in the http driver. The test then awaits all the tasks and
/// verifies each output, while keeping track of the number of tasks that remain to be verified.
/// The test succeeds if the number of remaining tasks reaches zero without any assertion failures
/// along the way.
#[tokio::test(flavor = "multi_thread")]
async fn test_stress() {
    let num_tasks_spawned = Arc::new(AtomicUsize::new(0));

    let num_tasks_outer = Arc::new(AtomicUsize::new(0));
    let num_tasks_inner = Arc::clone(&num_tasks_outer);

    check!()
        .with_generator(gen::<[u16; 1000]>())
        .with_iterations(1)
        .with_shrink_time(Duration::ZERO)
        .for_each(|xs| {
            let scripts = xs
                .iter()
                .enumerate()
                .map(|(index, x)| {
                    if index % 2 == 0 {
                        // Even indices run the following odd index
                        format!(r#"run("{}")"#, index + 1)
                    } else {
                        // Odd indices should return the index itself after some random sleep
                        format!(r#"get("{index},{}")"#, x % 333)
                    }
                })
                .collect::<Vec<String>>();

            let script_loader = Arc::new(RwLock::new(move |name: &str| -> Result<String, Error> {
                Ok(scripts.get(name.parse::<usize>().unwrap()).unwrap().clone())
            }));

            let (effect_tx, _) = unbounded_channel::<EffectInvocation>();

            for (index, _) in xs.iter().enumerate() {
                let task = tokio::spawn({
                    let name = index.to_string();
                    let task_script_loader = script_loader.clone();
                    let task_effect_tx = effect_tx.clone();

                    async move {
                        run::<StressTestHttpDriver>(
                            &name,
                            vec![],
                            HashMap::new(),
                            task_script_loader,
                            task_effect_tx,
                        )
                        .await
                    }
                });

                num_tasks_spawned.fetch_add(1, Ordering::SeqCst);
                num_tasks_inner.fetch_add(1, Ordering::SeqCst);

                tokio::spawn({
                    let num_tasks_inner_copy = Arc::clone(&num_tasks_inner);

                    async move {
                        let result = task.await;

                        assert_eq!(
                            result.unwrap().unwrap(),
                            if index % 2 == 0 {
                                // Even indices run the following odd index, so the return
                                // value should be the following odd index
                                vector![(index + 1).to_string()]
                            } else {
                                // Odd indices return the index itself
                                vector![index.to_string()]
                            }
                        );

                        num_tasks_inner_copy.fetch_sub(1, Ordering::SeqCst);
                    }
                });
            }

            assert_eq!(num_tasks_spawned.load(Ordering::SeqCst), xs.len());
        });

    let mut seen_active_tasks = false;

    for _ in 1..=1000 {
        sleep(Duration::from_millis(10)).await;

        if num_tasks_outer.load(Ordering::SeqCst) > 0 {
            seen_active_tasks = true;
            break;
        }
    }

    if !seen_active_tasks {
        panic!("test failure: no tasks seemed to start");
    }

    let mut prev_active_tasks = VecDeque::from([0usize; 9]);

    prev_active_tasks.push_back(num_tasks_outer.load(Ordering::SeqCst));

    loop {
        sleep(Duration::from_millis(100)).await;

        let curr_active_tasks = num_tasks_outer.load(Ordering::SeqCst);

        // Success
        if curr_active_tasks == 0 {
            return;
        }

        prev_active_tasks.pop_front();

        if prev_active_tasks.iter().all(|&x| x == curr_active_tasks) {
            panic!("test failure: progress stalled");
        }

        prev_active_tasks.push_back(curr_active_tasks);
    }
}
