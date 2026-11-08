// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.870(4): the 2D instrument approach operation.

use logbook_shared_kernel::{CurrencyCheck, CurrencyResult, Entry, EntryAccumulator, EventWindow};
use time::Date;

use super::instrument_approaches::{dme_or_gnss_arrivals, is_3d, WINDOW_DAYS};

const REQUIRED: usize = 1;

/// Reg 61.870(4): "a 2D instrument approach operation within the previous
/// 90 days in an aircraft or an approved flight simulation training device
/// for the purpose".
///
/// Each entry supplies its approaches that were not 3D operations, and its
/// DME or GNSS arrivals.
pub(crate) struct InstrumentApproaches2D {
    window: EventWindow,
}

impl InstrumentApproaches2D {
    pub(crate) fn new(reference: Date) -> Self {
        Self {
            window: EventWindow::new(reference, WINDOW_DAYS, REQUIRED),
        }
    }
}

impl EntryAccumulator for InstrumentApproaches2D {
    fn accept_entry(&mut self, entry: &Entry) {
        let operations = entry
            .approaches()
            .iter()
            .filter(|approach| !is_3d(approach))
            .count()
            .saturating_add(dme_or_gnss_arrivals(entry));
        self.window.record(entry, operations);
    }
}

impl CurrencyCheck for InstrumentApproaches2D {
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
    use crate::logbook::test_support::{aeroplane, approach, arrivals, circled, entry, simulated};

    const REFERENCE: Date = date!(2024 - 09 - 01);
    const ON: Date = date!(2024 - 08 - 20);

    fn result(entries: &[Entry]) -> CurrencyResult {
        let mut check = InstrumentApproaches2D::new(REFERENCE);
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
    fn one_2d_approach_is_current_and_expires_90_days_on() {
        let r = result(&[flying(ApproachType::Vor, ON)]);
        assert!(r.is_current());
        assert_eq!(r.expiry_date, Some(ON + Duration::days(90)));
    }

    #[test]
    fn every_2d_procedure_counts_and_no_3d_one_does() {
        let rnp = |minima| ApproachType::RnpApch { minima };
        for procedure in [
            ApproachType::Ndb,
            ApproachType::Vor,
            ApproachType::Localizer,
            rnp(RnpApchMinima::Lnav),
            rnp(RnpApchMinima::Lp),
        ] {
            assert!(
                result(&[flying(procedure, ON)]).is_current(),
                "{procedure:?}"
            );
        }
        for procedure in [
            ApproachType::Ils,
            ApproachType::Gls,
            ApproachType::RnpAr,
            rnp(RnpApchMinima::LnavVnav),
            rnp(RnpApchMinima::Lpv),
        ] {
            assert!(
                !result(&[flying(procedure, ON)]).is_current(),
                "{procedure:?}"
            );
        }
    }

    #[test]
    fn a_circled_3d_procedure_is_not_a_2d_operation() {
        let e = entry(EntryParts {
            approaches: vec![circled(ApproachType::Ils)],
            ..aeroplane("a", ON)
        });
        assert!(!result(&[e]).is_current());
    }

    #[test]
    fn a_dme_or_gnss_arrival_is_a_2d_operation() {
        assert!(result(&[entry(arrivals(1, aeroplane("a", ON)))]).is_current());
    }

    /// "Or an approved flight simulation training device": whether the
    /// device was approved is `Casa::counts`'.
    #[test]
    fn a_session_counts() {
        let session = entry(EntryParts {
            approaches: vec![approach(ApproachType::Vor)],
            ..simulated("a", ON, "R44")
        });
        assert!(result(&[session]).is_current());
    }

    #[test]
    fn the_windows_edge_is_90_days_counting_the_reference_date() {
        let inside = result(&[flying(ApproachType::Vor, REFERENCE - Duration::days(89))]);
        assert!(inside.is_current());

        let outside = result(&[flying(ApproachType::Vor, REFERENCE - Duration::days(90))]);
        assert!(!outside.is_current());
        assert_eq!(outside.found, 0);
    }

    #[test]
    fn an_entry_dated_after_the_reference_has_not_happened_yet() {
        let r = result(&[flying(ApproachType::Vor, REFERENCE + Duration::days(1))]);
        assert_eq!(r.found, 0);
    }
}
