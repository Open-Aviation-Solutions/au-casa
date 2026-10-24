# au-casa

A Rust library of the rules of Australia's Civil Aviation Safety Authority
(CASA) that a pilot's records need, from Part 61 of the Civil Aviation Safety
Regulations 1998 (CASR): the category, class and design features of an
aircraft type, whether a flight simulation training device is recognised, and
the Part 61 licence and category rating for each title of licence that ICAO's
Annex 1 defines.

It is for developers of aviation software, such as logbooks and training
records, who need Part 61's terms and classifications in one place, with the
regulation each one comes from. It builds on
[icao-shared-kernel](https://github.com/Open-Aviation-Solutions/icao-shared-kernel),
which holds the concepts ICAO defines the same way for every State; the rules
that are Australia's alone are kept here.

## Status

Early: the version is `0.1.0`. Until `1.0`, the API may change between any two
versions, patch releases included, so pin the version exactly. Each release
depends on one exact release of `icao-shared-kernel`; take the kernel at that
same version, or its types are not the ones this crate takes.

What is covered:

- **Aircraft classification** (`resolve_classification`, `classify`): an
  aircraft type's Part 61 category (reg 61.015, or reg 61.007(2) for a
  registered sailplane), class (reg 61.020) and design features (reg 61.755),
  from its ICAO type designator, with a caller's override for what varies
  between airframes of one type. The table of designators is small and grows
  as types are needed. A row checked against its type certificate data sheet
  is `Confidence::Confirmed`; the rest are `Confidence::Provisional`, each
  saying what is left to check.
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

What is not:

- Storage: no repositories, databases or file formats. Those belong to the
  application using the crate.
- Bindings for other languages, or a WebAssembly interface.

Known not to work:

- A designator that is not in the table is refused (`UnknownDesignator`)
  unless the caller supplies a category, so most types, `C700` among them,
  cannot yet be classified from their designator alone
  ([#7](https://github.com/Open-Aviation-Solutions/au-casa/issues/7)).
- An aeroplane with two or more engines is always put in the multi-engine
  class, though reg 61.020(2) puts a multi-engine centre-line thrust
  aeroplane, and any prescribed under reg 61.050, in the single-engine class
  ([#13](https://github.com/Open-Aviation-Solutions/au-casa/issues/13)).

The types serialise with serde, but that JSON is not a stored format that is
promised: it may change between versions, as the API may. The open
[issues](https://github.com/Open-Aviation-Solutions/au-casa/issues) are the
work still to do.

## What it is not

It is not the regulations. It follows CASR Part 61's terms and cites the
regulation behind each rule, in order to classify and name a pilot's records;
it does not reproduce, replace or interpret the regulations with any
authority. A classification it gives is a default, from facts about the type,
that an individual aircraft may need to override, and a `Provisional` row has
not been checked against its type certificate data sheet. Nothing in it
changes a pilot's own responsibility under the regulations for what they log
and what they fly.

## Usage

Add it from crates.io, with the kernel at the version it depends on:

```toml
[dependencies]
au-casa = "=0.1.0"
icao-shared-kernel = "=0.1.3"
```

```rust
use au_casa::{
    part61_term, resolve_classification, AircraftCategory, AircraftClassRating,
    ClassificationError, Confidence,
};
use icao_shared_kernel::{LicenceTitle, PrivateOrCommercialCategory};

fn main() -> Result<(), ClassificationError> {
    let c172 = resolve_classification("C172", None)?;
    assert_eq!(c172.category, AircraftCategory::Aeroplane);
    assert_eq!(c172.class, Some(AircraftClassRating::SingleEngineAeroplane));
    assert_eq!(c172.confidence, Confidence::Confirmed);

    // A designator that is not in the table is refused, not guessed at.
    assert!(resolve_classification("C700", None).is_err());

    let title = LicenceTitle::Private(PrivateOrCommercialCategory::Aeroplane);
    let term = part61_term(title).expect("CASA issues this title");
    assert_eq!(
        term.to_string(),
        "private pilot licence, aeroplane category rating"
    );
    Ok(())
}
```

Each rule's rustdoc names the regulation it comes from, and each rule has a
test under `tests/`.

## Other repositories

- **[icao-shared-kernel](https://github.com/Open-Aviation-Solutions/icao-shared-kernel)**
  holds the ICAO concepts this crate builds on: aircraft, licences, flights
  and flight simulation training devices.
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
