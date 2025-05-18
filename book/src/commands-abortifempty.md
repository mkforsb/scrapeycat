# The `abortIfEmpty` command

```lua
abortIfEmpty()
```

The `abortIfEmpty` aborts further processing without raising an error if the current list of
results is empty.

## Examples


<!-- test {
    "input": "Alice\nBob\nCharlie\n",
    "preamble": "template: get-and-split-by-newline",
    "expect": {
        "effects": []
    }
} -->
```lua
-- results = ["Alice", "Bob", "Charlie"]

extract("Diego")

-- results = []

abortIfEmpty() -- will abort execution

effect("notify", { "will", "be", "skipped" })
```
