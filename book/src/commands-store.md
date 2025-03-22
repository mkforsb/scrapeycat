# The `store` command

```haskell
store variableName
```

The `store` command stores the current list of results under a given variable name.

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
