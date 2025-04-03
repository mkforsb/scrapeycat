# The `map` command

```lua
---@param single_result string An entry from the current list of results
---@return string value Updated results entry
fn = function(single_result)
    ...
    return updated_result
end

map(fn)
```

```lua
map(function(single_result)
    ...
    return updated_result
end)
```

The `map` command applies a Lua function to each result individually, transforming the list
of results.

## Examples

```lua
-- results = ["Alice", "Bob", "Charlie"]

map(function(result)
    return result:lower()
end)

-- results = ["alice", "bob", "charlie"]
```
