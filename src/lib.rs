pub mod daemon;
pub mod effect;
pub mod scrapelang;
pub mod scraper;
pub mod util;

use std::{io, num::ParseIntError};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IOError(#[from] io::Error),

    #[error("Fetch error: {0}")]
    FetchError(#[from] reqwest::Error),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Parse error: {0}")]
    ParseIntError(#[from] ParseIntError),

    #[error("No such variable: `{0}`")]
    VariableNotFoundError(String),

    #[error("Stopped: {0}")]
    Stopped(String),

    #[error("Job not found")]
    JobNotFoundError,

    #[error("Effect error: {0}")]
    EffectError(String),

    #[error("Effect not found")]
    EffectNotFoundError,

    #[error("Value out of range")]
    ValueOutOfRangeError,

    #[error("Invalid range")]
    InvalidRangeError,

    #[error("Unsupported config version")]
    UnsupportedConfigVersionError,
}
