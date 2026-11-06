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
- `fstd` (implemented, task `0002`): recognition of a flight simulation
  training device under reg 61.010, and whether a device is one at all
  (`counts_for_part61`). Builds on the `FlightSimulationTrainingDevice` /
  `FstdSession` aggregates in `icao-shared-kernel`. Reg 61.010 defines two
  terms: a "flight simulation training device" (any of five recognised kinds,
  which is what `fstd` models), and an "approved flight simulation training
  device" for a purpose (one such device that, by the definition's
  paragraphs, may be used for that purpose). The recency rules (reg 61.870,
  for one) use the second, so whether a session
  counts for them is `logbook`'s `approved_fstd`, which the pilot records,
  not `counts_for_part61`.
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
- `logbook` (au-casa#29): CASA's rules for a pilot's logbook,
  as `Casa`, which implements logbook-shared-kernel's `LicensingAuthority`
  for `AU-CASA`; and CASA's authority-specific information on an entry
  (`EntryInformation`) and on an assessment (`AssessmentInformation`: the
  check type, outcome and purpose of regs 61.745(3), (3A) and 61.880), two
  stored formats whose every key and spelling is mapped by hand, with no
  derived serde, each under a version of its own (the kernel's reserved key
  `_version`). `Casa` says which entries count (`counts`: a session
  only in an approved flight simulation training device, reg 61.010), what
  a type counts as (`classify`), which requirements it checks, each with a
  stable id and a name in its regulation's words (`requirements`), the
  pilot's currency against them (`currency`), and CASA's own totals
  (`totals`, over the entries that count; solo flight time is the flights
  whose `sole_occupant` is true, and no rule makes solo count as pilot in
  command); the questions it asks for its information on an entry and on
  an assessment (`questions_for`, in `logbook::questions`: one for each key
  of the format, a question about an entry's subject, approaches or flight
  rules asked only of an entry it applies to); and the problems with the
  answers (`problems_for`): information it cannot read, so that the host
  does not stamp and save it, and reg 61.095(3)(b)'s "the person is not
  receiving flight training", where that question is shown. Stored information must carry its version
  (`read`); only information about to be saved may lack it
  (`read_to_save`), which is how `problems_for` reads what it is given. A `null` answer is no answer, as an
  absent key is. The currency checks are in `logbook::currency`, which
  also lists the requirements and runs them; see "Currency checks" below.

## Currency checks

- **One module for each regulation**, an `EntryAccumulator` and
  `CurrencyCheck` (or the assessment traits) over the kernel's
  `EventWindow`. Its doc comment quotes the regulation it implements.
- **Each sees only the entries `Casa::counts` lets through**: the kernel's
  `run` applies it once, so a check does not repeat it. A check over
  assessments that reads their aircraft, which `run` does not reach, is
  given those entries by its caller (`Casa::currency`) when it is built.
- **An assessment's aircraft is its flight's.** A check over assessments
  that reads it (the flight review) is given the entries, and an assessment
  is in the aircraft of the entry for the same `flight`; one whose flight
  has no entry is unclassified and does not count. The instrument
  proficiency check reads no aircraft, as pilot-logbook's did not, so it is
  given no entries.
- **An unknown type does not count.** A check that requires a category
  (`guards::is_category`) needs positive evidence. The type is classified
  from the table's description of its designator or, for a designator the
  table does not catalogue, from the description the entry carries, as
  pilot-logbook did; an entry with neither is not of any category.
- **Each requirement has a stable id**, from `logbook::currency`'s
  `CasaRequirement`: the regulation (bare, `61.395(1)`), and for a flight
  review `|` and its class rating's stored name, as pilot-logbook keyed
  them. A host may keep an id, so changing one is a breaking change. A new
  check is a `CasaRequirement` variant (a `match` on it will not compile
  without its id, name and regulation), a row of `REQUIREMENTS`, and its
  result paired with that variant in `currency`; a test fails until the
  last two agree.
- **Testing:** each check's unit tests assert its boundaries (the window's
  edge and the expiry, the category guard, an entry dated after the
  reference date), with entries built by `logbook::test_support` from the
  real table of types. A check over `EventWindow` does not depend on the
  order entries arrive in, by construction (it sorts and filters the whole
  set). A check that decides over the whole set itself, such as one that
  groups a flight's entries, asserts the same result for both orders.

`icao-shared-kernel`, logbook-shared-kernel and aircraft-type-descriptions
are taken by a caret range (`^0.5`, `^0.2` and `^0.1.0`), not pinned exactly
(au-casa#24): sibling country crates (nz-caa and the like) depend on them
too, and a consumer such as a logbook imports several at once. Exact pins on
different patches cannot be resolved together; with caret ranges Cargo builds
one copy of each, so the kernel's `LicenceTitle` and the table's `AirframeKind` are the same types in
every crate. A caret range takes up patches but not a minor version, and each
crate releases a change that could alter a classification as a minor version:
the kernel by [its rule](https://github.com/Open-Aviation-Solutions/icao-shared-kernel/blob/main/INSTRUCTIONS.md#consumers), the table by [its rule](https://github.com/Open-Aviation-Solutions/aircraft-type-descriptions/blob/main/INSTRUCTIONS.md#releasing). A patch of the
table can still change an answer (a new row, a confidence, a citation). A
minor release is taken by the country crates together. A consumer takes them
from the same source, by a caret range compatible with au-casa's (the same
`0.x` minor), since Cargo builds one copy only within one such range.

Before starting work, check whether any of the three has a newer release
(`cargo search icao-shared-kernel --limit 1`, and the same for
logbook-shared-kernel and aircraft-type-descriptions). A newer patch is taken
up with `cargo update -p <crate>` in the same pull request. A newer minor
version is breaking, and is moved to first, in a pull request of its own,
before the work; it then needs an au-casa minor release of its own, as above.
An issue can settle otherwise: au-casa#29 takes icao-shared-kernel 0.5 and
logbook-shared-kernel 0.2 in the first pull request of its stack, which
releases once, at its end.

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

CASA's information on an entry and on an assessment (`EntryInformation`,
`AssessmentInformation`) are stored formats: a host stores them in its
records. A change to either, to a key, a spelling, or what an answer means,
is breaking: it bumps that format's `VERSION`, is released as a breaking
release, and ships with a migration of the data repository's stored
information, merged with it.

While on `0.x`, a breaking release bumps the minor version (`0.2.0`) and a
compatible one the patch, as the kernel's do (decided on au-casa#24).
Breaking includes a change to the API and a change to the Part 61 reading
that changes an existing designator's category, class or design features, or
a title of licence's Part 61 term. Consumers take au-casa by a caret range
on the current minor (such as `^0.3.0`), so a `cargo update` takes up patches but not a breaking release,
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
