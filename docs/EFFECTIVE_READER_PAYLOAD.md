# Effective reader payload review

Measured 2026-10-27 with the pinned offline host toolchain:

```sh
cargo run -p yomoku-storage --example measure_effective_reader_payload --locked --offline
```

The reproducible synthetic fixture repeats `日本語。`, stores one Japanese B token
and one punctuation token per repeat, and resolves without overrides or alternates.
It projects the first page at the default 500-character preset. The existing
pagination rules produce the page sizes below; the fixture does not force a split.
The key includes complete resolved chapter boundaries and metadata/provenance.
Vocabulary is absent. All sizes below are bytes of compact `serde_json` output,
the representation already used for durable effective reservations.

| Chapter characters | Segments | Source UTF-8 | Page characters | Frozen key JSON | Page tokens JSON |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | 500 | 3,000 | 1,000 | 139,145 | 85,519 |
| 10,000 | 5,000 | 30,000 | 500 | 1,409,312 | 42,519 |
| 100,000 | 50,000 | 300,000 | 500 | 14,290,979 | 42,519 |

The full key grows with chapter length even when the visible page stays the same.
At 100,000 characters it is about 47.6 times the canonical UTF-8 source size.
Copying this representation into every rendered state, selection and callback
would multiply that cost. Keep this evidence available when defining the narrow
typed effective bridge; its identity representation and lifecycle remain pending.
Full frozen provenance must still authenticate selections and durable completions.

These are synthetic JSON size measurements, not generated Kotlin/JNI wire sizes,
real-book memory measurements or latency benchmarks. No new bridge representation,
storage migration, opaque handle/cache lifecycle or changed completion semantics is
introduced by this review. Real EPUB cost, peak allocation and device timings remain
unmeasured. Re-run the example when effective domain fields change.
