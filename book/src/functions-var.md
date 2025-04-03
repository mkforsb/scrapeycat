# The `var` function

```lua
var("my_variable")
```

The `var` function returns the value of the variable stored using [`store`](./commands-store.md)
under the given name if it exists, or throws a fatal error if the variable does not exist.

If the variable contains multiple results, the return value of `var` is the single string
obtained by concatenating the results using a single space to separate each sequential pair.

## Examples

```lua
-- results = ["Alice", "Bob", "Charlie"]

store("$x")
print(var("$x")) -- writes "Alice Bob Charlie\n" to stdout
```
