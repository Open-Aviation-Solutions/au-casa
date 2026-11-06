// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.395(2): carrying a passenger at night.

use logbook_shared_kernel::{CurrencyCheck, CurrencyResult, Entry, EntryAccumulator, EventWindow};
use time::Date;

use crate::aircraft::AircraftCategory;
use crate::logbook::guards::is_category;

const WINDOW_DAYS: u32 = 90;
const REQUIRED: usize = 3;

/// Reg 61.395(2): "within the previous 90 days, in an aircraft of that
/// category or an approved flight simulator for the purpose, conducted, at
/// night: (a) at least 3 take-offs; and (b) at least 3 landings", in an
/// aeroplane.
///
/// Each entry supplies the lesser of its take-offs and its landings at
/// night.
pub(crate) struct PassengerNight {
    window: EventWindow,
}

impl PassengerNight {
    pub(crate) fn new(reference: Date) -> Self {
        Self {
            window: EventWindow::new(reference, WINDOW_DAYS, REQUIRED),
        }
    }
}

impl EntryAccumulator for PassengerNight {
    fn accept_entry(&mut self, entry: &Entry) {
        if !is_category(entry, AircraftCategory::Aeroplane) {
            return;
        }
        let pairs = entry.take_offs().night.min(entry.landings().night);
        self.window
            .record(entry, usize::try_from(pairs).unwrap_or(usize::MAX));
    }
}

impl CurrencyCheck for PassengerNight {
    fn result(&self) -> CurrencyResult {
        self.window.result()
    }
}

#[cfg(test)]
mod tests {
    use logbook_shared_kernel::EntryParts;
    use time::macros::date;
    use time::Duration;

    use super::*;
    use crate::logbook::test_support::{aeroplane, day, entry, night, unclassified};

    const REFERENCE: Date = date!(2024 - 09 - 01);

    fn result(entries: &[Entry]) -> CurrencyResult {
        let mut check = PassengerNight::new(REFERENCE);
        for entry in entries {
            check.accept_entry(entry);
        }
        check.result()
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
    fn three_night_pairs_in_the_window_are_current_and_expire_90_days_on() {
        let on = date!(2024 - 08 - 20);
        let r = result(&[at_night(3, aeroplane("a", on))]);
        assert!(r.is_current());
        assert_eq!(r.expiry_date, Some(on + Duration::days(90)));
    }

    #[test]
    fn day_movements_do_not_count() {
        let e = entry(EntryParts {
            take_offs: day(3),
            landings: day(3),
            ..aeroplane("a", date!(2024 - 08 - 20))
        });
        assert!(!result(&[e]).is_current());
    }

    #[test]
    fn the_binding_constraint_is_the_lesser_of_night_take_offs_and_landings() {
        let e = entry(EntryParts {
            take_offs: night(5),
            landings: night(2),
            ..aeroplane("a", date!(2024 - 08 - 20))
        });
        assert!(!result(&[e]).is_current());
    }

    #[test]
    fn an_unclassified_type_does_not_count() {
        let r = result(&[at_night(3, unclassified("a", date!(2024 - 08 - 20)))]);
        assert!(!r.is_current());
    }

    #[test]
    fn an_entry_dated_after_the_reference_has_not_happened_yet() {
        let r = result(&[at_night(3, aeroplane("a", REFERENCE + Duration::days(1)))]);
        assert_eq!(r.found, 0);
    }
}
