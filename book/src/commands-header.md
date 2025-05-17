# The `header` command

```lua
header("Name", "Value")
```

The `header` command appends an HTTP header to the list of headers to include when making
subsequent HTTP requests. The list of headers can be cleared using the
[`clearHeaders`](commands-clearheaders.html) command.


## Examples

<!-- test {
    "input": "",
    "postamble": "template: get",
    "expect": {
        "headers": [ "User-Agent: Scrapeycat" ]
    }
} -->
```lua
-- headers = {}

header("User-Agent", "Scrapeycat")

-- headers = {"User-Agent": "Scrapeycat"}
```
