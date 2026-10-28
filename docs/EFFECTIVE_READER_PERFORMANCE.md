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

Real EPUB/image-heavy books, large Libraries, peak allocations, mapped-loader
performance, JNI/Android/FIFO latency, guarded on-demand generation, process death
and physical arm64 remain unmeasured. The coordinated live-reader/Library switch
and Android override controls remain pending. No automatic cache or new data
semantics is justified by this one host corpus.
