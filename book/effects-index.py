#!/usr/bin/python3

import os

print("# Effects")
print("")

for file in sorted([file for file in os.listdir("src") if "effects-" in file]):
    print("- [`{name}`](./effects-{name}.html)".format(name=file[8:-3]))
