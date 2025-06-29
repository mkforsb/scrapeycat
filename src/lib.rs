pub mod daemon;
pub mod effect;
pub mod scrapelang;
pub mod scraper;
pub mod util;

#[cfg(any(test, feature = "testutils"))]
pub mod testutils;

use std::{io, num::ParseIntError};

use jsonpath_rust::parser::errors::JsonPathError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IOError(#[from] io::Error),

    #[error("Script not found: {0}")]
    ScriptNotFoundError(String),

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

    #[error("Script loader locking error")]
    ScriptLoaderLockingError,

    #[error("HTTP driver error: {0}")]
    HTTPDriverError(String),

    #[error("Lua error: {0}")]
    LuaError(String),

    #[error("JSON parse error: {0}")]
    JsonParseError(String),

    #[error("JSONPath error: {0}")]
    JsonPathError(#[from] JsonPathError),
}
