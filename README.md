# au-casa

CASA (Australia) national aviation regulatory package. Derives Part 61
aircraft classification (category, class, design features) from ICAO Doc
8643 type designators, and hosts other CASA-specific concerns that don't
belong in the ICAO-universal [`icao-shared-kernel`](https://github.com/Open-Aviation-Solutions/icao-shared-kernel).

## What is (and isn't) here

- A stateless resolver: `designator -> CASA category/class/design features`.
- No repository protocols, no persistence, no `aircraft_id`-keyed storage —
  that belongs to whichever consuming application needs it (see
  `tasks/0001-aircraft-classification.md`).

See `INSTRUCTIONS.md` for working conventions and `tasks/` for design history.

## Status

Early: the version is `0.1.0`. Until `1.0`, the API may change between any two
versions, patch releases included, so pin the version exactly. Each release
depends on one exact release of `icao-shared-kernel`; take the kernel at that
same version, or its types are not the ones this crate takes.

## Usage

```toml
[dependencies]
au-casa = "=0.1.0"
icao-shared-kernel = "=0.1.3"
```

## Development

Requires a Rust toolchain and a C linker (`build-essential` on Debian/Ubuntu).

```sh
make help       # list targets
make dev        # build
make test       # run the test suite
make check      # clippy + fmt check + tests
```
