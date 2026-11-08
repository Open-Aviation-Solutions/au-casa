// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Regs 61.870(2) and (3): instrument approach operations, of any kind, in
//! any aircraft and in each category; and CASA's reading of an entry's
//! approaches, which regs 61.870(4) and (5) share.

use logbook_shared_kernel::{
    Approach, CurrencyCheck, CurrencyResult, Entry, EntryAccumulator, EventWindow,
};
use time::Date;

use crate::aircraft::AircraftCategory;
use crate::logbook::guards::is_category;
use crate::logbook::information;

/// "Within the previous 90 days", in each of regs 61.870(2) to (7).
pub(super) const WINDOW_DAYS: u32 = 90;

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
/// 20): none when it records no count, or has information that cannot be
/// read. A count too big for `usize` saturates at `usize::MAX`.
pub(super) fn dme_or_gnss_arrivals(entry: &Entry) -> usize {
    information(entry)
        .and_then(|information| information.dme_or_gnss_arrivals)
        .map_or(0, |count| usize::try_from(count).unwrap_or(usize::MAX))
}

/// The operations needed: "at least 3" in any category (reg 61.870(2)),
/// "at least one" in a category (reg 61.870(3)).
fn required(category: Option<AircraftCategory>) -> usize {
    if category.is_some() {
        1
    } else {
        3
    }
}

/// One of regs 61.870(2) and (3). Each entry supplies its approaches and
/// its DME or GNSS arrivals.
pub(crate) struct InstrumentApproaches {
    window: EventWindow,
    /// The category an entry must be known to be in, for reg 61.870(3).
    category: Option<AircraftCategory>,
}

impl InstrumentApproaches {
    /// Reg 61.870(2): "at least 3 instrument approach operations within the
    /// previous 90 days in an aircraft or an approved flight simulation
    /// training device for the purpose", in an aircraft of any category.
    pub(crate) fn new(reference: Date) -> Self {
        Self::counting(reference, None)
    }

    /// Reg 61.870(3), for one category (reg 61.015): "at least one
    /// instrument approach operation within the previous 90 days in an
    /// aircraft of the same category or an approved flight simulation
    /// training device for the purpose".
    ///
    /// A session counts towards the category of the type its device
    /// represented, as the totals count it; a session that recorded no type,
    /// or a type of no known category, counts towards none.
    pub(crate) fn in_category(reference: Date, category: AircraftCategory) -> Self {
        Self::counting(reference, Some(category))
    }

    fn counting(reference: Date, category: Option<AircraftCategory>) -> Self {
        Self {
            window: EventWindow::new(reference, WINDOW_DAYS, required(category)),
            category,
        }
    }
}

impl EntryAccumulator for InstrumentApproaches {
    fn accept_entry(&mut self, entry: &Entry) {
        if self
            .category
            .is_some_and(|category| !is_category(entry, category))
        {
            return;
        }
        let operations = entry
            .approaches()
            .len()
            .saturating_add(dme_or_gnss_arrivals(entry));
        self.window.record(entry, operations);
    }
}

impl CurrencyCheck for InstrumentApproaches {
    fn result(&self) -> CurrencyResult {
        self.window.result()
    }
}

#[cfg(test)]
mod tests {
    use logbook_shared_kernel::{ApproachType, EntryParts, RnpApchMinima, Subject};
    use time::macros::date;
    use time::Duration;

    use super::*;
    use crate::logbook::test_support::{
        aeroplane, approach, arrivals, circled, entry, flown, simulated, unclassified,
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
    fn an_entry_reports_every_operation_it_records() {
        let r = result(&[entry(arrivals(5, aeroplane("a", date!(2024 - 08 - 20))))]);
        assert!(r.is_current());
        assert_eq!(r.found, 5);
    }

    /// "In an aircraft": of any category, or none known.
    #[test]
    fn an_aircraft_of_any_category_counts() {
        let on = date!(2024 - 08 - 20);
        assert!(result(&[three(flown("a", on, "R44"))]).is_current());
        assert!(result(&[three(unclassified("a", on))]).is_current());
    }

    /// "Or an approved flight simulation training device": whether the
    /// device was approved is `Casa::counts`'.
    #[test]
    fn a_session_counts() {
        assert!(result(&[three(simulated("a", date!(2024 - 08 - 20), "R44"))]).is_current());
    }

    /// Reg 61.870(3)'s result for `category` over `entries`.
    fn in_category(category: AircraftCategory, entries: &[Entry]) -> CurrencyResult {
        let mut check = InstrumentApproaches::in_category(REFERENCE, category);
        for entry in entries {
            check.accept_entry(entry);
        }
        check.result()
    }

    #[test]
    fn one_operation_in_the_category_is_current_for_it_and_expires_90_days_on() {
        let on = date!(2024 - 08 - 20);
        let e = entry(EntryParts {
            approaches: vec![approach(ApproachType::Vor), approach(ApproachType::Ils)],
            ..flown("a", on, "R44")
        });
        let r = in_category(AircraftCategory::Helicopter, std::slice::from_ref(&e));
        assert!(r.is_current());
        assert_eq!(r.found, 2);
        assert_eq!(r.expiry_date, Some(on + Duration::days(90)));
        assert!(!in_category(AircraftCategory::Aeroplane, &[e]).is_current());
    }

    #[test]
    fn a_dme_or_gnss_arrival_is_an_operation_in_the_category() {
        let e = entry(arrivals(1, aeroplane("a", date!(2024 - 08 - 20))));
        assert!(in_category(AircraftCategory::Aeroplane, &[e]).is_current());
    }

    /// A session counts towards the category of the type its device
    /// represented, and one that recorded no type towards none.
    #[test]
    fn a_session_counts_towards_the_category_its_type_is_in() {
        let on = date!(2024 - 08 - 20);
        let session = three(simulated("a", on, "R44"));
        assert!(
            in_category(AircraftCategory::Helicopter, std::slice::from_ref(&session)).is_current()
        );
        assert!(!in_category(AircraftCategory::Aeroplane, &[session]).is_current());
        let mut untyped = simulated("a", on, "R44");
        if let Subject::Session { simulated, .. } = &mut untyped.subject {
            *simulated = None;
        }
        let untyped = three(untyped);
        for category in [AircraftCategory::Aeroplane, AircraftCategory::Helicopter] {
            assert!(!in_category(category, std::slice::from_ref(&untyped)).is_current());
        }
    }

    #[test]
    fn a_flight_of_no_known_category_counts_towards_none() {
        let e = three(unclassified("a", date!(2024 - 08 - 20)));
        assert!(!in_category(AircraftCategory::Aeroplane, &[e]).is_current());
    }

    #[test]
    fn the_windows_edge_in_the_category_is_90_days_counting_the_reference_date() {
        let inside = three(aeroplane("a", REFERENCE - Duration::days(89)));
        assert!(in_category(AircraftCategory::Aeroplane, &[inside]).is_current());
        let outside = three(aeroplane("a", REFERENCE - Duration::days(90)));
        assert!(!in_category(AircraftCategory::Aeroplane, &[outside]).is_current());
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
