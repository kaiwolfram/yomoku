# Yomoku coding sessions

Read `docs/NEXT_SESSION.md` first, then the relevant unchecked tasks in
`docs/IMPLEMENTATION_PLAN.md`. Read `docs/PRODUCT_SPEC.md` for the
requirements governing the slice. It is the product source of truth.

- Inspect `git status` and preserve existing/uncommitted work.
- Implement a small, independently testable slice; continue the plan rather than
  replanning the project. Prefer persistence and reader correctness over styling.
- Use Kotlin/Compose for Android, a pure Rust Crux core, separate Rust SQLite I/O
  and EffectRouter, and a narrow typed FFI. Do not put SQL/platform I/O in core.
- Do not invent product behavior, add excluded features, or change major
  architecture without user approval. Record routine choices in `docs/DECISIONS.md`.
  Ask before adopting libraries with major cost or changing data-destructive
  semantics. If blocked, continue an independent task where possible.
- Verify dependency APIs against primary documentation before adopting versions.
- Use Fedora-compatible CLI tooling; normal development must not need Android Studio.
- Run relevant tests, formatting and lint. Exact current commands are in
  `docs/NEXT_SESSION.md` and `docs/DEVELOPMENT.md`. Never claim unrun checks passed.
- At session end, check off only completed, verified tasks; update
  `docs/NEXT_SESSION.md` with next 1–3 tasks, commands, failures and blockers.
  Keep it brief (ideally under 80 lines), keep architecture/data docs current
  and append a brief session log. Leave incomplete tasks unchecked and record
  the precise continuation point; finish in a buildable state where possible.
- After each completed, verified implementation step, update its progress docs
  and commit the step automatically with a descriptive message. The user has
  authorized these local commits; do not wait for a separate commit request.
  Include only changes belonging to the step, preserve unrelated work, and
  report the commit hash. If verification or committing is blocked, report why.
- Give every commit a concise subject and a body explaining the changes and
  their purpose. Merge commit bodies must also identify the source branch and
  include relevant issue/PR references when available.
- Keep test counts, verification summaries, and build/lint/formatting results out
  of commit message bodies. Record them in session/progress docs and user-facing
  reports instead.
- Report implemented tasks, changed files, test results, next steps and blockers.
- Never commit credentials/signing keys. Publishing signed releases requires approval.

Nostr starter data is not supplied: keep configuration empty until provided.
Ask Q188 (Citrine retry queue versus best effort) before implementing forwarding.
These do not block local reader development.
