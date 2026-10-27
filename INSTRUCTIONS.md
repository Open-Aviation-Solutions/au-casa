# au-casa — Working Instructions

CASA (Australia) national aviation regulatory package. Sibling to
[`icao-shared-kernel`](https://github.com/Open-Aviation-Solutions/icao-shared-kernel)
(the ICAO-universal domain), on which it builds. Grew out of the earlier
Python kernel's tasks `0011` (which trimmed CASA-specific fields out of the
kernel `Aircraft`) and `0012` (which specified this package before it
existed), which are not published; this repo is where the CASA-specific
design now lives and evolves.

## Purpose and scope

This package resolves **national** (CASA Part 61) facts that the kernel
deliberately does not carry, because they aren't ICAO-universal (the
kernel's `INSTRUCTIONS.md` keeps a State's own rules out of it). Each module
answers one such question:

- `aircraft` (implemented, task `0001`): CASA category/class/design-feature
  classification, derived from an aircraft type designator, with the
  table of types in aircraft-type-descriptions.
- Simulator classification (ready, task `0002`): recognition of a flight
  simulation training device under reg 61.010, and whether a session counts
  for Part 61 currency. Builds on the `FlightSimulationTrainingDevice` /
  `FstdSession` aggregates in `icao-shared-kernel`.
- `licence` (issue #14): for each of the kernel's Annex 1 titles of licence
  (`icao_shared_kernel::LicenceTitle`), whether CASA issues it and, if so, its
  Part 61 term (the flight crew licence and aircraft category rating), with
  citations. A CASA host names and offers titles through it, not by the
  kernel's ICAO names. The `match` is exhaustive on purpose: a title the
  kernel adds is a compile error here, and a decision. The pairs Part 61
  grants that Annex 1 has no title for (the recreational pilot licence, and
  the gyroplane category rating) are `NationalTitle`s (issue #15), listed by
  an exhaustive `match` the same way, each with its licence's minimum age;
  `IssuedNationalTitle` is one held from a date of first issue. They sit
  beside the kernel's titles: a consumer stores them with the licence, keyed
  by its identity, and the kernel stays the source for what Annex 1 can say.
  `FlightCrewLicence::is_pilot_licence` says which licences are pilot
  licences (reg 61.010), so no caller matches on the licences' names for it.

`icao-shared-kernel` and aircraft-type-descriptions are taken by a caret
range (`^0.1.3` and `^0.1.0`), not pinned exactly (au-casa#24): sibling
country crates (nz-caa and the like) depend on them too, and a consumer such
as a logbook imports several at once. Exact pins on different patches cannot
be resolved together; with caret ranges Cargo builds one copy of each, so the
kernel's `LicenceTitle` and the table's `AirframeKind` are the same types in
every crate. A caret range takes up patches but not a minor version, and each
crate releases a change that could alter a classification as a minor version:
the kernel by [its rule](https://github.com/Open-Aviation-Solutions/icao-shared-kernel/blob/main/INSTRUCTIONS.md#consumers), the table by [its rule](https://github.com/Open-Aviation-Solutions/aircraft-type-descriptions/blob/main/INSTRUCTIONS.md#releasing). A patch of the
table can still change an answer (a new row, a confidence, a citation). A
minor release is taken by the country crates together. A consumer takes them
from the same source, by a caret range compatible with au-casa's (the same
`0.x` minor), since Cargo builds one copy only within one such range.

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

The recreational pilot licence is the exception to reading the pairs from
the regulations: reg 61.475 grants it with "the associated aircraft category
rating", naming none. Its pairs are those the Part 61 Manual of Standards has
standards for (Schedule 1, Section G, Appendices G.1 to G.3: aeroplane,
helicopter and gyroplane), each cited with reg 61.475. Appendix G.4, the
airship, is reserved, and there is no appendix for a powered-lift aircraft
(decided on issue #15).

**One part of `aircraft` is not national, and is not here.** `AircraftDescription`
and the `designator -> description` table (Layer A) are the same for any
country, so they live in
[aircraft-type-descriptions](https://github.com/Open-Aviation-Solutions/aircraft-type-descriptions)
(au-casa#7), which this crate depends on by a compatible range (see above). It
is the vocabulary for aircraft facts shared by the country crates, as the
kernel is for `AircraftType` and `LicenceTitle`, so au-casa does not wrap,
redefine or re-export its types (`AircraftDescription`, `AirframeKind`,
`EngineType` and `Confidence`): a consumer names them from that crate. Only Layer B, the derivation to CASA category/class/design
features, is regulatory interpretation, and it is what stays here. A type is
added to the table there, not here; a type the table lacks is
`UnknownDesignator` until it is, or the caller overrides the category.

## Conventions

- **Stateless capabilities only.** This package provides pure derivation
  functions and value objects — no repository protocols, no persistence, no
  identity-keyed storage. Per the Dependency Inversion Principle, storage of
  any consumer-specific data (e.g. a per-aircraft category override) belongs
  to the consuming application that owns that data's lifecycle, not here —
  the same reasoning that kept repository protocols out of
  `icao-shared-kernel`.
- **No vendored ICAO-copyrighted data.** The table of types, and how it is
  sourced, is aircraft-type-descriptions' to say (and it sources US public
  domain data).
- **The MPL notice heads each file** in `src/`, `tests/`, the `Makefile`,
  `Cargo.toml` and `.github/workflows/` (the licence's Exhibit A, in that
  file's comment syntax); `make license-add` adds it to each tracked file
  that has none (so `git add` a new file first). `make license-check`, part
  of `make check`, runs `addlicense -check` over the tracked files, which
  fails on a Rust, TOML or YAML file with no licence header, though it does
  not check that the header is the MPL's wording. Neither covers the
  `Makefile`, which addlicense does not recognise.
- **Regulatory terms only**: names and terminology must be verifiable against
  CASA Part 61 (or the specific regulation cited).
- **Discuss before implementing**: a design decision is discussed in an issue
  first, and the issue records what was decided. `tasks/archived/` holds the
  earlier design notes (`NNNN-*.md`), kept as history; tasks `0001` and `0002`
  cite tasks of the earlier Python kernel that are not published.

## Releasing

Released to crates.io as the kernel is, from `main` only. Bump `version` in
`Cargo.toml` in a pull request, and merge it: that is the release. On each
push to `main`, the Release workflow (`.github/workflows/release.yml`) asks
crates.io whether `Cargo.toml`'s version is there yet. When it is not, it runs
`make check`, publishes it through trusted publishing (no crates.io token is
kept in the repository), and tags the commit `v<version>`. Don't push version
tags by hand: nothing runs on them.

Only `main` can release. The publishing job runs in the `release` environment,
which GitHub allows only on `main`, and crates.io's trusted publisher accepts
only that environment; `main` changes only through a pull request with CI
passing.

While on `0.x`, a breaking release bumps the minor version (`0.2.0`) and a
compatible one the patch, as the kernel's do (decided on au-casa#24).
Breaking includes a change to the API and a change to the Part 61 reading
that changes an existing designator's category, class or design features, or
a title of licence's Part 61 term. Consumers take au-casa by a caret range
on its 0.x minor, so a `cargo update` takes up patches but not a breaking release,
which they move to on purpose.

## Commands

```sh
make test           # cargo test
make lint           # cargo clippy -- -D warnings
make fmt            # cargo fmt
make license-add    # add the MPL notice where a tracked file has none
make license-check  # addlicense -check over the tracked files
make check          # lint + fmt check + licence headers + test
```
