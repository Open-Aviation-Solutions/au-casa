// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Regs 61.395(1) and (2), carrying a passenger by day and at night, and
//! reg 61.965(a), the recent experience of the holder of a night VFR
//! rating: each requires take-offs and landings, counted separately, so
//! each is two requirements, one for its take-offs and one for its
//! landings. The checks differ only in their figures, so they are one.

use logbook_shared_kernel::{
    CurrencyCheck, CurrencyResult, DayNight, Entry, EntryAccumulator, EventWindow,
};
use time::Date;

use crate::aircraft::AircraftCategory;
use crate::logbook::guards::is_category;

/// "Within the previous 90 days" (reg 61.395(1) and (2)).
const PASSENGER_WINDOW_DAYS: u32 = 90;
const PASSENGER_REQUIRED: usize = 3;

/// "Within the previous 6 months" (reg 61.965), as pilot-logbook's 183 days
/// (6 × 30.5), kept for the move.
const NIGHT_VFR_WINDOW_DAYS: u32 = 183;
const NIGHT_VFR_REQUIRED: usize = 1;

/// The take-offs or the landings a requirement counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TakeOffsOrLandings {
    TakeOffs,
    Landings,
}

/// Both, take-offs first, as the regulations list them.
pub(crate) const TAKE_OFFS_AND_LANDINGS: [TakeOffsOrLandings; 2] =
    [TakeOffsOrLandings::TakeOffs, TakeOffsOrLandings::Landings];

impl TakeOffsOrLandings {
    /// One of them, in Part 61's words: "take-off" or "landing".
    pub(crate) fn one(self) -> &'static str {
        match self {
            Self::TakeOffs => "take-off",
            Self::Landings => "landing",
        }
    }

    /// The entry's take-offs or landings, by day and at night.
    fn of(self, entry: &Entry) -> DayNight<u32> {
        match self {
            Self::TakeOffs => entry.take_offs(),
            Self::Landings => entry.landings(),
        }
    }
}

/// The take-offs or the landings one of these regulations requires, in an
/// aeroplane. A take-off and a landing need not be in the same entry. A
/// take-off is counted as the pilot recorded it, which takes reg
/// 61.395(3)'s "each take-off must be followed by a climb to at least 500
/// ft AGL" as met.
pub(crate) struct TakeOffsAndLandings {
    window: EventWindow,
    counts: TakeOffsOrLandings,
    at_night_only: bool,
}

impl TakeOffsAndLandings {
    /// Reg 61.395(1): "within the previous 90 days, in an aircraft of that
    /// category or an approved flight simulator for the purpose, conducted,
    /// by day or night: (a) at least 3 take-offs; and (b) at least 3
    /// landings".
    pub(crate) fn passenger_day(reference: Date, counts: TakeOffsOrLandings) -> Self {
        let window = EventWindow::new(reference, PASSENGER_WINDOW_DAYS, PASSENGER_REQUIRED);
        Self::new(window, counts, false)
    }

    /// Reg 61.395(2): "within the previous 90 days, in an aircraft of that
    /// category or an approved flight simulator for the purpose, conducted,
    /// at night: (a) at least 3 take-offs; and (b) at least 3 landings".
    pub(crate) fn passenger_night(reference: Date, counts: TakeOffsOrLandings) -> Self {
        let window = EventWindow::new(reference, PASSENGER_WINDOW_DAYS, PASSENGER_REQUIRED);
        Self::new(window, counts, true)
    }

    /// Reg 61.965(a): "within the previous 6 months … (i) at least one
    /// night take-off; and (ii) at least one night landing; in an aircraft
    /// of that category while controlling the aircraft".
    pub(crate) fn night_vfr(reference: Date, counts: TakeOffsOrLandings) -> Self {
        let window = EventWindow::new(reference, NIGHT_VFR_WINDOW_DAYS, NIGHT_VFR_REQUIRED);
        Self::new(window, counts, true)
    }

    fn new(window: EventWindow, counts: TakeOffsOrLandings, at_night_only: bool) -> Self {
        Self {
            window,
            counts,
            at_night_only,
        }
    }

    /// Whether it counts take-offs or landings.
    pub(crate) fn counts(&self) -> TakeOffsOrLandings {
        self.counts
    }
}

impl EntryAccumulator for TakeOffsAndLandings {
    fn accept_entry(&mut self, entry: &Entry) {
        if !is_category(entry, AircraftCategory::Aeroplane) {
            return;
        }
        let recorded = self.counts.of(entry);
        let count = if self.at_night_only {
            u64::from(recorded.night)
        } else {
            recorded.total()
        };
        let count = usize::try_from(count).unwrap_or(usize::MAX);
        self.window.record(entry, count);
    }
}

impl CurrencyCheck for TakeOffsAndLandings {
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

    type Check = fn(Date, TakeOffsOrLandings) -> TakeOffsAndLandings;

    /// The take-offs' result and the landings'.
    fn result(check: Check, entries: &[Entry]) -> [CurrencyResult; 2] {
        TAKE_OFFS_AND_LANDINGS.map(|counts| {
            let mut check = check(REFERENCE, counts);
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

    /// `count` take-offs and landings at night.
    fn at_night(count: u32, parts: EntryParts) -> Entry {
        entry(EntryParts {
            take_offs: night(count),
            landings: night(count),
            ..parts
        })
    }

    /// The take-offs in one entry of flight 1 and the landings in another,
    /// as for a flight flown in two capacities.
    fn split(take_offs: DayNight<u32>, landings: DayNight<u32>, on: Date) -> [Entry; 2] {
        [
            entry(EntryParts {
                take_offs,
                ..on_flight(1, aeroplane("a", on))
            }),
            entry(EntryParts {
                landings,
                ..on_flight(1, aeroplane("b", on))
            }),
        ]
    }

    /// Each counts its own entry of [`split`]'s, and is current.
    fn assert_each_counts_its_own_entry([take_offs, landings]: [CurrencyResult; 2]) {
        assert!(take_offs.is_current());
        assert_eq!(take_offs.qualifying[0].entry, EntryId::new("a"));
        assert!(landings.is_current());
        assert_eq!(landings.qualifying[0].entry, EntryId::new("b"));
    }

    mod passenger_day {
        use super::*;

        fn result(entries: &[Entry]) -> [CurrencyResult; 2] {
            super::result(TakeOffsAndLandings::passenger_day, entries)
        }

        #[test]
        fn three_in_the_window_are_current_and_expire_90_days_on() {
            let on = date!(2024 - 08 - 20);
            for r in result(&[by_day(3, aeroplane("a", on))]) {
                assert!(r.is_current());
                assert_eq!(r.found, 3);
                assert_eq!(r.expiry_date, Some(on + Duration::days(90)));
                // Current for 90 days counting the day of the flight: the
                // 90th is the last, and expiry_date is the day after it.
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

        /// "In an aircraft of that category or an approved flight simulator
        /// for the purpose": whether the device was approved is
        /// `Casa::counts`'.
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

        #[test]
        fn a_take_off_and_a_landing_in_different_entries_both_count() {
            let on = date!(2024 - 08 - 20);
            assert_each_counts_its_own_entry(result(&split(day(3), day(3), on)));
        }

        #[test]
        fn night_take_offs_and_landings_count_towards_the_day_requirement() {
            // Reg 61.395(1): "by day or night".
            for r in result(&[at_night(3, aeroplane("a", date!(2024 - 08 - 20)))]) {
                assert!(r.is_current());
            }
        }

        /// "Within the previous 90 days": the 90th day back from the
        /// reference is the last inside the window.
        #[test]
        fn the_windows_edge_is_90_days_counting_the_reference_date() {
            let inside = REFERENCE - Duration::days(89);
            for r in result(&[by_day(3, aeroplane("a", inside))]) {
                assert!(r.is_current());
                assert_eq!(r.found, 3);
            }

            let outside = REFERENCE - Duration::days(90);
            for r in result(&[by_day(3, aeroplane("a", outside))]) {
                assert!(!r.is_current());
                assert_eq!(r.found, 0);
                assert_eq!(r.expiry_date, Some(REFERENCE));
            }
        }

        #[test]
        fn an_entry_reports_every_take_off_and_landing_by_day_and_night() {
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

    mod passenger_night {
        use super::*;

        fn result(entries: &[Entry]) -> [CurrencyResult; 2] {
            super::result(TakeOffsAndLandings::passenger_night, entries)
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
        fn take_offs_and_landings_by_day_do_not_count() {
            for r in result(&[by_day(3, aeroplane("a", date!(2024 - 08 - 20)))]) {
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

        #[test]
        fn a_take_off_and_a_landing_in_different_entries_both_count() {
            let on = date!(2024 - 08 - 20);
            assert_each_counts_its_own_entry(result(&split(night(3), night(3), on)));
        }

        #[test]
        fn an_unclassified_type_does_not_count() {
            for r in result(&[at_night(3, unclassified("a", date!(2024 - 08 - 20)))]) {
                assert!(!r.is_current());
            }
        }

        #[test]
        fn the_windows_edge_is_90_days_counting_the_reference_date() {
            let inside = REFERENCE - Duration::days(89);
            for r in result(&[at_night(3, aeroplane("a", inside))]) {
                assert!(r.is_current());
            }

            let outside = REFERENCE - Duration::days(90);
            for r in result(&[at_night(3, aeroplane("a", outside))]) {
                assert!(!r.is_current());
                assert_eq!(r.found, 0);
            }
        }

        #[test]
        fn an_entry_dated_after_the_reference_has_not_happened_yet() {
            for r in result(&[at_night(3, aeroplane("a", REFERENCE + Duration::days(1)))]) {
                assert_eq!(r.found, 0);
            }
        }
    }

    mod night_vfr {
        use super::*;

        fn result(entries: &[Entry]) -> [CurrencyResult; 2] {
            super::result(TakeOffsAndLandings::night_vfr, entries)
        }

        #[test]
        fn one_at_night_in_the_window_is_current_and_expires_183_days_on() {
            let on = date!(2024 - 06 - 01);
            for r in result(&[at_night(1, aeroplane("a", on))]) {
                assert!(r.is_current());
                assert_eq!(r.expiry_date, Some(on + Duration::days(183)));
            }
        }

        #[test]
        fn one_outside_the_window_leaves_a_past_expiry() {
            for r in result(&[at_night(1, aeroplane("a", date!(2023 - 06 - 13)))]) {
                assert!(!r.is_current());
                assert!(r.expiry_date.is_some());
            }
            for r in result(&[]) {
                assert_eq!(r.expiry_date, None);
            }
        }

        #[test]
        fn take_offs_and_landings_by_day_do_not_count() {
            for r in result(&[by_day(1, aeroplane("a", date!(2024 - 06 - 01)))]) {
                assert!(!r.is_current());
            }
        }

        /// "(i) at least one night take-off; and (ii) at least one night
        /// landing", in any entries.
        #[test]
        fn a_take_off_and_a_landing_in_different_entries_both_count() {
            let on = date!(2024 - 06 - 01);
            assert_each_counts_its_own_entry(result(&split(night(1), night(1), on)));
        }

        #[test]
        fn an_unclassified_type_does_not_count() {
            for r in result(&[at_night(1, unclassified("a", date!(2024 - 06 - 01)))]) {
                assert!(!r.is_current());
            }
        }

        #[test]
        fn the_windows_edge_is_183_days_counting_the_reference_date() {
            let inside = REFERENCE - Duration::days(182);
            for r in result(&[at_night(1, aeroplane("a", inside))]) {
                assert!(r.is_current());
            }

            let outside = REFERENCE - Duration::days(183);
            for r in result(&[at_night(1, aeroplane("a", outside))]) {
                assert!(!r.is_current());
                assert_eq!(r.found, 0);
            }
        }

        #[test]
        fn an_entry_dated_after_the_reference_has_not_happened_yet() {
            for r in result(&[at_night(1, aeroplane("a", REFERENCE + Duration::days(1)))]) {
                assert_eq!(r.found, 0);
            }
        }
    }
}
