# The `header` command

```haskell
header "Name" "Value"
```

The `header` command appends an HTTP header to the list of headers to include when making
subsequent HTTP requests. The list of headers can be cleared using the
[`clearheaders`](commands-clearheaders.html) command.


## Examples

```haskell
// headers = {}

header "User-Agent" "Scrapeycat"

// headers = {"User-Agent": "Scrapeycat"}
```
