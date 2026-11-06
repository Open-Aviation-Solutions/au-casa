// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.745: the flight review for an aircraft class rating.

use std::collections::BTreeSet;

use logbook_shared_kernel::{
    Assessment, AssessmentAccumulator, AssessmentCurrencyCheck, CurrencyResult, Entry, EventWindow,
    Subject,
};
use time::Date;
use uuid::Uuid;

use crate::aircraft::AircraftClassRating;
use crate::logbook::guards::is_class_rating;
use crate::logbook::{assessment_information, CheckOutcome, CheckType};

/// 24 months, as pilot-logbook counted them (2 × 365 days).
const WINDOW_DAYS: u32 = 730;
const REQUIRED: usize = 1;

/// Reg 61.745(1): the holder of an aircraft class rating may exercise its
/// privileges "only if the holder has a valid flight review for the
/// rating", which is valid to "the end of the 24th month after the month in
/// which the holder completes the review" (reg 61.745(1A)(a)). Here, one
/// passed check within the previous 730 days, as pilot-logbook had it.
///
/// A check counts when CASA's information on the assessment records it as
/// passed and as a check that reg 61.745(3) takes as a completed flight
/// review ([`satisfies_flight_review`]), and it was conducted "in an
/// aircraft of the class covered by the rating" (reg 61.745(2)(a)). The
/// assessment's aircraft is that of the entry for the same flight: an
/// assessment whose flight has no entry, or whose entry's type is of no
/// known class, does not count. A failed check counts for nothing; it does
/// not end a valid review.
pub(crate) struct FlightReview {
    window: EventWindow,
    /// The flights an entry records in an aircraft of the class.
    flights: BTreeSet<Uuid>,
}

impl FlightReview {
    /// The check for `class_rating`, finding each assessment's aircraft
    /// among `entries`.
    pub(crate) fn new<'a>(
        reference: Date,
        class_rating: AircraftClassRating,
        entries: impl IntoIterator<Item = &'a Entry>,
    ) -> Self {
        let flights = entries
            .into_iter()
            .filter(|entry| is_class_rating(entry, class_rating))
            .filter_map(|entry| match entry.subject() {
                Subject::Flight { flight, .. } => Some(*flight),
                Subject::Session { .. } => None,
            })
            .collect();
        Self {
            window: EventWindow::new(reference, WINDOW_DAYS, REQUIRED),
            flights,
        }
    }
}

/// Whether a passed check of this type is taken as a completed flight
/// review (regs 61.745(3) and (3A)). Every one is, as pilot-logbook had it;
/// the match is exhaustive so that a type added later is decided here.
fn satisfies_flight_review(check_type: CheckType) -> bool {
    match check_type {
        CheckType::FlightTest
        | CheckType::FlightReview
        | CheckType::OperatorProficiencyCheck
        | CheckType::InstrumentProficiencyCheck
        | CheckType::NightVisionImagingSystemProficiencyCheck
        | CheckType::AerialApplicationProficiencyCheck
        | CheckType::InstructorProficiencyCheck
        | CheckType::ExaminerProficiencyCheck => true,
    }
}

impl AssessmentAccumulator for FlightReview {
    fn accept_assessment(&mut self, assessment: &Assessment) {
        if !self.flights.contains(&assessment.flight) {
            return;
        }
        let Some(information) = assessment_information(assessment) else {
            return;
        };
        if information.outcome == Some(CheckOutcome::Passed)
            && information.check_type.is_some_and(satisfies_flight_review)
        {
            self.window.record_assessment(assessment);
        }
    }
}

impl AssessmentCurrencyCheck for FlightReview {
    fn result(&self) -> CurrencyResult {
        self.window.result()
    }
}

#[cfg(test)]
mod tests {
    use logbook_shared_kernel::{AssessmentId, AuthoritySpecificInformation, EntryParts};
    use serde_json::json;
    use time::macros::date;
    use time::Duration;

    use super::*;
    use crate::logbook::test_support::{
        aeroplane, assessed, entry, flown, on_flight, unclassified,
    };
    use crate::logbook::{casa, AssessmentInformation};

    use AircraftClassRating::{MultiEngineAeroplane as Mea, SingleEngineAeroplane as Sea};

    const REFERENCE: Date = date!(2024 - 09 - 01);
    const ON: Date = date!(2024 - 05 - 24);

    /// A check of this type with this outcome, on flight 1.
    fn checked(on: Date, check_type: CheckType, outcome: CheckOutcome) -> Assessment {
        assessed(
            "a",
            on,
            1,
            AssessmentInformation {
                check_type: Some(check_type),
                outcome: Some(outcome),
                check_purpose: None,
            },
        )
    }

    /// A passed flight review on flight 1.
    fn passed(on: Date) -> Assessment {
        checked(on, CheckType::FlightReview, CheckOutcome::Passed)
    }

    /// The check for `class_rating`, over these entries and assessments.
    fn result(
        class_rating: AircraftClassRating,
        entries: &[EntryParts],
        assessments: &[Assessment],
    ) -> CurrencyResult {
        let entries: Vec<Entry> = entries.iter().cloned().map(entry).collect();
        let mut check = FlightReview::new(REFERENCE, class_rating, &entries);
        for assessment in assessments {
            check.accept_assessment(assessment);
        }
        check.result()
    }

    /// Flight 1, in a single-engine aeroplane.
    fn single_engine() -> Vec<EntryParts> {
        vec![on_flight(1, aeroplane("e", ON))]
    }

    #[test]
    fn a_passed_review_is_current_and_expires_730_days_on() {
        let r = result(Sea, &single_engine(), &[passed(ON)]);
        assert!(r.is_current());
        assert_eq!(r.found, 1);
        assert_eq!(r.expiry_date, Some(ON + Duration::days(730)));
        assert_eq!(r.qualifying_assessments, [AssessmentId::new("a")]);
    }

    #[test]
    fn the_windows_edge_is_730_days_before_the_reference_date() {
        let inside = result(
            Sea,
            &single_engine(),
            &[passed(REFERENCE - Duration::days(729))],
        );
        assert!(inside.is_current());
        let outside = result(
            Sea,
            &single_engine(),
            &[passed(REFERENCE - Duration::days(730))],
        );
        assert!(!outside.is_current());
        assert_eq!(outside.found, 0);
        assert_eq!(outside.expiry_date, Some(REFERENCE));
    }

    #[test]
    fn only_a_passed_check_counts() {
        let failed = checked(ON, CheckType::FlightReview, CheckOutcome::Failed);
        assert_eq!(result(Sea, &single_engine(), &[failed]).expiry_date, None);
        let no_outcome = assessed(
            "a",
            ON,
            1,
            AssessmentInformation {
                check_type: Some(CheckType::FlightReview),
                ..AssessmentInformation::default()
            },
        );
        assert_eq!(
            result(Sea, &single_engine(), &[no_outcome]).expiry_date,
            None
        );
        let no_type = assessed(
            "a",
            ON,
            1,
            AssessmentInformation {
                outcome: Some(CheckOutcome::Passed),
                ..AssessmentInformation::default()
            },
        );
        assert_eq!(result(Sea, &single_engine(), &[no_type]).expiry_date, None);
    }

    #[test]
    fn every_check_type_is_taken_as_a_flight_review() {
        for check_type in [
            CheckType::FlightTest,
            CheckType::FlightReview,
            CheckType::OperatorProficiencyCheck,
            CheckType::InstrumentProficiencyCheck,
            CheckType::NightVisionImagingSystemProficiencyCheck,
            CheckType::AerialApplicationProficiencyCheck,
            CheckType::InstructorProficiencyCheck,
            CheckType::ExaminerProficiencyCheck,
        ] {
            let check = checked(ON, check_type, CheckOutcome::Passed);
            assert!(
                result(Sea, &single_engine(), &[check]).is_current(),
                "{check_type:?}"
            );
        }
    }

    #[test]
    fn a_check_counts_only_for_the_class_of_its_flights_aircraft() {
        let multi_engine = [on_flight(1, flown("e", ON, "PA44"))];
        let helicopter = [on_flight(1, flown("e", ON, "R44"))];
        let unknown = [on_flight(1, unclassified("e", ON))];
        assert!(result(Sea, &single_engine(), &[passed(ON)]).is_current());
        assert!(!result(Mea, &single_engine(), &[passed(ON)]).is_current());
        assert!(result(Mea, &multi_engine, &[passed(ON)]).is_current());
        assert!(!result(Sea, &multi_engine, &[passed(ON)]).is_current());
        for entries in [&helicopter, &unknown] {
            assert!(!result(Sea, entries, &[passed(ON)]).is_current());
            assert!(!result(Mea, entries, &[passed(ON)]).is_current());
        }
    }

    #[test]
    fn a_check_whose_flight_has_no_entry_does_not_count() {
        assert!(!result(Sea, &[], &[passed(ON)]).is_current());
        let another_flight = [on_flight(2, aeroplane("e", ON))];
        assert!(!result(Sea, &another_flight, &[passed(ON)]).is_current());
    }

    #[test]
    fn a_check_without_casas_readable_information_does_not_count() {
        let mut unreadable = passed(ON);
        unreadable.authority_information.insert(
            casa(),
            AuthoritySpecificInformation::new(json!({"_version": 2})),
        );
        let mut none = passed(ON);
        none.authority_information.clear();
        for assessment in [unreadable, none] {
            assert!(!result(Sea, &single_engine(), &[assessment]).is_current());
        }
    }

    #[test]
    fn an_assessment_dated_after_the_reference_has_not_happened_yet() {
        let future = passed(REFERENCE + Duration::days(1));
        assert_eq!(result(Sea, &single_engine(), &[future]).expiry_date, None);
    }
}
