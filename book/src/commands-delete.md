# The `delete` command

```lua
delete("regex pattern")
```

The `delete` command takes a regular expression (provided as a string), searches each current
result for matching regions, and replaces each matching region with the empty string.


## Examples

```lua
-- results = ["Alice", "Bob", "Charlie"]

delete("li.")

-- results = ["Ae", "Bob", "Char"]
```
