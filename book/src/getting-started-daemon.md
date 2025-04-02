# Running the Daemon

Scrapeycat can run as a *daemon*, a non-interactive background process. In this mode, Scrapeycat
will continuously execute a given set of scripts according to a given schedule. In order to use
the daemon mode, a configuration file must be provided. The configuration is specified in
[TOML](https://toml.io/en/) using [cron](https://en.wikipedia.org/wiki/Cron) syntax to specify
the schedule for each job.

### Example Configuration
```toml
config_version = 1
script_dirs = ["${HOME}/scripts"]
script_names = ["${NAME}.scrape"]

[suites.default]
jobs = [
    { name = "Local Weather", script = "weather", args = [], kwargs = { location = "tokyo" }, schedule = "*/10 * * * *", dedup = false },
    { script = "bbc", schedule = "*/5 * * * *", dedup = true },
]
```

This configuration would instruct Scrapeycat to perform the following:

| Schedule         | Execute script             | Args, Keyword Args             |
| ---------------- | -------------------------- | ------------------------------ |
| Every 10 mins.   | `~/scripts/weather.scrape` | `[]`, `{ location = "tokyo" }` |
| Every 5 mins.    | `~/scripts/bbc.scrape`     | `[]`, `{}`                     |

Additionally, by specifying `dedup = true` for the `bbc` job, any effects (such as notifications)
produced by that job will be deduplicated, meaning any repeated effects will be discarded.

Finally, the `bbc` job demonstrates how several properties may be omitted, namely `name`, `args`,
and `kwargs`.

### Launching the Daemon

With a configuration file saved under `./scrapeycat-daemon.conf`, we could launch a Scrapeycat daemon:
```
$ scrapeycat daemon scrapeycat-daemon.conf
```

Optionally, for verbose debug output, we could add the `--debug` flag:
```
$ scrapeycat daemon scrapeycat-daemon.conf --debug
```
