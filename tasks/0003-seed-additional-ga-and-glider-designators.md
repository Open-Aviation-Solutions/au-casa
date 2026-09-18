# Seed additional GA and glider designators

**Status:** ready to implement — table growth against the pattern task
`0001` already established, not a new mechanism.

## Purpose

Importing a real 226-entry personal logbook into `pilot-logbook` (its task
`0009`) showed that most of the aircraft types actually flown don't resolve
to a CASA classification at all: only `C172`, `PA25`, `PA34`, `PA44` (of the
aeroplane types already seeded) resolved. `C152`, `PA28`, `P28R`, `AS21`,
`DUOD`, `LS4` — all ordinary, real Doc 8643 designators — did not, simply
because this crate's table hasn't been seeded with them yet.

`pilot-logbook`'s task `0008` found the same root cause specifically for
`AircraftCategory.RegisteredSailplane`: it **cannot be reached at all**,
because no glider designator is seeded here, even though
`AirframeKind::Glider` and `EngineType::None` already exist in
`src/aircraft/description.rs` for exactly this purpose (reg 61.007(2)).

This is not a new design question — `designators.rs`'s own doc comment
already anticipates it: *"The table grows on demand rather than attempting
to mirror Doc 8643."* Six rows, following the exact existing pattern.

## Rows to add

Researched via public search (FAA/EASA/NZ CAA type-certificate references),
**not yet verified against the actual document text**. Every existing row in
this table started life the same way — `C172`/`PA34`/`PA44`/`C208`/`CH7A`/
`CH7B` were seeded `Provisional` and promoted to `Confirmed` only once
someone opened and read the cited sheet (`R44`/`PA25` still carry
`Provisional` for exactly that reason). These six follow the same
discipline: seeded now, promoted later.

| Designator | Airframe | Engines | Engine type | Candidate source | Notes |
|---|---|---|---|---|---|
| `C152` | `LandPlane` | 1 | `Piston` | FAA TCDS 3A19 (shared with the Cessna 150) | Lycoming O-235 |
| `PA28` | `LandPlane` | 1 | `Piston` | FAA TCDS 2A13 | Fixed-gear Cherokee/Warrior/Archer family; TC 2A13 spans PA-28-140 through -236 |
| `P28R` | `LandPlane` | 1 | `Piston` | FAA TCDS 2A13 | Same TC as `PA28` — retractable-gear Arrow variants (PA-28R-180/-200/-201 etc.); undercarriage is not derived here anyway, same reasoning as `PA25`/`CH7A` below |
| `AS21` | `Glider` | 0 | `None` | EASA.A.221 (Schleicher ASK 21) | Confirm on open whether EASA.A.221 covers the base ASK 21 or only the later ASK 21 B |
| `DUOD` | `Glider` | 0 | `None` | EASA.A.025 (Schempp-Hirth Duo Discus) | Confirm `DUOD` is the unpowered variant — the self-launching "Duo Discus T" is a separate designator under EASA.A.074, same kind of split as `CH7A`/`CH7B` |
| `LS4` | `Glider` | 0 | `None` | LBA type certificate 345 (Rolladen-Schneider LS4) | No direct EASA.A.xxx number found in this pass — resolve the exact EASA TCDS reference before citing it as `source`, same standard as every other row |

## Why `Glider` stays a distinct `AirframeKind`, not `LandPlane` + zero engines

Raised and settled before implementation, worth recording since it's a
natural question on first reading the table: could a glider just be
`LandPlane` with `engine_count == 0`, avoiding the extra variant?

No — that would misclassify a motorglider. A self-launching sailplane (e.g.
the powered "Duo Discus T", `EASA.A.074`, as distinct from the `DUOD` row
above) carries an engine but is still a registered sailplane under reg
61.007(2) — CASA's category there tracks *registration status*, not
presence of a powerplant. If `classify()` derived category from
`engine_count == 0` on `LandPlane`, that aircraft would flip to `Aeroplane`
the moment it has a sustainer engine, which is wrong. Keeping `Glider` as
its own `AirframeKind` value keeps "what kind of airframe is this" and "how
many engines does it have" orthogonal, which `AircraftDescription` already
does everywhere else — a `Glider` with `engine_type: Piston` and
`engine_count: 1` is a perfectly valid, correct value once a self-launching
type is ever added.

Also considered and rejected: renaming `Glider` to `SailPlane` for surface
consistency with `LandPlane`/`SeaPlane`. `description.rs`'s own doc comment
is explicit that `Glider` and `Airship` are **not** Doc 8643 ADC
first-character values, unlike `LandPlane`/`SeaPlane`/`Amphibian`/
`Helicopter`/`Gyrocopter`/`TiltRotor` — they were added specifically to
reach Part 61 categories Doc 8643 has no slot for. Layer A is deliberately
country-agnostic aviation vocabulary, kept separate from Layer B's CASA
regulatory terms (`AircraftCategory::RegisteredSailplane`) so the module can
be extracted cleanly if a second country arrives — see the module's own
"why country-agnostic types live in a national package" note. "Glider" is
the universal aviation term for the airframe; "sailplane" is CASA's
regulatory category word, and not every glider is a sailplane (hang
gliders, weight-shift gliders are gliders CASA doesn't register this way).
Renaming Layer A's term to match Layer B's would blur a distinction the
module goes out of its way to keep sharp. No change made.

## Acceptance criteria

- [ ] Six new `match` arms in `designators.rs::lookup`, each with a comment
      explaining what the designator covers and what's deliberately not
      derived (undercarriage, floats — per-airframe, same as every existing
      row).
- [ ] One test per new designator in `tests/aircraft.rs`, confirming the
      resolved category/class and `Confidence::Provisional`.
- [ ] `make check` (clippy + fmt + test) green.

## Out of scope

- Promoting any of these six rows to `Confirmed` — needs the actual TCDS/EASA
  document opened and read, same outstanding item `PA25`/`R44` already carry.
- `au-casa-py`'s `Cargo.lock` bump and `pilot-logbook`'s `uv.lock` bump to
  pick this up — separate, mechanical follow-ups in those repos once this
  merges.

## Related

- `pilot-logbook` tasks `0008` (sailplane blocker) and `0009` (sparse table
  found via a real logbook import) — this closes both.
- `au-casa` task `0001` — the table and its sourcing discipline, unchanged
  here, just grown.
