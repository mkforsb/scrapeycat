use std::collections::HashMap;

use chrono::{DateTime, Local};
use regex::Regex;

use crate::{daemon::cron::CronSpec, Error};

#[derive(Debug)]
pub struct Suite {
    name: String,
    jobs: Vec<Job>,
}

impl Suite {
    pub fn new(name: impl Into<String>, jobs: Vec<Job>) -> Self {
        Suite {
            name: name.into(),
            jobs,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn jobs(&self) -> impl Iterator<Item = &Job> {
        self.jobs.iter()
    }
}

#[expect(unused)]
#[derive(Debug)]
pub struct Job {
    name: String,
    script_name: String,
    args: Vec<String>,
    kwargs: HashMap<String, String>,
    schedule: CronSpec,
    schedule_regex: Regex,
    dedup: bool,
}

impl Job {
    pub fn new(
        name: impl Into<String>,
        script_name: impl Into<String>,
        schedule: CronSpec,
        dedup: bool,
    ) -> Result<Job, Error> {
        let schedule_regex = Regex::new(&schedule.to_regex_pattern())?;

        Ok(Job {
            name: name.into(),
            script_name: script_name.into(),
            args: vec![],
            kwargs: HashMap::new(),
            schedule,
            schedule_regex,
            dedup,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn script_name(&self) -> &str {
        &self.script_name
    }

    pub fn args(&self) -> &Vec<String> {
        &self.args
    }

    pub fn kwargs(&self) -> &HashMap<String, String> {
        &self.kwargs
    }

    pub fn is_due(&self) -> bool {
        self.is_due_at(Local::now())
    }

    pub fn is_due_at(&self, when: DateTime<Local>) -> bool {
        self.schedule_regex
            .is_match(&format!("{}", when.format("%M%H%d%m0%u")))
    }

    pub fn is_dedup(&self) -> bool {
        self.dedup
    }
}
