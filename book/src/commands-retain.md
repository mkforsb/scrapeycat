# The `retain` command

```lua
retain("regex pattern")
```

The `retain` command takes a regular expression (provided as a string) and removes each
result that does NOT contain a match for the pattern.

## Examples

<!-- test {
    "input": "Alice (busy)\nBob\nCharlie (busy)\n",
    "preamble": "template: get-and-split-by-newline",
    "expect": {
        "output": ["Alice (busy)", "Charlie (busy)"]
    }
} -->
```lua
-- results = ["Alice (busy)", "Bob", "Charlie (busy)"]

retain("busy")

-- results = ["Alice (busy)", "Charlie (busy)"]
```
