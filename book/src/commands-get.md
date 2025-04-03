# The `get` command

```lua
get("https://some/url")
```

The `get` command fetches a web page (or other text-based resource) over HTTP, appending the
text as a new entry in the list of results.

## Examples

```lua
-- results = []

get("<some url>")

-- results = ["<!doctype html ..."]

get("<another url>")

-- results = ["<!doctype html ...", "<html ..."]
```
