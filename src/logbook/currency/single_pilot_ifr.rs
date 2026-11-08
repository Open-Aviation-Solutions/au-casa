// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.875: the recent experience of the holder of an instrument rating
//! in a single-pilot operation.

use std::collections::BTreeMap;

use icao_shared_kernel::{Capacity, FlightRules};
use logbook_shared_kernel::{
    CurrencyCheck, CurrencyResult, Entry, EntryAccumulator, EntryId, EventWindow, SubjectId,
};
use time::Date;

/// "Within the previous 6 months" (reg 61.875(1)), in calendar months.
const WINDOW_MONTHS: u32 = 6;
const REQUIRED: usize = 1;
/// "A duration of at least one hour" (reg 61.875(2)(a)), in minutes.
const MIN_DURATION_MINUTES: i64 = 60;

/// One flight or session, from every entry the pilot logged it in.
#[derive(Debug)]
struct Logged {
    /// Whether every entry recorded it as flown under the IFR throughout.
    ifr: bool,
    /// Whether any entry was logged as co-pilot.
    co_pilot: bool,
    /// Its duration: the flight time of all its entries.
    minutes: i64,
    /// Its date: the latest of its entries' dates, by which all of it had
    /// been flown.
    on: Date,
    /// The entry it is reported by: of those recording an approach, the one
    /// with the lowest id, so that it does not depend on the order entries
    /// arrive in.
    with_approach: Option<EntryId>,
}

/// Reg 61.875(1): "a flight or simulated flight under the IFR in a
/// single-pilot operation within the previous 6 months", which must "(a)
/// have a duration of at least one hour; and (b) include at least one
/// instrument approach or simulated instrument approach" (reg 61.875(2)).
///
/// As pilot-logbook read it, kept for the move:
///
/// - **under the IFR**: every entry for it records it as flown under the
///   IFR throughout. One recorded as changing rules does not count, since
///   how long it was under the IFR is not recorded, nor does one with no
///   flight rules recorded;
/// - **a single-pilot operation** ("an operation in an aircraft, other than
///   a multi-crew operation", reg 61.010): no entry for it was logged as
///   co-pilot. Multi-crew operations are not recorded otherwise;
/// - **at least one hour**: the flight time of all the pilot's entries for
///   it, since one flown in two capacities has two;
/// - **an approach**: recorded on one of those entries.
///
/// Each is decided for the flight or session, grouped by the kernel's
/// `SubjectId` (which keeps a flight and a session of the same id apart),
/// once all its entries are in, and it is dated by the latest of them. An entry dated
/// after the reference date has not happened yet, so it is left out before
/// grouping: it neither adds to a flight nor disqualifies one. In any capacity but co-pilot, and in any category of
/// aircraft: reg 61.875 names none.
pub(crate) struct SinglePilotIfr {
    reference: Date,
    logged: BTreeMap<SubjectId, Logged>,
}

impl SinglePilotIfr {
    pub(crate) fn new(reference: Date) -> Self {
        Self {
            reference,
            logged: BTreeMap::new(),
        }
    }
}

impl EntryAccumulator for SinglePilotIfr {
    fn accept_entry(&mut self, entry: &Entry) {
        if entry.date() > self.reference {
            return;
        }
        let logged = self.logged.entry(entry.subject().id()).or_insert(Logged {
            ifr: true,
            co_pilot: false,
            minutes: 0,
            on: entry.date(),
            with_approach: None,
        });
        logged.ifr &= entry.flight_rules() == Some(FlightRules::Ifr);
        logged.co_pilot |= entry.capacity() == Capacity::CoPilot;
        logged.minutes = logged
            .minutes
            .saturating_add(entry.flight_time().total_minutes());
        logged.on = logged.on.max(entry.date());
        let reports = logged
            .with_approach
            .as_ref()
            .is_none_or(|other| entry.id() < other);
        if reports && !entry.approaches().is_empty() {
            logged.with_approach = Some(entry.id().clone());
        }
    }
}

impl CurrencyCheck for SinglePilotIfr {
    fn result(&self) -> CurrencyResult {
        let mut window = EventWindow::months(self.reference, WINDOW_MONTHS, REQUIRED);
        for logged in self.logged.values() {
            if !logged.ifr || logged.co_pilot || logged.minutes < MIN_DURATION_MINUTES {
                continue;
            }
            if let Some(entry) = &logged.with_approach {
                window.record_on(entry, logged.on, 1);
            }
        }
        window.result()
    }
}

#[cfg(test)]
mod tests {
    use icao_shared_kernel::FlightDuration;
    use logbook_shared_kernel::{ApproachType, EntryParts, Subject};
    use time::macros::date;
    use uuid::Uuid;

    use super::*;
    use crate::logbook::test_support::{aeroplane, approach, entry, sailplane, simulated};

    const REFERENCE: Date = date!(2026 - 09 - 28);
    const ON: Date = date!(2026 - 06 - 01);

    fn minutes(minutes: i64) -> FlightDuration {
        FlightDuration::new(minutes).unwrap()
    }

    /// An hour under the IFR with a VOR approach, in flight `flight`.
    fn qualifying(id: &str, on: Date, flight: u128) -> EntryParts {
        let mut parts = EntryParts {
            flight_rules: Some(FlightRules::Ifr),
            approaches: vec![approach(ApproachType::Vor)],
            ..aeroplane(id, on)
        };
        if let Subject::Flight { flight: id, .. } = &mut parts.subject {
            *id = Uuid::from_u128(flight);
        }
        parts
    }

    /// Another entry, `id`, logging the same flight as `parts`, with no
    /// approach.
    fn also_logging(parts: &EntryParts, id: &str) -> EntryParts {
        EntryParts {
            id: logbook_shared_kernel::EntryId::new(id),
            approaches: Vec::new(),
            ..parts.clone()
        }
    }

    fn result(entries: &[EntryParts]) -> CurrencyResult {
        let mut check = SinglePilotIfr::new(REFERENCE);
        for parts in entries {
            check.accept_entry(&entry(parts.clone()));
        }
        check.result()
    }

    #[test]
    fn a_qualifying_flight_is_recent_through_the_day_before_the_same_date_6_months_on() {
        let r = result(&[qualifying("a", date!(2025 - 12 - 09), 1)]);
        assert_eq!(r.current_through(), Some(date!(2026 - 06 - 08)));
        assert!(!r.is_current());

        let r = result(&[qualifying("a", date!(2026 - 03 - 31), 1)]);
        assert_eq!(r.current_through(), Some(date!(2026 - 09 - 30)));
        assert!(r.is_current());
        assert_eq!(r.found, 1);
    }

    #[test]
    fn the_windows_edge_is_6_calendar_months_before_the_reference_date() {
        let inside = result(&[qualifying("a", date!(2026 - 03 - 29), 1)]);
        assert!(inside.is_current());
        assert_eq!(inside.found, 1);

        let outside = result(&[qualifying("a", date!(2026 - 03 - 28), 1)]);
        assert!(!outside.is_current());
        assert_eq!(outside.found, 0);
    }

    #[test]
    fn only_a_flight_recorded_as_ifr_throughout_counts() {
        for rules in [
            None,
            Some(FlightRules::Vfr),
            Some(FlightRules::InitiallyIfr),
            Some(FlightRules::InitiallyVfr),
        ] {
            let e = EntryParts {
                flight_rules: rules,
                ..qualifying("a", ON, 1)
            };
            assert!(!result(&[e]).is_current(), "{rules:?}");
        }
    }

    #[test]
    fn any_capacity_but_co_pilot_counts() {
        for capacity in [
            Capacity::PilotInCommand,
            Capacity::PilotInCommandUnderSupervision,
            Capacity::DualInstruction,
        ] {
            let e = EntryParts {
                capacity,
                ..qualifying("a", ON, 1)
            };
            assert!(result(&[e]).is_current(), "{capacity:?}");
        }
        let e = EntryParts {
            capacity: Capacity::CoPilot,
            ..qualifying("a", ON, 1)
        };
        assert!(!result(&[e]).is_current());
    }

    #[test]
    fn a_flight_with_an_entry_as_co_pilot_was_not_single_pilot() {
        let pic = qualifying("a", ON, 1);
        let co_pilot = EntryParts {
            capacity: Capacity::CoPilot,
            ..also_logging(&pic, "b")
        };
        // Whichever order they arrive in.
        assert!(!result(&[pic.clone(), co_pilot.clone()]).is_current());
        assert!(!result(&[co_pilot.clone(), pic.clone()]).is_current());

        // However its entries record the flight rules.
        let unrecorded = EntryParts {
            flight_rules: None,
            ..co_pilot
        };
        assert!(!result(&[pic.clone(), unrecorded.clone()]).is_current());
        assert!(!result(&[unrecorded, pic]).is_current());
    }

    #[test]
    fn a_flight_is_under_the_ifr_only_if_every_entry_for_it_says_so() {
        let ifr = qualifying("a", ON, 1);
        let vfr = EntryParts {
            flight_rules: Some(FlightRules::Vfr),
            ..also_logging(&ifr, "b")
        };
        assert!(!result(&[ifr.clone(), vfr.clone()]).is_current());
        assert!(!result(&[vfr, ifr]).is_current());
    }

    #[test]
    fn another_flights_entries_do_not_change_it() {
        let ifr = qualifying("a", ON, 1);
        let other = EntryParts {
            capacity: Capacity::CoPilot,
            flight_rules: Some(FlightRules::Vfr),
            ..qualifying("b", ON, 2)
        };
        let r = result(&[ifr, other]);
        assert!(r.is_current());
        assert_eq!(r.found, 1);
    }

    #[test]
    fn a_flight_of_less_than_an_hour_does_not_count() {
        let short = EntryParts {
            flight_time: minutes(59),
            ..qualifying("a", ON, 1)
        };
        assert!(!result(&[short]).is_current());
    }

    #[test]
    fn a_flight_logged_in_two_capacities_lasts_as_long_as_both_entries() {
        let supervised = EntryParts {
            capacity: Capacity::PilotInCommandUnderSupervision,
            flight_time: minutes(40),
            ..qualifying("a", ON, 1)
        };
        let in_command = EntryParts {
            capacity: Capacity::PilotInCommand,
            flight_time: minutes(20),
            ..also_logging(&supervised, "b")
        };
        assert!(!result(std::slice::from_ref(&supervised)).is_current());
        // Whichever order they arrive in.
        for entries in [
            [in_command.clone(), supervised.clone()],
            [supervised.clone(), in_command.clone()],
        ] {
            let r = result(&entries);
            assert!(r.is_current());
            // The flight is one event, reported by the entry with the approach.
            assert_eq!(r.found, 1);
            assert_eq!(r.qualifying.len(), 1);
            assert_eq!(r.qualifying[0].entry.as_str(), "a");
        }

        // With an approach on both, by the lower id, whichever order they
        // arrive in.
        let both = EntryParts {
            approaches: vec![approach(ApproachType::Ndb)],
            ..in_command
        };
        for entries in [[supervised.clone(), both.clone()], [both, supervised]] {
            assert_eq!(result(&entries).qualifying[0].entry.as_str(), "a");
        }
    }

    #[test]
    fn a_flight_is_dated_by_its_latest_entry_whichever_reports_it() {
        // A flight across midnight, logged in two entries.
        let before = EntryParts {
            flight_time: minutes(30),
            ..qualifying("a", date!(2026 - 03 - 28), 1)
        };
        let after = EntryParts {
            date: date!(2026 - 03 - 29),
            ..also_logging(&before, "b")
        };
        for entries in [[before.clone(), after.clone()], [after, before]] {
            let r = result(&entries);
            assert!(r.is_current());
            assert_eq!(r.current_through(), Some(date!(2026 - 09 - 28)));
            assert_eq!(r.qualifying[0].entry.as_str(), "a");
        }
    }

    #[test]
    fn an_entry_dated_after_the_reference_is_left_out_of_its_flight() {
        let in_window = qualifying("b", ON, 1);
        let future = |parts: EntryParts| EntryParts {
            id: logbook_shared_kernel::EntryId::new("a"),
            date: REFERENCE + time::Duration::days(1),
            ..parts
        };
        // It neither reports the flight, though its id is the lowest,
        let reporting = future(in_window.clone());
        // nor disqualifies it, as co-pilot or under the VFR.
        let co_pilot = future(EntryParts {
            capacity: Capacity::CoPilot,
            ..in_window.clone()
        });
        let vfr = future(EntryParts {
            flight_rules: Some(FlightRules::Vfr),
            ..in_window.clone()
        });
        for other in [reporting, co_pilot, vfr] {
            for entries in [
                [in_window.clone(), other.clone()],
                [other, in_window.clone()],
            ] {
                let r = result(&entries);
                assert!(r.is_current());
                assert_eq!(r.qualifying[0].entry.as_str(), "b");
                assert_eq!(r.current_through(), Some(date!(2026 - 11 - 30)));
            }
        }
    }

    #[test]
    fn a_flight_without_an_approach_does_not_count() {
        let e = EntryParts {
            approaches: Vec::new(),
            ..qualifying("a", ON, 1)
        };
        assert!(!result(&[e]).is_current());
    }

    /// "Or simulated flight": whether the device was approved is
    /// `Casa::counts`'.
    #[test]
    fn a_session_and_any_category_count() {
        let session = EntryParts {
            flight_rules: Some(FlightRules::Ifr),
            approaches: vec![approach(ApproachType::Ils)],
            ..simulated("s", ON, "PA44")
        };
        assert!(result(&[session]).is_current());

        let sailplane = EntryParts {
            subject: sailplane("g", ON).subject,
            ..qualifying("g", ON, 1)
        };
        assert!(result(&[sailplane]).is_current());
    }

    #[test]
    fn a_session_is_not_grouped_with_a_flight_of_the_same_id() {
        // Both have the nil id.
        let flight = qualifying("a", ON, 0);
        let session = EntryParts {
            capacity: Capacity::CoPilot,
            ..simulated("s", ON, "PA44")
        };
        assert!(result(&[flight, session]).is_current());
    }

    #[test]
    fn an_entry_dated_after_the_reference_has_not_happened_yet() {
        let future = qualifying("a", date!(2026 - 09 - 29), 1);
        assert_eq!(result(&[future]).found, 0);
    }
}
