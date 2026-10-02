#!/usr/bin/env python3
"""Check required version metadata in an Apple archive or exported IPA."""
import argparse
import plistlib
import re
import zipfile
from pathlib import Path


def verify(name, data):
    info = plistlib.loads(data)
    for key in ("CFBundleShortVersionString", "CFBundleVersion"):
        value = info.get(key)
        if not isinstance(value, str) or not re.fullmatch(r"[0-9]+(?:\.[0-9]+){0,2}", value):
            raise SystemExit(f"{name}: missing or invalid {key}: {value!r}")
    print(f"{name}: {info['CFBundleShortVersionString']} ({info['CFBundleVersion']})")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package", type=Path)
    package = parser.parse_args().package
    if package.suffix == ".ipa":
        with zipfile.ZipFile(package) as archive:
            names = [name for name in archive.namelist() if name.startswith("Payload/")
                     and name.endswith((".app/Info.plist", ".framework/Info.plist"))]
            if not names:
                raise SystemExit("No app Info.plist found in IPA")
            for name in sorted(names):
                verify(name, archive.read(name))
    else:
        apps = list((package / "Products/Applications").glob("*.app"))
        if not apps:
            raise SystemExit("No application found in archive")
        for app in apps:
            for bundle in [app, *sorted((app / "Frameworks").glob("*.framework"))]:
                verify(str(bundle), (bundle / "Info.plist").read_bytes())


if __name__ == "__main__":
    main()
