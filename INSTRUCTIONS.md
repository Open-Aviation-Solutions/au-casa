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
