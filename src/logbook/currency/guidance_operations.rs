// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Regs 61.870(6) and (7): the azimuth guidance operation and the course
//! deviation indicator operation, each "an instrument approach operation"
//! (reg 61.870, note) that CASA's information on an entry records with a
//! yes or no.

use logbook_shared_kernel::{CurrencyCheck, CurrencyResult, Entry, EntryAccumulator, EventWindow};
use time::Date;

use super::instrument_approaches::WINDOW_DAYS;
use crate::logbook::{information, EntryInformation};

const REQUIRED: usize = 1;

/// One of regs 61.870(6) and (7): an operation of its kind "within the
/// previous 90 days in an aircraft or an approved flight simulation
/// training device for the purpose".
///
/// An entry supplies one operation when CASA's information on it answers
/// yes; none when it answers no, gives no answer, or cannot be read, or
/// records no approach, for which the question is not shown.
pub(crate) struct GuidanceOperations {
    window: EventWindow,
    conducted: fn(&EntryInformation) -> Option<bool>,
}

impl GuidanceOperations {
    /// Reg 61.870(6): "an azimuth guidance operation" ("an instrument
    /// approach operation using azimuth bearings for lateral navigation
    /// guidance", reg 61.010), recorded as `azimuth_guidance_operation`.
    pub(crate) fn azimuth_guidance(reference: Date) -> Self {
        Self::new(reference, |information| {
            information.azimuth_guidance_operation
        })
    }

    /// Reg 61.870(7): "a course deviation indicator operation" ("an
    /// instrument approach operation using a course deviation indicator for
    /// lateral navigation guidance", reg 61.010), recorded as
    /// `course_deviation_indicator_operation`.
    pub(crate) fn course_deviation_indicator(reference: Date) -> Self {
        Self::new(reference, |information| {
            information.course_deviation_indicator_operation
        })
    }

    fn new(reference: Date, conducted: fn(&EntryInformation) -> Option<bool>) -> Self {
        Self {
            window: EventWindow::new(reference, WINDOW_DAYS, REQUIRED),
            conducted,
        }
    }
}

impl EntryAccumulator for GuidanceOperations {
    fn accept_entry(&mut self, entry: &Entry) {
        let conducted = information(entry)
            .and_then(|information| (self.conducted)(&information))
            .unwrap_or(false);
        self.window.record(entry, usize::from(conducted));
    }
}

impl CurrencyCheck for GuidanceOperations {
    fn result(&self) -> CurrencyResult {
        self.window.result()
    }
}

#[cfg(test)]
mod tests {
    use logbook_shared_kernel::{ApproachType, EntryParts};
    use time::macros::date;
    use time::Duration;

    use super::*;
    use crate::logbook::test_support::{aeroplane, approach, entry, informed, simulated};

    const REFERENCE: Date = date!(2024 - 09 - 01);
    const ON: Date = date!(2024 - 08 - 20);

    /// Builds one of the two checks.
    type Check = fn(Date) -> GuidanceOperations;

    /// Each check, and the information that answers it `answer`.
    fn checks(answer: Option<bool>) -> [(&'static str, Check, EntryInformation); 2] {
        [
            (
                "azimuth guidance",
                GuidanceOperations::azimuth_guidance,
                EntryInformation {
                    azimuth_guidance_operation: answer,
                    ..EntryInformation::default()
                },
            ),
            (
                "course deviation indicator",
                GuidanceOperations::course_deviation_indicator,
                EntryInformation {
                    course_deviation_indicator_operation: answer,
                    ..EntryInformation::default()
                },
            ),
        ]
    }

    /// The parts, recording an approach, for which the guidance questions
    /// are shown.
    fn with_approach(mut parts: EntryParts) -> EntryParts {
        parts.approaches = vec![approach(ApproachType::Vor)];
        parts
    }

    /// Each check's result over one entry of `parts`, with an approach and
    /// information that answers it `answer`, and the check's name.
    fn results(answer: Option<bool>, parts: EntryParts) -> Vec<(&'static str, CurrencyResult)> {
        let parts = with_approach(parts);
        checks(answer)
            .into_iter()
            .map(|(name, check, information)| {
                let mut check = check(REFERENCE);
                check.accept_entry(&entry(informed(information, parts.clone())));
                (name, check.result())
            })
            .collect()
    }

    #[test]
    fn an_operation_is_current_and_expires_90_days_on() {
        for (name, r) in results(Some(true), aeroplane("a", ON)) {
            assert!(r.is_current(), "{name}");
            assert_eq!(r.found, 1, "{name}");
            assert_eq!(r.expiry_date, Some(ON + Duration::days(90)), "{name}");
        }
    }

    #[test]
    fn no_or_no_answer_is_no_operation() {
        for answer in [Some(false), None] {
            for (name, r) in results(answer, aeroplane("a", ON)) {
                assert!(!r.is_current(), "{name}, {answer:?}");
            }
        }
    }

    /// An answer stored before the entry's approaches were removed is to a
    /// question no longer shown, so it is not counted, whether or not the
    /// host dropped it on saving.
    #[test]
    fn an_answer_stored_without_an_approach_is_no_operation() {
        for (name, check, information) in checks(Some(true)) {
            let mut check = check(REFERENCE);
            check.accept_entry(&entry(informed(information, aeroplane("a", ON))));
            assert!(!check.result().is_current(), "{name}");
        }
    }

    #[test]
    fn each_reads_only_its_own_answer() {
        let [(_, azimuth_guidance, azimuth_yes), (_, course_deviation, course_deviation_yes)] =
            checks(Some(true));
        for (check, other) in [
            (azimuth_guidance, course_deviation_yes),
            (course_deviation, azimuth_yes),
        ] {
            let mut check = check(REFERENCE);
            check.accept_entry(&entry(informed(other, with_approach(aeroplane("a", ON)))));
            assert!(!check.result().is_current());
        }
    }

    /// "Or an approved flight simulation training device": whether the
    /// device was approved is `Casa::counts`'.
    #[test]
    fn a_session_counts() {
        for (name, r) in results(Some(true), simulated("a", ON, "R44")) {
            assert!(r.is_current(), "{name}");
        }
    }

    #[test]
    fn the_windows_edge_is_90_days_counting_the_reference_date() {
        for (name, r) in results(Some(true), aeroplane("a", REFERENCE - Duration::days(89))) {
            assert!(r.is_current(), "{name}");
        }
        for (name, r) in results(Some(true), aeroplane("a", REFERENCE - Duration::days(90))) {
            assert!(!r.is_current(), "{name}");
            assert_eq!(r.found, 0, "{name}");
        }
    }

    #[test]
    fn an_entry_dated_after_the_reference_has_not_happened_yet() {
        for (name, r) in results(Some(true), aeroplane("a", REFERENCE + Duration::days(1))) {
            assert_eq!(r.found, 0, "{name}");
        }
    }
}
