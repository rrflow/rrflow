"""Write a system's engram configuration for `install plan --configuration`.

A system with declared engine settings gets a copy of the bundled defaults with those
`[storage]` values replaced, and this prints its path. Installation seals that file, and an
installed configuration cannot be edited afterwards. Otherwise this prints nothing and the
system runs on the defaults.
"""
import json
import re
import sys

manifest, system, defaults, output = sys.argv[1:5]
binding = next(entry for entry in json.load(open(manifest))["systems"] if entry["id"] == system)
settings = binding.get("engine_settings", {})
if settings:
    text = open(defaults, encoding="utf-8").read()
    for name, value in settings.items():
        text, count = re.subn(rf"^{re.escape(name)} = \d+$", f"{name} = {value}", text, flags=re.M)
        if count != 1:
            sys.exit(f"{system}: engine setting {name} matched {count} configuration lines")
    open(output, "w", encoding="utf-8", newline="\n").write(text)
    print(output)
