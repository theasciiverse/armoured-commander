#!/usr/bin/env python3
"""Convert the original's definition files to JSON for the port.

Runs armcom_defs.py and armcom_vehicle_defs.py from the original source
against a stand-in libtcodpy that only knows the constants and colours
(read from the original's own libtcodpy_local), then writes every
upper-case definition to data/defs.json and data/vehicles.json.

    python3 tools/defs2json.py "../../references/Armoured Commander"
"""

import json
import os
import re
import sys
import types

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "data")


def stub_libtcod(src):
    """A libtcodpy module with the original's integer constants and colours."""
    mod = types.ModuleType("libtcodpy")
    text = open(os.path.join(src, "libtcodpy_local", "__init__.py"), encoding="latin-1").read()
    for name, value in re.findall(r"^([A-Z_0-9]+)\s*=\s*(-?\d+)\s*$", text, re.M):
        setattr(mod, name, int(value))
    for name, r, g, b in re.findall(r"^([a-z_]+)\s*=\s*Color\((\d+),\s*(\d+),\s*(\d+)\)", text, re.M):
        setattr(mod, name, [int(r), int(g), int(b)])
    mod.Color = lambda r, g, b: [r, g, b]
    return mod


def plain(value):
    """Python values to JSON-ready ones; objects become their fields."""
    if isinstance(value, (list, tuple)):
        return [plain(v) for v in value]
    if isinstance(value, dict):
        return {str(k): plain(v) for k, v in value.items()}
    if hasattr(value, "__dict__") and not isinstance(value, types.ModuleType):
        return {k: plain(v) for k, v in vars(value).items()}
    return value


def load(path):
    scope = {"__name__": "defs"}
    exec(compile(open(path, encoding="utf-8").read(), path, "exec"), scope)
    return {
        k: plain(v)
        for k, v in scope.items()
        if k.isupper() and not callable(v) and not isinstance(v, types.ModuleType)
    }


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    src = sys.argv[1]
    sys.modules["libtcodpy"] = stub_libtcod(src)

    defs = load(os.path.join(src, "armcom_defs.py"))
    vehicles = load(os.path.join(src, "armcom_vehicle_defs.py"))
    # each vehicle is [id, (key, value), ...]; keep the stats in order
    vehicles["VEHICLE_TYPES"] = [
        {"id": v[0], "stats": [[k, val] for k, val in v[1:]]} for v in vehicles["VEHICLE_TYPES"]
    ]

    for name, data in (("defs.json", defs), ("vehicles.json", vehicles)):
        with open(os.path.join(OUT, name), "w", encoding="utf-8") as f:
            json.dump(data, f, indent=1, ensure_ascii=False)
            f.write("\n")
        print(name, ", ".join(sorted(data)))


if __name__ == "__main__":
    main()
