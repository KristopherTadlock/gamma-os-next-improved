#!/usr/bin/env python3
"""Writes a device pack (pack.json from `thimsy-installer dev pack`) into the
catalog: the shared system artifact in catalog.yaml and the device's
artifacts, build steps and expected output hash in devices/<id>.yaml.

  apply_pack.py [--bundle BUNDLE_JSON] [--replace-others] PACK_JSON DEVICE_YAML CATALOG_YAML PROFILE RELEASE_TAG [EXPECT_SHA256]

Download URLs point at the installer release RELEASE_TAG on GitHub.

With --bundle (the entry `thimsy-installer dev bundle` printed), the device
pack is published as that one archive: the device's `bundles:` gets it, and
the system image and patches are listed by hash only (read from inside it).

Without --bundle, the profile's own old bundle is removed (the engine would
otherwise still want that archive instead of the new files).

Per-profile entries (`expect:` and `bundles:` in the device file, `systems:`
in the catalog) are merged: PROFILE's entry is set or added and the other
profiles' entries are kept. The device's artifacts and build steps are
shared by every profile and are replaced, so when they change and another
profile has a bundle or an expected hash (made with the old ones), the pack
is refused: those entries would no longer build. --replace-others applies it
anyway and removes the other profiles' bundle, expect and systems entries.
"""
import hashlib, json, os, re, sys

PART = 1_000_000_000  # GitHub release assets must be under 2 GiB


def read(path):
    with open(path, encoding="utf-8") as f:
        return f.read()


def write(path, text):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)


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


def scalar(v):
    """A YAML scalar: JSON strings and numbers are valid YAML. Non-ASCII text
    is written as is (YAML can't read JSON's surrogate-pair escapes)."""
    return json.dumps(v, ensure_ascii=False)


def flow(d):
    return "{ " + ", ".join(f"{k}: {scalar(v)}" for k, v in d.items()) + " }"


def yflow(v):
    """YAML flow style for nested dicts and lists."""
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{k}: {yflow(x)}" for k, x in v.items()) + " }"
    if isinstance(v, list):
        return "[" + ", ".join(yflow(x) for x in v) + "]"
    return scalar(v)


def split_flow(body):
    """Splits the inside of a flow mapping into (key, value text) pairs at
    top-level commas (not inside quotes, brackets or braces)."""
    items, depth, quote, start, i = [], 0, None, 0, 0
    while i < len(body):
        c = body[i]
        if quote:
            if c == "\\" and quote == '"':
                i += 1
            elif c == quote:
                quote = None
        elif c in "\"'":
            quote = c
        elif c in "{[":
            depth += 1
        elif c in "}]":
            depth -= 1
        elif c == "," and depth == 0:
            items.append(body[start:i])
            start = i + 1
        i += 1
    items.append(body[start:])
    out = []
    for item in items:
        item = item.strip()
        if item:
            k, _, v = item.partition(":")
            out.append((k.strip().strip("\"'"), v.strip()))
    return out


def insert_before(text, block, before):
    b = re.search(before, text, flags=re.M) if before else None
    if b:
        return text[:b.start()] + block + text[b.start():]
    return text.rstrip("\n") + "\n" + block


def set_entry(text, name, key, value, before=None):
    """Sets `key: value` in the top-level mapping `name:`, written in flow
    style (`name: { a: 1 }`) or block style (`name:` then `  a: 1` lines).
    The other keys are kept. The key, or the whole mapping, is added when
    missing (before the first line matching `before`, else at the end)."""
    m = re.search(rf"^{re.escape(name)}:[ \t]*\{{(.*)\}}[ \t]*$", text, flags=re.M)
    if m:
        items = dict(split_flow(m.group(1)))
        items[key] = value
        line = f"{name}: {{ " + ", ".join(f"{k}: {v}" for k, v in items.items()) + " }"
        return text[:m.start()] + line + text[m.end():]
    m = re.search(rf"^{re.escape(name)}:[ \t]*\n((?:[ \t]+\S.*\n|[ \t]*\n(?=[ \t]+\S))*)",
                  text, flags=re.M)
    if m and m.group(1).strip():
        body = m.group(1)
        new = f"  {key}: {value}\n"
        # The entry's line and any more-indented lines under it.
        entry = re.compile(rf"^  {re.escape(key)}:.*\n(?:   +\S.*\n)*", flags=re.M)
        if entry.search(body):
            body = entry.sub(lambda _: new, body, count=1)
        else:
            body = body.rstrip("\n") + "\n" + new
        return text[:m.start(1)] + body + text[m.end(1):]
    m = re.search(rf"^{re.escape(name)}:.*\n?", text, flags=re.M)
    if m:  # empty, or a placeholder value
        return text[:m.start()] + f"{name}:\n  {key}: {value}\n" + text[m.end():]
    return insert_before(text, f"{name}:\n  {key}: {value}\n", before)


def remove_entry(text, name, key):
    """Removes `key` from the top-level mapping `name:` (flow or block
    style); removes the mapping when nothing is left in it."""
    m = re.search(rf"^{re.escape(name)}:[ \t]*\{{(.*)\}}[ \t]*\n?", text, flags=re.M)
    if m:
        items = [(k, v) for k, v in split_flow(m.group(1)) if k != key]
        if not items:
            return text[:m.start()] + text[m.end():]
        line = f"{name}: {{ " + ", ".join(f"{k}: {v}" for k, v in items) + " }\n"
        return text[:m.start()] + line + text[m.end():]
    m = re.search(rf"^{re.escape(name)}:[ \t]*\n((?:[ \t]+\S.*\n|[ \t]*\n(?=[ \t]+\S))*)",
                  text, flags=re.M)
    if m and m.group(1).strip():
        entry = re.compile(rf"^  {re.escape(key)}:.*\n(?:   +\S.*\n)*", flags=re.M)
        body = entry.sub("", m.group(1), count=1)
        if not body.strip():
            return text[:m.start()] + text[m.end():]
        return text[:m.start(1)] + body + text[m.end(1):]
    return text


def entry_keys(text, name):
    """The keys of the top-level mapping `name:` (flow or block style)."""
    m = re.search(rf"^{re.escape(name)}:[ \t]*\{{(.*)\}}[ \t]*$", text, flags=re.M)
    if m:
        return [k for k, _ in split_flow(m.group(1))]
    m = re.search(rf"^{re.escape(name)}:[ \t]*\n((?:[ \t]+\S.*\n|[ \t]*\n(?=[ \t]+\S))*)",
                  text, flags=re.M)
    if m:
        return re.findall(r"^  ([^\s:#][^:]*):", m.group(1), flags=re.M)
    return []


def parse_scalar(v):
    try:
        return json.loads(v)
    except ValueError:
        return v.strip("'\"")


def list_items(text, name, drop=()):
    """The flow-mapping items of the top-level list `name:`, as dicts of
    parsed scalars (keys in `drop` left out), or [] when it's missing or a
    placeholder."""
    m = re.search(rf"^{re.escape(name)}:[ \t]*\n((?:  - .*\n?)+)", text, flags=re.M)
    if not m:
        return []
    out = []
    for line in m.group(1).splitlines():
        body = line.strip()[2:].strip()
        if body.startswith("{") and body.endswith("}"):
            body = body[1:-1]
        out.append({k: parse_scalar(v) for k, v in split_flow(body) if k not in drop})
    return out


def set_list(text, name, lines, before=None):
    """Replaces the top-level list `name:` (or its `# DEVICE_PACK_...`
    placeholder line) with `lines`; adds it when missing."""
    block = "\n".join(lines) + "\n"
    pat = rf"^{re.escape(name)}: .*# DEVICE_PACK_\w+[ \t]*\n?|^{re.escape(name)}:[ \t]*\n(?:  - .*\n)+"
    if re.search(pat, text, flags=re.M):
        return re.sub(pat, lambda _: block, text, count=1, flags=re.M)
    return insert_before(text, block, before)


def main(argv):
    args = argv[1:]
    bundle = None
    replace_others = False
    while args[:1] in (["--bundle"], ["--replace-others"]):
        if args[0] == "--replace-others":
            replace_others = True
            args = args[1:]
        else:
            if len(args) < 2:
                sys.exit(__doc__)
            bundle = json.loads(read(args[1]))
            args = args[2:]
    if len(args) < 5:
        sys.exit(__doc__)
    pack_json, device_yaml, catalog_yaml, profile, tag = args[:5]
    expect = args[5] if len(args) > 5 else None
    if not re.fullmatch(r"[A-Za-z0-9_-]+", profile):
        sys.exit(f"bad profile name {profile!r}")
    pack = json.loads(read(pack_json))
    base = f"https://github.com/KristopherTadlock/thimsy-os/releases/download/{tag}"

    arts = ["artifacts:"]
    for a in pack["artifacts"]:
        entry = {'id': a['id'], 'file': a['file'], 'size': a['size'], 'sha256': a['sha256']}
        if not bundle:
            entry['urls'] = [base + '/' + a['file']]
        arts.append(f"  - {flow(entry)}")
    build = ["build:"] + [f"  - {flow(op)}" for op in pack["build"]]

    dev = read(device_yaml)
    # The artifacts and build steps are shared: other profiles' bundles and
    # expected hashes were made with the current ones. Compared by content
    # (download addresses aside).
    others = sorted({k for k in entry_keys(dev, "bundles") + entry_keys(dev, "expect") if k != profile})
    new_arts = list_items("\n".join(arts) + "\n", "artifacts", drop=("urls",))
    new_build = list_items("\n".join(build) + "\n", "build")
    changed = (list_items(dev, "artifacts", drop=("urls",)) != new_arts
               or list_items(dev, "build") != new_build)
    if others and changed:
        if not replace_others:
            sys.exit(
                f"refusing: this pack changes {os.path.basename(device_yaml)}'s artifacts or build steps, "
                f"which profile(s) {', '.join(others)} also use (their bundle or expected hash was made "
                "with the current ones and would no longer match). Make all profiles' packs from the same "
                "build, or pass --replace-others to drop those profiles' bundle, expect and systems entries.")
        for o in others:
            dev = remove_entry(dev, "bundles", o)
            dev = remove_entry(dev, "expect", o)
    else:
        others = []
    dev = set_list(dev, "artifacts", arts, before=r"^flash:")
    dev = set_list(dev, "build", build, before=r"^flash:")
    dev = set_entry(dev, "expect", profile, scalar(expect or pack['reference_sha256']),
                    before=r"^flash:")
    if bundle:
        b = {'id': bundle['id'], 'file': bundle['file'], 'size': bundle['size'], 'sha256': bundle['sha256'],
             'note': f"Download {bundle['file']} from the Thimsy Installer release page, then choose it with Use a file I already downloaded."}
        if bundle.get('parts'):
            b['parts'] = [{'id': p['id'], 'file': p['file'], 'size': p['size'], 'sha256': p['sha256'],
                           'urls': [base + '/' + p['file']]} for p in bundle['parts']]
        else:
            b['urls'] = [base + '/' + bundle['file']]
        dev = set_entry(dev, "bundles", profile, yflow(b), before=r"^build:")
    else:
        # Loose files now: an old archive for this profile would still be
        # what the engine asks for.
        dev = remove_entry(dev, "bundles", profile)
    source_sha = scalar(pack["source_sha256"])
    dev = dev.replace('"SOURCE_PAC_SHA256"', source_sha)
    dev = re.sub(r'(container: \{ format: pac, size: \d+, sha256: )"[0-9a-f]{64}"',
                 lambda m: m.group(1) + source_sha, dev)
    write(device_yaml, dev)

    s = pack["system"]
    cat = read(catalog_yaml)
    sys_path = os.path.join(os.path.dirname(pack_json), s["file"])
    if bundle:
        note = "Inside the device pack."
    else:
        note = (f"Download the {s['file']}.0NN files from the installer's test release "
                "and choose their folder under Import downloaded files.")
    block = (
        f"  - id: {s['id']}\n"
        f"    file: {s['file']}\n"
        f"    size: {s['size']}\n"
        f"    sha256: {scalar(s['sha256'])}\n"
        f"    note: {scalar(note)}\n"
        f"    unpack: {{ format: zstd, size: {s['unpack']['size']}, sha256: {scalar(s['unpack']['sha256'])} }}\n"
    )
    if bundle:
        pass  # read from the device pack: no download address of its own
    elif s["size"] > 2 * PART - 100_000_000:
        block += "    parts:\n"
        for name, size, sha in split(sys_path):
            block += (f"      - {{ id: {name}, file: {name}, size: {size}, sha256: {scalar(sha)}, "
                      f"urls: [{scalar(base + '/' + name)}] }}\n")
    else:
        block += f"    urls: [{scalar(base + '/' + s['file'])}]\n"
    pat = rf"^  - id: {re.escape(s['id'])}\n(?:    .*\n)+"
    if re.search(pat, cat, flags=re.M):
        cat = re.sub(pat, lambda _: block, cat, count=1, flags=re.M)
    elif re.search(r"^artifacts:[ \t]*\n", cat, flags=re.M):  # a new system image
        cat = re.sub(r"^artifacts:[ \t]*\n", lambda m: m.group(0) + block, cat, count=1, flags=re.M)
    else:
        cat = cat.rstrip("\n") + "\n\nartifacts:\n" + block
    # Point the profile at this system image (the other profiles keep theirs).
    cat = set_entry(cat, "systems", profile, s['id'], before=r"^artifacts:")
    for o in others:  # --replace-others
        cat = remove_entry(cat, "systems", o)
    write(catalog_yaml, cat)
    print("applied", len(pack["artifacts"]), "artifacts,", len(pack["build"]), "steps")
    if others:
        print("removed the bundle, expect and systems entries of:", ", ".join(others))


if __name__ == "__main__":
    main(sys.argv)
