# The `jsonPath` command

```lua
jsonPath("expression")
```

The `jsonPath` command takes a [JSONPath](https://datatracker.ietf.org/doc/html/rfc9535) expression
(provided as a string) and for each current result, parses the result as JSON, evaluates the
expression, and replaces the result with the list of matched values.

String values are returned without surrounding quotes. Null values are returned as the string
`"null"`, and booleans and numbers are returned as their string representations.

## Examples

<!-- test {
    "input": "{\"store\": {\"book\": [{\"title\": \"Neuromancer\", \"price\": 9.99}, {\"title\": \"Snow Crash\", \"price\": 14.99}]}}",
    "preamble": "template: get",
    "expect": {
        "output": ["Neuromancer", "Snow Crash"]
    }
} -->
```lua
-- results = ['{"store": {"book": [{"title": "Neuromancer", ...}, {"title": "Snow Crash", ...}]}}']

jsonPath("$.store.book[*].title")

-- results = ["Neuromancer", "Snow Crash"]
```

<!-- test {
    "input": "{\"store\": {\"book\": [{\"title\": \"Neuromancer\", \"price\": 9.99}, {\"title\": \"Snow Crash\", \"price\": 14.99}]}}",
    "preamble": "template: get",
    "expect": {
        "output": ["9.99", "14.99"]
    }
} -->
```lua
-- results = ['{"store": {"book": [{"title": "Neuromancer", "price": 9.99}, ...]}}']

jsonPath("$.store.book[*].price")

-- results = ["9.99", "14.99"]
```
