#!/usr/bin/env python3
"""Writes a device pack (pack.json from `thimsy-installer dev pack`) into the
catalog: the shared system artifact in catalog.yaml and the device's
artifacts, build steps and expected output hash in devices/<id>.yaml.

  apply_pack.py [--bundle BUNDLE_JSON] PACK_JSON DEVICE_YAML CATALOG_YAML PROFILE RELEASE_TAG [EXPECT_SHA256]

Download URLs point at the installer release RELEASE_TAG on GitHub.

With --bundle (the entry `thimsy-installer dev bundle` printed), the device
pack is published as that one archive: the device's `bundles:` gets it, and
the system image and patches are listed by hash only (read from inside it).
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

args = sys.argv[1:]
bundle = None
if args[:1] == ["--bundle"]:
    bundle = json.load(open(args[1]))
    args = args[2:]
pack_json, device_yaml, catalog_yaml, profile, tag = args[:5]
expect = args[5] if len(args) > 5 else None
pack = json.load(open(pack_json))
base = f"https://github.com/KristopherTadlock/thimsy-os/releases/download/{tag}"

def flow(d):
    return "{ " + ", ".join(f"{k}: {json.dumps(v)}" for k, v in d.items()) + " }"


def yflow(v):
    """YAML flow style for nested dicts and lists (JSON scalars are valid YAML)."""
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{k}: {yflow(x)}" for k, x in v.items()) + " }"
    if isinstance(v, list):
        return "[" + ", ".join(yflow(x) for x in v) + "]"
    return json.dumps(v)

arts = ["artifacts:"]
for a in pack["artifacts"]:
    entry = {'id': a['id'], 'file': a['file'], 'size': a['size'], 'sha256': a['sha256']}
    if not bundle:
        entry['urls'] = [base + '/' + a['file']]
    arts.append(f"  - {flow(entry)}")
build = ["build:"] + [f"  - {flow(op)}" for op in pack["build"]]
exp = f"expect: {{ {profile}: {json.dumps(expect or pack['reference_sha256'])} }}"

dev = open(device_yaml).read()
dev = re.sub(r"^artifacts: .*# DEVICE_PACK_ARTIFACTS$|^artifacts:\n(  - .*\n)+", "\n".join(arts) + "\n", dev, flags=re.M)
dev = re.sub(r"^build: .*# DEVICE_PACK_BUILD$|^build:\n(  - .*\n)+", "\n".join(build) + "\n", dev, flags=re.M)
dev = re.sub(r"^expect: .*$", exp, dev, flags=re.M)
if bundle:
    b = {'id': bundle['id'], 'file': bundle['file'], 'size': bundle['size'], 'sha256': bundle['sha256'],
         'note': f"Download {bundle['file']} from the Thimsy Installer release page, then choose it with Use a file I already downloaded."}
    if bundle.get('parts'):
        b['parts'] = [{'id': p['id'], 'file': p['file'], 'size': p['size'], 'sha256': p['sha256'],
                       'urls': [base + '/' + p['file']]} for p in bundle['parts']]
    else:
        b['urls'] = [base + '/' + bundle['file']]
    block = f"bundles:\n  {profile}: {yflow(b)}\n"
    if re.search(r"^bundles:\n(  .*\n)+", dev, flags=re.M):
        dev = re.sub(r"^bundles:\n(  .*\n)+", lambda _: block, dev, flags=re.M)
    else:
        dev = re.sub(r"^build:", lambda m: block + m.group(0), dev, count=1, flags=re.M)
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
    + ("    note: \"Inside the device pack.\"\n" if bundle else
       f"    note: \"Download the {s['file']}.0NN files from the installer's test release and choose their folder under Import downloaded files.\"\n")
    + f"    unpack: {{ format: zstd, size: {s['unpack']['size']}, sha256: \"{s['unpack']['sha256']}\" }}\n"
)
if bundle:
    pass  # read from the device pack: no download address of its own
elif s["size"] > 2 * PART - 100_000_000:
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
