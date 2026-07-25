# CASA aircraft classification (category, class, design features)

**Status:** proposal

## Purpose

`icao-shared-kernel`'s task `0011` removed `AircraftCategory`,
`AircraftClassRating`, and `DesignFeature` from the kernel `Aircraft` — they're
CASA Part 61 concepts, not ICAO-universal, and its task `0012` specified where
they'd land: a shared CASA national package. This is that package's first
module, refined from `0012`'s design after a session of DIP-focused
discussion. It supersedes `0012` as the live design surface for this
question; `0012` stays in `icao-shared-kernel` as historical record.

This directly unblocks `pilot-logbook`'s task `0007`
(`adopt-icao-shared-kernel-py-for-core-aggregates`), confirmed by grep, not
theorised: `services/currency.py` and nine files under `metrics/currency/`
and `metrics/totals/` read `entry.aircraft.category` / `.class_rating`,
comparing against the old `aviation_core.AircraftCategory` /
`AircraftClassRating` values (`AEROPLANE`, `REGISTERED_SAILPLANE`,
`SINGLE_ENGINE_AEROPLANE`, `MULTI_ENGINE_AEROPLANE` — carried over here as
the terms already in use, not newly proposed). Simulator classification
(`SimType`, `is_regulatory_sim`) is a separate concern — see task `0002`.

## Design

Two layers, unchanged from `0012`:

- **Layer A — `designator -> ADC`.** The ICAO Aircraft Description Code:
  type (`L` landplane / `S` seaplane / `A` amphibian / `H` helicopter /
  `G` gyrocopter / `T` tiltrotor), engine count, engine type
  (`P` piston / `T` turboprop / `J` jet), plus Wake Turbulence Category.
  Country-agnostic.
- **Layer B — `ADC -> CASA category/class/design-features`.** Regulatory
  interpretation under CASA Part 61. Country-specific.

**Resolution is a pure function, not a stored aggregate**:

```
resolve_classification(designator: &str, override: Option<PartialClassification>) -> CasaAircraftClassification
```

`CasaAircraftClassification` is a **value object** (category, class, design
features) — it has no identity and no independent lifecycle in this package.
No I/O, no `aircraft_id`, nothing persisted here.

### What's deliberately *not* here

Per this repo's `INSTRUCTIONS.md` (DIP: storage belongs to the consumer that
owns the data's lifecycle, same reasoning that kept repository protocols out
of `icao-shared-kernel-rs`/`-py`):

- No repository protocol, no persistence, no `aircraft_id`-keyed storage.
- Per-airframe overrides — needed routinely, not just for uncatalogued
  designators: `design_features` in particular (floatplane vs wheels,
  tailwheel vs tricycle) can vary between two airframes of the *same* type
  designator, so this isn't a rare edge case. Storage of any such override is
  the consuming application's own aggregate, keyed by `aircraft_id: str`,
  referencing the kernel `Aircraft` by identity — the same "extend by
  reference" pattern as `pilot-logbook`'s `LogbookFlight.flight_id`. A
  consumer calls `resolve_classification(designator, its_own_stored_override)`
  to get the effective classification; this package never sees or stores an
  `aircraft_id`.
- This corrects `0012`'s original framing, which put override *storage* in
  "consumer aggregate" but didn't fully separate that from the classification
  value object, and treated per-airframe data as a rare exception rather than
  routine (`design_features` especially).

### Data sourcing

Unchanged from `0012`'s resolved decision: **do not vendor ICAO Doc 8643**
(ICAO copyright; the dataset is sold, not freely redistributable). Compile
facts independently, seeded from a public-domain spine (FAA Order JO
7360.1K, *Aircraft Type Designators* — a US Government work) for
`designator -> manufacturer/model`, then fill in the four CASA-relevant facts
(type aeroplane/helicopter/glider; single/multi engine; piston/turbine/jet;
land/sea) per designator, each with a `confidence` flag and a `source`
citation (FAA/EASA/CASA Type Certificate Data Sheets). Grow the table on
demand (YAGNI) rather than trying to cover all of Doc 8643 upfront. Return an
explicit "unknown" for uncatalogued designators rather than guessing —
callers fall back to their own override.

## Open questions (why this is `proposal`, not `ready`)

- Exact CASA category/class/design-feature enum values and their Part 61
  citations need verification against the regulation text, not just carried
  over from the old `aviation_core` names — per this project's "regulatory
  terms only" convention.
- Shape of the `confidence`/`source` metadata on `CasaAircraftClassification`
  — per-field, or per-classification?
- Whether `au-casa-py` (PyO3 bindings) starts alongside this module or after
  it lands, mirroring how `icao-shared-kernel-py` followed
  `icao-shared-kernel-rs`.

## Related

- `icao-shared-kernel` tasks `0011` (removed these fields from the kernel)
  and `0012` (original package spec, superseded here).
- `pilot-logbook` task `0007` — the consumer this unblocks.
- `au-casa` task `0002` — simulator classification, a separate concern.
