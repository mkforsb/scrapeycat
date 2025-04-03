use std::{cmp::min, future::Future, marker::PhantomData};

use im::{vector, HashMap, Vector};
use log::debug;
use regex::Regex;
use reqwest::{
    header::{HeaderMap, HeaderName, InvalidHeaderValue},
    ClientBuilder,
};

use crate::Error;

#[derive(Debug)]
pub enum HttpHeaders<'a> {
    NoHeaders,
    Headers(&'a HashMap<String, String>),
}

// #[allow(async_fn_in_trait)]
pub trait HttpDriver: Clone {
    fn get(
        url: &str,
        headers: HttpHeaders<'_>,
    ) -> impl Future<Output = Result<String, Error>> + Send;

    // TODO: post(url, content)

    // TODO(?): other request methods?
}

#[derive(Clone)]
pub struct NullHttpDriver;

impl HttpDriver for NullHttpDriver {
    async fn get(_url: &str, _headers: HttpHeaders<'_>) -> Result<String, Error> {
        Ok("".to_string())
    }
}

#[derive(Clone)]
pub struct ReqwestHttpDriver;

impl HttpDriver for ReqwestHttpDriver {
    async fn get(url: &str, headers: HttpHeaders<'_>) -> Result<String, Error> {
        let mut reqwest_headers = HeaderMap::new();

        if let HttpHeaders::Headers(map) = headers {
            for (key, value) in map {
                reqwest_headers.insert(
                    HeaderName::from_bytes(key.as_bytes())
                        .map_err(|e| Error::HTTPDriverError(e.to_string()))?,
                    value
                        .parse()
                        .map_err(|e: InvalidHeaderValue| Error::HTTPDriverError(e.to_string()))?,
                );
            }
        }

        let client = ClientBuilder::new()
            .default_headers(reqwest_headers)
            .build()?;

        debug!("reqwest http driver: request to {url} (headers={headers:?})");

        let result = client.get(url).send().await?.text().await?;

        debug!("reqwest http driver: response from {url}");
        Ok(result)
    }
}

#[derive(Clone)]
pub struct Scraper<H: HttpDriver> {
    results: Vector<String>,
    headers: HashMap<String, String>,
    _marker: PhantomData<H>,
}

impl<H> std::fmt::Debug for Scraper<H>
where
    H: HttpDriver,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, str) in self.results.iter().enumerate() {
            f.write_fmt(format_args!("{}: {}\n", i, str))?;
        }

        Ok(())
    }
}

impl<H> Default for Scraper<H>
where
    H: HttpDriver,
{
    fn default() -> Self {
        Self::new()
    }
}

// result.append(" Rules:")
// result.append("   $random N        Take N results randomly")
// result.append("   $str.replace S1 S2   Replace all occurrences of string S1 with S2")
// result.append("   $re.replace P S       Replace all occurrences of regex pattern P with S")
// result.append("   $post URL        POST to URL, keyvals from query string ?k1=v1&k2=v2...")
// result.append("   $group N S       Group results into sets of N using S as glue")
// result.append("   $join S          Join all results to a single string using S as glue")
// result.append("   $jsonpath E      Replace each result R with json.enc(E.find(json.dec(R)))")
// result.append("   $jsonvals E      Replace results with [x for x in E.find(json.dec(R)) for R in results]")

impl<H> Scraper<H>
where
    H: HttpDriver,
{
    pub fn new() -> Scraper<H> {
        Scraper {
            results: Vector::new(),
            headers: HashMap::new(),
            _marker: PhantomData,
        }
    }

    pub fn results(&self) -> &Vector<String> {
        &self.results
    }

    pub fn with_results(self, results: Vector<String>) -> Scraper<H> {
        Scraper { results, ..self }
    }

    pub async fn get(&self, url: &str) -> Result<Scraper<H>, Error> {
        let mut new_results = self.results.clone();

        new_results.push_back(H::get(url, HttpHeaders::Headers(&self.headers)).await?);

        Ok(Scraper::<H> {
            results: new_results,
            ..self.clone()
        })
    }

    pub fn extract(&self, pattern: &str) -> Result<Scraper<H>, Error> {
        let regex = Regex::new(pattern)?;

        Ok(Scraper {
            results: self
                .results
                .iter()
                .flat_map(|str| {
                    regex
                        .captures_iter(str)
                        .filter_map(|matched| {
                            let group = if matched.len() > 1 { 1 } else { 0 };

                            matched.get(group).map(|x| x.as_str().to_owned())
                        })
                        .collect::<Vector<_>>()
                })
                .collect(),
            ..self.clone()
        })
    }

    pub fn delete(&self, pattern: &str) -> Result<Scraper<H>, Error> {
        let regex = Regex::new(pattern)?;

        Ok(Scraper {
            results: self
                .results
                .iter()
                .map(|str| regex.replace_all(str, "").into_owned())
                .collect(),
            ..self.clone()
        })
    }

    pub fn retain(&self, pattern: &str) -> Result<Scraper<H>, Error> {
        let regex = Regex::new(pattern)?;

        let mut results = self.results.clone();
        results.retain(|str| regex.is_match(str));

        Ok(Scraper {
            results,
            ..self.clone()
        })
    }

    pub fn discard(&self, pattern: &str) -> Result<Scraper<H>, Error> {
        let regex = Regex::new(pattern)?;

        let mut results = self.results.clone();
        results.retain(|str| !regex.is_match(str));

        Ok(Scraper {
            results,
            ..self.clone()
        })
    }

    pub fn first(&self) -> Scraper<H> {
        Scraper {
            results: if self.results.is_empty() {
                vector![]
            } else {
                self.results.take(1)
            },
            ..self.clone()
        }
    }

    pub fn last(&self) -> Scraper<H> {
        Scraper {
            results: if self.results.is_empty() {
                vector![]
            } else {
                vector![self.results.back().unwrap().clone()]
            },
            ..self.clone()
        }
    }

    pub fn take(&self, n: usize) -> Scraper<H> {
        Scraper {
            results: if self.results.is_empty() {
                vector![]
            } else {
                self.results.take(min(n, self.results.len()))
            },
            ..self.clone()
        }
    }

    pub fn drop(&self, n: usize) -> Scraper<H> {
        Scraper {
            results: if self.results.is_empty() {
                vector![]
            } else {
                self.results.skip(min(n, self.results.len()))
            },
            ..self.clone()
        }
    }

    pub fn prepend(&self, prefix: &str) -> Scraper<H> {
        Scraper {
            results: self
                .results
                .iter()
                .map(|str| format!("{prefix}{str}").to_string())
                .collect(),
            ..self.clone()
        }
    }

    pub fn append(&self, suffix: &str) -> Scraper<H> {
        Scraper {
            results: self
                .results
                .iter()
                .map(|str| format!("{str}{suffix}").to_string())
                .collect(),
            ..self.clone()
        }
    }

    pub fn join(&self, separator: &str) -> Scraper<H> {
        Scraper {
            results: if self.results.is_empty() {
                vector![]
            } else {
                vector![self
                    .results
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(separator)]
            },
            ..self.clone()
        }
    }

    pub fn clear(&self) -> Scraper<H> {
        Scraper {
            results: vector![],
            ..self.clone()
        }
    }

    pub fn set_header(&self, key: String, value: String) -> Scraper<H> {
        Scraper {
            headers: self.headers.update(key, value),
            ..self.clone()
        }
    }

    pub fn clear_headers(&self) -> Scraper<H> {
        Scraper {
            headers: HashMap::new(),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nullscraper() -> Scraper<NullHttpDriver> {
        Scraper::<NullHttpDriver>::new()
    }

    fn no_results() -> Vector<String> {
        Vector::<String>::new()
    }

    macro_rules! results {
        ($($result:expr),+$(,)?) => {
            vector![$($result.to_string()),+]
        };
    }

    #[derive(Clone)]
    pub struct HeaderTestingHttpDriver;

    impl HttpDriver for HeaderTestingHttpDriver {
        async fn get(_url: &str, headers: HttpHeaders<'_>) -> Result<String, Error> {
            Ok(match headers {
                HttpHeaders::NoHeaders => "".to_string(),
                HttpHeaders::Headers(map) => map
                    .iter()
                    .map(|(key, value)| format!("[{key}]:[{value}]"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            })
        }
    }

    #[test]
    fn test_extract() {
        let s1 = nullscraper();
        let s2 = nullscraper().with_results(results!["its raining cats and dogs"]);
        let s3 = nullscraper().with_results(results![
            "its raining cats and dogs",
            "dogs will sometimes chase cats",
        ]);

        assert_eq!(s1.extract("rain").unwrap().results, no_results());
        assert_eq!(s2.extract("rain").unwrap().results, results!["rain"]);
        assert_eq!(s3.extract("rain").unwrap().results, results!["rain"]);

        assert_eq!(s1.extract("cat|dog").unwrap().results, no_results());
        assert_eq!(
            s2.extract("cat|dog").unwrap().results,
            results!["cat", "dog"]
        );
        assert_eq!(
            s3.extract("cat|dog").unwrap().results,
            results!["cat", "dog", "dog", "cat"]
        );
        assert_eq!(s3.extract("some(...)").unwrap().results, results!["tim"]);
        assert_eq!(
            s3.extract("cat|dog")
                .unwrap()
                .extract("[cad]")
                .unwrap()
                .results,
            results!["c", "a", "d", "d", "c", "a"]
        );

        assert_eq!(s1.extract("rust").unwrap().results, no_results());
        assert_eq!(s2.extract("rust").unwrap().results, no_results());
        assert_eq!(s3.extract("rust").unwrap().results, no_results());
    }

    #[test]
    fn test_retain() {
        let s1 = nullscraper();
        let s2 = nullscraper().with_results(results![
            "its raining cats and dogs",
            "dogs will sometimes chase cats",
        ]);

        assert_eq!(s1.retain("test").unwrap().results, no_results());
        assert_eq!(s2.retain("cats").unwrap().results, s2.results);
        assert_eq!(s2.retain("dogs").unwrap().results, s2.results);
        assert_eq!(
            s2.retain("rain").unwrap().results,
            results!["its raining cats and dogs"]
        )
    }

    #[test]
    fn test_first() {
        let s1 = nullscraper();
        let s2 = nullscraper().with_results(results!["a"]);
        let s3 = nullscraper().with_results(results!["a", "b", "c"]);

        assert_eq!(s1.first().results, no_results());
        assert_eq!(s2.first().results, results!["a"]);
        assert_eq!(s3.first().results, results!["a"]);
    }

    #[test]
    fn test_last() {
        let s1 = nullscraper();
        let s2 = nullscraper().with_results(results!["a"]);
        let s3 = nullscraper().with_results(results!["a", "b", "c"]);

        assert_eq!(s1.last().results, no_results());
        assert_eq!(s2.last().results, results!["a"]);
        assert_eq!(s3.last().results, results!["c"]);
    }

    #[test]
    fn test_take() {
        let s1 = nullscraper();
        let s2 = nullscraper().with_results(results!["a"]);
        let s3 = nullscraper().with_results(results!["a", "b", "c"]);

        assert_eq!(s1.take(0).results, no_results());
        assert_eq!(s1.take(1).results, no_results());
        assert_eq!(s2.take(0).results, no_results());
        assert_eq!(s2.take(1).results, results!["a"]);
        assert_eq!(s2.take(7).results, results!["a"]);
        assert_eq!(s3.take(2).results, results!["a", "b"]);
        assert_eq!(s3.take(3).results, results!["a", "b", "c"]);
    }

    #[test]
    fn test_drop() {
        let s1 = nullscraper();
        let s2 = nullscraper().with_results(results!["a"]);
        let s3 = nullscraper().with_results(results!["a", "b", "c"]);

        assert_eq!(s1.drop(0).results, no_results());
        assert_eq!(s1.drop(1).results, no_results());
        assert_eq!(s1.drop(5).results, no_results());
        assert_eq!(s2.drop(0).results, results!["a"]);
        assert_eq!(s2.drop(1).results, no_results());
        assert_eq!(s2.drop(5).results, no_results());
        assert_eq!(s3.drop(0).results, results!["a", "b", "c"]);
        assert_eq!(s3.drop(1).results, results!["b", "c"]);
        assert_eq!(s3.drop(2).results, results!["c"]);
        assert_eq!(s3.drop(3).results, no_results());
        assert_eq!(s3.drop(5).results, no_results());
    }

    #[test]
    fn test_prepend() {
        let s1 = nullscraper();
        let s2 = nullscraper().with_results(results!["a"]);
        let s3 = nullscraper().with_results(results!["a", "b", "c"]);

        assert_eq!(s1.prepend("_").results, no_results());
        assert_eq!(s2.prepend("_").results, results!["_a"]);
        assert_eq!(s3.prepend("_").results, results!["_a", "_b", "_c"]);
    }

    #[test]
    fn test_append() {
        let s1 = nullscraper();
        let s2 = nullscraper().with_results(results!["a"]);
        let s3 = nullscraper().with_results(results!["a", "b", "c"]);

        assert_eq!(s1.append("_").results, no_results());
        assert_eq!(s2.append("_").results, results!["a_"]);
        assert_eq!(s3.append("_").results, results!["a_", "b_", "c_"]);
    }

    #[test]
    fn test_join() {
        let s1 = nullscraper();
        let s2 = nullscraper().with_results(results!["a"]);
        let s3 = nullscraper().with_results(results!["a", "b", "c"]);

        assert_eq!(s1.join(",").results, no_results());
        assert_eq!(s2.join("--").results, results!["a"]);
        assert_eq!(s3.join("~~~").results, results!["a~~~b~~~c"]);
    }

    #[test]
    fn test_clear() {
        let s1 = nullscraper();
        let s2 = nullscraper().with_results(results!["a", "b", "c"]);

        assert_eq!(s1.clear().results, no_results());
        assert_eq!(s2.clear().results, no_results());
    }

    #[tokio::test]
    async fn test_set_header() {
        let scraper = Scraper::<HeaderTestingHttpDriver>::new()
            .get("foo")
            .await
            .unwrap();

        assert_eq!(scraper.results.len(), 1);

        assert!(!scraper
            .results
            .get(0)
            .unwrap()
            .contains("[User-Agent]:[Scrapeycat 1.2.3]"));

        assert!(!scraper
            .results
            .get(0)
            .unwrap()
            .contains("[Accept-Charset]:[utf-8]"));

        let scraper = Scraper::<HeaderTestingHttpDriver>::new()
            .set_header("User-Agent".to_string(), "Scrapeycat 1.2.3".to_string())
            .set_header("Accept-Charset".to_string(), "utf-8".to_string())
            .get("foo")
            .await
            .unwrap();

        assert_eq!(scraper.results.len(), 1);

        assert!(scraper
            .results
            .get(0)
            .unwrap()
            .contains("[User-Agent]:[Scrapeycat 1.2.3]"));

        assert!(scraper
            .results
            .get(0)
            .unwrap()
            .contains("[Accept-Charset]:[utf-8]"));
    }

    #[tokio::test]
    async fn test_clear_headers() {
        let scraper = Scraper::<HeaderTestingHttpDriver>::new()
            .set_header("User-Agent".to_string(), "Scrapeycat 1.2.3".to_string())
            .clear_headers()
            .get("foo")
            .await
            .unwrap();

        assert!(!scraper
            .results
            .get(0)
            .unwrap()
            .contains("[User-Agent]:[Scrapeycat 1.2.3]"));
    }

    #[test]
    fn test_discard() {
        let scraper = nullscraper().with_results(results!["cat", "dog", "puma", "snake", "sheep"]);

        assert_eq!(
            scraper.discard("a").unwrap().results(),
            &results!["dog", "sheep"]
        );
    }
}
