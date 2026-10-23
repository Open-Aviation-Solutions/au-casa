# au-casa — Working Instructions

CASA (Australia) national aviation regulatory package. Sibling to
[`icao-shared-kernel`](https://github.com/Open-Aviation-Solutions/icao-shared-kernel)
(the ICAO-universal domain), following the same Rust-crate-plus-Python-bindings
pattern (`au-casa-py`). Grew out of the Python kernel's tasks `0011` (which
trimmed CASA-specific fields out of the kernel `Aircraft`) and `0012` (which
specified this package before it existed), in `icao-shared-kernel-python`
(archived) — see those for the "why ICAO-universal vs national" background;
this repo is where the CASA-specific design now lives and evolves.

## Purpose and scope

This package resolves **national** (CASA Part 61) facts that the kernel
deliberately does not carry, because they aren't ICAO-universal — see the
kernel's thin-hub discipline, written down in `icao-shared-kernel-python`'s
`INSTRUCTIONS.md` (archived). Each module answers one such question:

- `aircraft` (implemented, task `0001`): CASA category/class/design-feature
  classification, derived from an ICAO Doc 8643 type designator.
- Simulator classification (ready, task `0002`): recognition of a flight
  simulation training device under reg 61.010, and whether a session counts
  for Part 61 currency. Builds on the `FlightSimulationTrainingDevice` /
  `FstdSession` aggregates in `icao-shared-kernel`.
- `licence` (issue #14): for each of the kernel's Annex 1 titles of licence
  (`icao_shared_kernel::LicenceTitle`), whether CASA issues it and, if so, its
  Part 61 term (the flight crew licence and aircraft category rating), with
  citations. A CASA host names and offers titles through it, not by the
  kernel's ICAO names. The `match` is exhaustive on purpose: a title the
  kernel adds is a compile error here, and a decision. National licences and
  categories that are not Annex 1's (the recreational pilot licence, the
  gyroplane category rating) are not expressed here: they are issue #15.

`icao-shared-kernel` is a dependency pinned exactly (`=0.1.2`), as it is
released as patch versions; moving to a new one is a deliberate change.
A consumer must take the kernel from the same source and version, or its
`LicenceTitle` is a different type from the one `part61_term` takes.

## Reading Part 61 for licences and category ratings

Division 61.L.2 does not list which aircraft category ratings each licence can
carry. Reg 61.730 grants a category rating on meeting the requirements for
"the pilot licence with the aircraft category rating", so the pairs are those
each licence's Subpart (61.H to 61.K) gives aeronautical experience
requirements for. A pair cites those requirements and, for the multi-crew
and air transport pilot licences, also the privileges regulation that names
the category (regs 61.635 and 61.665). Some pairs appear only in the
requirements for applicants who have not completed an integrated training
course (Divisions 61.H.3 and 61.I.3); CASA still issues them. The glider pilot licence is a flight crew licence but not a pilot
licence (reg 61.010), and a glider is not a category of aircraft (reg 61.015),
so it carries no category rating (Subpart 61.Z).

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
  `icao-shared-kernel`/`icao-shared-kernel-py`.
- **No vendored ICAO-copyrighted data.** Doc 8643 itself may not be
  redistributed; only independently-compiled facts, each with a source
  citation. See task `0001` for the sourcing approach.
- **Regulatory terms only**: names and terminology must be verifiable against
  CASA Part 61 (or the specific regulation cited).
- **Discuss before implementing**: design decisions get a `tasks/NNNN-*.md`
  discussion first. This repository follows the conventions written down in
  `icao-shared-kernel-python`'s `tasks/INSTRUCTIONS.md` (archived); `tasks/`
  here has none of its own.

## Commands

```sh
make test   # cargo test
make lint   # cargo clippy -- -D warnings
make fmt    # cargo fmt
make check  # lint + fmt check + test
```
