# The `run` command

```haskell
run scriptname
run scriptname(<Args>)

where:

  <Args>       ::= <Arg>*                          // zero or more
  <Arg>        ::= <SimpleArg> | <KeywordArg>
  <SimpleArg>  ::= "string" | variableName
  <KeywordArg> ::= keyword=<SimpleArg>
```

The `run` command executes the given (by name) script, optionally passing one or more arguments,
and appends its results (if any) to the current list of results.

If arguments are provided, they become available in the called script according to the 
following table:

| Argument type | Variable name in called script |
| ------------- | ------------------------------ |
| Non-keyword   | `1`, `2`, ...                  |
| Keyword       | Same name as keyword           |

## Examples

```haskell
// results = []

run temperature(location="Sweden/Stockholm")

// results = ["11 °C"]
```
