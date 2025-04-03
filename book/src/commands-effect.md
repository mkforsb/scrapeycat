# The `effect` command

```lua
effect("effectname")
effect("effectname", { <Args> })

-- where:
-- 
--   <Args>       ::= <Arg>*                          // zero or more
--   <Arg>        ::= <SimpleArg> | <KeywordArg>
--   <SimpleArg>  ::= "string" | LuaExpression<Output = String>
--   <KeywordArg> ::= keyword=<SimpleArg>
```

The `effect` command executes the given (by name) [effect](effects.html), optionally passing
one or more arguments.

## Examples

```lua
-- results = ["Hello, World!"]

effect("print")                          -- writes "Hello, World!\n" to stdout
effect("print", {end=""})                -- writes "Hello, World!" to stdout
```

```lua
-- Regardless of current list of results.

effect("print", {"Hello, World!"})         -- writes "Hello, World!\n" to stdout
effect("print", {"Hello, World!", end=""}) -- writes "Hello, World!" to stdout
effect("print", {var("$x"), var("$y")})    -- writes variable contents to stdout
```
