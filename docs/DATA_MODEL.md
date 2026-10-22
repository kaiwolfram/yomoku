# Data model

## Implemented domain values
- `Surface`: nonblank exact written Unicode string; equality/hash/order use exact
  contents. Different readings share one identity; inflected spellings differ.
  This is not a tokenizer or Japanese detector.
- `VocabularyStatus`: New (default), Learning, Familiar, Known, Ignored.
- `Registration`: CreateNew for absent entry, AlreadyRegistered preserving status
  otherwise. New registration never implies a completed-reading encounter.
- Completion status policy: only New changes to Known.

No database exists yet. These enums have no persisted/FFI encoding contract.

## Transient application state
`app::Destination` is Library (default), Nostr, Statistics or Vocabulary.
`Model` owns the selected destination; `ViewModel` projects it for the shell.
`Event::SelectDestination` selects a destination and requests a render. Event,
destination and view derive Serde serialization, but no stable wire format or
persisted discriminants are established. A fresh core always starts at Library.

## Required future schema constraints
- Stable document/source/chapter IDs; immutable content with explicit offset
  encoding tested across Rust/Kotlin. Encoding/anchor design is still open.
- Unique active surface entry, linked analyzer dictionary form (nullable), dated
  registration and immutable status history. Reading never participates in identity.
- Completed-reading events with unique command IDs, occurrence rows preserving
  repeated tokens, provenance and active time; reread uses a new command identity.
- Pending page time and current text offset separate from completed events;
  permanent finished-at-least-once separate from current percentage.
- Document deletion removes content/progress but preserves historical provenance,
  encounters and global vocabulary. Reimport creates fresh document progress.
- Vocabulary deletion removes its encounter/status history, recalculates statistics
  and does not trigger rescans. Future registration starts New.
- Versioned SQLite migrations from first storage implementation; complete page
  events, counters and New→Known changes committed in one transaction.

Refine schema with executable migration/transaction tests before claiming storage
or accurate historical statistics are complete.
