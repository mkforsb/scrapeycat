# The `clearheaders` command

```haskell
clearheaders
```

The `clearheaders` command clears all headers previously set using the
[`header`](commands-header.html) command.

## Examples

```haskell
header "User-Agent" "Scrapeycat"

// headers = {"User-Agent": "Scrapeycat"}

clearheaders

// headers = {}
```
