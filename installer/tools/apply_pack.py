#!/usr/bin/env python3
"""Writes a device pack (pack.json from `thimsy-installer dev pack`) into the
catalog: the shared system artifact in catalog.yaml and the device's
artifacts, build steps and expected output hash in devices/<id>.yaml.

  apply_pack.py PACK_JSON DEVICE_YAML CATALOG_YAML PROFILE RELEASE_TAG [EXPECT_SHA256]

Download URLs point at the installer release RELEASE_TAG on GitHub.
"""
import hashlib, json, os, re, sys

PART = 1_000_000_000  # GitHub release assets must be under 2 GiB


def split(path):
    """Splits path into path.001, .002, ... and returns (name, size, sha256) per part."""
    parts = []
    with open(path, "rb") as f:
        i = 1
        while True:
            chunk = f.read(PART)
            if not chunk:
                break
            name = f"{path}.{i:03d}"
            with open(name, "wb") as o:
                o.write(chunk)
            parts.append((os.path.basename(name), len(chunk), hashlib.sha256(chunk).hexdigest()))
            i += 1
    return parts

pack_json, device_yaml, catalog_yaml, profile, tag = sys.argv[1:6]
expect = sys.argv[6] if len(sys.argv) > 6 else None
pack = json.load(open(pack_json))
base = f"https://github.com/KristopherTadlock/thimsy-os/releases/download/{tag}"

def flow(d):
    return "{ " + ", ".join(f"{k}: {json.dumps(v)}" for k, v in d.items()) + " }"

arts = ["artifacts:"]
for a in pack["artifacts"]:
    arts.append(f"  - {flow({'id': a['id'], 'file': a['file'], 'size': a['size'], 'sha256': a['sha256'], 'urls': [base + '/' + a['file']]})}")
build = ["build:"] + [f"  - {flow(op)}" for op in pack["build"]]
exp = f"expect: {{ {profile}: {json.dumps(expect or pack['reference_sha256'])} }}"

dev = open(device_yaml).read()
dev = re.sub(r"^artifacts: .*# DEVICE_PACK_ARTIFACTS$|^artifacts:\n(  - .*\n)+", "\n".join(arts) + "\n", dev, flags=re.M)
dev = re.sub(r"^build: .*# DEVICE_PACK_BUILD$|^build:\n(  - .*\n)+", "\n".join(build) + "\n", dev, flags=re.M)
dev = re.sub(r"^expect: .*$", exp, dev, flags=re.M)
dev = dev.replace('"SOURCE_PAC_SHA256"', json.dumps(pack["source_sha256"]))
dev = re.sub(r'(container: \{ format: pac, size: \d+, sha256: )"[0-9a-f]{64}"', lambda m: m.group(1) + json.dumps(pack["source_sha256"]), dev)
open(device_yaml, "w").write(dev)

s = pack["system"]
cat = open(catalog_yaml).read()
sys_path = os.path.join(os.path.dirname(pack_json), s["file"])
block = (
    f"  - id: {s['id']}\n"
    f"    file: {s['file']}\n"
    f"    size: {s['size']}\n"
    f"    sha256: \"{s['sha256']}\"\n"
    f"    note: \"Download the {s['file']}.0NN files from the installer's test release and choose their folder under Import downloaded files.\"\n"
    f"    unpack: {{ format: zstd, size: {s['unpack']['size']}, sha256: \"{s['unpack']['sha256']}\" }}\n"
)
if s["size"] > 2 * PART - 100_000_000:
    block += "    parts:\n"
    for name, size, sha in split(sys_path):
        block += f"      - {{ id: {name}, file: {name}, size: {size}, sha256: \"{sha}\", urls: [\"{base}/{name}\"] }}\n"
else:
    block += f"    urls: [\"{base}/{s['file']}\"]\n"
pat = rf"  - id: {re.escape(s['id'])}\n(    .*\n)+"
if re.search(pat, cat):
    cat = re.sub(pat, lambda _: block, cat)
else:  # a new system image: add it to the artifacts list
    cat = re.sub(r"^artifacts:\n", lambda m: m.group(0) + block, cat, count=1, flags=re.M)
# Point the profile at this system image.
cat = re.sub(r"^(systems: *\{[^}]*?)\b%s: *[^,}]+" % re.escape(profile), lambda m: m.group(1) + f"{profile}: {s['id']}", cat, flags=re.M)
cat = re.sub(r"^(systems:\n(?:  .*\n)*?  )%s: .*$" % re.escape(profile), lambda m: m.group(1) + f"{profile}: {s['id']}", cat, flags=re.M)
open(catalog_yaml, "w").write(cat)
print("applied", len(pack["artifacts"]), "artifacts,", len(pack["build"]), "steps")
