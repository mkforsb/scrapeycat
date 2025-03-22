# The `load` command

```haskell
load variableName
```

The `load` command appends the results stored in the given variable name to the current
list of results.

## Examples

```haskell
// results = ["Alice", "Bob", "Charlie"]

store listOfNames

// results = ["Alice", "Bob", "Charlie"]
// listOfNames = ["Alice", "Bob", "Charlie"]

clear

// results = []
// listOfNames = ["Alice", "Bob", "Charlie"]

load listOfNames

// results = ["Alice", "Bob", "Charlie"]
// listOfNames = ["Alice", "Bob", "Charlie"]
```
