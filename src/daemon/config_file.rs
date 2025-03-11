#![expect(dead_code)]

use std::{collections::HashMap, fs};

use serde::Deserialize;

use crate::{
    daemon::{
        config::Config,
        suite::{Job, Suite},
    },
    Error,
};

use super::cron::CronSpec;

#[derive(Debug, Clone, Deserialize)]
struct ConfigFile {
    config_version: usize,
}

impl ConfigFile {
    pub fn get_version(path: &str) -> Result<usize, Error> {
        match toml::from_str::<ConfigFile>(fs::read_to_string(path)?.as_str())
            .map_err(|e| Error::ParseError(e.to_string()))?
            .config_version
        {
            version @ 1 => Ok(version),
            _ => Err(Error::UnsupportedConfigVersionError),
        }
    }

    pub fn config_from_file(path: &str) -> Result<Config, Error> {
        match ConfigFile::get_version(path)? {
            1 => Ok(
                toml::from_str::<ConfigFileV1>(fs::read_to_string(path)?.as_str())
                    .map_err(|e| Error::ParseError(e.to_string()))?
                    .try_into()?,
            ),
            _ => Err(Error::UnsupportedConfigVersionError),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ConfigFileV1 {
    config_version: usize,
    script_dirs: Vec<String>,
    script_names: Vec<String>,
    suites: Option<HashMap<String, SuiteV1>>,
}

#[derive(Debug, Clone, Deserialize)]
struct SuiteV1 {
    jobs: Vec<JobV1>,
}

#[derive(Debug, Clone, Deserialize)]
struct JobV1 {
    name: Option<String>,
    script: String,
    args: Option<Vec<String>>,
    kwargs: Option<HashMap<String, String>>,
    schedule: String,
    dedup: bool,
}

impl TryFrom<ConfigFileV1> for Config {
    type Error = Error;

    fn try_from(value: ConfigFileV1) -> Result<Self, Error> {
        let suites = if let Some(config_suites) = value.suites {
            let mut suites = vec![];

            for (name, suite) in config_suites {
                let mut jobs = vec![];

                for job in suite.jobs {
                    jobs.push(Job::new(
                        job.name.unwrap_or("unnamed".to_string()),
                        job.script,
                        job.args,
                        job.kwargs,
                        job.schedule.parse::<CronSpec>()?,
                        job.dedup,
                    )?);
                }

                suites.push(Suite::new(name, jobs));
            }

            Some(suites)
        } else {
            None
        };

        Ok(Config::new(value.script_dirs, value.script_names, suites))
    }
}

#[cfg(test)]
mod tests {
    use crate::daemon::config::Config;

    use super::*;

    #[test]
    fn test_basics() {
        let config_text = r#"
config_version = 1
script_dirs = [".", "${HOME}/.scrapeycat/scripts"]
script_names = ["${NAME}", "${NAME}.scrape"]

[suites.default]
jobs = [
    { name = "x", script = "print", args = ["hi", "bye"], schedule = "0 12 * * *", dedup = false },
    { script = "foo", kwargs = { foo = "bar" }, schedule = "*/5 * * * *", dedup = true },
]
"#;
        let config: ConfigFile = toml::from_str(config_text).unwrap();
        assert_eq!(config.config_version, 1);

        let config: ConfigFileV1 = toml::from_str(config_text).unwrap();

        assert_eq!(
            config.script_dirs,
            vec![".".to_string(), "${HOME}/.scrapeycat/scripts".to_string()]
        );
        assert_eq!(
            config.script_names,
            vec!["${NAME}".to_string(), "${NAME}.scrape".to_string()]
        );

        let suites = config.suites.unwrap();

        assert_eq!(suites.len(), 1);

        let suite_default = suites.get("default").unwrap();

        assert_eq!(suite_default.jobs.len(), 2);

        assert_eq!(&suite_default.jobs[0].name, &Some("x".to_string()));
        assert_eq!(&suite_default.jobs[0].script, "print");
        assert_eq!(suite_default.jobs[0].args.as_ref().unwrap().len(), 2);
        assert_eq!(suite_default.jobs[0].args.as_ref().unwrap()[0], "hi");
        assert_eq!(suite_default.jobs[0].args.as_ref().unwrap()[1], "bye");
        assert!(suite_default.jobs[0].kwargs.is_none());
        assert_eq!(suite_default.jobs[0].schedule, "0 12 * * *");
        assert!(!suite_default.jobs[0].dedup);

        assert_eq!(&suite_default.jobs[1].name, &None::<String>);
        assert_eq!(&suite_default.jobs[1].script, "foo");
        assert!(suite_default.jobs[1].args.is_none());
        assert!(suite_default.jobs[1]
            .kwargs
            .as_ref()
            .is_some_and(|kwargs| { kwargs.get("foo").is_some_and(|value| value == "bar") }));
        assert_eq!(suite_default.jobs[1].schedule, "*/5 * * * *");
        assert!(suite_default.jobs[1].dedup);
    }

    #[test]
    fn test_into_domain() {
        let config_text = r#"
config_version = 1
script_dirs = ["/var/scraper"]
script_names = ["${NAME}.txt"]

[suites.common]
jobs = [
    { script = "get-temperature", args = ["stockholm"], schedule = "*/10 * * * *", dedup = false },
]
"#;
        let config: Config = toml::from_str::<ConfigFileV1>(config_text)
            .unwrap()
            .try_into()
            .unwrap();

        assert_eq!(config.script_dirs(), &vec!["/var/scraper".to_string()]);
        assert_eq!(config.script_names(), &vec!["${NAME}.txt".to_string()]);
        assert_eq!(config.suites().unwrap().len(), 1);
        assert_eq!(config.suites().unwrap()[0].name(), "common");
        assert_eq!(config.suites().unwrap()[0].jobs().count(), 1);
    }
}
