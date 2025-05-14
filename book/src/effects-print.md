# The `print` effect

<!-- test {
    "kwargs": { "$x": "value-of-x" },
    "expect": {
        "effects": [
            {
                "name": "print"
            },
            {
                "name": "print",
                "kwargs": { "eol": "" }
            },
            {
                "name": "print",
                "args": [ "hello", "world" ]
            },
            {
                "name": "print",
                "args": [ "hello", "world" ],
                "kwargs": { "eol": "" }
            },
            {
                "name": "print",
                "args": [ "value-of-x" ]
            }
        ]
    }
} -->
```lua
-- print all results joined by spaces, end with newline
effect("print")

-- print all results joined by spaces, no newline at end
effect("print", {eol=""})

-- print "hello world\n"
effect("print", {"hello", "world"})

-- print "hello world"
effect("print", {"hello", "world", eol=""})

-- print value of variable $x
effect("print", { var("$x") })
```

### Arguments
Given one or more non-keyword arguments, `print` will print all of the arguments on a single line
with each pair of sequential arguments separated by a single space.

Given no non-keyword arguments, `print` will use the current list of results as arguments.


### Keyword arguments
| Name    | Description                                   |
| ------- | --------------------------------------------- |
| **eol** | Define the ending character (default: `"\n"`) |
