# The `append` command

```lua
append("string")
```

The `append` command appends a given string to the end of each result.

## Examples

<!-- test {
    "input": "Alice\nBob\nCharlie\n",
    "preamble": "template: get-and-split-by-newline",
    "output": ["Alice (busy)", "Bob (busy)", "Charlie (busy)"]
} -->
```lua
-- results = ["Alice", "Bob", "Charlie"]

append(" (busy)")

-- results = ["Alice (busy)", "Bob (busy)", "Charlie (busy)"]
```
