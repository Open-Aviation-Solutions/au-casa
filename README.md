# au-casa

A Rust library of the rules of Australia's Civil Aviation Safety Authority
(CASA) that a pilot's records need, from Part 61 of the Civil Aviation Safety
Regulations 1998 (CASR): the category, class and design features of an
aircraft type, whether a flight simulation training device is recognised, and
the Part 61 licence and category rating for each title of licence that ICAO's
Annex 1 defines, and CASA's rules for a pilot's logbook (its currency checks,
totals and questions).

It is for developers of aviation software, such as logbooks and training
records, who need Part 61's terms and classifications in one place, with the
regulation each one comes from. It builds on
[icao-shared-kernel](https://github.com/Open-Aviation-Solutions/icao-shared-kernel),
which holds the concepts ICAO defines the same way for every State; the rules
that are Australia's alone are kept here.

## Status

Early: a `0.x` release. Take it by a caret range on its minor version (such as `"0.12"`): until `1.0`
a breaking release is a minor version (0.2 stopped re-exporting the aircraft
description types; 0.3 moved to `icao-shared-kernel` 0.5; 0.4 counts every category in reg
61.870's checks and adds reg 61.870(3) for each category; 0.5 moves to logbook-shared-kernel
0.5, counts take-offs and landings separately, and changes CASA's information on an
assessment to its format version 2; 0.6 moves to logbook-shared-kernel 0.6 and words the
flight test's question in Part 61's words, marking a failed outcome as adverse; 0.7 makes CASA's information on an entry and on an assessment crate-private, so a host reads it only through `LicensingAuthority`, and drops the assessment migration with it; 0.8 moves to logbook-shared-kernel 0.8, classifies an aircraft from its recorded characteristics as well as its type's description, giving the design features it requires and an id for each term and total, puts a multi-engine centre-line thrust aeroplane in the single-engine class, and drops the classification override and the public design features, and lists the titles of licence CASA issues, with an id for each, and its minimum ages, through `LicensingAuthority`; 0.9 moves to logbook-shared-kernel 0.9 and icao-shared-kernel 0.6, and classifies a session's type with the characteristics of the configuration its device represented, so a centre-line thrust twin counts as single-engine whether flown or simulated; 0.10 asks whether a flight was a cross-country flight, totals cross-country flight time, and changes CASA's information on an entry to its format version 2, with a migration from version 1; 0.11 counts reg 61.965's 6 months as calendar months, and has a flight review valid to the end of the 24th month after the month it was completed in, as reg 61.745(1A)(a) says, where each was counted in days; 0.12 renews a flight review completed in the 3 months before the existing one expires from that expiry, as reg 61.745(1A)(b) says, as the instrument proficiency check already did, so such a review's validity is now longer than 0.11 reported), as
[`INSTRUCTIONS.md`, "Releasing"](INSTRUCTIONS.md#releasing) sets out. It takes `icao-shared-kernel` and
[aircraft-type-descriptions](https://github.com/Open-Aviation-Solutions/aircraft-type-descriptions)
by caret range; `0.1.0` pinned them exactly (`=0.1.3` and `=0.1.0`), `0.1.1`
takes `^0.1.3` and `^0.1.0`, `0.2.0` takes `^0.2.0` and `^0.1.0`, and `0.3.0`
and `0.4.0` take `^0.5` and `^0.1.0`, with
[logbook-shared-kernel](https://github.com/Open-Aviation-Solutions/logbook-shared-kernel)
at `^0.2`, `0.5.0` takes it at `^0.5`, `0.6.0` and `0.7.0` at `^0.6`, and
`0.8.0` at `^0.8`, `0.9.0` at `^0.9` and `0.10.0` to `0.12.0` at `^0.9.1`; `0.9.0` to `0.12.0` take icao-shared-kernel at `^0.6`.

What is covered:

- **Aircraft classification** (`classify`, and `Casa`'s): an aircraft's
  Part 61 category (reg 61.015, or reg 61.007(2) for a registered
  sailplane), class (reg 61.020) and the design features that require an
  endorsement (reg 61.755), from the description of its type and the
  characteristics recorded for its airframe (logbook-shared-kernel's
  `AircraftCharacteristic`), which vary between airframes of one type.
  Only `Casa::classify` gives the design features, each with a stable id.
  The table of designators is small and grows
  as types are needed; it is kept in
  [aircraft-type-descriptions](https://github.com/Open-Aviation-Solutions/aircraft-type-descriptions),
  which this crate depends on, and each row cites its sources. A row checked
  against its type certificate data sheet is `Confidence::Confirmed`; the rest
  are `Confidence::Provisional`.
- **Flight simulation training devices** (`FstdRecognition`,
  `counts_for_part61`): the five kinds of device in reg 61.010's definition of
  a flight simulation training device, including those qualified by a
  recognised foreign State, and whether time on one counts for Part 61.
- **Titles of licence** (`part61_term`): for each of the kernel's Annex 1
  titles of licence, the flight crew licence and aircraft category rating
  CASA issues for it, with the regulations that say so, or that CASA does not
  issue it.
- **National titles** (`NationalTitle`, `IssuedNationalTitle`): the licence and
  category rating pairs Part 61 grants and Annex 1 has no title for, the
  recreational pilot licence and the gyroplane category rating, with what is
  cited for each and the licence's minimum age.
- **A pilot's logbook** (`Casa`): logbook-shared-kernel's `LicensingAuthority`
  for CASA. It says which entries count (a session only in an approved flight
  simulation training device, reg 61.010), what a type counts as, the
  recency and currency requirements of regs 61.395, 61.745, 61.870, 61.875,
  61.880 and 61.965 and the pilot's currency against them, CASA's own totals,
  the titles of licence it issues, each with an id, and its minimum ages,
  and the questions CASA asks about an entry and an assessment, with the
  problems with their answers (such as reg 61.095(3)(b)'s "not receiving
  flight training"). CASA's answers are kept as its information on an entry
  and on an assessment, which a host reads through `LicensingAuthority`, not
  directly.

What is not:

- Storage: no repositories or databases, and no file formats beyond CASA's
  information on an entry and on an assessment (below). Those belong to the
  application using the crate.
- Bindings for other languages, or a WebAssembly interface.

Known not to work:

- A type that
  [aircraft-type-descriptions](https://github.com/Open-Aviation-Solutions/aircraft-type-descriptions)
  does not yet describe cannot be classified from its designator alone: the
  host describes it. That table grows in batches; a type is added there, not
  here.
- An aeroplane with two or more engines is put in the multi-engine class
  unless centre-line thrust is recorded for it, though reg 61.020(2)(b) puts
  any prescribed under reg 61.050 in the single-engine class too.

The types serialise with serde, but that JSON is not a stored format that is
promised: it may change between versions, as the API may. CASA's information
on an entry and on an assessment is the exception. From 0.3.0 each is a stored
format: a JSON object whose every key and spelling is mapped by hand, with the
version of its format under `_version` (each has its own). They are read inside
the crate only, through `LicensingAuthority` (0.7). A change to
either is a breaking release, and a consumer that stores them migrates its
stored information with it, through the function au-casa ships for it
(`logbook::migrate_entry_information_from_version_1`, from 0.10). The open
[issues](https://github.com/Open-Aviation-Solutions/au-casa/issues) are the
work still to do.

## What it is not

It is not the regulations. It follows CASR Part 61's terms and cites the
regulation behind each rule, in order to classify and name a pilot's records;
it does not reproduce, replace or interpret the regulations with any
authority. A classification it gives is only as good as the facts about the
type and the characteristics recorded for the aircraft, and a `Provisional` row has
not been checked against its type certificate data sheet. Nothing in it
changes a pilot's own responsibility under the regulations for what they log
and what they fly.

## Usage

Add it from crates.io, with the kernel at a version it accepts. au-casa takes
the kernel at a matching `0.x` minor (au-casa `0.1.x` with kernel `0.1.x`,
`0.2.x` with `0.2.x`, `0.3.x` to `0.8.x` with `0.5.x`, `0.9.x` to `0.12.x` with `0.6.x`); mixing them builds two copies of `LicenceTitle`. From `0.3`, `Casa`
implements logbook-shared-kernel's `LicensingAuthority`, so a consumer takes
that crate at the minor au-casa takes too (`0.2` for au-casa `0.3` and `0.4`, `0.5` for
au-casa `0.5`, `0.6` for `0.6` and `0.7`, `0.8` for `0.8`, `0.9` for `0.9`, and `0.9.1` for `0.10` to `0.12`): with another,
`Casa` implements a different trait from the one the consumer names. The
published set is:

```toml
[dependencies]
au-casa = "0.12"
aircraft-type-descriptions = "0.1.0"
icao-shared-kernel = "0.6"
logbook-shared-kernel = "0.9.1"
```

The aircraft description types (`AircraftDescription`, `AirframeKind`,
`EngineType`, `Confidence`) are aircraft-type-descriptions' own, and
au-casa does not re-export them: name them from that crate, taken by a caret
range compatible with au-casa's own dependency on it (the same 0.x minor; see
`Cargo.toml`), so that Cargo builds one copy.

```rust
use std::collections::BTreeSet;

use au_casa::{part61_term, Casa};
use aircraft_type_descriptions::DesignatorTable;
use icao_shared_kernel::{LicenceTitle, PrivateOrCommercialCategory};
use logbook_shared_kernel::{AircraftCharacteristic, LicensingAuthority, Term};

fn main() {
    let c172 = DesignatorTable::packaged()
        .lookup("C172")
        .expect("the table describes the C172")
        .description;
    // This one has a tailwheel, as recorded for the aircraft.
    let recorded = BTreeSet::from([AircraftCharacteristic::TailwheelUndercarriage]);
    let classification = Casa::new()
        .classify(&c172, &recorded)
        .expect("CASA classifies every description");
    let ids = |terms: &[Term]| terms.iter().map(|t| t.id.as_str().to_owned()).collect::<Vec<_>>();
    assert_eq!(
        ids(&classification.counts_as),
        ["aeroplane", "single-engine-aeroplane"]
    );
    assert_eq!(ids(&classification.requires), ["tailwheel-undercarriage"]);

    let title = LicenceTitle::Private(PrivateOrCommercialCategory::Aeroplane);
    let term = part61_term(title).expect("CASA issues this title");
    assert_eq!(
        term.to_string(),
        "private pilot licence, aeroplane category rating"
    );
}
```

Each rule's rustdoc names the regulation it comes from, and each rule has a
test under `tests/`.

## Other repositories

- **[icao-shared-kernel](https://github.com/Open-Aviation-Solutions/icao-shared-kernel)**
  holds the ICAO concepts this crate builds on: aircraft, licences, flights
  and flight simulation training devices.
- **[aircraft-type-descriptions](https://github.com/Open-Aviation-Solutions/aircraft-type-descriptions)**
  holds the table of aircraft types by designator, with each row's sources.
  This crate reads those facts under Part 61.
- A pilot's logbook, not yet public, builds on this crate.

## Reporting a problem, and contributing

Report a problem, or propose a change, in this repository's
[issues](https://github.com/Open-Aviation-Solutions/au-casa/issues).

## Development

Requires a Rust toolchain, a C linker (`build-essential` on Debian/Ubuntu) and
`curl`. The first `make check` downloads
[addlicense](https://github.com/google/addlicense)'s release binary into
`.bin/`, and checks it against the checksum pinned in the `Makefile`.

```sh
make help       # list targets
make dev        # build
make test       # run the test suite
make check      # clippy, format check, licence headers, tests
```

See `INSTRUCTIONS.md` for working conventions and `tasks/` for design history.

## Licence

[Mozilla Public License 2.0](LICENSE). Each source file carries the licence's
notice.
