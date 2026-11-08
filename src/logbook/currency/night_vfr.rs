// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.965: the recent experience of the holder of a night VFR rating.

use logbook_shared_kernel::{CurrencyCheck, CurrencyResult, Entry, EntryAccumulator, EventWindow};
use time::Date;

use crate::aircraft::AircraftCategory;
use crate::logbook::guards::is_category;

/// "Within the previous 6 months", as pilot-logbook's 183 days (6 × 30.5),
/// kept for the move.
const WINDOW_DAYS: u32 = 183;
const REQUIRED: usize = 1;

/// Reg 61.965(a): "within the previous 6 months … (i) at least one night
/// take-off; and (ii) at least one night landing; in an aircraft of that
/// category while controlling the aircraft", in an aeroplane.
///
/// Each entry supplies the lesser of its take-offs and its landings at
/// night.
pub(crate) struct NightVfr {
    window: EventWindow,
}

impl NightVfr {
    pub(crate) fn new(reference: Date) -> Self {
        Self {
            window: EventWindow::new(reference, WINDOW_DAYS, REQUIRED),
        }
    }
}

impl EntryAccumulator for NightVfr {
    fn accept_entry(&mut self, entry: &Entry) {
        if !is_category(entry, AircraftCategory::Aeroplane) {
            return;
        }
        let pairs = entry.take_offs().night.min(entry.landings().night);
        let pairs = usize::try_from(pairs).unwrap_or(usize::MAX);
        self.window.record(entry, pairs);
    }
}

impl CurrencyCheck for NightVfr {
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
        let mut check = NightVfr::new(REFERENCE);
        for entry in entries {
            check.accept_entry(entry);
        }
        check.result()
    }

    /// One take-off and one landing at night.
    fn at_night(parts: EntryParts) -> Entry {
        entry(EntryParts {
            take_offs: night(1),
            landings: night(1),
            ..parts
        })
    }

    #[test]
    fn one_night_pair_in_the_window_is_current_and_expires_183_days_on() {
        let on = date!(2024 - 06 - 01);
        let r = result(&[at_night(aeroplane("a", on))]);
        assert!(r.is_current());
        assert_eq!(r.expiry_date, Some(on + Duration::days(183)));
    }

    #[test]
    fn a_pair_outside_the_window_leaves_a_past_expiry() {
        let r = result(&[at_night(aeroplane("a", date!(2023 - 06 - 13)))]);
        assert!(!r.is_current());
        assert!(r.expiry_date.is_some());
        assert_eq!(result(&[]).expiry_date, None);
    }

    #[test]
    fn day_movements_do_not_count() {
        let e = entry(EntryParts {
            take_offs: day(1),
            landings: day(1),
            ..aeroplane("a", date!(2024 - 06 - 01))
        });
        assert!(!result(&[e]).is_current());
    }

    #[test]
    fn an_unclassified_type_does_not_count() {
        let r = result(&[at_night(unclassified("a", date!(2024 - 06 - 01)))]);
        assert!(!r.is_current());
    }

    #[test]
    fn the_windows_edge_is_183_days_counting_the_reference_date() {
        let inside = result(&[at_night(aeroplane("a", REFERENCE - Duration::days(182)))]);
        assert!(inside.is_current());

        let outside = result(&[at_night(aeroplane("a", REFERENCE - Duration::days(183)))]);
        assert!(!outside.is_current());
        assert_eq!(outside.found, 0);
    }

    #[test]
    fn an_entry_dated_after_the_reference_has_not_happened_yet() {
        let r = result(&[at_night(aeroplane("a", REFERENCE + Duration::days(1)))]);
        assert_eq!(r.found, 0);
    }
}
