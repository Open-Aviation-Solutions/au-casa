// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.870(5): the 3D instrument approach operation.

use logbook_shared_kernel::{CurrencyCheck, CurrencyResult, Entry, EntryAccumulator, EventWindow};
use time::Date;

use super::instrument_approaches::{is_3d, WINDOW_DAYS};
use crate::aircraft::AircraftCategory;
use crate::logbook::guards::is_category;

const REQUIRED: usize = 1;

/// Reg 61.870(5): "a 3D instrument approach operation within the previous
/// 90 days in an aircraft or an approved flight simulation training device
/// for the purpose", in an aeroplane.
///
/// Each entry supplies its approaches that were 3D operations.
pub(crate) struct InstrumentApproaches3D {
    window: EventWindow,
}

impl InstrumentApproaches3D {
    pub(crate) fn new(reference: Date) -> Self {
        Self {
            window: EventWindow::new(reference, WINDOW_DAYS, REQUIRED),
        }
    }
}

impl EntryAccumulator for InstrumentApproaches3D {
    fn accept_entry(&mut self, entry: &Entry) {
        if !is_category(entry, AircraftCategory::Aeroplane) {
            return;
        }
        let operations = entry
            .approaches()
            .iter()
            .filter(|approach| is_3d(approach))
            .count();
        // One entry supplies at most the operation the check needs.
        self.window.record(entry, operations.min(REQUIRED));
    }
}

impl CurrencyCheck for InstrumentApproaches3D {
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
    use crate::logbook::test_support::{aeroplane, approach, arrivals, circled, entry};

    const REFERENCE: Date = date!(2024 - 09 - 01);
    const ON: Date = date!(2024 - 08 - 20);

    fn result(entries: &[Entry]) -> CurrencyResult {
        let mut check = InstrumentApproaches3D::new(REFERENCE);
        for entry in entries {
            check.accept_entry(entry);
        }
        check.result()
    }

    fn flying(procedure: ApproachType, on: Date) -> Entry {
        entry(EntryParts {
            approaches: vec![approach(procedure)],
            ..aeroplane("a", on)
        })
    }

    #[test]
    fn one_3d_approach_is_current_and_expires_90_days_on() {
        let r = result(&[flying(ApproachType::Ils, ON)]);
        assert!(r.is_current());
        assert_eq!(r.expiry_date, Some(ON + Duration::days(90)));
    }

    #[test]
    fn every_3d_procedure_counts_and_no_2d_one_does() {
        let rnp = |minima| ApproachType::RnpApch { minima };
        for procedure in [
            ApproachType::Ils,
            ApproachType::Gls,
            ApproachType::RnpAr,
            rnp(RnpApchMinima::LnavVnav),
            rnp(RnpApchMinima::Lpv),
        ] {
            assert!(
                result(&[flying(procedure, ON)]).is_current(),
                "{procedure:?}"
            );
        }
        for procedure in [
            ApproachType::Ndb,
            ApproachType::Vor,
            ApproachType::Localizer,
            rnp(RnpApchMinima::Lnav),
            rnp(RnpApchMinima::Lp),
        ] {
            assert!(
                !result(&[flying(procedure, ON)]).is_current(),
                "{procedure:?}"
            );
        }
    }

    #[test]
    fn a_circled_3d_procedure_is_still_a_3d_operation() {
        let e = entry(EntryParts {
            approaches: vec![circled(ApproachType::Ils)],
            ..aeroplane("a", ON)
        });
        assert!(result(&[e]).is_current());
    }

    #[test]
    fn a_dme_or_gnss_arrival_is_not_a_3d_operation() {
        assert!(!result(&[entry(arrivals(1, aeroplane("a", ON)))]).is_current());
    }

    #[test]
    fn the_windows_edge_is_90_days_counting_the_reference_date() {
        let inside = result(&[flying(ApproachType::Ils, REFERENCE - Duration::days(89))]);
        assert!(inside.is_current());

        let outside = result(&[flying(ApproachType::Ils, REFERENCE - Duration::days(90))]);
        assert!(!outside.is_current());
        assert_eq!(outside.found, 0);
    }

    #[test]
    fn an_entry_dated_after_the_reference_has_not_happened_yet() {
        let r = result(&[flying(ApproachType::Ils, REFERENCE + Duration::days(1))]);
        assert_eq!(r.found, 0);
    }
}
