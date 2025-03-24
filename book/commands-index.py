#!/usr/bin/python3

import os

print("# Commands")
print("")

for file in sorted([file for file in os.listdir("src") if "commands-" in file]):
    print("- [`{name}`](./commands-{name}.html)".format(name=file[9:-3]))
