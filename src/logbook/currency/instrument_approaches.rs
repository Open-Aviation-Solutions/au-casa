// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.870(2): instrument approach operations, of any kind; and CASA's
//! reading of an entry's approaches, which regs 61.870(4) and (5) share.

use logbook_shared_kernel::{
    Approach, CurrencyCheck, CurrencyResult, Entry, EntryAccumulator, EventWindow,
};
use time::Date;

use crate::aircraft::AircraftCategory;
use crate::logbook::guards::is_category;
use crate::logbook::information;

/// "Within the previous 90 days", in each of regs 61.870(2), (4) and (5).
pub(super) const WINDOW_DAYS: u32 = 90;
const REQUIRED: usize = 3;

/// Whether an approach was a "3D instrument approach operation" ("an
/// instrument approach operation using lateral and vertical navigation
/// guidance", CASR Dictionary) rather than a "2D instrument approach
/// operation" ("using lateral navigation guidance only"): the kernel's
/// `is_3d` of its procedure, as Annex 6 classes it. Whether it was circled
/// does not change it: a circling approach is "an extension of an
/// instrument approach operation" (CASR Dictionary).
pub(super) fn is_3d(approach: &Approach) -> bool {
    approach.procedure().is_3d()
}

/// The DME or GNSS arrivals an entry records in CASA's information, each "an
/// instrument approach operation under the IFR using distance measuring
/// equipment or a global navigation satellite system" (Table 61.935, item
/// 20), at most `required` of them: none when it records no count, or has
/// information that cannot be read.
pub(super) fn dme_or_gnss_arrivals(entry: &Entry, required: usize) -> usize {
    information(entry)
        .and_then(|information| information.dme_or_gnss_arrivals)
        .map_or(0, |count| {
            usize::try_from(count).map_or(required, |count| count.min(required))
        })
}

/// Reg 61.870(2): "at least 3 instrument approach operations within the
/// previous 90 days in an aircraft or an approved flight simulation
/// training device for the purpose".
///
/// Only in an aeroplane: pilot-logbook's reading, kept for the move.
///
/// Each entry supplies its approaches and its DME or GNSS arrivals.
pub(crate) struct InstrumentApproaches {
    window: EventWindow,
}

impl InstrumentApproaches {
    pub(crate) fn new(reference: Date) -> Self {
        Self {
            window: EventWindow::new(reference, WINDOW_DAYS, REQUIRED),
        }
    }
}

impl EntryAccumulator for InstrumentApproaches {
    fn accept_entry(&mut self, entry: &Entry) {
        if !is_category(entry, AircraftCategory::Aeroplane) {
            return;
        }
        let operations = entry.approaches().len() + dme_or_gnss_arrivals(entry, REQUIRED);
        // One entry supplies at most the operations the check needs.
        self.window.record(entry, operations.min(REQUIRED));
    }
}

impl CurrencyCheck for InstrumentApproaches {
    fn result(&self) -> CurrencyResult {
        self.window.result()
    }
}

#[cfg(test)]
mod tests {
    use logbook_shared_kernel::{ApproachType, EntryParts, RnpApchMinima};
    use time::macros::date;
    use time::Duration;

    use super::*;
    use crate::logbook::test_support::{
        aeroplane, approach, arrivals, circled, entry, simulated, unclassified,
    };

    const REFERENCE: Date = date!(2024 - 09 - 01);

    fn result(entries: &[Entry]) -> CurrencyResult {
        let mut check = InstrumentApproaches::new(REFERENCE);
        for entry in entries {
            check.accept_entry(entry);
        }
        check.result()
    }

    /// A VOR, an ILS and an NDB approach.
    fn three(parts: EntryParts) -> Entry {
        entry(EntryParts {
            approaches: vec![
                approach(ApproachType::Vor),
                approach(ApproachType::Ils),
                approach(ApproachType::Ndb),
            ],
            ..parts
        })
    }

    #[test]
    fn three_approaches_in_the_window_are_current_and_expire_90_days_on() {
        let on = date!(2024 - 08 - 20);
        let r = result(&[three(aeroplane("a", on))]);
        assert!(r.is_current());
        assert_eq!(r.found, 3);
        assert_eq!(r.expiry_date, Some(on + Duration::days(90)));
    }

    #[test]
    fn approaches_from_more_than_one_flight_accumulate() {
        let e1 = entry(EntryParts {
            approaches: vec![approach(ApproachType::Vor)],
            ..aeroplane("a", date!(2024 - 08 - 10))
        });
        let e2 = entry(EntryParts {
            approaches: vec![
                approach(ApproachType::RnpApch {
                    minima: RnpApchMinima::Lpv,
                }),
                circled(ApproachType::Ndb),
            ],
            ..aeroplane("b", date!(2024 - 08 - 20))
        });
        assert!(result(&[e1, e2]).is_current());
    }

    #[test]
    fn dme_or_gnss_arrivals_are_instrument_approach_operations() {
        let e = entry(EntryParts {
            approaches: vec![approach(ApproachType::Vor)],
            ..arrivals(2, aeroplane("a", date!(2024 - 08 - 20)))
        });
        let r = result(&[e]);
        assert!(r.is_current());
        assert_eq!(r.found, 3);
    }

    #[test]
    fn an_entry_supplies_at_most_the_three_operations_needed() {
        let r = result(&[entry(arrivals(
            u64::MAX,
            aeroplane("a", date!(2024 - 08 - 20)),
        ))]);
        assert!(r.is_current());
        assert_eq!(r.found, 3);
    }

    /// "Or an approved flight simulation training device": whether the
    /// device was approved is `Casa::counts`'.
    #[test]
    fn a_session_representing_an_aeroplane_counts() {
        assert!(result(&[three(simulated("a", date!(2024 - 08 - 20), "C172"))]).is_current());
    }

    #[test]
    fn a_session_representing_a_helicopter_does_not_count() {
        assert!(!result(&[three(simulated("a", date!(2024 - 08 - 20), "R44"))]).is_current());
    }

    #[test]
    fn an_unclassified_type_does_not_count() {
        assert!(!result(&[three(unclassified("a", date!(2024 - 08 - 20)))]).is_current());
    }

    #[test]
    fn the_windows_edge_is_90_days_counting_the_reference_date() {
        let inside = result(&[three(aeroplane("a", REFERENCE - Duration::days(89)))]);
        assert!(inside.is_current());

        let outside = result(&[three(aeroplane("a", REFERENCE - Duration::days(90)))]);
        assert!(!outside.is_current());
        assert_eq!(outside.found, 0);
    }

    #[test]
    fn an_entry_dated_after_the_reference_has_not_happened_yet() {
        let r = result(&[three(aeroplane("a", REFERENCE + Duration::days(1)))]);
        assert_eq!(r.found, 0);
    }
}
