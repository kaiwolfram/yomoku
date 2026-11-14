# Dependency upgrade handoff

## User authorization and scope

The user requested this handoff on 2026-11-14. Implement the medium- and
high-priority upgrades below in a later session. This session only prepared the
handoff; no dependencies or toolchains were upgraded.

**Do not upgrade Rust or Cargo: the user already upgraded them.** Inspect the
installed versions and use the user's toolchain for verification. Do not revert it
to the older version mentioned in DEVELOPMENT.md.

Read AGENTS.md and the relevant sections of docs/PRODUCT_SPEC.md and
docs/DEVELOPMENT.md first. Preserve existing work. Follow the repository's policy
of automatically committing each completed, verified step with a concise subject
and a body explaining the changes and purpose. Report verification separately.

## Upgrades to implement

These versions were recorded during the preceding review on 2026-11-14. Recheck
current pins, upstream stable releases, release notes, and compatibility before
selecting exact targets. The newer versions are candidates, not a verified
compatible combination. Keep explicit pins and update Cargo.lock deliberately.

| Component | Reviewed version | Newer version found | Purpose / constraints |
| --- | --- | --- | --- |
| Gradle wrapper | 9.1.0 | 9.9.1 | Prioritize security maintenance; update wrapper files and distribution checksum. |
| Android Gradle Plugin | 9.0.1 | 9.4.x (9.4.1 also identified upstream) | Select and verify together with Gradle and the Kotlin plugins. |
| Compose BOM | 2026.03.01 | Newer stable BOM; stable Compose UI 1.12.1 identified | Select the actual published stable BOM and inspect its mapping; do not infer its version from the UI version. Existing build reports resolved UI 1.10.6. |
| AndroidX Lifecycle | 2.9.4 | 2.11.0 | Upgrade with the Android toolchain/Compose changes as required by artifact metadata. |
| System OpenJDK 25 patch | Red Hat 25.0.4.1 | Upstream 25.0.5 | Check Fedora package availability and vendor backports; update through the package manager within the environment's permissions. |
| zip | 8.6.0 | 9.5.1 | Move to a maintained release with the extraction fixes; verify EPUB compatibility. |
| rusqlite | 0.40.2 | 0.42.0 | Review API and bundled SQLite changes; protect existing storage behavior. |
| BoltFFI runtime and CLI | 0.30.1 | 0.32.1 | Upgrade together and verify generated Kotlin/JNI for both Android ABIs. |
| serde | 1.0.228 | 1.0.229 | Maintenance update; preserve serialization contracts. |
| regex | 1.12.4 | 1.13.1 | Maintenance update; verify analysis behavior. |
| flate2 | 1.1.9 | 1.1.11 | Import/compression maintenance; retain the intended Rust backend. |
| encoding_rs | 0.8.35 | 0.8.44 | Verify strict decoding and Japanese text fixtures. |
| roxmltree | 0.21.1 | 0.21.2 | Verify XML/EPUB parsing and malformed-input handling. |
| zeroize | 1.9.0 | 1.9.1 | Maintenance update; retain existing backup guarantees. |

Excluded lower-priority upgrades: **Argon2 0.5.3 to 0.6.0 and ktfmt 0.64 to
0.65**. Do not add these to the work. No standalone Kotlin, JDK major-version,
NDK, Crux, Facet, Sudachi, AndroidSVG, or dictionary-data upgrade was requested.
Change another dependency only when necessary for a selected upgrade, and explain
that requirement. Do not implement deferred product features.

## Compatibility and security findings to retain

- Gradle's published security assessment lists two high-severity advisories for
  9.0.0 through 9.2.1. Prioritize replacing our 9.1.0 wrapper. Consult the actual
  advisories when assessing exposure; the review was not a full security scan.
- Kotlin/Compose compiler is pinned to 2.4.21. At review time, JetBrains' fully
  tested range ended at Gradle 9.7.0 and AGP 9.3.1. Newer versions are not
  automatically incompatible, but do not claim the newest candidate versions form
  a validated combination. Check AGP's built-in Kotlin behavior as well.
- AGP 9.4 requires Gradle 9.6.0 or newer and JDK 17. Keep Java/Kotlin compilation
  targeting 17 unless an actual upgrade requirement demands otherwise. The local
  compilation JDK at `.local/jdk-17/jdk-17.0.21+9` was already patched to the
  October baseline. The shell had JAVA_HOME unset and Gradle's launcher selected
  system OpenJDK 25; those are separate roles.
- Check AAR metadata for compileSdk requirements before upgrading Compose and
  Lifecycle. Current compileSdk/targetSdk are 36, build tools 36.0.0, minSdk 29.
  A necessary compileSdk increase does not itself require increasing targetSdk;
  avoid unrelated platform behavior changes. Retain NDK 30.0.16248370.
- zip advisories RUSTSEC-2026-0367, -0368, and -0369 list fixes from 9.0.0.
  `crates/yomoku-import/src/epub/archive.rs` reads entries into memory using
  indexed access and rejects symlinks; it does not use the affected filesystem
  extraction APIs. The specific extraction exploits appear outside current
  usage. Keep those safeguards and bounded parsing during migration.
- Rust dependency pins are repeated in several crate manifests. Update all
  applicable pins consistently. BoltFFI's CLI version guards appear in
  `tools/build-android-native.sh` and `tools/test-kotlin-bridge.sh`; inspect all
  version references and setup documentation when changing it.
- Do not force unrelated major updates into the vendored Sudachi source or
  refresh prepared dictionaries as part of a general dependency update.

## Execution and verification

1. Inspect git status, installed tools, and the current lockfile. Revalidate
   candidates against primary sources. Fetch dependencies explicitly before
   offline checks; report network or package-manager blockers without silently
   omitting an authorized upgrade.
2. Upgrade and verify in manageable commits: Android build tools, Android UI
   libraries, Rust import/maintenance dependencies, SQLite, and BoltFFI. Combine
   steps where compatibility requires it. Handle the host JDK patch separately;
   it is a machine change, not a repository dependency pin.
3. Use the applicable commands in DEVELOPMENT.md: Rust workspace build/tests,
   rustfmt/Clippy, Kotlin formatting checks, and debug/release Android assembly
   and lint, including the instrumentation APK. Run existing import/analysis,
   storage, and backup coverage relevant to the changed libraries. Add regression
   tests only for substantive migration changes.
4. For BoltFFI, install the matching CLI, regenerate ignored bindings, run
   `tools/test-kotlin-bridge.sh`, and build arm64-v8a and x86_64 for both variants.
   Serialize host JNI generation and standalone Cargo checks as documented.
   Verify release JNI/ELF/ZIP alignment and absence of debug probe artifacts.
5. Run relevant instrumentation on a disposable emulator. Cover EPUB import,
   Japanese decoding, lookup, reader layout/touch, navigation, and activity
   recreation. Recommend a physical arm64 trial; distinguish emulator/build
   results from actual device verification. Never wipe a personal installation or
   change schema/data-destructive behavior to make checks pass.
6. Update setup/version documentation when it changes, and report final selected
   versions, checks actually run, commit hashes, and remaining blockers. Do not
   publish or sign a release. A dependency upgrade does not complete the separate
   unfinished backup/restore product work.

## Primary references

- [Gradle releases](https://gradle.org/releases/) and
  [security assessment](https://blog.gradle.org/gradle-security-subscription)
- [AGP 9.4 compatibility](https://developer.android.com/build/releases/agp-9-4-0-release-notes)
- [Kotlin Gradle compatibility](https://kotlinlang.org/docs/gradle-configure-project.html)
- [AndroidX versions](https://developer.android.com/jetpack/androidx/versions),
  [Compose BOM mapping](https://developer.android.com/develop/ui/compose/bom/bom-mapping),
  and [Lifecycle releases](https://developer.android.com/jetpack/androidx/releases/lifecycle)
- [JDK 25.0.5 update](https://www.oracle.com/java/technologies/javase/25-0-5-relnotes.html)
- [ZIP advisories: 0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html),
  [0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html), and
  [0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html)
- [zip](https://docs.rs/zip/latest/zip/),
  [rusqlite](https://docs.rs/rusqlite/latest/rusqlite/), and
  [BoltFFI](https://docs.rs/boltffi/latest/boltffi/) published crate documentation
