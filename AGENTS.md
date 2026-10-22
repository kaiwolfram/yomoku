# Yomoku coding sessions

Read `docs/NEXT_SESSION.md` first, then the relevant unchecked tasks in
`docs/IMPLEMENTATION_PLAN.md`. Read `YOMOKU_CODING_AGENT_HANDOFF.md` for the
requirements governing the slice. It is the product source of truth.

- Inspect `git status` and preserve existing/uncommitted work.
- Implement a small, independently testable slice; continue the plan rather than
  replanning the project. Prefer persistence and reader correctness over styling.
- Use Kotlin/Compose for Android, a pure Rust Crux core, separate Rust SQLite I/O
  and EffectRouter, and a narrow typed FFI. Do not put SQL/platform I/O in core.
- Do not invent product behavior, add excluded features, or change major
  architecture without user approval. Record routine choices in `docs/DECISIONS.md`.
- Verify dependency APIs against primary documentation before adopting versions.
- Use Fedora-compatible CLI tooling; normal development must not need Android Studio.
- Run relevant tests, formatting and lint. Exact current commands are in
  `docs/NEXT_SESSION.md` and `docs/DEVELOPMENT.md`. Never claim unrun checks passed.
- At session end, check off only completed, verified tasks; update
  `docs/NEXT_SESSION.md` with next 1–3 tasks, commands, failures and blockers.
  Keep architecture/data docs current and append a brief session log.
- After each completed, verified implementation step, update its progress docs
  and commit the step automatically with a descriptive message. The user has
  authorized these local commits; do not wait for a separate commit request.
  Include only changes belonging to the step, preserve unrelated work, and
  report the commit hash. If verification or committing is blocked, report why.
- Report implemented tasks, changed files, test results, next steps and blockers.
- Never commit credentials/signing keys. Publishing signed releases requires approval.

Nostr starter data is not supplied: keep configuration empty until provided.
Ask Q188 (Citrine retry queue versus best effort) before implementing forwarding.
These do not block local reader development.
