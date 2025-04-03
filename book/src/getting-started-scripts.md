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

```lua
get("https://feeds.bbci.co.uk/news/world/rss.xml")
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

```lua
get("https://feeds.bbci.co.uk/news/world/rss.xml")
extract("(?s)<title>(.+?)</title>")
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

```lua
get("https://feeds.bbci.co.uk/news/world/rss.xml")
extract("(?s)<title>(.+?)</title>")
drop(2)
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


```lua
get("https://feeds.bbci.co.uk/news/world/rss.xml")
extract("(?s)<title>(.+?)</title>")
drop(2)
extract("(?s)CDATA\\[(.+?)\\]\\]")
```

Running the script now produces a neat list of titles:

```
~ $ scrapeycat run bbc

[
    "Fierce protests in ... ",
    ...
]
```

### Doing something useful with the result

While simply producing a list of strings is a valid outcome for many scripts in Scrapeycat,
sometimes we may want a script to produce more of a noticable [effect](./effects.md). For
this example, we'll try sending a desktop notification containing the most recent result. To
achieve this we'll add a `first` command to discard everything except for the first result,
along with a call to the [notify](./effects-notify.md) effect.

```lua
get("https://feeds.bbci.co.uk/news/world/rss.xml")
extract("(?s)<title>(.+?)</title>")
drop(2)
extract("(?s)CDATA\\[(.+?)\\]\\]")
first()
effect("notify", {title="BBC"})
```

Running this final version of the script should produce a desktop notification displaying the
`"BBC"` title along with the text of the first headline as its body.
