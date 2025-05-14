# The `extract` command

```lua
extract("regex pattern")
```

The `extract` command takes a regular expression (provided as a string) and for each current
result, applies the pattern to the result text, removes the result and appends the list of matches
(or captures) to the list of current results. In other words, the `extract` commands replaces each
result with the list of regex matches/captures of the pattern on the result.

For patterns with no explicit capture groups, the full pattern ("group 0") is used as an implicit
capture group. For patterns with one or more explicit capture groups, group 1 is used. 

## Examples

<!-- test {
    "input": "Temperature: 8.2, 8.0, 7.7, 7.2, 7.1, 7.1",
    "preamble": "template: get",
    "expect": {
        "output": [".2", ".0", ".7", ".2", ".1", ".1"]
    }
} -->
```lua
-- results = ["Temperature: 8.2, 8.0, 7.7, 7.2, 7.1, 7.1"]

extract("\\d+.\\d+")

-- results = ["8.2", "8.0", "7.7", "7.2", "7.1", "7.1"]

extract("\\d+(.\\d+)")

-- results = [".2", ".0", ".7", ".2", ".1", ".1"]
```
