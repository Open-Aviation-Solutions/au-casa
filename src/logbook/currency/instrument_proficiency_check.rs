// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.880: the instrument proficiency check of the holder of an
//! instrument rating, for an aircraft category or for multi-engine
//! aeroplanes or helicopters.

use std::collections::BTreeSet;

use logbook_shared_kernel::authority::window::{
    add_months_clamped, count_in_window, end_of_nth_month_after,
};
use logbook_shared_kernel::{
    Assessment, AssessmentAccumulator, AssessmentCurrencyCheck, AssessmentId, CurrencyResult,
    Entry, SubjectId,
};
use time::{Date, Duration};

use crate::aircraft::{AircraftCategory, AircraftClassRating};
use crate::logbook::guards::{
    category_of, is_category, is_class_rating, is_multi_engine_helicopter,
};
use crate::logbook::{
    assessment_information, AssessedPurpose, CheckOutcome, CheckPurpose, CheckType,
};

const REQUIRED: usize = 1;
/// The look-back for the assessments reported as supporting evidence:
/// pilot-logbook's, a year and a 30-day buffer. It is not the regulation's.
const DISPLAY_WINDOW_DAYS: i64 = 395;
/// The look-back for counting qualifying events, as pilot-logbook had it.
const FOUND_WINDOW_DAYS: i64 = 365;
/// "The end of the 12th month after the month in which" (reg 61.880(3)).
const VALID_MONTHS: u32 = 12;
/// "More than 6 months after the holder passes the flight test for the
/// rating" (reg 61.880(3)(b)(ii)).
const ENDORSEMENT_AFTER_MONTHS: i32 = 6;
/// "Within 3 months before the validity of the existing check expires"
/// (reg 61.880(3)(f)(ii)).
const EARLY_RENEWAL_MONTHS: i32 = 3;

/// What an instrument proficiency check is for: an aircraft category (reg
/// 61.880(1)), or multi-engine aeroplanes or helicopters (reg 61.880(2)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProficiencyCheckFor {
    /// "A valid instrument proficiency check for the aircraft category"
    /// (reg 61.880(1)).
    Category(AircraftCategory),
    /// "A valid instrument proficiency check for multi-engine aeroplanes"
    /// (reg 61.880(2)(a)).
    MultiEngineAeroplanes,
    /// "A valid instrument proficiency check for multi-engine helicopters"
    /// (reg 61.880(2)(b)).
    MultiEngineHelicopters,
}

impl ProficiencyCheckFor {
    /// Whether the entry's aircraft is a "relevant aircraft" for the check
    /// (reg 61.880(9)): "a single-engine or multi-engine aircraft of that
    /// category" for a category, so a multi-engine aeroplane is one for the
    /// aeroplane category too; a multi-engine aeroplane (the class of reg
    /// 61.020, as Part 61 defines it) for multi-engine aeroplanes; a
    /// multi-engine helicopter for multi-engine helicopters. Its type must
    /// be known to be one.
    fn is_relevant(self, entry: &Entry) -> bool {
        match self {
            Self::Category(category) => is_category(entry, category),
            Self::MultiEngineAeroplanes => {
                is_class_rating(entry, AircraftClassRating::MultiEngineAeroplane)
            }
            Self::MultiEngineHelicopters => is_multi_engine_helicopter(entry),
        }
    }

    /// The category a failed check ends it in: "the aircraft category in
    /// which the holder attempted the failed check", or "multi-engine
    /// aircraft of the category" (reg 61.880(4)).
    fn category(self) -> AircraftCategory {
        match self {
            Self::Category(category) => category,
            Self::MultiEngineAeroplanes => AircraftCategory::Aeroplane,
            Self::MultiEngineHelicopters => AircraftCategory::Helicopter,
        }
    }
}

/// Why a passed assessment can qualify, of the paragraphs of reg 61.880(3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Basis {
    /// (a): "the flight test for the instrument rating".
    RatingTest,
    /// (b): "the flight test for an instrument endorsement", which
    /// qualifies only if it is "conducted more than 6 months after the
    /// holder passes the flight test for the rating".
    EndorsementTest,
    /// (c) and (e): "an operator proficiency check that covers IFR
    /// operations", or "an instrument proficiency check".
    ProficiencyCheck,
}

#[derive(Debug)]
struct Passed {
    id: AssessmentId,
    on: Date,
    basis: Basis,
}

/// Reg 61.880(1): the holder of an instrument rating may exercise its
/// privileges in an aircraft of a category "only if the holder has a valid
/// instrument proficiency check for the aircraft category", and (2), in a
/// multi-engine aeroplane or helicopter, only with one "for multi-engine
/// aeroplanes" or "for multi-engine helicopters". Here, one passed check
/// of reg 61.880(3)'s kinds in a relevant aircraft
/// ([`ProficiencyCheckFor::is_relevant`]), valid "to the end of the 12th
/// month after the month in which" it was passed, as pilot-logbook had it:
///
/// - (a) a passed flight test whose purpose is the instrument rating;
/// - (b) a passed flight test whose purpose is an instrument endorsement,
///   when it is more than 6 months after the latest passed rating test
///   before it;
/// - (c) a passed operator proficiency check (that it covered IFR
///   operations, and its examiner, are not recorded);
/// - (e) a passed instrument proficiency check.
///
/// (d), an operator's training and checking system, is not recorded. A
/// renewal "within 3 months before the validity of the existing check
/// expires" runs from that expiry (reg 61.880(3)(f)), the 3 months before
/// the day after its last valid day. After a failed instrument proficiency
/// check the holder "is no longer taken to have a valid instrument
/// proficiency check" for the category it was attempted in, or for
/// multi-engine aircraft of that category (reg 61.880(4)), so nothing
/// passed before it counts or is renewed by a pass after it; a pass on the
/// same day as the failure is taken as after it. A failure in a
/// single-engine aircraft ends the multi-engine check of its category too.
/// A failure whose aircraft is not known (its flight has no entry, or its
/// type no known category) ends every check, as it may have been in any
/// category; one known to be in another category ends nothing. A failed
/// check of another kind ends nothing.
///
/// A check's aircraft is that of the entry for its flight, as for a flight
/// review: one whose flight has no entry, or whose type is not known to be
/// a relevant aircraft, does not count.
///
/// What a check was, its outcome and a flight test's purposes, each with
/// its own outcome, are CASA's information on the assessment: one with
/// none, or with information that cannot be read, counts for nothing,
/// failed or passed. A flight test passed for both the rating and an
/// endorsement counts as the rating's.
///
/// Whether an endorsement test qualifies depends on the rating tests before
/// it, so the passes are kept and decided in
/// [`result`](AssessmentCurrencyCheck::result), over the whole set, which
/// makes the answer independent of the order assessments arrive in.
pub(crate) struct InstrumentProficiencyCheck {
    reference: Date,
    /// The flights an entry records in a relevant aircraft.
    relevant: BTreeSet<SubjectId>,
    /// The flights whose aircraft is known to be of another category only,
    /// on which a failure does not end this check.
    other_category: BTreeSet<SubjectId>,
    passed: Vec<Passed>,
    /// The dates of the failed instrument proficiency checks that end it.
    failed: Vec<Date>,
}

impl InstrumentProficiencyCheck {
    /// The check for `check_for`, finding each assessment's aircraft among
    /// `entries`. The caller passes the entries
    /// [`Casa::counts`](crate::Casa) lets through, as the kernel's `run`
    /// would: this check is not run over entries, so it does not apply that
    /// gate itself.
    pub(crate) fn new<'a>(
        reference: Date,
        check_for: ProficiencyCheckFor,
        entries: impl IntoIterator<Item = &'a Entry>,
    ) -> Self {
        let mut relevant = BTreeSet::new();
        let mut in_category = BTreeSet::new();
        let mut other_category = BTreeSet::new();
        for entry in entries {
            let flight = entry.subject().id();
            if !matches!(flight, SubjectId::Flight(_)) {
                continue;
            }
            if check_for.is_relevant(entry) {
                relevant.insert(flight);
            }
            match category_of(entry) {
                Some(category) if category == check_for.category() => {
                    in_category.insert(flight);
                }
                Some(_) => {
                    other_category.insert(flight);
                }
                None => {}
            }
        }
        Self {
            reference,
            relevant,
            other_category: &other_category - &in_category,
            passed: Vec::new(),
            failed: Vec::new(),
        }
    }

    fn none(&self) -> CurrencyResult {
        CurrencyResult {
            expiry_date: None,
            reference: self.reference,
            qualifying: Vec::new(),
            qualifying_assessments: Vec::new(),
            found: 0,
            required: REQUIRED,
            look_back_days: DISPLAY_WINDOW_DAYS,
        }
    }

    /// Whether a pass counts, given the dates of every rating test passed.
    fn qualifies(passed: &Passed, rating_tests: &[Date]) -> bool {
        match passed.basis {
            Basis::RatingTest | Basis::ProficiencyCheck => true,
            Basis::EndorsementTest => rating_tests
                .iter()
                .filter(|on| **on < passed.on)
                .max()
                .is_some_and(|latest| {
                    // Six months on from the 31st, in a shorter month, is
                    // that month's last day.
                    passed.on
                        > add_months_clamped(*latest, ENDORSEMENT_AFTER_MONTHS).unwrap_or(Date::MAX)
                }),
        }
    }
}

impl AssessmentAccumulator for InstrumentProficiencyCheck {
    fn accept_assessment(&mut self, assessment: &Assessment) {
        // An assessment dated after the reference has not happened yet.
        if assessment.date > self.reference {
            return;
        }
        let Some(information) = assessment_information(assessment) else {
            return;
        };
        let flight = SubjectId::Flight(assessment.flight);
        let basis = if information.check_type == Some(CheckType::FlightTest) {
            // A flight test's purposes each have their own outcome, and it
            // has none of its own; a failed one ends nothing.
            let passed = |purpose| {
                information.check_purposes.contains(&AssessedPurpose {
                    purpose,
                    outcome: CheckOutcome::Passed,
                })
            };
            if passed(CheckPurpose::InstrumentRating) {
                Basis::RatingTest
            } else if passed(CheckPurpose::InstrumentEndorsement) {
                Basis::EndorsementTest
            } else {
                // Passed for neither, or for another purpose only.
                return;
            }
        } else {
            match information.outcome {
                Some(CheckOutcome::Passed) => {}
                Some(CheckOutcome::Failed) => {
                    if information.check_type == Some(CheckType::InstrumentProficiencyCheck)
                        && !self.other_category.contains(&flight)
                    {
                        self.failed.push(assessment.date);
                    }
                    return;
                }
                None => return,
            }
            match information.check_type {
                Some(
                    CheckType::InstrumentProficiencyCheck | CheckType::OperatorProficiencyCheck,
                ) => Basis::ProficiencyCheck,
                // A check of another kind.
                _ => return,
            }
        };
        if !self.relevant.contains(&flight) {
            return;
        }
        self.passed.push(Passed {
            id: assessment.id.clone(),
            on: assessment.date,
            basis,
        });
    }
}

impl AssessmentCurrencyCheck for InstrumentProficiencyCheck {
    fn result(&self) -> CurrencyResult {
        let rating_tests: Vec<Date> = self
            .passed
            .iter()
            .filter(|passed| passed.basis == Basis::RatingTest)
            .map(|passed| passed.on)
            .collect();
        // Nothing passed before the latest failure counts after it.
        let failed_on = self.failed.iter().max();
        let mut events: Vec<&Passed> = self
            .passed
            .iter()
            .filter(|passed| failed_on.is_none_or(|failed| passed.on >= *failed))
            .filter(|passed| Self::qualifies(passed, &rating_tests))
            .collect();
        events.sort_by(|a, b| a.on.cmp(&b.on).then_with(|| a.id.cmp(&b.id)));
        if events.is_empty() {
            return self.none();
        }

        // The last day it is valid to, and the pass that made it so,
        // chaining an early renewal from the expiry it renews.
        let mut valid: Option<(Date, &Passed)> = None;
        for event in &events {
            let renewed = valid.map(|(to, _)| to).filter(|existing| {
                // "Within 3 months before the validity ... expires": the
                // validity ends with its last day, so the 3 months run back
                // from the day after.
                let window_opens = existing
                    .next_day()
                    .and_then(|expires| add_months_clamped(expires, -EARLY_RENEWAL_MONTHS))
                    .unwrap_or(Date::MIN);
                window_opens <= event.on && event.on <= *existing
            });
            let to = end_of_nth_month_after(renewed.unwrap_or(event.on), VALID_MONTHS)
                .unwrap_or(Date::MAX);
            if valid.is_none_or(|(existing, _)| to >= existing) {
                valid = Some((to, event));
            }
        }
        let valid_to = valid.map(|(to, _)| to);

        let dates: Vec<Date> = events.iter().map(|event| event.on).collect();
        let shown_after = self
            .reference
            .saturating_sub(Duration::days(DISPLAY_WINDOW_DAYS));
        let found_after = self
            .reference
            .saturating_sub(Duration::days(FOUND_WINDOW_DAYS));
        let mut shown: Vec<AssessmentId> = events
            .iter()
            .filter(|event| event.on > shown_after)
            .map(|event| event.id.clone())
            .collect();
        // An early renewal can keep the pilot current for up to 15 months
        // after it, past the display window: still show the pass that does.
        if let Some((to, giving)) = valid.filter(|_| shown.is_empty()) {
            if to >= self.reference {
                shown.push(giving.id.clone());
            }
        }
        CurrencyResult {
            // The first day it is not valid.
            expiry_date: valid_to.and_then(Date::next_day),
            reference: self.reference,
            qualifying: Vec::new(),
            qualifying_assessments: shown,
            found: count_in_window(&dates, found_after),
            required: REQUIRED,
            look_back_days: DISPLAY_WINDOW_DAYS,
        }
    }
}

#[cfg(test)]
mod tests {
    use logbook_shared_kernel::AuthoritySpecificInformation;
    use serde_json::json;
    use time::macros::date;

    use super::*;
    use crate::logbook::test_support::{
        aeroplane, assessed, entry, flight_test, flown, on_flight, unclassified,
    };
    use crate::logbook::{casa, AssessmentInformation};

    use AircraftCategory::{Aeroplane, Helicopter};
    use ProficiencyCheckFor::{Category, MultiEngineAeroplanes, MultiEngineHelicopters};

    const REFERENCE: Date = date!(2024 - 09 - 01);

    /// A check of this type with this outcome and purpose: a flight test's
    /// outcome is its purpose's, as it has none of its own.
    fn checked(
        id: &str,
        on: Date,
        check_type: CheckType,
        outcome: CheckOutcome,
        check_purpose: Option<CheckPurpose>,
    ) -> Assessment {
        let information = if check_type == CheckType::FlightTest {
            let purposes: Vec<_> = check_purpose
                .map(|purpose| (purpose, outcome))
                .into_iter()
                .collect();
            flight_test(&purposes)
        } else {
            AssessmentInformation {
                check_type: Some(check_type),
                outcome: Some(outcome),
                check_purposes: Vec::new(),
            }
        };
        assessed(id, on, 1, information)
    }

    fn ipc(id: &str, on: Date, outcome: CheckOutcome) -> Assessment {
        checked(id, on, CheckType::InstrumentProficiencyCheck, outcome, None)
    }

    /// A passed flight test for this purpose.
    fn test(id: &str, on: Date, purpose: CheckPurpose) -> Assessment {
        checked(
            id,
            on,
            CheckType::FlightTest,
            CheckOutcome::Passed,
            Some(purpose),
        )
    }

    /// The flights the checks here are on: 1 in a single-engine aeroplane
    /// (a C172), 2 in a multi-engine aeroplane (a PA44), 3 in a
    /// single-engine helicopter (an R44), 4 in a multi-engine helicopter
    /// (an EC35) and 5 in a type of no known category; flight 6 has no
    /// entry.
    fn flights() -> Vec<Entry> {
        [
            aeroplane("1", REFERENCE),
            flown("2", REFERENCE, "PA44"),
            flown("3", REFERENCE, "R44"),
            flown("4", REFERENCE, "EC35"),
            unclassified("5", REFERENCE),
        ]
        .into_iter()
        .zip(1..)
        .map(|(parts, flight)| entry(on_flight(flight, parts)))
        .collect()
    }

    /// The check for `check_for` over these assessments, on [`flights`].
    fn result_for(
        check_for: ProficiencyCheckFor,
        reference: Date,
        assessments: &[Assessment],
    ) -> CurrencyResult {
        let mut check = InstrumentProficiencyCheck::new(reference, check_for, &flights());
        for assessment in assessments {
            check.accept_assessment(assessment);
        }
        check.result()
    }

    /// The aeroplane category's check, the checks above being on flight 1.
    fn result_on(reference: Date, assessments: &[Assessment]) -> CurrencyResult {
        result_for(Category(Aeroplane), reference, assessments)
    }

    fn result(assessments: &[Assessment]) -> CurrencyResult {
        result_on(REFERENCE, assessments)
    }

    fn ids(result: &CurrencyResult) -> Vec<&str> {
        result
            .qualifying_assessments
            .iter()
            .map(AssessmentId::as_str)
            .collect()
    }

    #[test]
    fn a_passed_ipc_is_valid_to_the_end_of_the_12th_month_after_its_month() {
        let r = result(&[ipc("a", date!(2024 - 08 - 02), CheckOutcome::Passed)]);
        assert!(r.is_current());
        assert_eq!(r.found, 1);
        assert_eq!(r.current_through(), Some(date!(2025 - 08 - 31)));
        assert_eq!(r.expiry_date, Some(date!(2025 - 09 - 01)));
        assert_eq!(ids(&r), ["a"]);

        let passed = [ipc("a", date!(2024 - 08 - 02), CheckOutcome::Passed)];
        assert!(result_on(date!(2025 - 08 - 31), &passed).is_current());
        assert!(!result_on(date!(2025 - 09 - 01), &passed).is_current());
    }

    #[test]
    fn an_expired_pass_has_a_past_expiry_and_none_found() {
        let r = result(&[ipc("a", date!(2023 - 07 - 28), CheckOutcome::Passed)]);
        assert!(!r.is_current());
        assert_eq!(r.found, 0);
        assert_eq!(r.expiry_date, Some(date!(2024 - 08 - 01)));
        assert_eq!(result(&[]).expiry_date, None);
    }

    #[test]
    fn a_failed_ipc_ends_the_validity_only_after_the_latest_pass() {
        let pass_then_fail = result(&[
            ipc("p", date!(2024 - 07 - 03), CheckOutcome::Passed),
            ipc("f", date!(2024 - 08 - 22), CheckOutcome::Failed),
        ]);
        assert!(!pass_then_fail.is_current());
        assert_eq!(pass_then_fail.expiry_date, None);
        assert_eq!(pass_then_fail.found, 0);

        let fail_then_pass = result(&[
            ipc("f", date!(2024 - 05 - 04), CheckOutcome::Failed),
            ipc("p", date!(2024 - 08 - 02), CheckOutcome::Passed),
        ]);
        assert!(fail_then_pass.is_current());

        let fail_alone = result(&[ipc("f", date!(2024 - 08 - 22), CheckOutcome::Failed)]);
        assert!(!fail_alone.is_current());
        assert_eq!(fail_alone.expiry_date, None);
    }

    #[test]
    fn a_pass_after_a_failed_ipc_renews_nothing_before_it() {
        // Valid to 2024-06-30, but failed on 2024-03-01: the pass on
        // 2024-04-15 is not an early renewal, and runs from its own month.
        let r = result(&[
            ipc("a", date!(2023 - 06 - 01), CheckOutcome::Passed),
            ipc("f", date!(2024 - 03 - 01), CheckOutcome::Failed),
            ipc("b", date!(2024 - 04 - 15), CheckOutcome::Passed),
        ]);
        assert_eq!(r.current_through(), Some(date!(2025 - 04 - 30)));
        assert_eq!(ids(&r), ["b"]);

        // Renewed to 2025-06-30, then failed: a pass after the failure is
        // valid to its own end, not the renewal's.
        let r = result(&[
            ipc("a", date!(2023 - 06 - 01), CheckOutcome::Passed),
            ipc("b", date!(2024 - 04 - 15), CheckOutcome::Passed),
            ipc("f", date!(2024 - 05 - 01), CheckOutcome::Failed),
            ipc("c", date!(2024 - 05 - 10), CheckOutcome::Passed),
        ]);
        assert_eq!(r.current_through(), Some(date!(2025 - 05 - 31)));
        assert_eq!(r.found, 1);
        assert_eq!(ids(&r), ["c"]);
    }

    #[test]
    fn only_reg_61_880_3s_checks_count() {
        let on = date!(2024 - 08 - 02);
        let operator = checked(
            "opc",
            on,
            CheckType::OperatorProficiencyCheck,
            CheckOutcome::Passed,
            None,
        );
        for assessment in [
            ipc("ipc", on, CheckOutcome::Passed),
            test("ir", on, CheckPurpose::InstrumentRating),
            operator,
        ] {
            assert!(
                result(std::slice::from_ref(&assessment)).is_current(),
                "{}",
                assessment.id.as_str()
            );
        }

        let no_purpose = checked(
            "no purpose",
            on,
            CheckType::FlightTest,
            CheckOutcome::Passed,
            None,
        );
        let review = checked(
            "review",
            on,
            CheckType::FlightReview,
            CheckOutcome::Passed,
            None,
        );
        for assessment in [no_purpose, review] {
            assert!(
                !result(std::slice::from_ref(&assessment)).is_current(),
                "{}",
                assessment.id.as_str()
            );
        }

        // A failed check of another kind is not a pass, and does not end
        // the validity.
        let failed_test = checked(
            "failed test",
            on,
            CheckType::FlightTest,
            CheckOutcome::Failed,
            Some(CheckPurpose::InstrumentRating),
        );
        let failed_operator = checked(
            "failed opc",
            date!(2024 - 08 - 22),
            CheckType::OperatorProficiencyCheck,
            CheckOutcome::Failed,
            None,
        );
        for assessment in [failed_test, failed_operator.clone()] {
            assert!(
                !result(std::slice::from_ref(&assessment)).is_current(),
                "{}",
                assessment.id.as_str()
            );
        }
        assert!(result(&[ipc("ipc", on, CheckOutcome::Passed), failed_operator]).is_current());
    }

    /// Each purpose of a flight test has its own outcome: a test passed for
    /// the class rating and failed for the instrument rating is no pass of
    /// "the flight test for the instrument rating" (reg 61.880(3)(a)), and
    /// one passed for both the rating and an endorsement counts once, as the
    /// rating's.
    #[test]
    fn a_flight_test_counts_by_its_instrument_purposes_outcomes() {
        use CheckOutcome::{Failed, Passed};
        let on = date!(2024 - 08 - 02);
        let test =
            |purposes: &[(CheckPurpose, CheckOutcome)]| assessed("t", on, 1, flight_test(purposes));
        let class = CheckPurpose::MultiEngineAeroplaneClassRating;
        let partly = test(&[(class, Passed), (CheckPurpose::InstrumentRating, Failed)]);
        assert!(!result(&[partly]).is_current());
        let both = test(&[
            (class, Passed),
            (CheckPurpose::InstrumentRating, Passed),
            (CheckPurpose::InstrumentEndorsement, Passed),
        ]);
        assert_eq!(ids(&result(&[both])), ["t"]);

        // A flight test's own outcome, which does not apply to it, is
        // dropped, not counted: failed, it neither undoes the passed rating
        // nor ends a valid check.
        let with_its_own = assessed(
            "t",
            on,
            1,
            AssessmentInformation {
                outcome: Some(Failed),
                ..flight_test(&[(CheckPurpose::InstrumentRating, Passed)])
            },
        );
        assert_eq!(ids(&result(&[with_its_own])), ["t"]);
    }

    #[test]
    fn an_endorsement_test_counts_only_more_than_6_months_after_a_rating_test() {
        let rating = test("ir", date!(2023 - 11 - 06), CheckPurpose::InstrumentRating);
        let late = test(
            "late",
            date!(2024 - 05 - 24),
            CheckPurpose::InstrumentEndorsement,
        );
        assert_eq!(ids(&result(&[rating, late])), ["ir", "late"]);

        let recent = test("ir", date!(2024 - 05 - 24), CheckPurpose::InstrumentRating);
        let early = test(
            "early",
            date!(2024 - 06 - 23),
            CheckPurpose::InstrumentEndorsement,
        );
        assert_eq!(ids(&result(&[recent, early])), ["ir"]);

        let alone = test(
            "alone",
            date!(2024 - 08 - 02),
            CheckPurpose::InstrumentEndorsement,
        );
        assert!(!result(&[alone]).is_current());
    }

    #[test]
    fn six_months_after_the_31st_is_the_last_day_of_a_shorter_month() {
        let rating = test("ir", date!(2025 - 08 - 31), CheckPurpose::InstrumentRating);
        let on_the_day = test(
            "on",
            date!(2026 - 02 - 28),
            CheckPurpose::InstrumentEndorsement,
        );
        let after = test(
            "after",
            date!(2026 - 03 - 01),
            CheckPurpose::InstrumentEndorsement,
        );
        // "on" is six months on, not more than six; "after" is more.
        let r = result_on(date!(2026 - 06 - 01), &[rating, on_the_day, after]);
        assert_eq!(ids(&r), ["ir", "after"]);
    }

    #[test]
    fn the_result_does_not_depend_on_the_order_assessments_arrive_in() {
        let rating = test("ir", date!(2023 - 11 - 06), CheckPurpose::InstrumentRating);
        let endorsement = test(
            "end",
            date!(2024 - 05 - 24),
            CheckPurpose::InstrumentEndorsement,
        );
        let forward = result(&[rating.clone(), endorsement.clone()]);
        let backward = result(&[endorsement, rating]);
        assert_eq!(forward, backward);
        assert_eq!(ids(&forward), ["ir", "end"]);
    }

    #[test]
    fn a_renewal_in_the_3_months_before_expiry_runs_from_that_expiry() {
        // The first is valid to 2024-06-30, and 2024-04-15 is within the 3
        // months before.
        let chained = result(&[
            ipc("a", date!(2023 - 06 - 01), CheckOutcome::Passed),
            ipc("b", date!(2024 - 04 - 15), CheckOutcome::Passed),
        ]);
        assert_eq!(chained.current_through(), Some(date!(2025 - 06 - 30)));

        // 2024-02-01 is before 2024-04-01, so it runs from its own month.
        let natural = result(&[
            ipc("a", date!(2023 - 06 - 01), CheckOutcome::Passed),
            ipc("b", date!(2024 - 02 - 01), CheckOutcome::Passed),
        ]);
        assert_eq!(natural.current_through(), Some(date!(2025 - 02 - 28)));
    }

    #[test]
    fn the_renewal_window_is_the_3_months_before_the_day_after_the_expiry() {
        let renewed_on = |first: Date, second: Date| {
            result(&[
                ipc("a", first, CheckOutcome::Passed),
                ipc("b", second, CheckOutcome::Passed),
            ])
            .current_through()
        };
        // Valid to 2024-06-30: the window opens on 2024-04-01.
        let june = date!(2023 - 06 - 01);
        assert_eq!(
            renewed_on(june, date!(2024 - 03 - 31)),
            Some(date!(2025 - 03 - 31))
        );
        assert_eq!(
            renewed_on(june, date!(2024 - 04 - 01)),
            Some(date!(2025 - 06 - 30))
        );
        // Valid to 2024-02-29: the window opens on 2023-12-01.
        let february = date!(2023 - 02 - 10);
        assert_eq!(
            renewed_on(february, date!(2023 - 11 - 30)),
            Some(date!(2024 - 11 - 30))
        );
        assert_eq!(
            renewed_on(february, date!(2023 - 12 - 01)),
            Some(date!(2025 - 02 - 28))
        );
    }

    #[test]
    fn found_counts_the_passes_of_the_previous_365_days() {
        let on = |days| ipc("a", REFERENCE - Duration::days(days), CheckOutcome::Passed);
        assert_eq!(result(&[on(364)]).found, 1);
        assert_eq!(result(&[on(365)]).found, 0);
    }

    #[test]
    fn a_pilot_current_on_an_early_renewal_is_shown_the_renewal() {
        // 14 months on, the renewal is outside the 395 days shown, but it is
        // what keeps the pilot current.
        let r = result_on(
            date!(2025 - 06 - 15),
            &[
                ipc("a", date!(2023 - 06 - 01), CheckOutcome::Passed),
                ipc("b", date!(2024 - 04 - 15), CheckOutcome::Passed),
            ],
        );
        assert!(r.is_current());
        assert_eq!(ids(&r), ["b"]);

        // A later pass valid to an earlier day is not what keeps them
        // current.
        let r = result_on(
            date!(2025 - 06 - 15),
            &[
                ipc("a", date!(2023 - 06 - 01), CheckOutcome::Passed),
                ipc("b", date!(2024 - 04 - 15), CheckOutcome::Passed),
                ipc("c", date!(2024 - 05 - 01), CheckOutcome::Passed),
            ],
        );
        assert_eq!(r.current_through(), Some(date!(2025 - 06 - 30)));
        assert_eq!(ids(&r), ["b"]);
    }

    #[test]
    fn an_assessment_dated_after_the_reference_has_not_happened_yet() {
        let tomorrow = REFERENCE + Duration::days(1);
        assert!(!result(&[ipc("future", tomorrow, CheckOutcome::Passed)]).is_current());
        // Nor does a failure after the reference end a valid pass.
        let r = result(&[
            ipc("a", date!(2024 - 08 - 02), CheckOutcome::Passed),
            ipc("f", tomorrow, CheckOutcome::Failed),
        ]);
        assert!(r.is_current());
    }

    #[test]
    fn a_check_without_casas_readable_information_counts_for_nothing() {
        let on = date!(2024 - 08 - 02);
        let mut unreadable = ipc("a", on, CheckOutcome::Passed);
        unreadable.authority_information.insert(
            casa(),
            AuthoritySpecificInformation::new(json!({"_version": 2})),
        );
        let mut none = ipc("a", on, CheckOutcome::Passed);
        none.authority_information.clear();
        for assessment in [unreadable.clone(), none] {
            assert!(!result(&[assessment]).is_current());
        }

        // Nor does an unreadable failure end a valid pass.
        let mut failed = ipc("f", date!(2024 - 08 - 22), CheckOutcome::Failed);
        failed.authority_information = unreadable.authority_information;
        assert!(result(&[ipc("p", on, CheckOutcome::Passed), failed]).is_current());
    }
    /// An instrument proficiency check on this flight.
    fn ipc_on(flight: u128, on: Date, outcome: CheckOutcome) -> Assessment {
        let information = AssessmentInformation {
            check_type: Some(CheckType::InstrumentProficiencyCheck),
            outcome: Some(outcome),
            check_purposes: Vec::new(),
        };
        assessed(&flight.to_string(), on, flight, information)
    }

    /// Which of the checks a pass on each flight makes current: a
    /// multi-engine aeroplane or helicopter is a relevant aircraft for its
    /// category as well (reg 61.880(9)(a)), a single-engine one is not for
    /// multi-engine aircraft, and one not known to be either is for none.
    #[test]
    fn a_pass_counts_for_the_checks_its_aircraft_is_relevant_to() {
        let checks = [
            Category(Aeroplane),
            MultiEngineAeroplanes,
            Category(Helicopter),
            MultiEngineHelicopters,
            Category(AircraftCategory::Gyroplane),
        ];
        let current = |flight| -> Vec<ProficiencyCheckFor> {
            let passed = [ipc_on(flight, date!(2024 - 08 - 02), CheckOutcome::Passed)];
            checks
                .into_iter()
                .filter(|&check_for| result_for(check_for, REFERENCE, &passed).is_current())
                .collect()
        };
        assert_eq!(current(1), [Category(Aeroplane)]);
        assert_eq!(current(2), [Category(Aeroplane), MultiEngineAeroplanes]);
        assert_eq!(current(3), [Category(Helicopter)]);
        assert_eq!(current(4), [Category(Helicopter), MultiEngineHelicopters]);
        assert!(current(5).is_empty());
        assert!(current(6).is_empty());
    }

    /// A failure ends the checks for its category and for multi-engine
    /// aircraft of it (reg 61.880(4)), whatever aircraft of the category it
    /// was in; one in another category ends neither, and one in an aircraft
    /// not known ends both.
    #[test]
    fn a_failure_ends_the_checks_of_the_category_it_was_attempted_in() {
        let passed = ipc_on(2, date!(2024 - 07 - 03), CheckOutcome::Passed);
        let after = |flight| {
            [
                passed.clone(),
                ipc_on(flight, date!(2024 - 08 - 22), CheckOutcome::Failed),
            ]
        };
        for check_for in [Category(Aeroplane), MultiEngineAeroplanes] {
            for flight in [1, 2, 5, 6] {
                let r = result_for(check_for, REFERENCE, &after(flight));
                assert!(!r.is_current(), "{check_for:?} after a failure on {flight}");
            }
            for flight in [3, 4] {
                let r = result_for(check_for, REFERENCE, &after(flight));
                assert!(r.is_current(), "{check_for:?} after a failure on {flight}");
            }
        }
    }
}
