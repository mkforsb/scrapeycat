# The `effect` command

```haskell
effect effectname
effect effectname(<Args>)

where:

  <Args>       ::= <Arg>*                          // zero or more
  <Arg>        ::= <SimpleArg> | <KeywordArg>
  <SimpleArg>  ::= "string" | variableName
  <KeywordArg> ::= keyword=<SimpleArg>
```

The `effect` command executes the given (by name) [effect](effects.html), optionally passing
one or more arguments.

## Examples

```haskell
// results = ["Hello, World!"]

effect print                        // writes "Hello, World!\n" to stdout
effect print(end="")                // writes "Hello, World!" to stdout
```

```haskell
// results = []

effect print("Hello, World!")       // writes "Hello, World!\n" to stdout
```
