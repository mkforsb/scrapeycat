# The `notify` effect

```lua
-- send notification containing current list of results
effect("notify")

-- send notification containing text "hello wonderful world"
effect("notify", {"hello wonderful", "world"})

-- send notification containing current list of results, with title "foo"
effect("notify", {title="foo"})

-- send notification titled "foo" with text "bar"
effect("notify", {title="foo", body="bar"})

-- all available keyword args, using the value of variable $headline as text body
effect("notify", {title="News", body=var("$headline"), appname="MyScript", icon="/tmp/image.png", sound="/tmp/sound.wav"})
```

### Arguments
Given one or more non-keyword arguments, `notify` will produce a desktop notification containing
the text of all of the arguments with each sequential pair of arguments separated by a single
space.

Given no non-keyword arguments, `notify` will use the current list of results as arguments.


### Keyword arguments
| Name        | Description                                  |
| ----------- | -------------------------------------------- |
| **title**   | Notification title.                          |
| **body**    | Notification text.                           |
| **appname** | Application name for notification grouping.  |
| **icon**    | Path to icon file, or name of system icon.   |
| **sound**   | Path to sound file, or name of system sound. |
