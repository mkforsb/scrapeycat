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
    use std::env;

    use crate::daemon::config::Config;

    use super::*;

    macro_rules! asset_path {
        ($filename:expr) => {
            &format!(
                "{}/tests/assets/daemon/config/{}",
                env::var("CARGO_MANIFEST_DIR").unwrap(),
                $filename,
            )
        };
    }

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

    #[test]
    fn test_get_version() {
        assert!(ConfigFile::get_version(asset_path!("valid/v1_empty.toml"))
            .is_ok_and(|version| version == 1));

        assert!(
            ConfigFile::get_version(asset_path!("valid/v1_one_suite.toml"))
                .is_ok_and(|version| version == 1)
        );

        assert!(
            ConfigFile::get_version(asset_path!("valid/v1_two_suites.toml"))
                .is_ok_and(|version| version == 1)
        );

        assert!(ConfigFile::get_version(asset_path!("invalid/empty_file.toml")).is_err());
        assert!(ConfigFile::get_version(asset_path!("invalid/gibberish.toml")).is_err());
        assert!(ConfigFile::get_version(asset_path!("invalid/small_parse_error.toml")).is_err());
        assert!(ConfigFile::get_version(asset_path!("invalid/bad_version_empty_a.toml")).is_err());
        assert!(ConfigFile::get_version(asset_path!("invalid/bad_version_empty_b.toml")).is_err());
        assert!(ConfigFile::get_version(asset_path!("invalid/bad_version_empty_c.toml")).is_err());
    }

    #[test]
    fn test_config_from_file() {
        assert!(
            ConfigFile::config_from_file(asset_path!("valid/v1_empty.toml")).is_ok_and(|config| {
                assert_eq!(config.script_dirs(), &vec!["/v1_empty".to_string()]);
                assert_eq!(config.script_names(), &vec!["${NAME}.v1_empty".to_string()]);
                assert!(config.suites().is_none());
                true
            })
        );

        assert!(
            ConfigFile::config_from_file(asset_path!("valid/v1_one_suite.toml")).is_ok_and(
                |config| {
                    config.suites().is_some_and(|suites| {
                        assert_eq!(
                            config.script_dirs(),
                            &vec!["/v1".to_string(), "/one".to_string(), "/suite".to_string()]
                        );

                        assert_eq!(
                            config.script_names(),
                            &vec![
                                "${NAME}.v1".to_string(),
                                "${NAME}.one".to_string(),
                                "${NAME}.suite".to_string()
                            ]
                        );

                        assert_eq!(suites.len(), 1);

                        assert!(suites.first().is_some_and(|suite| {
                            assert_eq!(suite.name(), "default");
                            assert_eq!(
                                suite
                                    .jobs()
                                    .map(|job| job.script_name())
                                    .collect::<Vec<_>>(),
                                vec!["aaa", "bbb", "ccc"]
                            );

                            true
                        }));

                        true
                    })
                }
            )
        );

        assert!(
            ConfigFile::config_from_file(asset_path!("valid/v1_two_suites.toml")).is_ok_and(
                |config| {
                    config.suites().is_some_and(|suites| {
                        assert_eq!(config.script_dirs(), &vec!["/v1_two_suites".to_string()],);
                        assert_eq!(config.script_names(), &vec!["${NAME}.txt".to_string(),]);
                        assert_eq!(suites.len(), 2);

                        let suites_map: HashMap<&str, &Suite> =
                            HashMap::from_iter(suites.iter().map(|suite| (suite.name(), suite)));

                        assert!(suites_map.get("first").is_some_and(|suite| {
                            assert_eq!(suite.name(), "first");

                            assert_eq!(
                                suite
                                    .jobs()
                                    .map(|job| job.script_name())
                                    .collect::<Vec<_>>(),
                                vec!["foo", "bar"],
                            );

                            true
                        }));

                        assert!(suites_map.get("second").is_some_and(|suite| {
                            assert_eq!(suite.name(), "second");

                            assert_eq!(
                                suite
                                    .jobs()
                                    .map(|job| job.script_name())
                                    .collect::<Vec<_>>(),
                                vec!["baz", "qux"],
                            );

                            true
                        }));

                        true
                    })
                }
            )
        );

        assert!(ConfigFile::config_from_file(asset_path!("invalid/empty_file.toml")).is_err());
        assert!(ConfigFile::config_from_file(asset_path!("invalid/gibberish.toml")).is_err());
        assert!(
            ConfigFile::config_from_file(asset_path!("invalid/small_parse_error.toml")).is_err()
        );
        assert!(
            ConfigFile::config_from_file(asset_path!("invalid/bad_version_empty_a.toml")).is_err()
        );
        assert!(
            ConfigFile::config_from_file(asset_path!("invalid/bad_version_empty_b.toml")).is_err()
        );
        assert!(
            ConfigFile::config_from_file(asset_path!("invalid/bad_version_empty_c.toml")).is_err()
        );
    }
}
