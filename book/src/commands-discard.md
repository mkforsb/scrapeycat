# The `discard` command

```haskell
discard "regex pattern"
```

The `discard` command takes a regular expression (provided as a string) and removes each
result that contains a match for the pattern.

## Examples

```haskell
// results = ["Alice (busy)", "Bob", "Charlie (busy)"]

discard "busy"

// results = ["Bob"]
```
