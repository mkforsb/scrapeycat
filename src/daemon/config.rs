use crate::daemon::suite::Suite;

#[derive(Debug, Clone)]
pub struct Config {
    pub script_dirs: Vec<String>,
    pub script_names: Vec<String>,
    pub suites: Option<Vec<Suite>>,
}

impl Config {
    pub fn new(
        script_dirs: Vec<String>,
        script_names: Vec<String>,
        suites: Option<Vec<Suite>>,
    ) -> Self {
        Config {
            script_dirs,
            script_names,
            suites,
        }
    }
}
