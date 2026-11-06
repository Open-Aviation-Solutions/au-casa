// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.395(1): carrying a passenger by day.

use logbook_shared_kernel::{CurrencyCheck, CurrencyResult, Entry, EntryAccumulator, EventWindow};
use time::Date;

use crate::aircraft::AircraftCategory;
use crate::logbook::guards::is_category;

const WINDOW_DAYS: u32 = 90;
const REQUIRED: usize = 3;

/// Reg 61.395(1): "within the previous 90 days, in an aircraft of that
/// category or an approved flight simulator for the purpose, conducted, by
/// day or night: (a) at least 3 take-offs; and (b) at least 3 landings", in
/// an aeroplane.
///
/// Each entry supplies the lesser of its take-offs and its landings, by day
/// and night together.
pub(crate) struct PassengerDay {
    window: EventWindow,
}

impl PassengerDay {
    pub(crate) fn new(reference: Date) -> Self {
        Self {
            window: EventWindow::new(reference, WINDOW_DAYS, REQUIRED),
        }
    }
}

impl EntryAccumulator for PassengerDay {
    fn accept_entry(&mut self, entry: &Entry) {
        if !is_category(entry, AircraftCategory::Aeroplane) {
            return;
        }
        let pairs = entry.take_offs().total().min(entry.landings().total());
        // One entry supplies at most the events the check needs.
        let pairs = usize::try_from(pairs).map_or(REQUIRED, |pairs| pairs.min(REQUIRED));
        self.window.record(entry, pairs);
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
        aeroplane, day, entry, night, sailplane, simulated, unclassified,
    };

    const REFERENCE: Date = date!(2024 - 09 - 01);

    fn result(entries: &[Entry]) -> CurrencyResult {
        let mut check = PassengerDay::new(REFERENCE);
        for entry in entries {
            check.accept_entry(entry);
        }
        check.result()
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
    fn three_pairs_in_the_window_are_current_and_expire_90_days_on() {
        let on = date!(2024 - 08 - 20);
        let r = result(&[by_day(3, aeroplane("a", on))]);
        assert!(r.is_current());
        assert_eq!(r.found, 3);
        assert_eq!(r.expiry_date, Some(on + Duration::days(90)));
        // Current for 90 days counting the day of the flight: the 90th is the
        // last, and expiry_date is the day after it.
        assert_eq!(r.current_through(), Some(on + Duration::days(89)));
        assert_eq!(r.look_back_days, 90);
        // The one flight supplied all three pairs.
        assert_eq!(
            r.qualifying,
            [Contribution {
                entry: EntryId::new("a"),
                events: 3
            }]
        );
    }

    #[test]
    fn only_two_pairs_is_not_current_and_needs_one_more() {
        let r = result(&[
            by_day(1, aeroplane("a", date!(2024 - 08 - 20))),
            by_day(1, aeroplane("b", date!(2024 - 08 - 15))),
        ]);
        assert!(!r.is_current());
        assert_eq!(r.found, 2);
        assert_eq!(r.needed(), 1);
    }

    #[test]
    fn expired_currency_still_reports_a_past_expiry_date() {
        let r = result(&[
            by_day(1, aeroplane("a", date!(2024 - 05 - 28))),
            by_day(1, aeroplane("b", date!(2024 - 05 - 25))),
            by_day(1, aeroplane("c", date!(2024 - 05 - 22))),
        ]);
        assert!(!r.is_current());
        assert_eq!(r.found, 0);
        assert!(r.expiry_date.is_some_and(|expiry| expiry < REFERENCE));
    }

    #[test]
    fn expiry_is_none_when_the_pilot_has_never_had_enough() {
        let r = result(&[by_day(1, aeroplane("a", date!(2024 - 08 - 20)))]);
        assert_eq!(r.expiry_date, None);
        assert_eq!(result(&[]).expiry_date, None);
    }

    #[test]
    fn a_known_sailplane_does_not_count() {
        let r = result(&[by_day(3, sailplane("a", date!(2024 - 08 - 20)))]);
        assert!(!r.is_current());
    }

    #[test]
    fn an_unclassified_type_does_not_count() {
        let r = result(&[by_day(3, unclassified("a", date!(2024 - 08 - 20)))]);
        assert!(!r.is_current());
    }

    /// "In an aircraft of that category or an approved flight simulator for
    /// the purpose": whether the device was approved is `Casa::counts`'.
    #[test]
    fn a_session_representing_an_aeroplane_counts() {
        let r = result(&[by_day(3, simulated("a", date!(2024 - 08 - 20), "C172"))]);
        assert!(r.is_current());
    }

    #[test]
    fn the_binding_constraint_is_the_lesser_of_take_offs_and_landings() {
        // 5 take-offs, only 2 landings: only 2 pairs, so 3 is not reached.
        let e = entry(EntryParts {
            take_offs: day(5),
            landings: day(2),
            ..aeroplane("a", date!(2024 - 08 - 20))
        });
        assert!(!result(&[e]).is_current());
    }

    #[test]
    fn night_movements_count_towards_the_day_requirement() {
        // Reg 61.395(1): "by day or night".
        let e = entry(EntryParts {
            take_offs: night(3),
            landings: night(3),
            ..aeroplane("a", date!(2024 - 08 - 20))
        });
        assert!(result(&[e]).is_current());
    }

    /// "Within the previous 90 days": the 90th day back from the reference
    /// is the last inside the window.
    #[test]
    fn the_windows_edge_is_90_days_counting_the_reference_date() {
        let inside = result(&[by_day(3, aeroplane("a", REFERENCE - Duration::days(89)))]);
        assert!(inside.is_current());
        assert_eq!(inside.found, 3);

        let outside = result(&[by_day(3, aeroplane("a", REFERENCE - Duration::days(90)))]);
        assert!(!outside.is_current());
        assert_eq!(outside.found, 0);
        assert_eq!(outside.expiry_date, Some(REFERENCE));
    }

    #[test]
    fn an_entry_supplies_at_most_the_three_pairs_needed() {
        let e = entry(EntryParts {
            take_offs: day(u32::MAX),
            landings: night(u32::MAX),
            ..aeroplane("a", date!(2024 - 08 - 20))
        });
        let r = result(&[e]);
        assert!(r.is_current());
        assert_eq!(r.found, 3);
    }

    #[test]
    fn an_entry_dated_after_the_reference_has_not_happened_yet() {
        let r = result(&[by_day(3, aeroplane("a", REFERENCE + Duration::days(1)))]);
        assert_eq!(r.found, 0);
        assert_eq!(r.qualifying, []);
        assert_eq!(r.expiry_date, None);

        // The reference date itself has "happened" and counts.
        assert!(result(&[by_day(3, aeroplane("a", REFERENCE))]).is_current());
    }
}
