# Simulator classification (FSTD recognition under Part 61)

**Status:** in progress — the `au-casa` layer is implemented in the working
tree, uncommitted. The kernel aggregates it builds on are on
`icao-shared-kernel-rs` PR #3 (that repo's task `0001`).

## Purpose

`icao-shared-kernel` task `0011` removed `SimType` and `is_regulatory_sim`
from the kernel `Aircraft` and left their home explicitly undecided:
*"pilot-training owning a `Simulator`, or the CASA package if logbook also
needs sim currency — a separate downstream task."* `pilot-logbook`'s task
`0007` confirms it's the latter, at least in part: `metrics/_run.py` and five
files under `metrics/currency/`/`metrics/totals/` depend on `SimType`/
`is_regulatory_sim` for real currency/totals logic, blocking that branch —
same as task `0001`'s aircraft classification, not deferrable.

Kept as a **separate task from `0001`** deliberately: it raises a modelling
question `0001` doesn't have, resolved below.

## What consumers actually need (grepped, not theorised)

Six call sites in `pilot-logbook`, needing only a **three-way** distinction —
real aircraft / recognised FSTD / unrecognised device:

- `metrics/_run.py:36` — skip the entry entirely if the device is
  non-approved (`SimType.NON_APPROVED`).
- `metrics/totals/fstd_minutes.py:24` — count minutes only for a recognised
  FSTD.
- `metrics/currency/instrument_2d.py`, `instrument_3d.py`,
  `single_pilot_ifr.py`, `instrument_approaches.py` — identical guard:
  include only if `category == AEROPLANE` **or** it's a recognised FSTD.

Nothing branches on the specific `SimType` variant. Note these six files are
currently *broken*, not merely stale — they import from `aviation_core`,
which was renamed `icao_shared_kernel`.

## The open question, and its resolution

Task `0011` reasoned that *"a simulator has no ICAO identity... sims do not
become a kernel aggregate."* That is correct for **Doc 8643** — no device has
a type designator of its own — but it does not make simulators
ICAO-agnostic, and the verification below shows why:

- ICAO Annex 1 defines *flight simulation training device* generically, as
  three kinds of apparatus (see Verification). CASR's own umbrella term,
  *synthetic training device*, decomposes into a near-identical triple — CASA
  is tracking an international taxonomy here, not inventing a national one.
- CASR 60.020's qualification levels are *defined by reference to FAA and
  EASA levels*, and AC 60-01 v2.0 maps devices onto ICAO Doc 9625 Types.
  The level taxonomy is cross-national by construction.
- Both device definitions are framed "*for a specific type (or a specific
  make, model and series) of aircraft*" — so a device has no designator, but
  it always **references** a type that does. That is the ICAO link, by
  identity rather than by pretending the device is an aircraft.

**Decided: sibling aggregates in `icao-shared-kernel-rs`, not a widening of
`Flight`.** A simulator session is not a physical flight event, and the
kernel's thin-hub rule forbids widening `Flight` for a field group. The
session becomes its own aggregate alongside `Flight`, and the device becomes
its own aggregate referenced by identity.

## Design

### Kernel (`icao-shared-kernel-rs`) — ICAO-universal, reusable by any State

Two new aggregates, siblings to `Flight`:

**`FlightSimulationTrainingDevice`** — the physical device.

- `id: Uuid`
- device kind: `FlightSimulator` / `FlightProceduresTrainer` /
  `BasicInstrumentFlightTrainer` — the ICAO Annex 1 triple (see
  Verification).
- identifying description (CASR 60.035(2)(a) requires the certificate to
  carry "information identifying the simulator or device" — manufacturer,
  model, serial).
- `qualification: Option<DeviceQualification>` — the device's **current**
  qualification only.

**`DeviceQualification`** (value object): issuing authority, level, and the
validity period. Qualifications are time-bounded — CASR 60.040 gives 12
months from certificate issue (or a shorter specified period), and 60.050
allows variation, cancellation or suspension — so "is this an approved
device" has no answer without a date.

`level` stays a **string for now**, not an enum (YAGNI): nothing branches on
it today. The eventual enum would span ICAO Doc 9625 Types I–VII, FS Levels
A–D, FAA FTD Levels 4–7 and EASA FTD Levels 1–3; build it when a consumer
needs to compare levels, not before.

**`FstdSession`** — the session, sibling to `Flight`.

- `id: Uuid`
- `device_id: Uuid`
- `simulated_aircraft_type: Option<AircraftType>` — **on the session, not the
  device.** A *qualified* device is bound to one simulated aircraft by its
  certificate (CASR 60.035(2)(b)), but an unqualified personal device is
  routinely flown as a different type each session. Session-level placement
  serves both with one field; `None` covers a generic device not representing
  any specific type.
- simulated departure / arrival (`Option`) — a session may be pure manoeuvre
  practice with no route. Existing logbook data does carry route information
  for sim rows, so this isn't speculative.
- session start / end times, mirroring `Flight`'s `first_movement` /
  `last_movement`.

The device carries only its *current* qualification; historical accuracy is
the consumer's snapshot, below.

### `au-casa` — the national layer

A value object plus a predicate. **No resolver, and this is the key
asymmetry with task `0001`**: aircraft classification is *derivable* from a
designator, but a device's qualification is a certificate CASA issues to one
specific physical device. There is nothing to derive it from, so there is no
lookup table and no `resolve_*` function here.

```rust
pub enum FstdRecognition {
    QualifiedFlightSimulator,                       // 61.010(a) — Part 60
    QualifiedFlightTrainingDevice,                  // 61.010(b) — Part 60
    SyntheticTrainerCao45,                          // 61.010(c) — see below
    PrescribedUnderReg61045,                        // 61.010(d)
    ForeignStateQualified(RecognisedForeignState),  // 61.010(e)
}
```

`Option<FstdRecognition>`, where `None` means an unrecognised device —
**replacing the old `NON_APPROVED` enum variant**. "Non-approved" is not one
of reg 61.010's five sub-types; it is the *absence* of recognition, and
modelling it as a sentinel variant inside the enum misrepresented the
regulation. This directly serves the primary use case: a personal device is
logged normally, with notes and reflections, and simply contributes no
countable time.

All five sub-types count for Part 61 purposes — the regulation draws no
distinction between them — so the predicate is not a per-variant match:

```rust
pub fn counts_for_part61(recognition: Option<&FstdRecognition>) -> bool
```

**No date parameter, and no dependency on `icao-shared-kernel-rs`.** An
earlier draft of this task had the predicate take the qualification and a
date. That was redundant once snapshotting was chosen: the consumer records
the recognition *that applied at the time of the session*, checking
`DeviceQualification::is_in_force_on` against the session date as it does so.
The temporal question is answered when the snapshot is taken, not when the
predicate is asked — which is the whole point of snapshotting. Keeping the
date out also keeps this crate dependency-free, matching task `0001`'s
`resolve_classification(designator: &str, ...)`.

`RecognisedForeignState` is a closed list, defined in reg 61.010 itself (see
Verification), extensible by instrument under reg 61.047.

### Consumer snapshot (`pilot-logbook`)

**Decided: snapshot the recognition onto the logbook record at the time the
session is logged**, rather than reconstructing it from dated qualification
periods. The kernel device holds only its current qualification; the logbook
holds what was true when flown. This matches how a paper logbook works, and
avoids the kernel carrying a qualification history that only one consumer
needs.

Per the kernel's "extend before creating" guidance for downstream domains,
prefer extending `LogbookFlight` (a `session_id` alongside `flight_id`, with
a validator enforcing exactly one, plus the snapshot fields) over minting a
parallel aggregate. `EnrichedEntry` then needs a discriminated subject —
flown-aircraft vs simulated-session — so each accumulator's guard stays
explicit. That work belongs in a `pilot-logbook` task, not here.

## Verification (2026-07-26)

Sources fetched directly and grepped, not taken from summaries.

### Reg 61.010, verbatim

CASR 1998, compilation as at 2024-10-14 ([Federal Register of
Legislation](https://www.legislation.gov.au/F1998B00220/2024-10-14/2024-10-14/text/original/epub/OEBPS/document_2/document_2.html)):

> **flight simulation training device** means: (a) a qualified flight
> simulator; or (b) a qualified flight training device; or (c) a synthetic
> trainer that is approved under Civil Aviation Order 45.0; or (d) a device
> that meets the qualification standards prescribed by a legislative
> instrument under regulation 61.045; or (e) a device that is qualified
> (however described) by the national aviation authority of a recognised
> foreign State.

The old `SimType` enum covered only (a)–(c) plus the `NON_APPROVED`
sentinel; (d) and (e) had no representation at all.

### Sub-type (c) is dead letter

[AC 60-01 v2.0, *Flight simulator and flight training device qualification
(Subpart 60.B)*](https://www.casa.gov.au/flight-simulator-and-flight-training-device-qualification-subpart-60b),
July 2026 — a major rewrite following consultation between 28 January and 22
February 2026, superseding v1.1 (November 2022). Reproducing the 61.010
definition, it notes:

> "Readers are reminded that Civil Aviation Order (CAO) 45.0, although
> mentioned in this definition, is **no longer in force**. Devices formerly
> approved under this CAO are now recognised through a regulation 61.045
> instrument."

So the old `SYNTHETIC_TRAINER` variant rested on a revoked instrument, and
those devices have migrated into sub-type (d). The regulation text still
cites CAO 45.0; the AC is the only place this is flagged. Keep the variant
for historical records, documented as such.

### The definitional chain leaves Part 61

CASR Dictionary: *qualified flight simulator* means "a flight simulator that
is qualified under **Part 60** of CASR"; likewise *qualified flight training
device*. Part 60 is titled **Synthetic training devices**, and the Dictionary
defines:

> **synthetic training device** means: (a) a flight simulator; or (b) a
> flight training device; or (c) a basic instrument flight trainer.

Both device definitions open "*for a specific type (or a specific make,
model and series) of aircraft*".

### Part 60 structure

- **60.020 Qualification levels** — flight simulator: Level A, B, C, D
  (table 60.020‑1). Flight training device: FAA Level 4–7, EASA Level 1–3
  (table 60.020‑2).
- **60.035(2)** — the qualification certificate must include information
  identifying the device, **specify the aircraft that is simulated**, and
  specify the qualification level.
- **60.040** — a qualification is in force for 12 months from issue of the
  certificate, or a shorter period if the certificate specifies one.
- **60.050** — qualifications may be varied, cancelled or suspended.
- Subparts: 60.A Preliminary, 60.B Flight simulators and flight training
  devices, 60.C Basic instrument flight trainers.

### Recognised foreign States (reg 61.010)

Canada; Hong Kong; New Zealand; United States of America; the EASA member
States Belgium, Czech Republic, Denmark, Finland, France, Germany, Ireland,
Italy, Netherlands, Norway, Portugal, Spain, Sweden, Switzerland, United
Kingdom; plus any country prescribed by instrument under reg 61.047.

### ICAO Doc 9625 — cited, not read

*Manual of Criteria for the Qualification of Flight Simulation Training
Devices*, Volume I (Aeroplanes) / Volume II (Helicopters), Fourth Edition
2015. It defines **seven FSTD Types, I to VII**. AC 60-01 v2.0 lists it as a
reference and maps a flight simulator to "EASA/FAA Level A to D or ICAO Type
VII standards" and a flight training device to "EASA Level 1 to 3 or FAA
Level 4 to 7".

**Not verified against the manual itself.** Doc 9625 is sold by ICAO and is
not freely redistributable — the same copyright position as Doc 8643 in task
`0001`, so it must not be vendored here either. The Type I–VII structure
above comes from AC 60-01's references and the ICAO Store catalogue.

### ICAO Annex 1 — verified, and it does not match CASR exactly

ICAO Annex 1 (Personnel Licensing), Fourteenth Edition, July 2022, Chapter 1:

> **Flight simulation training device (FSTD).** Any one of the following three
> types of apparatus in which flight conditions are simulated on the ground:
> a **flight simulator** […]; a **flight procedures trainer**, which […]
> simulates […] the performance and flight characteristics of aircraft of a
> particular class; a **basic instrument flight trainer** […].

Two differences from CASR matter here:

1. **ICAO's FSTD is generic**; approval is separate, and left to each State's
   licensing authority. CASR reg 61.010's FSTD means specifically a
   *recognised* device. The kernel follows ICAO, so an unqualified personal
   device is an ordinary `FlightSimulationTrainingDevice` with no
   qualification — and this task's `FstdRecognition` is precisely the national
   narrowing that CASR applies on top.
2. **The middle kind is named differently.** ICAO: *flight procedures
   trainer*, defined by aircraft **class**. CASR Dictionary: *flight training
   device*, defined "for a specific type (or a specific make, model and
   series) of aircraft".

**Open question for this task**: whether a CASR *flight training device* is
the same apparatus as an ICAO *flight procedures trainer*, or merely adjacent.
The class-vs-type framing suggests they are not simply synonyms. This only
bites if something needs to map a CASA-qualified FTD onto `FstdKind`; nothing
does yet, and the `counts_for_part61` predicate below does not branch on kind.
Resolve it when a mapping is actually required.

## Sequencing

1. **`icao-shared-kernel-rs`** — task `0001`, the two aggregates. On PR #3.

2. **`au-casa`** — this task: the enum, the recognition value object, the
   `counts_for_part61` predicate.
3. **`pilot-logbook`** — snapshot fields, the `EnrichedEntry` discriminated
   subject, and rewiring the six broken call sites. Its own task.

`au-casa-py` (PyO3 bindings) remains deferred, same as task `0001`.

## Acceptance criteria

- [x] `FstdRecognition` — five variants, each doc comment citing its reg
      61.010 paragraph; `SyntheticTrainerCao45` documented as historical,
      citing AC 60-01 v2.0 on CAO 45.0's revocation.
- [x] No `NonApproved` variant; unrecognised devices are `Option::None`.
- [x] `RecognisedForeignState` — the reg 61.010 list (19 States), doc comment
      noting reg 61.047 extensibility.
- [x] `counts_for_part61(recognition) -> bool` — pure, no I/O; false when
      there is no recognition. Temporal validity is the consumer's snapshot,
      per the decision above.
- [x] Tests: one case per 61.010 sub-type, the unrecognised case, and
      serialisation round-trips.
- [x] No repository protocol, no persistence, no device identity stored in
      this crate.
- [ ] Aircraft *category* of a simulated session — the four currency
      accumulators guard on `category == AEROPLANE || is_regulatory_sim`, and
      with `FstdSession::simulated_aircraft_type` a session's category can now
      be resolved through task `0001`'s resolver like any other aircraft.
      Confirm that composes once `0001` lands; no code needed here.

## Related

- `icao-shared-kernel` task `0011` — origin of the undecided simulator home.
- `pilot-logbook` task `0007` — the consumer this blocks.
- `au-casa` task `0001` — aircraft classification; sibling concern, and the
  contrast that explains why this task has no resolver.
