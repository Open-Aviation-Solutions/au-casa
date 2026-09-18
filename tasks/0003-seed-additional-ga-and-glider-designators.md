# Seed additional GA and glider designators

**Status:** ready to implement — table growth against the pattern task
`0001` already established, not a new mechanism. Revised after the first
pass turned up two real designator-correctness problems, not just missing
TCDS citations — see "What changed on review" below.

## Purpose

Importing a real 226-entry personal logbook into `pilot-logbook` (its task
`0009`) showed that most of the aircraft types actually flown don't resolve
to a CASA classification at all: only `C172`, `PA25`, `PA34`, `PA44` (of the
aeroplane types already seeded) resolved. `C152`, `PA28`, `P28R`, `AS21`,
`DUOD`, `LS4` were the literal strings stored against those aircraft and
none of them resolved — but as this task discovered, at least one of those
strings (`PA28`) is not actually a valid designator at all, and two more
(`AS21`, `DUOD`) name a more specific aircraft than assumed.

`pilot-logbook`'s task `0008` found the same root cause specifically for
`AircraftCategory.RegisteredSailplane`: it **cannot be reached at all**,
because no glider designator is seeded here, even though
`AirframeKind::Glider` and `EngineType::None` already exist in
`src/aircraft/description.rs` for exactly this purpose (reg 61.007(2)).

This is not a new design question — `designators.rs`'s own doc comment
already anticipates it: *"The table grows on demand rather than attempting
to mirror Doc 8643."*

## What changed on review

The first pass of this task assumed the six strings found in
`pilot-logbook`'s data were themselves valid, unambiguous Doc 8643
designators and just needed TCDS facts attached. Checking each one against
public designator references (`doc8643.com`, FAA JO 7360.1K, ICAO's own
Doc 8643 site) before opening the PR found that assumption wrong twice:

1. **`PA28` is not a Doc 8643 designator.** It was retired around 1998 and
   split into `P28A` (fixed gear, fixed-pitch prop — Cherokee/Warrior/
   Archer), `P28B` (fixed gear, variable-pitch prop), `P28R` (Arrow,
   retractable gear), `P28T`/`P28U` (Turbo Arrow variants). `pilot-logbook`'s
   own data has three aircraft stored with the invalid `PA28` string
   (VH-INH, VH-MDL, VH-HTK) — seeding a `PA28` row here would perpetuate the
   error rather than fix it. **This table seeds `P28A`, not `PA28`**, and
   `pilot-logbook` needs a follow-up data correction (flagged separately,
   see Related) to rewrite those three `Aircraft` records' designator to
   `P28A` — the same shape of fix as its task `0010`'s `ZZZZ` corrections.
2. **`AS21` and `DUOD` are not the plain, unpowered gliders.** Doc 8643 only
   assigns an individual designator to a type that needs one for ATC/
   flight-plan purposes — in practice, a self-launching motorglider variant
   with a retractable engine, which flies (and files) more like a powered
   aircraft. `AS21` is specifically the **ASK-21Mi**, fitted with a 41 kW
   IAE R50-AA rotary engine; `DUOD` is specifically the **Duo Discus T**,
   fitted with a Solo 2350D sustainer. Doc 8643's own entry for `DUOD` names
   "Duo Discus T" directly. The plain, unpowered ASK 21 and Duo Discus have
   no individual designator at all — they fall under Doc 8643's **generic**
   `GLID` ("(any manufacturer) Glider") designator, which this table now
   also seeds. `pilot-logbook`'s two real aircraft under these designators
   (VH-GBW as `AS21`, VH-GKX as `DUOD`) may genuinely be the self-launching
   variants — plausible for club two-seat trainers, where self-launch
   removes the need for a tow plane — but that's independent registration
   information this task can't verify from here.

`LS4` was checked the same way and no individual Doc 8643 entry, powered
variant, or contradiction was found — kept as its own row, though whether
it is truly Doc 8643's designator for this type (rather than it, too,
falling under the generic `GLID`) remains unconfirmed; see its row comment.

This changes the *facts* seeded for `AS21`/`DUOD` (one piston engine, not
zero) but not the *conclusion* `pilot-logbook`'s tasks care about — `Glider`
still derives to `RegisteredSailplane` regardless of engine count, so the
sailplane blocker is closed exactly as before. It's recorded here because
an aircraft's description should describe the real aircraft, and because a
future column that does key off engine facts (there is none today) must not
inherit a wrong assumption silently.

## Rows to add

Researched via public search (FAA/EASA/NZ CAA type-certificate references,
`doc8643.com`), **not yet verified against the actual TCDS/EASA document
text**. Every existing row in this table started life the same way —
`C172`/`PA34`/`PA44`/`C208`/`CH7A`/`CH7B` were seeded `Provisional` and
promoted to `Confirmed` only once someone opened and read the cited sheet
(`R44`/`PA25` still carry `Provisional` for exactly that reason). These
follow the same discipline: seeded now, promoted later.

| Designator | Airframe | Engines | Engine type | Candidate source | Notes |
|---|---|---|---|---|---|
| `C152` | `LandPlane` | 1 | `Piston` | FAA TCDS 3A19 (shared with the Cessna 150) | Lycoming O-235 |
| `P28A` | `LandPlane` | 1 | `Piston` | FAA TCDS 2A13 | Fixed-gear Cherokee/Warrior/Archer, fixed-pitch prop; TC 2A13 spans PA-28-140 through -236. **Not** `PA28` — see above |
| `P28R` | `LandPlane` | 1 | `Piston` | FAA TCDS 2A13 | Same TC as `P28A` — retractable-gear Arrow variants; undercarriage is not derived here anyway, same reasoning as `PA25`/`CH7A` |
| `AS21` | `Glider` | 1 | `Piston` | EASA.A.221 | The self-launching **ASK-21Mi**, not the plain ASK 21 — see above. Confirm on open whether EASA.A.221 covers the Mi directly or the conversion is a supplemental certificate on the base airframe's own sheet |
| `DUOD` | `Glider` | 1 | `Piston` | EASA.A.025 | The self-launching **Duo Discus T**, not the plain Duo Discus — see above. Confirm on open whether the self-launch conversion is covered by this sheet or by EASA.A.074, a separate reference found for the powered variant |
| `GLID` | `Glider` | 0 | `None` | ICAO Doc 8643's generic "(any manufacturer) Glider" entry | Catch-all for an unpowered glider with no individual designator — most real-world sailplanes, unlike the self-launching pair above |
| `LS4` | `Glider` | 0 | `None` | LBA type certificate 345 (Rolladen-Schneider LS4) | No individual Doc 8643 entry confirmed either way this session — may in fact belong under `GLID` rather than its own row; kept because it's what the pilot's stored data uses |

## Why `Glider` stays a distinct `AirframeKind`, not `LandPlane` + zero engines

Raised and settled before implementation, worth recording since it's a
natural question on first reading the table: could a glider just be
`LandPlane` with `engine_count == 0`, avoiding the extra variant?

No — and the `AS21`/`DUOD` correction above makes this concrete rather than
hypothetical: both are real, currently-flying self-launching sailplanes with
a piston engine, still registered and classified as sailplanes under reg
61.007(2) — CASA's category there tracks *registration status*, not
presence of a powerplant. If `classify()` derived category from
`engine_count == 0` on `LandPlane`, both of these real aircraft would flip
to `Aeroplane`, which is wrong. Keeping `Glider` as its own `AirframeKind`
value keeps "what kind of airframe is this" and "how many engines does it
have" orthogonal, which `AircraftDescription` already does everywhere else.

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

- [x] New `match` arms in `designators.rs::lookup` for `C152`, `P28A`,
      `P28R`, `AS21`, `DUOD`, `GLID`, `LS4`, each with a comment explaining
      what the designator covers and what's deliberately not derived
      (undercarriage, floats — per-airframe, same as every existing row).
      `PA28` deliberately **not** seeded — see above.
- [x] Tests confirming each new designator's resolved category/class,
      `Confidence::Provisional`, that `P28A`/`P28R` classify identically
      (mirroring the Citabria pair), that `PA28` still errors as unknown,
      and that `AS21`/`DUOD` carry the corrected engine facts (checked via
      `designators::lookup` directly, since `resolve_classification`'s
      return type doesn't expose `AircraftDescription`).
- [x] `make check` (clippy + fmt + test) green.

## Out of scope

- Promoting any of these rows to `Confirmed` — needs the actual TCDS/EASA
  document opened and read, same outstanding item `PA25`/`R44` already carry.
- Resolving whether `LS4` is really Doc 8643's designator for that type, or
  should be removed in favour of relying on `GLID` — flagged, not settled.
- `au-casa-py`'s `Cargo.lock` bump and `pilot-logbook`'s `uv.lock` bump to
  pick this up — separate, mechanical follow-ups in those repos once this
  merges.
- Correcting `pilot-logbook`'s three `PA28`-designated `Aircraft` records to
  `P28A` — a data fix in that repo, not this one, flagged for the same
  reason task `0010` flagged the `ZZZZ` placeholders.

## Related

- `pilot-logbook` tasks `0008` (sailplane blocker) and `0009` (sparse table
  found via a real logbook import) — this closes both.
- `pilot-logbook` task `0010` (`ZZZZ` placeholder designators) — the `PA28`
  data-correction finding above is the same shape of problem, tracked as a
  new follow-up there rather than duplicated here.
- `au-casa` task `0001` — the table and its sourcing discipline, unchanged
  here, just grown.
