# Using Lua

One major design goal of Scrapeycat is for the tool and the language to feel like a purpose-built
domain-specific system. Scrapeycat's language was originally implemented using a fully custom
parser and interpreter. The language was subsequently reworked to use Lua for parsing and execution,
where the decision was made to maintain state separately from the Lua environment in order for the
language not to simply turn into a web scraping support library for Lua. In practice, this design
choice manifests in the way the commands of the language implicitly operate on hidden, global state.
For example, the `get(url)` command appends the retrieved text to a scraper-internal list of results
rather than simply return the text to the Lua caller.

However, being able to use arbitrary Lua code in scraper scripts can obviously be tremendously
beneficial. As such, the attempt is made to support this by means of providing a couple of commands
specifically designed to enable processing of results using arbitrary Lua code, along with this
document that is intended to demonstrate how to get the most out of using arbitrary Lua in
combination with the Scrapeycat language.

## `map` and `apply`

The `map` and `apply` commands provide the simplest and least clunky facilities for doing results
processing using arbitrary Lua code.

### `map`

The `map(fn)` command takes an arbitrary Lua function `fn` and applies it to each current entry in
the list of results, with `fn` being passed the string value of the entry and returning a string
value:

<!-- test {
    "input": "alice\nbob\ncharlie\n",
    "output": ["__alice__", "__bob__", "__charlie__"]
} -->
```lua
get("https://somedomain.com/names.txt")   -- get newline-separated list of names
extract(".+")                             -- split by newline

map(function(result)                      -- surround each result with double underlines
    return "__" .. result .. "__"
end)
```

### `apply`

The `apply(fn)` command is very similar to `map(fn)`, but operates on the entire list of results at
once instead of processing each individual result separately. As such, the function `fn` is passed
the entire list of current results in the form of a Lua array-like table (entries in slots `[1]`,
`[2]` and so on) and should return a Lua array-like table.

<!-- test {
    "input": "alice\nbob\ncharlie\n",
    "output": ["alice", "bob", "charlie", "hello"]
} -->
```lua
get("https://somedomain.com/names.txt")   -- get newline-separated list of names
extract(".+")                             -- split by newline

apply(function(all_results)               -- add a result
    table.insert(all_results, "hello")
    return all_results                    -- note: you don't have to modify and return the input
end)                                      --       argument, you can return any table
```

## `var` and `list`

The `var` and `list` functions provide read-only access to scraper variables stored using the
`store` command. A scraper variable is a list of zero or more strings, corresponding to a snapshot
of the list of scraper results at the time the variable was stored.

### `var`

The `var(name)` function takes a string `name` and attempts to retrieve the value stored under the
scraper variable of that name. If the variable exists, the `var` function concatenates the list of
strings using a single space as glue between each sequential pair. Attempting to retrieve a
nonexistent variable name is an error.

Example uses for the `var` function include conditionals and arguments passed to sub-scripts and/or
effects:

<!-- test {
    "input": "alice\nbob\ncharlie\n",
    "effects": [
        {
            "name": "notify",
            "args": [ "bob is online!" ]
        },
        {
            "name": "notify",
            "args": [ "alice", "bob", "charlie" ],
            "kwargs": {
                "title": "People Online",
                "body": "alice bob charlie"
            }
        }
    ]
} -->
```lua
get("https://somedomain.com/names.txt")   -- get newline-separated list of names
extract(".+")                             -- split by newline
store("names")                            -- store in variable so we can access using `var`

if var("names"):find("bob") then
    effect("notify", {"bob is online!"})
    effect("notify", {title="People Online", body=var("names")})
end
```

### `list`

The `list(name)` function is very similar to `var(name)`, but skips performing the concatenation
and instead returns an array-like Lua table containing the individual strings separately.

The primary use for the `list` function is for doing conditionals. There is currently no support
for passing lists (array-like Lua tables) as arguments to sub-scripts and/or effects, making the
`list` function ill suited for such contexts. However, you can concatenate the list returned from
`list` (or any list, really) using `table.concat(xs, "separator")`.

<!-- test {
    "input": "alice\nbob\ncharlie\n",
    "effects": [
        {
            "name": "notify",
            "args": [ "alice is online!" ]
        },
        {
            "name": "notify",
            "args": [ "bob is online!" ]
        }
    ]
} -->
```lua
get("https://somedomain.com/names.txt")   -- get newline-separated list of names
extract(".+")                             -- split by newline
store("names")                            -- store in variable so we can access using `list`

friends = {"alice", "bob"}

for _, friend in pairs(friends) do
    for _, online in pairs(list("names")) do
        if friend == online then effect("notify", {friend .. " is online!"}) end
    end
end
```
