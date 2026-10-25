#!/usr/bin/env python3
"""Build a pinned, read-only JMdict English/JMnedict SQLite index (stdlib only).

Default preparation is offline. --download explicitly fetches the pinned inputs;
upstream URLs roll daily, so newer bytes fail closed until the lock is reviewed.
No Android build or application database is changed by this tool.
"""

import argparse
from contextlib import closing
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import sqlite3
import sys
import tempfile
import urllib.request
import xml.etree.ElementTree as ET


REPO = Path(__file__).resolve().parent.parent
NOTICES = REPO / "third_party/dictionaries"
LOCK_PATH = NOTICES / "sources.lock.json"
LOCAL = REPO / ".local/dictionaries"
XML_LANG = "{http://www.w3.org/XML/1998/namespace}lang"
SCHEMA_VERSION = 1
APPLICATION_ID = 0x594F4449  # YODI; separate from the mutable user database.
LOOKUP_SQL = """
SELECT e.source, e.entry_id, e.payload
FROM forms f JOIN entries e USING (source, entry_id)
WHERE f.surface = ? COLLATE BINARY
ORDER BY e.source, e.entry_id
"""


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def checksum(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify(path, expected):
    if checksum(path) != expected:
        raise ValueError(f"SHA-256 mismatch: {path.name}; pinned inputs required")


def children(element, allowed):
    if element.attrib or any(child.tag not in allowed for child in element):
        raise ValueError(f"Unsupported XML structure: {element.tag}")


def text(element):
    if len(element) or not element.text or not element.text.strip():
        raise ValueError(f"Missing/scalar text required: {element.tag}")
    return element.text


def values(element, tag):
    result = []
    for child in element.findall(tag):
        if child.attrib:
            raise ValueError(f"Unsupported XML attributes: {tag}")
        result.append(text(child))
    return result


def one(element, tag):
    result = values(element, tag)
    if len(result) != 1:
        raise ValueError(f"Exactly one {tag} required")
    return result[0]


def lists(element, tags):
    return {tag: found for tag in tags if (found := values(element, tag))}


def translated(element):
    allowed = {XML_LANG, "g_type", "g_gend"}
    if set(element.attrib) - allowed:
        raise ValueError(f"Unsupported translation attributes: {element.tag}")
    result = {"text": text(element), "lang": element.get(XML_LANG, "eng")}
    for key in ("g_type", "g_gend"):
        if key in element.attrib:
            result[key] = element.attrib[key]
    return result


def entry_payload(element, source):
    section = "sense" if source == "JMdict" else "trans"
    children(element, {"ent_seq", "k_ele", "r_ele", section})
    entry_id = int(one(element, "ent_seq"))
    if not 0 < entry_id < 2**63:
        raise ValueError("Entry ID outside positive i64")
    kanji = []
    for item in element.findall("k_ele"):
        children(item, {"keb", "ke_inf", "ke_pri"})
        kanji.append({"text": one(item, "keb"), **lists(item, ("ke_inf", "ke_pri"))})
    readings = []
    for item in element.findall("r_ele"):
        children(item, {"reb", "re_nokanji", "re_restr", "re_inf", "re_pri"})
        reading = {"text": one(item, "reb"), **lists(item, ("re_restr", "re_inf", "re_pri"))}
        markers = item.findall("re_nokanji")
        if markers:
            if len(markers) != 1 or markers[0].attrib or len(markers[0]) or markers[0].text:
                raise ValueError("Invalid re_nokanji marker")
            reading["no_kanji"] = True
        if set(reading.get("re_restr", [])) - {k["text"] for k in kanji}:
            raise ValueError("Unknown reading spelling restriction")
        readings.append(reading)
    if not readings:
        raise ValueError("At least one reading required")
    result = {"kanji": kanji, "readings": readings}
    sections = []
    inherited_pos = []
    for item in element.findall(section):
        if section == "sense":
            tags = ("stagk", "stagr", "pos", "xref", "ant", "field", "misc", "s_inf", "dial")
            children(item, {*tags, "gloss", "lsource"})
            sense = lists(item, tags)
            inherited_pos = sense.get("pos", inherited_pos)
            if inherited_pos:
                sense["pos"] = inherited_pos
            # Apply inheritance before filtering languages: intervening senses count.
            glosses = [translated(g) for g in item.findall("gloss")]
            sense["glosses"] = [g for g in glosses if g["lang"] == "eng"]
            loans = []
            for loan in item.findall("lsource"):
                if len(loan) or set(loan.attrib) - {XML_LANG, "ls_type", "ls_wasei"}:
                    raise ValueError("Unsupported loan source")
                loans.append({"text": loan.text or "", "lang": loan.get(XML_LANG, "eng"),
                              "type": loan.get("ls_type", "full"),
                              "wasei": loan.get("ls_wasei") == "y"})
            if loans:
                sense["loan_sources"] = loans
            for tag, valid in (("stagk", kanji), ("stagr", readings)):
                if set(sense.get(tag, [])) - {v["text"] for v in valid}:
                    raise ValueError(f"Unknown sense restriction: {tag}")
            if sense["glosses"]:
                sections.append(sense)
        else:
            children(item, {"name_type", "xref", "trans_det"})
            details = [translated(t) for t in item.findall("trans_det")]
            english = [t for t in details if t["lang"] == "eng"]
            if english:
                sections.append({**lists(item, ("name_type", "xref")), "translations": english})
    if not element.findall(section):
        raise ValueError(f"Missing {section}")
    result["senses" if section == "sense" else "names"] = sections
    return entry_id, result


def import_source(db, path, source, spec):
    count = 0
    with gzip.open(path, "rb") as stream:
        header = stream.read(100_000)
        expected = f"<!-- {source} created: {spec['date']} -->".encode()
        if expected not in header:
            raise ValueError(f"Missing expected {source} creation date")
        stream.seek(0)
        events = ET.iterparse(stream, events=("start", "end"))
        _, root = next(events)
        if root.tag != source or root.attrib:
            raise ValueError(f"Unexpected XML root for {source}")
        for event, element in events:
            if event == "end" and element.tag == "entry":
                entry_id, payload = entry_payload(element, source)
                if payload.get("senses", payload.get("names")):
                    db.execute("INSERT INTO entries VALUES (?, ?, ?)",
                               (source, entry_id, canonical(payload)))
                    forms = sorted({v["text"] for v in payload["kanji"] + payload["readings"]})
                    db.executemany("INSERT INTO forms VALUES (?, ?, ?)",
                                   ((form, source, entry_id) for form in forms))
                count += 1
                root.remove(element)  # Bound memory, including empty entry shells.
            elif event == "end" and element is not root and element in root:
                raise ValueError(f"Unexpected root child: {element.tag}")
    if count != spec["entries"]:
        raise ValueError(f"{source} entry count {count} != pinned {spec['entries']}")
    return count


def lookup(db, surface):
    """Exact candidate inspection only; preserve restrictions for runtime selection."""
    return [{"source": source, "id": entry_id, **json.loads(payload)}
            for source, entry_id, payload in db.execute(LOOKUP_SQL, (surface,))]


def create_index(path, sources, lock):
    with closing(sqlite3.connect(path)) as db, db:
        db.executescript(f"""
            PRAGMA page_size = 4096;
            PRAGMA foreign_keys = ON;
            PRAGMA application_id = {APPLICATION_ID};
            PRAGMA user_version = {SCHEMA_VERSION};
            CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
            CREATE TABLE entries (
                source TEXT NOT NULL CHECK (source IN ('JMdict', 'JMnedict')),
                entry_id INTEGER NOT NULL CHECK (entry_id > 0), payload TEXT NOT NULL,
                PRIMARY KEY (source, entry_id)
            ) WITHOUT ROWID;
            CREATE TABLE forms (
                surface TEXT COLLATE BINARY NOT NULL, source TEXT NOT NULL,
                entry_id INTEGER NOT NULL, PRIMARY KEY (surface, source, entry_id),
                FOREIGN KEY (source, entry_id) REFERENCES entries (source, entry_id)
            ) WITHOUT ROWID;
        """)
        db.execute("INSERT INTO metadata VALUES ('sources', ?)", (canonical(lock),))
        for source, spec in sorted(lock["sources"].items()):
            import_source(db, sources / spec["filename"], source, spec)
        db.commit()
        if db.execute("PRAGMA integrity_check").fetchall() != [("ok",)]:
            raise ValueError("Dictionary index integrity check failed")
        if db.execute("PRAGMA foreign_key_check").fetchall():
            raise ValueError("Dictionary index foreign-key check failed")
        db.execute("VACUUM")


def file_hashes(directory):
    return {p.relative_to(directory).as_posix(): checksum(p)
            for p in sorted(directory.rglob("*")) if p.is_file()}


def prepare(sources, destination, lock):
    """Stage/verify/build before atomic publish; never overwrite published data."""
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="prepare-", dir=destination.parent) as temporary:
        staged = Path(temporary) / "prepared"
        staged.mkdir()
        upstream = staged / "upstream"
        upstream.mkdir()
        for spec in lock["sources"].values():
            target = upstream / spec["filename"]
            shutil.copyfile(sources / spec["filename"], target)
            verify(target, spec["sha256"])
        notices = staged / "notices"
        notices.mkdir()
        for name, expected in lock["notices"].items():
            target = notices / name
            shutil.copyfile(NOTICES / name, target)
            verify(target, expected)
        (staged / "sources.lock.json").write_text(canonical(lock) + "\n", encoding="utf-8")
        create_index(staged / "dictionary.sqlite3", upstream, lock)
        build = {"schema_version": SCHEMA_VERSION, "sqlite_version": sqlite3.sqlite_version,
                 "python_version": sys.version.split()[0],
                 "builder_sha256": checksum(Path(__file__)), "files": file_hashes(staged)}
        (staged / "build.json").write_text(canonical(build) + "\n", encoding="utf-8")
        if destination.exists():
            if file_hashes(destination) != file_hashes(staged):
                raise ValueError("Published output differs; use a fresh --output path")
        else:
            for path in staged.rglob("*"):
                if path.is_file():
                    path.chmod(0o444)
            staged.rename(destination)
    return destination


def download(sources, lock):
    sources.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="download-", dir=sources) as temporary:
        for spec in lock["sources"].values():
            target = sources / spec["filename"]
            if target.exists():
                verify(target, spec["sha256"])
                continue
            staged = Path(temporary) / spec["filename"]
            with urllib.request.urlopen(spec["url"], timeout=120) as response:
                with staged.open("wb") as output:
                    shutil.copyfileobj(response, output)
            verify(staged, spec["sha256"])
            staged.rename(target)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sources", type=Path, default=LOCAL / "upstream")
    parser.add_argument("--output", type=Path, default=LOCAL / "index-v1-20261025")
    parser.add_argument("--download", action="store_true", help="explicitly fetch pinned inputs")
    parser.add_argument("--lookup", help="inspect exact candidates in an already built index")
    args = parser.parse_args()
    if args.lookup is not None:
        uri = (args.output.resolve() / "dictionary.sqlite3").as_uri() + "?mode=ro"
        with closing(sqlite3.connect(uri, uri=True)) as db:
            print(json.dumps(lookup(db, args.lookup), ensure_ascii=False, indent=2))
        return
    lock = json.loads(LOCK_PATH.read_text(encoding="utf-8"))
    if args.download:
        download(args.sources, lock)
    print(f"Prepared offline dictionaries: {prepare(args.sources, args.output, lock)}")


if __name__ == "__main__":
    main()
