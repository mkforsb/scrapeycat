# The `discard` command

```lua
discard("regex pattern")
```

The `discard` command takes a regular expression (provided as a string) and removes each
result that contains a match for the pattern.

## Examples


<!-- test {
    "input": "Alice (busy)\nBob\nCharlie (busy)\n",
    "preamble": "template: get-and-split-by-newline",
    "expect": {
        "output": ["Bob"]
    }
} -->
```lua
-- results = ["Alice (busy)", "Bob", "Charlie (busy)"]

discard("busy")

-- results = ["Bob"]
```
