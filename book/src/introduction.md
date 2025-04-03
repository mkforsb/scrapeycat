# Introduction

**Scrapeycat** is a *web scraping* tool suitable for extracting information from regular
web pages fetched over HTTP. Scrapeycat consists of the following parts:

* A simple domain-specific scripting language based on Lua, with full Lua available.
* A command line application for executing scripts.
* A daemon application for continuously executing scripts based on a schedule.

The core workflow in Scrapeycat is centered around working with a list of *results* and
can be described as follows:

* Fetch a web page (or other text-based resource) over HTTP (0 → 1 result)
* Use regular expressions to extract (capture) interesting information (1 → n results)
* Apply filters, selectors, transformers and combinators (n → m results)
* Perform some action (e.g print or send notification) using the final list of results.

The core workflow can be extended by using additional features:

* Multiple HTTP requests can be made in a single script.
* Support for storing and loading variables.
* Variable substitution in strings (e.g URLs, headers).
* Scripts can call other scripts, passing arguments and receiving output.
* **TODO** JSON support

Scrapeycat is implemented in asynchronous Rust using [Tokio](https://tokio.rs/) and
[Reqwest](https://github.com/seanmonstar/reqwest).
