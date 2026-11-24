// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.745: the flight review for an aircraft class rating.

use std::collections::BTreeMap;

use logbook_shared_kernel::authority::window::add_months_clamped;
use logbook_shared_kernel::{
    Assessment, AssessmentAccumulator, AssessmentCurrencyCheck, AssessmentId, CurrencyResult,
    Entry, EntryId, SubjectId,
};
use time::Date;

use super::renewal;
use crate::aircraft::AircraftClassRating;
use crate::logbook::guards::is_class_rating;
use crate::logbook::{assessment_information, AssessmentInformation, CheckOutcome, CheckType};

/// "The end of the 24th month after the month in which the holder
/// completes the review" (reg 61.745(1A)(a)).
const VALID_MONTHS: u32 = 24;
/// "The validity of the previous flight review is due to expire within 3
/// months after the holder successfully completes the flight review" (reg
/// 61.745(1A)(b)(ii)).
const EARLY_RENEWAL_MONTHS: i32 = 3;
const REQUIRED: usize = 1;

/// Reg 61.745(1): the holder of an aircraft class rating may exercise its
/// privileges "only if the holder has a valid flight review for the
/// rating", which is valid to "the end of the 24th month after the month in
/// which the holder completes the review" (reg 61.745(1A)(a)): a review
/// completed on any day of a month is valid to the last day of the 24th
/// month after it, so one completed on 1 March 2024 and one on 31 March 2024
/// are both valid to 31 March 2026. If "the holder already has a
/// valid flight review for the rating (the previous flight review)" and its
/// "validity ... is due to expire within 3 months after the holder
/// successfully completes the flight review", the review is valid "at the
/// end of the 24th month after the validity of the previous flight review
/// expires" (reg 61.745(1A)(b)), as the instrument proficiency check's is
/// ([`renewal`]): one completed from 1 January 2026 to the last day of a
/// previous flight review valid to 31 March 2026 is valid to 31 March 2028.
/// Every check that satisfies [`satisfies_flight_review`] is "taken to have
/// successfully completed a flight review" for that (reg 61.745(3)), so
/// each may renew. The previous flight review is read as the one valid
/// longest. The review the holder is valid by is the one valid longest.
///
/// A check counts when CASA's information on the assessment records it as
/// passed ([`passed`]: a flight test, for any of its purposes) and as a
/// check that reg 61.745(3) takes as a completed flight
/// review ([`satisfies_flight_review`]), and it was conducted "in an
/// aircraft of the class covered by the rating" (reg 61.745(2)(a)). The
/// assessment's aircraft is that of the entry it names: an assessment whose
/// entry is not among those given, is of a session, or whose entry's type is
/// of no known class, does not count. A failed check counts for nothing; it does
/// not end a valid review.
pub(crate) struct FlightReview<'a> {
    reference: Date,
    /// The entries that record a flight in an aircraft of the class.
    entries: BTreeMap<&'a EntryId, &'a Entry>,
    /// The checks that count, by the date of each, up to the reference.
    passed: Vec<(Date, AssessmentId)>,
}

impl<'a> FlightReview<'a> {
    /// The check for `class_rating`, finding each assessment's aircraft
    /// among `entries`. The caller passes the entries
    /// [`Casa::counts`](crate::Casa) lets through, as the kernel's `run`
    /// would: this check is not run over entries, so it does not apply that
    /// gate itself.
    pub(crate) fn new(
        reference: Date,
        class_rating: AircraftClassRating,
        entries: impl IntoIterator<Item = &'a Entry>,
    ) -> Self {
        let entries = entries
            .into_iter()
            .filter(|entry| matches!(entry.subject().id(), SubjectId::Flight(_)))
            .filter(|entry| is_class_rating(entry, class_rating))
            .map(|entry| (entry.id(), entry))
            .collect();
        Self {
            reference,
            entries,
            passed: Vec::new(),
        }
    }
}

/// The latest day a review can have been completed on and not be valid on
/// `reference`, as one that renews nothing: the last day of the 25th month
/// before `reference`'s, since one completed in the 24th month before it is
/// valid to the end of `reference`'s month. A review that renews a previous
/// one within the 3 months before it expires can be valid from up to 3
/// months earlier still (reg 61.745(1A)(b)), so the look-back reaches back
/// that far more. Only the reported `look_back_days` is taken from it;
/// whether a review is valid is [`renewal::validity`]'s.
fn cutoff(reference: Date) -> Date {
    reference
        .replace_day(1)
        .ok()
        .and_then(|first| {
            add_months_clamped(
                first,
                -i32::try_from(VALID_MONTHS).ok()? - EARLY_RENEWAL_MONTHS,
            )
        })
        .and_then(Date::previous_day)
        .unwrap_or(Date::MIN)
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

impl AssessmentAccumulator for FlightReview<'_> {
    fn accept_assessment(&mut self, assessment: &Assessment) {
        // An assessment dated after the reference has not happened yet.
        if assessment.date > self.reference {
            return;
        }
        let Some(entry) = self.entries.get(&assessment.entry) else {
            return;
        };
        let Some(information) = assessment_information(assessment, entry) else {
            return;
        };
        if passed(&information) && information.check_type.is_some_and(satisfies_flight_review) {
            self.passed.push((assessment.date, assessment.id.clone()));
        }
    }
}

impl AssessmentCurrencyCheck for FlightReview<'_> {
    fn result(&self) -> CurrencyResult {
        let mut passed = self.passed.clone();
        passed.sort();
        // Each kind of check that counts renews: reg 61.745(3).
        let passes: Vec<(Date, bool)> = passed.iter().map(|(on, _)| (*on, true)).collect();
        let validity = renewal::validity(&passes, VALID_MONTHS, EARLY_RENEWAL_MONTHS);
        let latest = validity.longest.map(|(to, _)| to);
        let mut valid: Vec<AssessmentId> = passed
            .into_iter()
            .zip(validity.each)
            .filter(|(_, to)| *to >= self.reference)
            .map(|((_, id), _)| id)
            .collect();
        valid.sort();
        CurrencyResult {
            // The first day it is not valid.
            expiry_date: latest.and_then(Date::next_day),
            reference: self.reference,
            qualifying: Vec::new(),
            found: valid.len(),
            qualifying_assessments: valid,
            required: REQUIRED,
            look_back_days: (self.reference - cutoff(self.reference)).whole_days(),
        }
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
        aeroplane, assessed, entry, flight_test, flown, simulated, unclassified,
    };
    use crate::logbook::{casa, CheckPurpose};

    use AircraftClassRating::{MultiEngineAeroplane as Mea, SingleEngineAeroplane as Sea};

    const REFERENCE: Date = date!(2024 - 09 - 01);
    const ON: Date = date!(2024 - 05 - 24);

    /// A check of this type with this outcome, in entry `e`: a flight test's
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
        assessed("a", on, "e", information)
    }

    /// A passed flight review, in entry `e`.
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

    /// Entry `e`, a flight in a single-engine aeroplane.
    fn single_engine() -> Vec<EntryParts> {
        vec![aeroplane("e", ON)]
    }

    #[test]
    fn a_passed_review_is_valid_to_the_end_of_the_24th_month_after_its_month() {
        let r = result(Sea, &single_engine(), &[passed(ON)]);
        assert!(r.is_current());
        assert_eq!(r.found, 1);
        assert_eq!(r.current_through(), Some(date!(2026 - 05 - 31)));
        assert_eq!(r.qualifying_assessments, [AssessmentId::new("a")]);
    }

    /// Every day of a month is valid to the same month's end, a leap
    /// February's included, and across a year's end.
    #[test]
    fn every_day_of_a_month_is_valid_to_the_same_months_end() {
        let current_through = |on| {
            let entries = [aeroplane("e", on)].map(entry);
            let mut check = FlightReview::new(on, Sea, &entries);
            check.accept_assessment(&passed(on));
            check.result().current_through()
        };
        for (first, last, through) in [
            (
                date!(2022 - 02 - 01),
                date!(2022 - 02 - 28),
                date!(2024 - 02 - 29),
            ),
            (
                date!(2023 - 12 - 01),
                date!(2023 - 12 - 31),
                date!(2025 - 12 - 31),
            ),
        ] {
            let mut on = first;
            while on <= last {
                assert_eq!(current_through(on), Some(through), "{on}");
                on = on.next_day().unwrap();
            }
        }
    }

    /// One completed in the 24th month before the reference's is valid to
    /// that month's end; one completed in the month before that is not.
    #[test]
    fn the_windows_edge_is_the_first_day_of_the_24th_month_before_the_references() {
        let inside = result(Sea, &single_engine(), &[passed(date!(2022 - 09 - 01))]);
        assert!(inside.is_current());
        assert_eq!(inside.found, 1);
        assert_eq!(inside.current_through(), Some(date!(2024 - 09 - 30)));
        // Reaching back 3 months further, for a review that renews one.
        assert_eq!(inside.look_back_days, 824);
        let outside = result(Sea, &single_engine(), &[passed(date!(2022 - 08 - 31))]);
        assert!(!outside.is_current());
        assert_eq!(outside.found, 0);
        assert_eq!(outside.expiry_date, Some(REFERENCE));
    }

    /// A review is counted as found exactly while it is valid, and the
    /// look-back reported reaches it whenever it is, over every day of three
    /// years: it reaches back 3 months further, for a review that renews one.
    #[test]
    fn a_review_is_found_exactly_while_it_is_valid() {
        let mut on = date!(2021 - 06 - 01);
        while on <= REFERENCE {
            let r = result(Sea, &single_engine(), &[passed(on)]);
            assert_eq!(r.found == 1, r.is_current(), "{on}");
            let reached = on > REFERENCE - Duration::days(r.look_back_days);
            assert!(reached || !r.is_current(), "{on}");
            on = on.next_day().unwrap();
        }
    }

    /// The latest review is the one the holder's is valid by.
    #[test]
    fn the_latest_passed_review_sets_the_expiry() {
        let earlier = assessed(
            "b",
            date!(2023 - 01 - 10),
            "e",
            AssessmentInformation {
                check_type: Some(CheckType::FlightReview),
                outcome: Some(CheckOutcome::Passed),
                check_purposes: Vec::new(),
            },
        );
        let r = result(Sea, &single_engine(), &[passed(ON), earlier]);
        assert_eq!(r.found, 2);
        assert_eq!(
            r.qualifying_assessments,
            [AssessmentId::new("a"), AssessmentId::new("b")]
        );
        assert_eq!(r.current_through(), Some(date!(2026 - 05 - 31)));
    }

    /// A review completed from the 1st of the 3rd month before the one the
    /// previous flight review ends in runs from that expiry (reg 61.745(1A)(b)),
    /// as the instrument proficiency check's does; one before the window
    /// opens does not, and so shortens nothing but renews nothing.
    #[test]
    fn a_review_in_the_3_months_before_expiry_runs_from_the_expiry() {
        let existing = passed(ON);
        let renewed = |on| {
            let later = assessed(
                "b",
                on,
                "e",
                AssessmentInformation {
                    check_type: Some(CheckType::FlightReview),
                    outcome: Some(CheckOutcome::Passed),
                    check_purposes: Vec::new(),
                },
            );
            let entries = single_engine().into_iter().map(entry).collect::<Vec<_>>();
            let mut check = FlightReview::new(date!(2026 - 07 - 01), Sea, &entries);
            check.accept_assessment(&existing);
            check.accept_assessment(&later);
            let r = check.result();
            r.current_through()
        };
        // The previous flight review is valid to 31 May 2026.
        assert_eq!(renewed(date!(2026 - 02 - 28)), Some(date!(2028 - 02 - 29)));
        assert_eq!(renewed(date!(2026 - 03 - 01)), Some(date!(2028 - 05 - 31)));
        assert_eq!(renewed(date!(2026 - 05 - 31)), Some(date!(2028 - 05 - 31)));
        assert_eq!(renewed(date!(2026 - 06 - 01)), Some(date!(2028 - 06 - 30)));
    }

    #[test]
    fn only_a_passed_check_counts() {
        let failed = checked(ON, CheckType::FlightReview, CheckOutcome::Failed);
        assert_eq!(result(Sea, &single_engine(), &[failed]).expiry_date, None);
        let no_outcome = assessed(
            "a",
            ON,
            "e",
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
            "e",
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
        let test = |information| assessed("a", ON, "e", information);
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
            "e",
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
            "e",
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
    fn a_check_in_a_session_does_not_count() {
        let session = simulated("s", ON, "C172");
        assert!(is_class_rating(&entry(session.clone()), Sea));
        let review = assessed(
            "a",
            ON,
            "s",
            AssessmentInformation {
                check_type: Some(CheckType::FlightReview),
                outcome: Some(CheckOutcome::Passed),
                check_purposes: Vec::new(),
            },
        );
        assert!(!result(Sea, &[session], &[review]).is_current());
    }

    #[test]
    fn a_check_counts_only_for_the_class_of_its_entrys_aircraft() {
        let multi_engine = [flown("e", ON, "PA44")];
        let helicopter = [flown("e", ON, "R44")];
        let unknown = [unclassified("e", ON)];
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
    fn a_check_whose_entry_is_not_given_does_not_count() {
        assert!(!result(Sea, &[], &[passed(ON)]).is_current());
        let another_entry = [aeroplane("f", ON)];
        assert!(!result(Sea, &another_entry, &[passed(ON)]).is_current());
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
