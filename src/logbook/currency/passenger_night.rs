// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.395(2): carrying a passenger at night.

use logbook_shared_kernel::{CurrencyCheck, CurrencyResult, Entry, EntryAccumulator, EventWindow};
use time::Date;

use super::Movement;
use crate::aircraft::AircraftCategory;
use crate::logbook::guards::is_category;

const WINDOW_DAYS: u32 = 90;
const REQUIRED: usize = 3;

/// Reg 61.395(2): "within the previous 90 days, in an aircraft of that
/// category or an approved flight simulator for the purpose, conducted, at
/// night: (a) at least 3 take-offs; and (b) at least 3 landings", in an
/// aeroplane.
///
/// Its paragraphs are two requirements, one check for each: (a) counts each
/// entry's take-offs at night and (b) its landings at night, so a take-off
/// and a landing need not be in the same entry. A take-off is counted as the pilot
/// recorded it, which takes subregulation (3)'s "each take-off must be
/// followed by a climb to at least 500 ft AGL" as met.
pub(crate) struct PassengerNight {
    window: EventWindow,
    movement: Movement,
}

impl PassengerNight {
    pub(crate) fn new(reference: Date, movement: Movement) -> Self {
        Self {
            window: EventWindow::new(reference, WINDOW_DAYS, REQUIRED),
            movement,
        }
    }
}

impl EntryAccumulator for PassengerNight {
    fn accept_entry(&mut self, entry: &Entry) {
        if !is_category(entry, AircraftCategory::Aeroplane) {
            return;
        }
        let count = self.movement.of(entry).night;
        self.window
            .record(entry, usize::try_from(count).unwrap_or(usize::MAX));
    }
}

impl CurrencyCheck for PassengerNight {
    fn result(&self) -> CurrencyResult {
        self.window.result()
    }
}

#[cfg(test)]
mod tests {
    use logbook_shared_kernel::{EntryId, EntryParts};
    use time::macros::date;
    use time::Duration;

    use super::*;
    use crate::logbook::test_support::{aeroplane, day, entry, night, on_flight, unclassified};

    const REFERENCE: Date = date!(2024 - 09 - 01);

    /// The take-offs' result and the landings'.
    fn result(entries: &[Entry]) -> [CurrencyResult; 2] {
        [Movement::TakeOff, Movement::Landing].map(|movement| {
            let mut check = PassengerNight::new(REFERENCE, movement);
            for entry in entries {
                check.accept_entry(entry);
            }
            check.result()
        })
    }

    /// `count` take-offs and landings at night.
    fn at_night(count: u32, parts: EntryParts) -> Entry {
        entry(EntryParts {
            take_offs: night(count),
            landings: night(count),
            ..parts
        })
    }

    #[test]
    fn three_at_night_in_the_window_are_current_and_expire_90_days_on() {
        let on = date!(2024 - 08 - 20);
        for r in result(&[at_night(3, aeroplane("a", on))]) {
            assert!(r.is_current());
            assert_eq!(r.expiry_date, Some(on + Duration::days(90)));
        }
    }

    #[test]
    fn day_movements_do_not_count() {
        let e = entry(EntryParts {
            take_offs: day(3),
            landings: day(3),
            ..aeroplane("a", date!(2024 - 08 - 20))
        });
        for r in result(&[e]) {
            assert!(!r.is_current());
        }
    }

    /// "(a) at least 3 take-offs; and (b) at least 3 landings": each is
    /// counted on its own, not paired within an entry.
    #[test]
    fn night_take_offs_and_landings_are_counted_separately() {
        let e = entry(EntryParts {
            take_offs: night(5),
            landings: night(2),
            ..aeroplane("a", date!(2024 - 08 - 20))
        });
        let [take_offs, landings] = result(&[e]);
        assert!(take_offs.is_current());
        assert!(!landings.is_current());
        assert_eq!(landings.found, 2);
    }

    /// A flight flown in two capacities is two entries, with the take-offs
    /// logged under one and the landings under the other: each counts.
    #[test]
    fn a_take_off_and_a_landing_in_different_entries_both_count() {
        let on = date!(2024 - 08 - 20);
        let took_off = entry(EntryParts {
            take_offs: night(3),
            ..on_flight(1, aeroplane("a", on))
        });
        let landed = entry(EntryParts {
            landings: night(3),
            ..on_flight(1, aeroplane("b", on))
        });
        let [take_offs, landings] = result(&[took_off, landed]);
        assert!(take_offs.is_current());
        assert_eq!(take_offs.qualifying[0].entry, EntryId::new("a"));
        assert!(landings.is_current());
        assert_eq!(landings.qualifying[0].entry, EntryId::new("b"));
    }

    #[test]
    fn an_unclassified_type_does_not_count() {
        for r in result(&[at_night(3, unclassified("a", date!(2024 - 08 - 20)))]) {
            assert!(!r.is_current());
        }
    }

    #[test]
    fn the_windows_edge_is_90_days_counting_the_reference_date() {
        for inside in result(&[at_night(3, aeroplane("a", REFERENCE - Duration::days(89)))]) {
            assert!(inside.is_current());
        }

        for outside in result(&[at_night(3, aeroplane("a", REFERENCE - Duration::days(90)))]) {
            assert!(!outside.is_current());
            assert_eq!(outside.found, 0);
        }
    }

    #[test]
    fn an_entry_dated_after_the_reference_has_not_happened_yet() {
        for r in result(&[at_night(3, aeroplane("a", REFERENCE + Duration::days(1)))]) {
            assert_eq!(r.found, 0);
        }
    }
}
