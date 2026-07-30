# au-casa — Working Instructions

CASA (Australia) national aviation regulatory package. Sibling to
[`icao-shared-kernel-rs`](https://github.com/Open-Aviation-Solutions/icao-shared-kernel-rs)
(the ICAO-universal domain), following the same Rust-crate-plus-Python-bindings
pattern (`au-casa-py`, not started yet). Grew out of `icao-shared-kernel`'s
tasks `0011` (which trimmed CASA-specific fields out of the kernel `Aircraft`)
and `0012` (which specified this package before it existed) — see those for
the "why ICAO-universal vs national" background; this repo is where the
CASA-specific design now lives and evolves.

## Purpose and scope

This package resolves **national** (CASA Part 61) facts that the kernel
deliberately does not carry, because they aren't ICAO-universal — see
`icao-shared-kernel`'s thin-hub discipline. Each module answers one such
question:

- `aircraft` (implemented, task `0001`): CASA category/class/design-feature
  classification, derived from an ICAO Doc 8643 type designator.
- Simulator classification (ready, task `0002`): recognition of a flight
  simulation training device under reg 61.010, and whether a session counts
  for Part 61 currency. Builds on the `FlightSimulationTrainingDevice` /
  `FstdSession` aggregates in `icao-shared-kernel-rs`.

**One part of `aircraft` is not national, and knows it.** `AircraftDescription`
and the `designator -> description` table (Layer A) are the ICAO Doc 8643
description code — a New Zealand or UK package would need them unchanged.
Only Layer B, the derivation to CASA category/class/design features, is
regulatory interpretation.

Layer A stays here because `au-casa` is the only national package that
exists; extracting a shared crate for a hypothetical second country would be
building the extension rather than leaving room for it. The module boundary
is already the seam, so the move stays cheap when a second country arrives.
`src/aircraft/description.rs` records what must be undone on extraction —
chiefly restoring the wake turbulence category, dropped because no Part 61
rule uses it. **Do not narrow Layer A further to suit Part 61 without noting
it there**, since a consumer of a shared table cannot see what is missing.

## Conventions

- **Stateless capabilities only.** This package provides pure derivation
  functions and value objects — no repository protocols, no persistence, no
  identity-keyed storage. Per the Dependency Inversion Principle, storage of
  any consumer-specific data (e.g. a per-aircraft category override) belongs
  to the consuming application that owns that data's lifecycle, not here —
  the same reasoning that kept repository protocols out of
  `icao-shared-kernel-rs`/`icao-shared-kernel-py`.
- **No vendored ICAO-copyrighted data.** Doc 8643 itself may not be
  redistributed; only independently-compiled facts, each with a source
  citation. See task `0001` for the sourcing approach.
- **Regulatory terms only**: names and terminology must be verifiable against
  CASA Part 61 (or the specific regulation cited).
- **Discuss before implementing**: design decisions get a `tasks/NNNN-*.md`
  discussion first (see that directory's own conventions, mirrored from
  `icao-shared-kernel`).

## Commands

```sh
make test   # cargo test
make lint   # cargo clippy -- -D warnings
make fmt    # cargo fmt
make check  # lint + fmt check + test
```
