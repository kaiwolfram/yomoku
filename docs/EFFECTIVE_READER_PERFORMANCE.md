# Effective reader host measurements

Measured 2026-10-28 with the opt-in `yomoku-analysis` example
`measure_effective_reader`. It accepts existing TXT/EPUB files, imports through
production repositories into a newly created temporary SQLite database, and removes
only that scratch directory on exit. It never opens a supplied user database or
writes input files. Exact duplicate inputs remain one document. No dependency,
migration, wire format, registration policy or live-reader switch is introduced.

## Corpus and method

The real-book input is Aozora Bunko's Shift-JIS ruby TXT of Natsume Soseki's
[*I Am a Cat*](https://www.aozora.gr.jp/cards/000148/card789.html), downloaded from
the card's `789_ruby_5639.zip` link. The corpus stays outside Git.

- ZIP SHA-256: `6545750b89ee2c57f215a65079eeab56ee7c997373b7b86e5b00ae74fe69208f`.
- Extracted TXT SHA-256: `f8c511ab3e69e3a1ccfea475783c2e4d64917a2cdedfb67441ea27655aca5f75`.
- 749,051 original bytes; production TXT parsing produces 329,187 canonical
  characters and 211,016 B tokens. Aozora headers/notes follow the normal parser.
- One document and one chapter: TXT does not infer chapters. Initial database
  size is 14,880,768 bytes. The first logical page contains 922 characters under
  the default 500-character preset's paragraph/sentence rules.
- Fedora x86_64, Linux `7.2.12-200.fc44.x86_64`, AMD Ryzen 5 7533HS;
  Rust `1.99.1`, release optimization, SQLite `3.53.2`, pinned Full 20261015,
  verified **owned** loader. CPU frequency/OS caches are uncontrolled.
- One untimed warmup and seven timed iterations per metric; report minimum /
  median / maximum milliseconds. Outputs are consumed and dropped. Pure resolution
  includes output destruction; page loading includes invalidating/dropping the
  previous retained key and accepting the next snapshot. The final key remains
  retained until that phase ends.
- `b_only` has no alternates/rules; `stored_abc` adds whole-chapter same-profile
  A/C snapshots without edits; `global_split` adds the first aligned alternate
  split as an All-occurrences rule (two parts, zero new registrations here).
- Library metrics scan the complete fixture Library. They include SQLite reads,
  validation, resolution, deduplication and live-status lookup. Page metrics use
  the first page of the largest chapter, including the repository's whole-document
  validation. These are direct host calls, without FIFO scheduling, JNI or Compose.
- Baseline resolver is from `38dab541d31e4637b07b2bdbaf50a7a1007b81d9`.
  Both final executables use the same benchmark workload/reporting; they run
  sequentially without competing builds/tests. Exploratory timings overlapped
  compilation and were discarded. Each phase verifies unchanged database SHA-256;
  before edits, effective and original Library counts must agree.

## Results

| Phase | Metric | Before min / median / max (ms) | After min / median / max (ms) |
| --- | --- | ---: | ---: |
| b_only | resolve_all_chapters | 64.252 / 64.946 / 67.104 | 41.148 / 41.270 / 42.578 |
| b_only | original_library | 72.905 / 75.452 / 76.966 | 76.173 / 76.913 / 77.970 |
| b_only | effective_library | 214.906 / 226.238 / 260.007 | 204.318 / 215.602 / 226.084 |
| b_only | largest_chapter_page_load_accept | 183.393 / 185.920 / 197.692 | 136.397 / 163.422 / 183.364 |
| stored_abc | resolve_all_chapters | 84.086 / 84.837 / 86.291 | 44.105 / 60.639 / 72.442 |
| stored_abc | original_library | 74.274 / 75.478 / 75.829 | 77.148 / 78.110 / 78.400 |
| stored_abc | effective_library | 454.300 / 456.738 / 477.302 | 389.373 / 412.289 / 428.947 |
| stored_abc | largest_chapter_page_load_accept | 405.081 / 406.592 / 432.204 | 351.404 / 369.982 / 383.913 |
| global_split | resolve_all_chapters | 103.104 / 104.473 / 106.480 | 41.823 / 42.288 / 43.788 |
| global_split | original_library | 75.408 / 76.359 / 76.927 | 77.259 / 78.140 / 79.269 |
| global_split | effective_library | 454.296 / 455.940 / 460.671 | 394.479 / 401.287 / 404.957 |
| global_split | largest_chapter_page_load_accept | 407.339 / 409.380 / 412.324 | 379.140 / 382.952 / 385.880 |

The change removes the temporary metadata BTreeMap over every B/A/C token.
Boundary resolution already records exact B indices; use those directly and
binary-search validated sorted A, then C, only for changed spans. Complete snapshot
validation and exact start/end/occurrence provenance remain mandatory. An exhaustive
4,096-partition Unicode regression compares every metadata field to a full-scan
oracle. Existing stale-key, durable replay, eligibility and furigana tests still pass.

B-only pure resolution's median falls about 36%; end-to-end gains are smaller and
variable. These timings establish neither an allocation reduction in bytes nor an
app responsiveness guarantee. On this fixture, stored-A/B/C page reads and effective
Library scans still take roughly 370–410 ms; snapshot I/O/validation remains material.

Single untimed setup observations after the change: verified owned Full loading
509 ms, analyzed TXT import 1,964 ms, whole-chapter A analysis 482 ms plus snapshot
storage 1,113 ms, C analysis 472 ms plus storage 1,116 ms. These do not measure the
guarded FIFO on-demand command or a cold-start distribution.

## Reproduce

From the repository root, with the pinned dictionary already prepared:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
curl --fail --location --silent --show-error \
  https://www.aozora.gr.jp/cards/000148/files/789_ruby_5639.zip \
  --output /tmp/yomoku-aozora-cat.zip
python3 - <<'PY'
from pathlib import Path
from zipfile import ZipFile
from hashlib import sha256
archive = Path('/tmp/yomoku-aozora-cat.zip').read_bytes()
assert sha256(archive).hexdigest() == '6545750b89ee2c57f215a65079eeab56ee7c997373b7b86e5b00ae74fe69208f'
with ZipFile('/tmp/yomoku-aozora-cat.zip') as z:
    names = [name for name in z.namelist() if name.endswith('.txt')]
    assert len(names) == 1
    data = z.read(names[0])
assert sha256(data).hexdigest() == 'f8c511ab3e69e3a1ccfea475783c2e4d64917a2cdedfb67441ea27655aca5f75'
Path('/tmp/yomoku-aozora-cat.txt').write_bytes(data)
PY
cargo run -p yomoku-analysis --example measure_effective_reader --release \
  --locked --offline -- .local/sudachi/full-20261015/system_full.dic \
  7 /tmp/yomoku-aozora-cat.txt
```

Additional TXT/EPUB paths build a multi-document fixture (duplicates are reused).
The largest chapter can be in any document/spine position. An additional synthetic
two-chapter EPUB + TXT + duplicate-TXT smoke run verifies this branch and even
iteration medians; it is not a real-EPUB performance result. Corpus paths/hashes,
canonical counts, page length, engine/dictionary revisions and setup timings go to
stderr; CSV measurements go to stdout. Invalid iteration counts fail before import.

For a baseline comparison, use a separate source copy at the revision above, copy
this example into it and use a separate Cargo target directory. Rebuild and retain
each executable before sequential runs. Do not run compilers/tests during timings.

## Real EPUB and repeated-book Library scaling

The second corpus is IDPF's [Kusamakura EPUB sample](https://idpf.github.io/epub3-samples/30/samples.html),
downloaded unchanged from the [20230704 release](https://github.com/IDPF/epub3-samples/releases/download/20230704/kusamakura-japanese-vertical-writing.epub).
The publisher lists ruby, media overlays, non-ASCII paths and embedded fonts; its
sample collection is CC BY-SA 3.0 unless specified otherwise. The EPUB stays outside
Git. Yomoku uses its normal horizontal, CSS-ignoring extraction and supported assets;
these measurements do not claim support for media overlays or embedded fonts.

- Original archive: 17,940,673 bytes, SHA-256
  `6d4d4ed5eda3f612e3263c54fa0f74ccfa87260f43e81bb58ea658636e52eeb7`.
- One EPUB imports as 16 chapters, 96,296 canonical characters and 62,310 B tokens.
  Initial database: 22,540,288 bytes. The largest chapter has 15,271 characters;
  its first page has 614 characters at the default preset.
- `--epub-copies 100` imports the original plus 99 generated archives through the
  same production EPUB repository. Each changes only the ZIP archive comment,
  keeping all members and canonical content identical while giving exact-byte
  deduplication distinct inputs. The fixture has 100 documents, 1,600 chapters,
  9,629,600 canonical characters, 6,231,000 B tokens and a 2,188,136,448-byte initial
  database. It exercises repeated global spellings, not a diverse 100-book corpus.
- Same host/toolchain/dictionary/owned loader and direct-host method as above;
  one warmup and five measured iterations. Separate before/after executables run
  sequentially without competing builds/tests. Setup and SHA-256 validation are
  outside timings. Database hashing now streams 64 KiB blocks rather than allocating
  a second database-sized buffer. Every timed phase still verifies unchanged bytes.
- The single-book database is on RAM-backed `/tmp`; the 100-book database is on
  the disk-backed workspace under `.local/effective-measure`. Compare before/after
  within each fixture; the two fixture sizes also differ in storage medium/cache
  pressure. Peak memory, CPU frequency and OS caches are uncontrolled.
- Before production code: `ab9ed06c37166f96f6d338caa725982f7870f37e` plus this
  slice's harness. The saved before/after executable SHA-256 values are
  `2c49eab5d8e119a6bd6a097e488defc682865c635579528bd1cdb8f117236044` and
  `21aa76bd0e3d57e6d08ad055921883d4bb6eaba10ccd3c5d6d96a4c2f5489460`.
  The final single-book pair uses those same verified executables; initial
  observations using a pre-existing target artifact are excluded from this table.

Effective Library scan results, minimum / median / maximum milliseconds:

| Documents | Phase | Before (ms) | After (ms) |
| ---: | --- | ---: | ---: |
| 1 | b_only | 59.768 / 59.926 / 60.037 | 61.316 / 61.526 / 62.342 |
| 1 | stored_abc | 119.416 / 120.367 / 120.925 | 121.724 / 122.702 / 131.074 |
| 1 | global_split | 120.395 / 120.686 / 122.488 | 130.966 / 139.310 / 147.132 |
| 100 | b_only | 6375.808 / 6413.813 / 6470.057 | 4239.719 / 4367.281 / 4415.313 |
| 100 | stored_abc | 12607.967 / 12700.945 / 13311.292 | 11007.700 / 11551.966 / 11605.262 |
| 100 | global_split | 12857.390 / 13904.929 / 14321.956 | 10844.141 / 11078.235 / 11317.180 |

The only production change shares a referenced spelling's validated status/absence
across documents within the existing read transaction. The next scan starts empty;
it still validates every chapter and deduplicates each document independently.
The map does not preload unreferenced lifetime vocabulary or survive a failure.
Its memory grows with referenced distinct spellings; peak allocation is unmeasured.

The 100-book effective-scan medians fall about 32% / 9% / 20% across the three
phases. Single-book scans do not improve. Other unchanged metrics also vary:
100-book original-Library medians move 2933/3214/2688→2646/2588/2726 ms, pure
resolution 717/1513/1631→760/1488/1606 ms, and page-load medians stay near
39/51/51 ms. In the single-book global phase, pure resolution rises
14.338→15.602 ms and page-load median 54.767→60.846 ms alongside the scan increase.
These runs do not separate host variation from allocation effects or establish a
universal speedup. Stored A/B/C repository scans remain expensive; no page-load,
Android responsiveness or memory improvement is claimed.

Untimed 100-book setup, before→after: Full loading 409→400 ms, EPUB imports
67.680→67.963 seconds, A analysis 15.894→15.158 seconds plus storage
44.870→43.285 seconds, C analysis 14.244→14.858 seconds plus storage
44.066→44.133 seconds. Setup is separate from guarded FIFO on-demand timing.

The harness accepts `--epub-copies N` for exactly one EPUB (1–1,000). It never
modifies the supplied archive or accepts a user database. The generated files exist
only in memory during import; the disposable database is removed on exit. A unit
fixture verifies distinct deterministic source bytes with identical ordered member
names, compression, checksums and contents, including literal Unicode/NUL data.
Use `TMPDIR` on a filesystem with room for source bytes plus stored B/A/C snapshots
when running larger fixtures; the default `/tmp` may be RAM-backed.

```sh
curl --fail --location --silent --show-error \
  https://github.com/IDPF/epub3-samples/releases/download/20230704/kusamakura-japanese-vertical-writing.epub \
  --output /tmp/yomoku-kusamakura.epub
echo '6d4d4ed5eda3f612e3263c54fa0f74ccfa87260f43e81bb58ea658636e52eeb7  /tmp/yomoku-kusamakura.epub' | sha256sum --check
mkdir -p .local/effective-measure
cargo build -p yomoku-analysis --example measure_effective_reader --release --locked --offline
target/release/examples/measure_effective_reader \
  .local/sudachi/full-20261015/system_full.dic 5 /tmp/yomoku-kusamakura.epub
TMPDIR="$PWD/.local/effective-measure" target/release/examples/measure_effective_reader \
  .local/sudachi/full-20261015/system_full.dic 5 --epub-copies 100 /tmp/yomoku-kusamakura.epub
```

Peak allocations, diverse/image-heavy book Libraries, mapped-loader performance,
JNI/Android/FIFO latency, guarded on-demand generation, process death and physical
arm64 remain unmeasured. The coordinated live-reader/Library switch and Android
override controls remain pending.
