# Yomoku coding sessions

Start with the issue or change the user reports. Keep the work small and focused;
do not resume deferred features unless requested.

- Inspect `git status` and preserve existing/uncommitted work. Inspect the relevant
  code and reproduce reported bugs when practical before fixing them.
- Consult the relevant sections of `docs/PRODUCT_SPEC.md` when checking product
  behavior. It is the product source of truth. Read technical docs only as needed
  for the affected feature; historical reports are reference material.
- Use `/home/kai/Documents/books/japanese` as the default directory for local
  Japanese book examples. Keep book files outside Git.
- Store finished signed APKs for local testing in the Git-ignored `.local/apks/`
  directory at the repository root. Name signed release APKs
  `yomoku-<version>-arm64-v8a.apk` and `yomoku-<version>-universal.apk`.
  Keep the `-unsigned` suffix on Gradle release APKs before signing. Keep
  associated checksums and signature sidecars in the same directory.
- Use Kotlin/Compose for Android, a pure Rust Crux core, separate Rust SQLite I/O
  and EffectRouter, and a narrow typed FFI. Do not put SQL/platform I/O in core.
- Do not invent product behavior, add excluded features, or change major
  architecture without user approval.
  Ask before adopting libraries with major cost or changing data-destructive
  semantics. Report blockers without expanding into unrelated work.
- Verify dependency APIs against primary documentation before adopting versions.
- Use Fedora-compatible CLI tooling; normal development must not need Android Studio.
- Run focused tests and applicable formatting/lint. Add regression coverage when
  it meaningfully protects the fix. Expand to workspace, cross-ABI, JNI or device
  checks when the affected code warrants them. Use relevant commands from
  `docs/DEVELOPMENT.md` and existing scripts. Never claim unrun checks passed.
- Update documentation only when behavior, architecture, data contracts or setup
  changes. Record consequential technical decisions, not routine edits. Do not
  create roadmaps, handoff files or session logs unless the user requests them.
- After each completed, verified change, commit it automatically with a descriptive
  message. The user has authorized these local commits; do not wait for a request.
  Include only changes belonging to the step, preserve unrelated work, and
  report the commit hash. If verification or committing is blocked, report why.
- Give every commit a concise subject and a body explaining the changes and
  their purpose. Merge commit bodies must also identify the source branch and
  include relevant issue/PR references when available.
- Keep test counts and verification summaries out of commit message bodies;
  report them to the user along with the changes, commit hash and any remaining
  work or blockers. Recommend a device trial when it helps validate the fix.
- Never commit credentials/signing keys. Publishing signed releases requires approval.

Nostr starter data is not supplied: keep configuration empty until provided.
Ask Q188 (Citrine retry queue versus best effort) before implementing forwarding.
These do not block local reader development.
