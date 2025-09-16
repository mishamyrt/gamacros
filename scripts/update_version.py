#!/usr/bin/env python3
"""Update version in packages metadata files"""

import re
import sys

CLI_CRATE_MANIFEST = "crates/gamacrosd/Cargo.toml"

TOML_VERSION_RE = r"version = \"(.*)\""

def set_cli_cargo_version(version: str):
    """Update version in CLI Cargo.toml"""
    with open(CLI_CRATE_MANIFEST, "r", encoding="utf-8") as file:
        cargo_toml = file.read()
    cargo_toml = re.sub(TOML_VERSION_RE, f"version = \"{version}\"", cargo_toml)
    with open(CLI_CRATE_MANIFEST, "w", encoding="utf-8") as file:
        file.write(cargo_toml)

if __name__ == "__main__":
    target = sys.argv[1]
    set_cli_cargo_version(target)
