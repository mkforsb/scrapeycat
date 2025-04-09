#![cfg(any(test, feature = "testutils"))]

use std::{env, fs};

use crate::{
    scraper::{HttpDriver, HttpHeaders},
    Error,
};

/// `path_in_project_root!("foo")` -> `"/<projectroot>/foo"`, where `<projectroot>` is the path
/// to the directory that contains the Cargo.toml project manifest.
#[macro_export]
macro_rules! path_in_project_root {
    ($path:expr) => {
        format!("{}/{}", env::var("CARGO_MANIFEST_DIR").unwrap(), $path)
    };
}

pub use path_in_project_root;

/// The TestHttpDriver supports the following forms of URLs:
///
/// * `file://<path>`: returns contents of local filesystem at `<path>`.
/// * `string://<content>`: returns the string `<content>`.
#[derive(Debug, Clone)]
pub struct TestHttpDriver;

impl HttpDriver for TestHttpDriver {
    async fn get(url: &str, _headers: HttpHeaders<'_>) -> Result<String, Error> {
        if url.starts_with("file://") {
            Ok(fs::read_to_string(path_in_project_root!(url
                .strip_prefix("file://")
                .unwrap()))?)
        } else if url.starts_with("string://") {
            Ok(url.strip_prefix("string://").unwrap().to_string())
        } else {
            Err(Error::HTTPDriverError("invalid url".to_string()))
        }
    }
}

#[derive(Debug, Clone)]
pub struct HeaderTestHttpDriver;

impl HttpDriver for HeaderTestHttpDriver {
    async fn get(_url: &str, headers: HttpHeaders<'_>) -> Result<String, Error> {
        match headers {
            HttpHeaders::NoHeaders => Ok("NoHeaders".to_string()),
            HttpHeaders::Headers(hash_map) => {
                let mut keyvals = hash_map
                    .iter()
                    .map(|(key, value)| format!("\"{key}\": \"{value}\""))
                    .collect::<Vec<_>>();

                // Sorted output
                keyvals.sort();

                let keyvals = keyvals.join(", ");

                let mut result = vec!["Headers({"];

                result.push(&keyvals);
                result.push("})");

                Ok(result.join(""))
            }
        }
    }
}
