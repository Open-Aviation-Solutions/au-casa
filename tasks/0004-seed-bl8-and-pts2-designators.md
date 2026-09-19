# Seed BL8 and PTS2 designators

**Status:** ready to implement — table growth against the pattern task
`0003` established, not a new mechanism.

## Purpose

A fresh CSV import into `pilot-logbook` picked up two aircraft types not yet
in the compiled table: an American Champion 8GCBC Scout (VH-VPW) and an
Aerotek/Christen Pitts S-2A (VH-IPU). Both are ordinary, real Doc 8643
designators, confirmed against public `doc8643.com` listings before writing
the row (same discipline as task `0003`, after that task found relying on
the CSV's raw string without checking it first was a mistake):

- `BL8` — "AMERICAN CHAMPION 8 Scout BL8 — ICAO Type Designator L1P L/G".
  The pilot's own CSV already used this exact string, and it checks out —
  unlike `PA28` in task `0003`, no correction needed here.
- `PTS2` — "AEROTEK (1) Pitts S-2 Special PTS2 — ICAO Type Designator L1P
  L/G". The CSV's raw string was `"Pitts S2A"`, a model name, not the
  designator; the row is keyed `PTS2`.

Both ADC codes (`L1P`) decode the same way as every existing single-engine
piston landplane row in this table: `L` landplane, `1` one engine, `P`
piston.

## Rows

| Designator | Airframe | Engines | Engine type | Confidence | Source |
|---|---|---|---|---|---|
| `BL8` | LandPlane | 1 | Piston | Provisional | FAA TCDS for the 8GCBC Scout — exact number not yet found; the Scout is a Citabria/Decathlon derivative (see `CH7B`'s TCDS A-759), but American Champion's later designs use their own sheets, so A-759 is not assumed here without checking |
| `PTS2` | LandPlane | 1 | Piston | Provisional | FAA TCDS for the Aerotek/Christen Pitts S-2A — exact number not yet found |

Neither citation is a checked TCDS revision, so both rows stay
`Confidence::Provisional`, same standard as every other row seeded from
public search rather than from reading the actual sheet.

## Also in this PR: firm up the GLID comment

Task `0003` seeded `GLID` as a "working hypothesis, not yet verified
against Doc 8643 itself." Verified this session: `doc8643.com`'s own page
title for the code reads "(any manufacturer) Glider GLID — ICAO Type
Designator - -/-", which is ICAO's stated fallback designator for a glider
with no individual type code, not an inference from pattern-matching other
rows. The row's confidence stays `Provisional` (the underlying assignment
of *which* gliders get GLID vs. an individual code is still this crate's
own working rule, not something Doc 8643 states directly), but the comment
no longer needs to hedge on whether GLID itself exists as ICAO's generic
glider designator — it does.

## Acceptance criteria

- [x] `BL8` resolves to `AirframeKind::LandPlane`, 1 engine, `Piston`,
      `Confidence::Provisional`.
- [x] `PTS2` resolves the same way.
- [x] Both covered by a test in `tests/aircraft.rs`.
- [x] `make check` passes.

## Out of scope

- Finding the exact TCDS numbers and promoting either row to `Confirmed` —
  future work, same as every other `Provisional` row in this table.

## Related

- Task `0003` — the designator-verification discipline this task follows,
  and the `GLID` row this task's comment update refers to.
