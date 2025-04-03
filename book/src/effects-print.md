# The `print` effect

```lua
-- print all results joined by spaces, end with newline
effect("print")

-- print all results joined by spaces, no newline at end
effect("print", {end=""})

-- print "hello world\n"
effect("print", {"hello", "world"})

-- print "hello world"
effect("print", {"hello", "world", end=""})

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
| **end** | Define the ending character (default: `"\n"`) |
