# Pinned Sudachi resources

- Engine: [Sudachi.rs v0.7.0](https://github.com/WorksApplications/sudachi.rs/tree/5de0410b6edfd428fc510cc9763440772e9619d5),
  commit `5de0410b6edfd428fc510cc9763440772e9619d5`, Apache-2.0.
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

`Analyzer::open_full` independently verifies the dictionary bytes before parsing;
an arbitrary compatible dictionary cannot silently acquire this artifact identity.
Host fixtures use the actual Full artifact. Android packaging, attribution display
and offline/device memory/startup/tokenization measurements remain unchecked.
When packaging, distribute the complete notices with the dictionary and include
app-accessible attribution. Do not substitute Core/Small or a moving latest release.
