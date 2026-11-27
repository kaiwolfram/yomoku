# Yomoku coding sessions

Work on the current request. Check `git status`, preserve existing work, and
reproduce reported bugs when practical before fixing them.

- Use the [product specification](docs/PRODUCT_SPEC.md) for behavior and scope.
  Ask for approval before changing major architecture, adopting a library with
  major cost, or changing when user data is deleted or overwritten.
- Use Kotlin and Compose on Android and keep the Rust Crux core pure. Keep
  SQLite and platform I/O outside the core, route effects through `EffectRouter`,
  and keep the FFI narrow and typed.
- Follow the [CLI development guide](docs/DEVELOPMENT.md) for setup and
  verification. Run focused tests and applicable formatting and lint checks.
  Add regression coverage and broaden verification when the change warrants it.
- Use `/home/kai/Documents/books/japanese` for local Japanese book examples and
  keep book files outside Git. Store signed APKs for local testing and their
  sidecars in the Git-ignored `.local/apks/` directory. Follow the
  [README](README.md#build-the-android-release) for build, signing, and filename
  details. Keep credentials and signing keys out of Git and get approval before
  publishing a signed release.
- Commit each completed, verified change locally and stage only related files.
  Use a concise subject and a body that explains what changed and why. For merge
  commits, include the source branch and any relevant issue or PR references.
  Report verification and the commit hash to the user.
