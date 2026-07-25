# Simulator classification (SimType, is_regulatory_sim)

**Status:** proposal — not yet designed, placeholder so it isn't lost

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
question `0001` doesn't have, below.

## The open question

Under the new domain (`icao-shared-kernel-rs`), a simulator has no ICAO Doc
8643 type designator — task `0011`'s own reasoning: *"a simulator has no
ICAO identity... sims do not become a kernel aggregate."* So unlike aircraft
classification (a resolver keyed by an `Aircraft`'s designator), simulator
classification can't hang off a kernel `Aircraft` at all — there may not be
one.

Today, `pilot-logbook` reads `entry.aircraft.sim_type` /
`entry.aircraft.is_regulatory_sim` — i.e. it currently models a sim session
as if it flew an `Aircraft`. That has to change, but *how* is a
`pilot-logbook`-side (or possibly `icao-shared-kernel-rs`-side, if there's a
case for some ICAO-agnostic "flown device" concept) modelling decision, not
something this package can resolve on its own. Needs its own discussion
before any resolver design here makes sense.

## Verification note (2026-07-25, found while checking task 0001)

Reg 61.010's definition of "flight simulation training device" has 5
sub-types: (a) qualified flight simulator, (b) qualified flight training
device, (c) synthetic trainer approved under CAO 45.0, (d) a device meeting
qualification standards prescribed under reg 61.045, (e) a device qualified
by a recognised foreign State's national aviation authority. The old
`SimType` enum only covers (a)-(c) plus a non-regulatory `NON_APPROVED`
sentinel — (d) and (e) have no representation. Not investigated further here;
flagging so it's not lost once this task's modelling question is resolved
and the enum itself gets designed.

## Related

- `icao-shared-kernel` task `0011` — origin of the undecided simulator home.
- `pilot-logbook` task `0007` — the consumer this blocks.
- `au-casa` task `0001` — aircraft classification, the sibling concern that
  doesn't have this modelling problem.
