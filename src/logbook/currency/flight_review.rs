// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.745: the flight review for an aircraft class rating.

use std::collections::BTreeSet;

use logbook_shared_kernel::{
    Assessment, AssessmentAccumulator, AssessmentCurrencyCheck, CurrencyResult, Entry, EventWindow,
    SubjectId,
};
use time::Date;

use crate::aircraft::AircraftClassRating;
use crate::logbook::guards::is_class_rating;
use crate::logbook::{assessment_information, AssessmentInformation, CheckOutcome, CheckType};

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
/// passed ([`passed`]: a flight test, for any of its purposes) and as a
/// check that reg 61.745(3) takes as a completed flight
/// review ([`satisfies_flight_review`]), and it was conducted "in an
/// aircraft of the class covered by the rating" (reg 61.745(2)(a)). The
/// assessment's aircraft is that of the entry for the same flight: an
/// assessment whose flight has no entry, or whose entry's type is of no
/// known class, does not count. A failed check counts for nothing; it does
/// not end a valid review.
pub(crate) struct FlightReview {
    window: EventWindow,
    /// The flights an entry records in an aircraft of the class.
    flights: BTreeSet<SubjectId>,
}

impl FlightReview {
    /// The check for `class_rating`, finding each assessment's aircraft
    /// among `entries`. The caller passes the entries
    /// [`Casa::counts`](crate::Casa) lets through, as the kernel's `run`
    /// would: this check is not run over entries, so it does not apply that
    /// gate itself.
    pub(crate) fn new<'a>(
        reference: Date,
        class_rating: AircraftClassRating,
        entries: impl IntoIterator<Item = &'a Entry>,
    ) -> Self {
        let flights = entries
            .into_iter()
            .filter(|entry| is_class_rating(entry, class_rating))
            .map(|entry| entry.subject().id())
            .filter(|id| matches!(id, SubjectId::Flight(_)))
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

/// Whether the check was passed: a flight test, which has no outcome of its
/// own, when it was passed for any of its purposes (until au-casa#69 reads
/// which), and any other check by its outcome.
fn passed(information: &AssessmentInformation) -> bool {
    if information.check_type == Some(CheckType::FlightTest) {
        information
            .check_purposes
            .iter()
            .any(|assessed| assessed.outcome == CheckOutcome::Passed)
    } else {
        information.outcome == Some(CheckOutcome::Passed)
    }
}

impl AssessmentAccumulator for FlightReview {
    fn accept_assessment(&mut self, assessment: &Assessment) {
        if !self.flights.contains(&SubjectId::Flight(assessment.flight)) {
            return;
        }
        let Some(information) = assessment_information(assessment) else {
            return;
        };
        if passed(&information) && information.check_type.is_some_and(satisfies_flight_review) {
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
        aeroplane, assessed, entry, flight_test, flown, on_flight, simulated, unclassified,
    };
    use crate::logbook::{casa, CheckPurpose};

    use AircraftClassRating::{MultiEngineAeroplane as Mea, SingleEngineAeroplane as Sea};

    const REFERENCE: Date = date!(2024 - 09 - 01);
    const ON: Date = date!(2024 - 05 - 24);

    /// A check of this type with this outcome, on flight 1: a flight test's
    /// for the private pilot licence, as a flight test has no outcome of
    /// its own.
    fn checked(on: Date, check_type: CheckType, outcome: CheckOutcome) -> Assessment {
        let information = if check_type == CheckType::FlightTest {
            flight_test(&[(CheckPurpose::PrivatePilotLicence, outcome)])
        } else {
            AssessmentInformation {
                check_type: Some(check_type),
                outcome: Some(outcome),
                check_purposes: Vec::new(),
            }
        };
        assessed("a", on, 1, information)
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

    /// Until au-casa#69 reads which, a flight test passed for any of its
    /// purposes is taken as passed. One with an outcome of its own, which
    /// it does not have, is not.
    #[test]
    fn a_flight_test_is_passed_when_any_of_its_purposes_is() {
        use CheckOutcome::{Failed, Passed};
        use CheckPurpose::{InstrumentRating, MultiEngineAeroplaneClassRating as Class};
        let test = |information| assessed("a", ON, 1, information);
        let partly = test(flight_test(&[(Class, Passed), (InstrumentRating, Failed)]));
        assert!(result(Sea, &single_engine(), &[partly]).is_current());
        let failed = test(flight_test(&[(Class, Failed), (InstrumentRating, Failed)]));
        assert!(!result(Sea, &single_engine(), &[failed]).is_current());
        let outcome_only = test(AssessmentInformation {
            outcome: Some(Passed),
            ..flight_test(&[])
        });
        assert!(!result(Sea, &single_engine(), &[outcome_only]).is_current());
    }

    /// An answer that does not apply is dropped, not counted: a flight
    /// review's purposes do not make a failed review passed, and a flight
    /// test's own failed outcome does not undo its passed purposes.
    #[test]
    fn an_answer_that_does_not_apply_is_not_counted() {
        use CheckOutcome::{Failed, Passed};
        let purposes = flight_test(&[(CheckPurpose::PrivatePilotLicence, Passed)]).check_purposes;
        let review = assessed(
            "a",
            ON,
            1,
            AssessmentInformation {
                check_type: Some(CheckType::FlightReview),
                outcome: Some(Failed),
                check_purposes: purposes,
            },
        );
        assert!(!result(Sea, &single_engine(), &[review]).is_current());
        let test = assessed(
            "a",
            ON,
            1,
            AssessmentInformation {
                outcome: Some(Failed),
                ..flight_test(&[(CheckPurpose::PrivatePilotLicence, Passed)])
            },
        );
        assert!(result(Sea, &single_engine(), &[test]).is_current());
    }

    #[test]
    fn every_check_type_is_taken_as_a_flight_review() {
        for check_type in CheckType::ALL {
            let check = checked(ON, check_type, CheckOutcome::Passed);
            assert!(
                result(Sea, &single_engine(), &[check]).is_current(),
                "{check_type:?}"
            );
        }
    }

    /// "In an aircraft of the class" (reg 61.745(2)(a)): a session in a
    /// device simulating one is not a flight.
    #[test]
    fn a_session_is_not_taken_for_a_flight_of_the_same_id() {
        // Both have the nil id.
        let session = simulated("s", ON, "C172");
        assert!(is_class_rating(&entry(session.clone()), Sea));
        let review = assessed(
            "a",
            ON,
            0,
            AssessmentInformation {
                check_type: Some(CheckType::FlightReview),
                outcome: Some(CheckOutcome::Passed),
                check_purposes: Vec::new(),
            },
        );
        assert!(!result(Sea, &[session], &[review]).is_current());
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
