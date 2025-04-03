# The `apply` command

```lua
---@param list_of_results string[] Current list of results
---@return string[] results Updated list of results
fn = function(list_of_results)
    ...
    return updated_list_of_results
end

apply(fn)
```

```lua
apply(function(list_of_results)
    ...
    return updated_list_of_results
end)
```

The `apply` command applies a Lua function to the current list of results as a whole, taking
the return value to be the new list of results.

## Examples

```lua
-- results = ["Alice", "Bob", "Charlie"]

apply(function(results)
    return { "Hello, ", "world!" }
end)

-- results = ["Hello, ", "world!"]
```
