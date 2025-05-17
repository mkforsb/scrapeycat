# The `clearHeaders` command

```lua
clearHeaders()
```

The `clearHeaders` command clears all headers previously set using the
[`header`](commands-header.html) command.

## Examples

```lua
header("User-Agent", "Scrapeycat")

-- headers = {"User-Agent": "Scrapeycat"}

clearHeaders()

-- headers = {}
```
