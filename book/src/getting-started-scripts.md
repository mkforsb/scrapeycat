# Writing and Running Scripts

As a first example we'll write a script to fetch world news headlines from an RSS feed available
from the BBC.

### Creating the script file

First we create a file for our script, named `bbc.scrape`, and open it in our editor.

```
~ $ vim bbc.scrape
```

### Running the script

We can run our script by issuing the terminal command `scrapeycat run bbc`. As the script is
currently empty, running it will produce an empty list of results:

```
~ $ scrapeycat run bbc

[]
```

### Fetching the feed

We start our script by fetching the RSS feed:

```haskell
get "https://feeds.bbci.co.uk/news/world/rss.xml"
```

Running the script produces the full XML document as a single result:

```
~ $ scrapeycat run bbc

[
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss xmlns ... "
]
```

### Extracting titles

Next we add an `extract` command to our script to grab content inside of `<title>` tags:

```haskell
get "https://feeds.bbci.co.uk/news/world/rss.xml"
extract "(?s)<title>(.+?)</title>"
```

Running the script now produces a list of many results:

```
~ $ scrapeycat run bbc

[
    "<![CDATA[BBC News]]>",
    "BBC News",
    "<![CDATA[Fierce protests in ... ",
    ...
]
```

### Dropping unwanted results

It seems like the first two results never contain any headlines, so we remove them with a `drop`
command:

```haskell
get "https://feeds.bbci.co.uk/news/world/rss.xml"
extract "(?s)<title>(.+?)</title>"
drop 2
```

Running the script confirms we no longer see the two unwanted results:

```
~ $ scrapeycat run bbc

[
    "<![CDATA[Fierce protests in ... ",
    ...
]
```

### Removing XML syntax

Finally we want to get rid of the `<![CDATA[..]]>` XML syntax. We could add two `delete` commands
to remove the leading and trailing parts of the syntax respectively, but we can also simply use
another `extract` command with a capture group to achieve the same effect:


```haskell
get "https://feeds.bbci.co.uk/news/world/rss.xml"
extract "(?s)<title>(.+?)</title>"
drop 2
extract "(?s)CDATA\[(.+?)\]\]"
```

Running the script now produces a neat list of titles:

```
~ $ scrapeycat run bbc

[
    "Fierce protests in ... ",
    ...
]
```
