# The `prepend` command

```lua
prepend("string")
```

The `prepend` command prepends a given string to the beginning of each result.

## Examples

<!-- test {
    "input": "Alice\nBob\nCharlie\n",
    "preamble": "template: get-and-split-by-newline",
    "output": ["##Alice", "##Bob", "##Charlie"]
} -->
```lua
-- results = ["Alice", "Bob", "Charlie"]

prepend("##")

-- results = ["##Alice", "##Bob", "##Charlie"]
```
