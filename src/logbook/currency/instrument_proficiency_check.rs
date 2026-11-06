// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.880: the instrument proficiency check of the holder of an
//! instrument rating.

use logbook_shared_kernel::authority::window::{
    add_months_clamped, count_in_window, end_of_nth_month_after,
};
use logbook_shared_kernel::{
    Assessment, AssessmentAccumulator, AssessmentCurrencyCheck, AssessmentId, CurrencyResult,
};
use time::{Date, Duration};

use crate::logbook::{assessment_information, CheckOutcome, CheckPurpose, CheckType};

const REQUIRED: usize = 1;
/// The look-back for the assessments reported as supporting evidence: 12
/// months and a month's early renewal, as pilot-logbook had it.
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
/// instrument proficiency check for the aircraft category". Here, one
/// passed check of reg 61.880(3)'s kinds, valid "to the end of the 12th
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
/// expires" runs from that expiry (reg 61.880(3)(f)). A failed instrument
/// proficiency check after the latest qualifying pass ends the validity
/// (reg 61.880(4)); a failed check of another kind does not.
///
/// What a check was, its outcome and its purpose are CASA's information on
/// the assessment: one with none, or with information that cannot be read,
/// counts for nothing, failed or passed.
///
/// Whether an endorsement test qualifies depends on the rating tests before
/// it, so the passes are kept and decided in
/// [`result`](AssessmentCurrencyCheck::result), over the whole set, which
/// makes the answer independent of the order assessments arrive in.
pub(crate) struct InstrumentProficiencyCheck {
    reference: Date,
    passed: Vec<Passed>,
    /// The dates of the failed instrument proficiency checks.
    failed: Vec<Date>,
}

impl InstrumentProficiencyCheck {
    pub(crate) fn new(reference: Date) -> Self {
        Self {
            reference,
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
        match information.outcome {
            Some(CheckOutcome::Passed) => {}
            Some(CheckOutcome::Failed) => {
                if information.check_type == Some(CheckType::InstrumentProficiencyCheck) {
                    self.failed.push(assessment.date);
                }
                return;
            }
            None => return,
        }
        let basis = match (information.check_type, information.check_purpose) {
            (Some(CheckType::FlightTest), Some(CheckPurpose::InstrumentRating)) => {
                Basis::RatingTest
            }
            (Some(CheckType::FlightTest), Some(CheckPurpose::InstrumentEndorsement)) => {
                Basis::EndorsementTest
            }
            (
                Some(CheckType::InstrumentProficiencyCheck | CheckType::OperatorProficiencyCheck),
                _,
            ) => Basis::ProficiencyCheck,
            // A flight test with no purpose, or a check of another kind.
            _ => return,
        };
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
        let mut events: Vec<&Passed> = self
            .passed
            .iter()
            .filter(|passed| Self::qualifies(passed, &rating_tests))
            .collect();
        events.sort_by(|a, b| a.on.cmp(&b.on).then_with(|| a.id.cmp(&b.id)));

        let Some(latest) = events.last() else {
            return self.none();
        };
        if self.failed.iter().any(|failed| *failed > latest.on) {
            return self.none();
        }

        // The last day each pass is valid to, chaining an early renewal
        // from the expiry it renews.
        let mut valid_to: Option<Date> = None;
        for event in &events {
            let natural = end_of_nth_month_after(event.on, VALID_MONTHS).unwrap_or(Date::MAX);
            let renewed = valid_to.filter(|existing| {
                let window_opens =
                    add_months_clamped(*existing, -EARLY_RENEWAL_MONTHS).unwrap_or(Date::MIN);
                window_opens <= event.on && event.on <= *existing
            });
            let to = match renewed {
                Some(existing) => {
                    end_of_nth_month_after(existing, VALID_MONTHS).unwrap_or(Date::MAX)
                }
                None => natural,
            };
            valid_to = Some(valid_to.map_or(to, |existing| existing.max(to)));
        }

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
        // after it, past the display window: still show what does.
        if shown.is_empty() && valid_to.is_some_and(|to| to >= self.reference) {
            shown.push(latest.id.clone());
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
    use crate::logbook::test_support::assessed;
    use crate::logbook::{casa, AssessmentInformation};

    const REFERENCE: Date = date!(2024 - 09 - 01);

    /// A check of this type with this outcome and purpose.
    fn checked(
        id: &str,
        on: Date,
        check_type: CheckType,
        outcome: CheckOutcome,
        check_purpose: Option<CheckPurpose>,
    ) -> Assessment {
        assessed(
            id,
            on,
            1,
            AssessmentInformation {
                check_type: Some(check_type),
                outcome: Some(outcome),
                check_purpose,
            },
        )
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

    fn result_on(reference: Date, assessments: &[Assessment]) -> CurrencyResult {
        let mut check = InstrumentProficiencyCheck::new(reference);
        for assessment in assessments {
            check.accept_assessment(assessment);
        }
        check.result()
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

        // A failed check of another kind does not end the validity.
        let failed_operator = checked(
            "failed opc",
            date!(2024 - 08 - 22),
            CheckType::OperatorProficiencyCheck,
            CheckOutcome::Failed,
            None,
        );
        assert!(result(&[ipc("ipc", on, CheckOutcome::Passed), failed_operator]).is_current());
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

        // 2024-02-01 is before 2024-03-30, so it runs from its own month.
        let natural = result(&[
            ipc("a", date!(2023 - 06 - 01), CheckOutcome::Passed),
            ipc("b", date!(2024 - 02 - 01), CheckOutcome::Passed),
        ]);
        assert_eq!(natural.current_through(), Some(date!(2025 - 02 - 28)));
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
}
