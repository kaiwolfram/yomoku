# Pinned Sudachi resources

- Engine: [Sudachi.rs v0.7.0](https://github.com/WorksApplications/sudachi.rs/tree/5de0410b6edfd428fc510cc9763440772e9619d5),
  commit `5de0410b6edfd428fc510cc9763440772e9619d5`, Apache-2.0, plus the local
  Android loader patch described below.
- Dictionary: [Full 20261015 release](https://github.com/WorksApplications/SudachiDict/releases/tag/v20261015),
  V1 format compatible with engine 0.7. ZIP size 137,444,817 bytes; dictionary
  330,506,339 bytes. `tools/prepare-sudachi.py` pins the exact URL and SHA-256 of
  both ZIP and dictionary, and retains all notices next to prepared data.
- `LEGAL` and `LICENSE-2.0.txt` here are unmodified copies from that exact archive.
  LEGAL includes UniDic's BSD conditions and the complete NEologd attribution.
  The engine's Apache-2.0 license text is identical to `LICENSE-2.0.txt`.

The large dictionary stays outside Git under ignored `.local/sudachi/`. No build
downloads it implicitly. Prepare explicitly with Python 3.11+:

```sh
python3 tools/prepare-sudachi.py
# Or use an already downloaded pinned archive without network:
python3 tools/prepare-sudachi.py --archive .local/sudachi/sudachi-dictionary-20261015-full.zip
tools/test-sudachi.sh
```

Both `Analyzer::open_full` and the unsafe `Analyzer::open_full_mapped`
independently verify the exact dictionary storage before parsing;
an arbitrary compatible dictionary cannot silently acquire this artifact identity.
Mapped callers must prevent backing-file mutation until analyzer drop. Host
fixtures run against both loaders using the actual Full artifact; never prepare
or replace/truncate the dictionary during those runs. Android builds require prepared data:
Gradle runs `python3 tools/prepare-sudachi.py --android-assets` to verify and stage
all three files under ignored `target/generated/sudachi-assets/sudachi/`, then
includes them as APK assets. No build downloads data. Settings → Sudachi licenses
and notices displays the complete LEGAL and Apache-2.0 license offline.
The dictionary is APK-compressed; no analyzer or private-file copy is loaded at
app startup. Standalone Android owned/mapped memory/startup/tokenization and
offline results are in [SUDACHI_MEASUREMENTS.md](../../docs/SUDACHI_MEASUREMENTS.md).
Private extraction, combined app/lifecycle and physical arm64 measurements remain
pending. Do not substitute Core/Small or a moving latest release.

## Dependency-only engine snapshot and Android patch

`engine/` contains the exact pinned crate's `src/`, upstream `resources/`, README,
LICENSE and manifests. The root Cargo patch selects this local copy; no shared
Cargo cache is modified. The complete local delta is
[`patches/android-loader.patch`](patches/android-loader.patch):

- Include Android in the existing Linux/FreeBSD `lib*.so` plugin-name cfg. This
  fixes upstream E0425; Android was omitted. Embedded plugins and analysis logic
  are unchanged. Yomoku still accepts only embedded defaults, no external plugins.
- Restrict the nested workspace to the library and remove unused dev dependencies
  and benchmark targets. CLI, Python, dynamic plugin packages, upstream integration
  tests/benchmarks and their fixtures are omitted. This snapshot is an application
  dependency; use Yomoku's tests to verify it. Do not run its upstream test targets.

`UPSTREAM_SHA256SUMS` records every included file's **unmodified upstream** hash,
including both original manifests. The verifier reverses the entire patch in a
temporary copy, checks the exact file set and all hashes, and needs only Python
3.11+ and Git. It does not download anything or modify the snapshot/cache:

```sh
python3 tools/verify-sudachi-source.py
cargo check -p yomoku-analysis --lib --target aarch64-linux-android --target x86_64-linux-android --locked --offline
```

Git attributes preserve upstream bytes and patch context without whitespace or
line-ending cleanup; the original source includes trailing spaces. Source
integrity is enforced by the verifier above.
The root build-directory ignore has a narrow exception for `src/dic/build/`,
which is upstream Rust source required to compile the engine.

The engine profile records `+android-loader-v1` so saved snapshots identify this
source patch. Version, dictionaries, configuration and segmentation policy stay
pinned. Android compilation is verified separately from APK bundling/device use.
