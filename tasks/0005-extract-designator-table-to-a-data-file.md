# Extract the designator table to a data file

**Status:** proposal — format not decided, flagging for discussion before
implementing.

## Purpose

Every designator addition so far (tasks `0001`, `0003`, `0004`) has been a
Rust source change: a new `match` arm in `designators.rs`, a new `const`
source string if the citation is novel, a new `#[case(...)]` in
`tests/aircraft.rs`. That's a real PR each time, reviewed as Rust — clippy,
fmt, borrow-checker noise all in the diff even though the actual change is
"one more row of facts." Three of the last four PRs against this crate
have been exactly that shape.

It's also the *first* step of a three-repo chain every time: this crate's
PR, then `au-casa-py`'s `Cargo.lock` bump (`cargo update -p au-casa`), then
every consumer's own lockfile bump (`pilot-logbook`'s `uv.lock`). Extracting
the table to a data file doesn't remove that chain — the data still gets
baked into the compiled wheel either way, so `au-casa-py` and consumers
still need a new pinned commit — but it does shrink each *au-casa-side* PR
to a pure data diff, reviewable by anyone who can read the table without
reading Rust.

## What moves, what doesn't

Moves: the `lookup()` match arms — designator, `AirframeKind`, engine
count, `EngineType`, `Confidence`, and the citation/caveat text currently
held in the `const TCDS_*` strings and match-arm comments.

Stays in Rust: `Confidence`, `DesignatorFacts`, `AirframeKind`, `EngineType`
themselves (the types), and `lookup()`'s signature and `None`-on-uncatalogued
contract. The file becomes data `lookup()` reads once (via `include_str!` +
parse into a `HashMap<&str, DesignatorFacts>` behind a `LazyLock`, so the
lookup stays a cheap map access, not a re-parse per call) — not a new
mechanism, no behavioural change from a consumer's point of view.

## Format: not decided — the real requirement is comments-and-structure

The content isn't purely tabular. Alongside the five structured fields
(designator, airframe, engines, engine type, confidence), several rows
carry real prose: `AS21`'s and `DUOD`'s comments explain *why* they're the
self-launching variant and not the plain glider; `GLID`'s explains the
working rule for when the generic code applies; `LS4`'s flags that its own
Doc 8643 status is unconfirmed. That commentary is load-bearing — it's the
citation trail task `0003` built by hand — so whatever format is chosen
needs to carry it *with* each row, not lose it to "well-named identifiers
don't need comments." Options, roughly in order of how well they fit that:

- **TOML** — native comments, array-of-tables (`[[designator]]`) reads
  reasonably as a table, strong Rust support (`toml` crate is actively
  maintained), and it's already the house format for `Cargo.toml` in every
  repo in this org. Verbosity of array-of-tables syntax for ~20 rows is the
  main downside.
- **YAML** — native comments, and reads the most like the existing
  `match`-arm-with-a-comment-above shape. **But `serde_yaml` was archived
  by its maintainer in March 2024** (marked `+deprecated`, no official
  successor) — a real risk to build a crate's core data path on, not just
  a style objection. A community fork (`serde_yml`) exists but inherits
  none of `serde_yaml`'s prior maintenance track record.
- **JSON** — no comments at all, so the citation/caveat text would need to
  become an explicit `"notes"` field rather than a comment. Arguably not
  worse — a structured, queryable `notes` string is more consistent than
  prose-in-a-comment — but it's a bigger shape change from what's there
  today, and every existing row's commentary would need rewriting to fit
  the field, not just moving.
- **CSV/TSV** — the most naturally tabular fit for the five structured
  fields, and the easiest possible PR diff (one line), but no comment
  support and the citation text is often 2-3 sentences — awkward as a
  quoted CSV cell, and loses the "read the row, read why" locality the
  current Rust comments have.

No recommendation is locked in here on purpose — this needs a decision,
not an implementation, per this repo's own "discuss before implementing"
convention.

## Open questions

- Does `source`/citation text become one field, or split into a short
  machine-checkable reference (e.g. "FAA TCDS 2A13") plus a separate free
  text caveat, now that it doesn't have to fit a Rust `&str` constant?
- Does the file live at the crate root, under `src/aircraft/`, or in a new
  top-level `data/` directory? Whichever it is, `include_str!`'s relative
  path needs to survive `cargo publish`, which flattens the package layout.
- Should `Confidence::Overridden` (wholly consumer-supplied) be excluded
  from the schema entirely, since it never appears as a compiled row? (It
  doesn't today either — `lookup()` only ever returns `Confirmed` or
  `Provisional` — so this is just confirming the schema matches that, not
  a change.)

## Explicitly out of scope here

- **Moving the table to a separate repo** (e.g. an `icao-type-designators`
  crate/data-repo). `description.rs`'s own doc comment already earmarks
  this as the likely long-term home, since none of this data is actually
  Australian — but that's a bigger, separate decision (new repo, new
  release process, a real "is this worth it yet" call with only one
  national package existing) and not blocked on this task either way.
- **Any resolve-time network fetch.** Discussed and deliberately not
  pursued as part of this task — `resolve_classification` stays pure and
  offline by default. If an opt-in "fetch latest" mode is ever built, it
  belongs behind a separate, non-default cargo feature, pinned to a
  caller-supplied ref rather than a floating branch tip, and is its own
  task with its own discussion — not a side effect of moving the file
  format.

## Acceptance criteria

- [ ] Format decided and recorded in this task doc before implementation
      starts.
- [ ] `lookup()`'s public signature and behaviour are unchanged — same
      `None`-on-uncatalogued contract, same `DesignatorFacts` shape.
- [ ] Every row currently in `designators.rs` round-trips: same
      `AirframeKind`, engine count, `EngineType`, `Confidence`, and no
      citation/caveat text lost in the move.
- [ ] `tests/aircraft.rs` passes unchanged (the tests exercise `lookup()`
      and `resolve_classification()`, not the storage format, so they
      shouldn't need to change at all — if they do, that's a signal the
      extraction leaked its shape into the public API).
- [ ] A new designator row is addable by editing only the data file — no
      Rust change, no new `const`, no recompilation-shaped review needed
      for the data itself (CI still rebuilds to run the tests, but the
      *diff* a reviewer reads is pure data).

## Related

- `src/aircraft/description.rs`'s module doc comment — already names a
  separate `icao-type-designators` crate as the likely eventual home for
  Layer A, independent of this task.
- Task `0003` — where the citation/caveat commentary this task needs to
  preserve was built up.
