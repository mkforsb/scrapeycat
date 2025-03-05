use crate::daemon::suite::Suite;

#[derive(Debug, Clone)]
pub struct Config {
    script_dirs: Vec<String>,
    script_names: Vec<String>,
    suites: Option<Vec<Suite>>,
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

    pub fn script_dirs(&self) -> &Vec<String> {
        &self.script_dirs
    }

    pub fn script_names(&self) -> &Vec<String> {
        &self.script_names
    }

    pub fn suites(&self) -> Option<&Vec<Suite>> {
        self.suites.as_ref()
    }
}
