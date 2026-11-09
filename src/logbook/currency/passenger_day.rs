// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.395(1): carrying a passenger by day.

use logbook_shared_kernel::{CurrencyCheck, CurrencyResult, Entry, EntryAccumulator, EventWindow};
use time::Date;

use super::Movement;
use crate::aircraft::AircraftCategory;
use crate::logbook::guards::is_category;

const WINDOW_DAYS: u32 = 90;
const REQUIRED: usize = 3;

/// Reg 61.395(1): "within the previous 90 days, in an aircraft of that
/// category or an approved flight simulator for the purpose, conducted, by
/// day or night: (a) at least 3 take-offs; and (b) at least 3 landings", in
/// an aeroplane.
///
/// Its paragraphs are two requirements, one check for each: (a) counts each
/// entry's take-offs and (b) its landings, by day and night together, so a
/// take-off and a landing need not be in the same entry. A take-off is
/// counted as the pilot recorded it, which takes subregulation (3)'s "each
/// take-off must be followed by a climb to at least 500 ft AGL" as met.
pub(crate) struct PassengerDay {
    window: EventWindow,
    movement: Movement,
}

impl PassengerDay {
    pub(crate) fn new(reference: Date, movement: Movement) -> Self {
        Self {
            window: EventWindow::new(reference, WINDOW_DAYS, REQUIRED),
            movement,
        }
    }
}

impl EntryAccumulator for PassengerDay {
    fn accept_entry(&mut self, entry: &Entry) {
        if !is_category(entry, AircraftCategory::Aeroplane) {
            return;
        }
        let count = self.movement.of(entry).total();
        self.window
            .record(entry, usize::try_from(count).unwrap_or(usize::MAX));
    }
}

impl CurrencyCheck for PassengerDay {
    fn result(&self) -> CurrencyResult {
        self.window.result()
    }
}

#[cfg(test)]
mod tests {
    use logbook_shared_kernel::{Contribution, EntryId, EntryParts};
    use time::macros::date;
    use time::Duration;

    use super::*;
    use crate::logbook::test_support::{
        aeroplane, day, entry, night, on_flight, sailplane, simulated, unclassified,
    };

    const REFERENCE: Date = date!(2024 - 09 - 01);

    /// The take-offs' result and the landings'.
    fn result(entries: &[Entry]) -> [CurrencyResult; 2] {
        [Movement::TakeOff, Movement::Landing].map(|movement| {
            let mut check = PassengerDay::new(REFERENCE, movement);
            for entry in entries {
                check.accept_entry(entry);
            }
            check.result()
        })
    }

    /// `count` take-offs and landings by day.
    fn by_day(count: u32, parts: EntryParts) -> Entry {
        entry(EntryParts {
            take_offs: day(count),
            landings: day(count),
            ..parts
        })
    }

    #[test]
    fn three_in_the_window_are_current_and_expire_90_days_on() {
        let on = date!(2024 - 08 - 20);
        for r in result(&[by_day(3, aeroplane("a", on))]) {
            assert!(r.is_current());
            assert_eq!(r.found, 3);
            assert_eq!(r.expiry_date, Some(on + Duration::days(90)));
            // Current for 90 days counting the day of the flight: the 90th
            // is the last, and expiry_date is the day after it.
            assert_eq!(r.current_through(), Some(on + Duration::days(89)));
            assert_eq!(r.look_back_days, 90);
            // The one flight supplied all three.
            assert_eq!(
                r.qualifying,
                [Contribution {
                    entry: EntryId::new("a"),
                    events: 3
                }]
            );
        }
    }

    #[test]
    fn only_two_is_not_current_and_needs_one_more() {
        for r in result(&[
            by_day(1, aeroplane("a", date!(2024 - 08 - 20))),
            by_day(1, aeroplane("b", date!(2024 - 08 - 15))),
        ]) {
            assert!(!r.is_current());
            assert_eq!(r.found, 2);
            assert_eq!(r.needed(), 1);
        }
    }

    #[test]
    fn expired_currency_still_reports_a_past_expiry_date() {
        for r in result(&[
            by_day(1, aeroplane("a", date!(2024 - 05 - 28))),
            by_day(1, aeroplane("b", date!(2024 - 05 - 25))),
            by_day(1, aeroplane("c", date!(2024 - 05 - 22))),
        ]) {
            assert!(!r.is_current());
            assert_eq!(r.found, 0);
            assert!(r.expiry_date.is_some_and(|expiry| expiry < REFERENCE));
        }
    }

    #[test]
    fn expiry_is_none_when_the_pilot_has_never_had_enough() {
        for r in result(&[by_day(1, aeroplane("a", date!(2024 - 08 - 20)))]) {
            assert_eq!(r.expiry_date, None);
        }
        for r in result(&[]) {
            assert_eq!(r.expiry_date, None);
        }
    }

    #[test]
    fn a_known_sailplane_does_not_count() {
        for r in result(&[by_day(3, sailplane("a", date!(2024 - 08 - 20)))]) {
            assert!(!r.is_current());
        }
    }

    #[test]
    fn an_unclassified_type_does_not_count() {
        for r in result(&[by_day(3, unclassified("a", date!(2024 - 08 - 20)))]) {
            assert!(!r.is_current());
        }
    }

    /// "In an aircraft of that category or an approved flight simulator for
    /// the purpose": whether the device was approved is `Casa::counts`'.
    #[test]
    fn a_session_representing_an_aeroplane_counts() {
        for r in result(&[by_day(3, simulated("a", date!(2024 - 08 - 20), "C172"))]) {
            assert!(r.is_current());
        }
    }

    /// "(a) at least 3 take-offs; and (b) at least 3 landings": each is
    /// counted on its own, not paired within an entry.
    #[test]
    fn take_offs_and_landings_are_counted_separately() {
        let e = entry(EntryParts {
            take_offs: day(5),
            landings: day(2),
            ..aeroplane("a", date!(2024 - 08 - 20))
        });
        let [take_offs, landings] = result(&[e]);
        assert!(take_offs.is_current());
        assert_eq!(take_offs.found, 5);
        assert!(!landings.is_current());
        assert_eq!(landings.found, 2);
    }

    /// A flight flown in two capacities is two entries, with the take-offs
    /// logged under one and the landings under the other: each counts.
    #[test]
    fn a_take_off_and_a_landing_in_different_entries_both_count() {
        let on = date!(2024 - 08 - 20);
        let took_off = entry(EntryParts {
            take_offs: day(3),
            ..on_flight(1, aeroplane("a", on))
        });
        let landed = entry(EntryParts {
            landings: day(3),
            ..on_flight(1, aeroplane("b", on))
        });
        let [take_offs, landings] = result(&[took_off, landed]);
        assert!(take_offs.is_current());
        assert_eq!(take_offs.qualifying[0].entry, EntryId::new("a"));
        assert!(landings.is_current());
        assert_eq!(landings.qualifying[0].entry, EntryId::new("b"));
    }

    #[test]
    fn night_movements_count_towards_the_day_requirement() {
        // Reg 61.395(1): "by day or night".
        let e = entry(EntryParts {
            take_offs: night(3),
            landings: night(3),
            ..aeroplane("a", date!(2024 - 08 - 20))
        });
        for r in result(&[e]) {
            assert!(r.is_current());
        }
    }

    /// "Within the previous 90 days": the 90th day back from the reference
    /// is the last inside the window.
    #[test]
    fn the_windows_edge_is_90_days_counting_the_reference_date() {
        for inside in result(&[by_day(3, aeroplane("a", REFERENCE - Duration::days(89)))]) {
            assert!(inside.is_current());
            assert_eq!(inside.found, 3);
        }

        for outside in result(&[by_day(3, aeroplane("a", REFERENCE - Duration::days(90)))]) {
            assert!(!outside.is_current());
            assert_eq!(outside.found, 0);
            assert_eq!(outside.expiry_date, Some(REFERENCE));
        }
    }

    #[test]
    fn an_entry_reports_every_movement_it_records_by_day_and_night() {
        let e = entry(EntryParts {
            take_offs: day(5),
            landings: night(5),
            ..aeroplane("a", date!(2024 - 08 - 20))
        });
        for r in result(&[e]) {
            assert!(r.is_current());
            assert_eq!(r.found, 5);
        }
    }

    #[test]
    fn an_entry_dated_after_the_reference_has_not_happened_yet() {
        for r in result(&[by_day(3, aeroplane("a", REFERENCE + Duration::days(1)))]) {
            assert_eq!(r.found, 0);
            assert_eq!(r.qualifying, []);
            assert_eq!(r.expiry_date, None);
        }

        // The reference date itself has "happened" and counts.
        for r in result(&[by_day(3, aeroplane("a", REFERENCE))]) {
            assert!(r.is_current());
        }
    }
}
