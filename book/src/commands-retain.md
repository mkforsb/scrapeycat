# The `retain` command

```haskell
retain "regex pattern"
```

The `retain` command takes a regular expression (provided as a string) and removes each
result that does NOT contain a match for the pattern.

## Examples

```haskell
// results = ["Alice (busy)", "Bob", "Charlie (busy)"]

retain "busy"

// results = ["Alice (busy)", "Charlie (busy)"]
```
