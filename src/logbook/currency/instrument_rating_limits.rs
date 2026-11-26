// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.860(2) and (3): the limits on an instrument rating that turn on
//! what its flight test or a proficiency check included.

use logbook_shared_kernel::CurrencyResult;

use super::assessed_flights::{AssessedFlights, Pass, Passed};

/// "Within the previous 12 months" (reg 61.860(3)(a)), in calendar months.
const CIRCLING_TEST_MONTHS: u32 = 12;

/// Reg 61.860(2): "the holder of an instrument rating is authorised to pilot
/// an aircraft in a single-pilot operation under the IFR only if the holder
/// has: (a) passed the flight test for the rating in a single-pilot
/// aircraft; or (b) completed an instrument proficiency check in a
/// single-pilot aircraft".
///
/// Met for good by a passed flight test for the rating, or a passed
/// instrument proficiency check (not an operator proficiency check, which
/// reg 61.860(4) counts for subregulation (3)(b) only), whose flight was a
/// single-pilot operation as reg 61.875's check reads one: no entry for it
/// logged as co-pilot. The regulation sets no period. The latest is
/// reported.
pub(super) fn single_pilot_operation(assessed: &AssessedFlights) -> CurrencyResult {
    let passes = assessed.passes(|pass| {
        matches!(pass.passed, Passed::RatingTest | Passed::ProficiencyCheck)
            && pass.flown.is_some_and(|flown| flown.single_pilot)
    });
    match passes.last() {
        Some(latest) => assessed.never(latest),
        None => assessed.within(0, &[]),
    }
}

/// Reg 61.860(3): "the holder of an instrument rating is authorised to
/// conduct a circling approach under the IFR on a flight only if: (a) the
/// holder passed the flight test for the rating within the previous 12
/// months, and the flight test included a circling approach; or (b) the
/// holder's most recent instrument proficiency check included a circling
/// approach; or (c)" an approved operator's training and checking system,
/// which is not recorded.
///
/// (b) holds, with no end, while the most recent passed instrument or
/// operator proficiency check (reg 61.860(4)) recorded an approach that
/// ended in a circling manoeuvre on its entry; otherwise, (a) while a
/// passed flight test for the rating whose entry recorded one is within the
/// previous 12 months, to the day. A check whose entry is not a flight that
/// counts included none that is recorded.
pub(super) fn circling_approach(assessed: &AssessedFlights) -> CurrencyResult {
    let circled = |pass: &Pass| pass.flown.is_some_and(|flown| flown.circling);
    if let Some(check) = assessed.most_recent_check().filter(|check| circled(check)) {
        return assessed.never(check);
    }
    let tests = assessed.passes(|pass| pass.passed == Passed::RatingTest && circled(pass));
    assessed.within(CIRCLING_TEST_MONTHS, &tests)
}

#[cfg(test)]
mod tests {
    use icao_shared_kernel::Capacity;
    use logbook_shared_kernel::{Approach, ApproachType, AssessmentId, Entry, EntryParts, Expiry};
    use time::macros::date;
    use time::Date;

    use super::super::assessed_flights::{checked, passed};
    use super::*;
    use crate::logbook::test_support::{aeroplane, approach, circled, entry, simulated};

    const REFERENCE: Date = date!(2026 - 09 - 01);

    fn flight(id: &str, on: Date, approaches: Vec<Approach>) -> Entry {
        entry(EntryParts {
            approaches,
            ..aeroplane(id, on)
        })
    }

    #[test]
    fn a_single_pilot_rating_test_or_proficiency_check_is_met_for_good() {
        let on = date!(2010 - 01 - 05);
        let entries = [flight("e", on, Vec::new())];
        for kind in [Passed::RatingTest, Passed::ProficiencyCheck] {
            let r = checked(
                single_pilot_operation,
                REFERENCE,
                &entries,
                &[passed("a", on, "e", kind)],
            );
            assert_eq!(r.expiry, Some(Expiry::Never));
            assert_eq!(r.qualifying_assessments.len(), 1);
        }
    }

    #[test]
    fn an_operator_check_endorsement_test_or_multi_crew_flight_does_not_meet_single_pilot() {
        let on = date!(2026 - 01 - 05);
        let co_pilot = entry(EntryParts {
            capacity: Capacity::CoPilot,
            ..aeroplane("c", on)
        });
        let entries = [flight("e", on, Vec::new()), co_pilot];
        let assessments = [
            passed("op", on, "e", Passed::OperatorCheck),
            passed("end", on, "e", Passed::EndorsementTest),
            passed("ipc", on, "c", Passed::ProficiencyCheck),
            // Dated after the reference: not yet passed.
            passed("later", date!(2026 - 09 - 02), "e", Passed::RatingTest),
        ];
        let r = checked(single_pilot_operation, REFERENCE, &entries, &assessments);
        assert_eq!(r.expiry, None);
        assert!(!r.is_current());
    }

    #[test]
    fn the_most_recent_check_with_a_circling_approach_holds_until_the_next() {
        let entries = [
            flight(
                "circled",
                date!(2020 - 01 - 01),
                vec![circled(ApproachType::Vor)],
            ),
            flight(
                "straight",
                date!(2026 - 01 - 01),
                vec![approach(ApproachType::Vor)],
            ),
        ];
        let earlier = passed(
            "old",
            date!(2020 - 01 - 01),
            "circled",
            Passed::OperatorCheck,
        );
        let r = checked(circling_approach, REFERENCE, &entries, &[earlier]);
        assert_eq!(r.expiry, Some(Expiry::Never));
        assert_eq!(r.qualifying_assessments, [AssessmentId::new("old")]);

        // A later check with no circling approach replaces it.
        let later = passed(
            "new",
            date!(2026 - 01 - 01),
            "straight",
            Passed::ProficiencyCheck,
        );
        let earlier = passed(
            "old",
            date!(2020 - 01 - 01),
            "circled",
            Passed::OperatorCheck,
        );
        let r = checked(circling_approach, REFERENCE, &entries, &[later, earlier]);
        assert_eq!(r.expiry, None);
    }

    #[test]
    fn a_check_in_a_session_included_no_recorded_circling_approach() {
        let entries = [
            flight(
                "circled",
                date!(2025 - 01 - 01),
                vec![circled(ApproachType::Vor)],
            ),
            entry(EntryParts {
                approaches: vec![circled(ApproachType::Vor)],
                ..simulated("s", date!(2026 - 01 - 01), "C172")
            }),
        ];
        let assessments = [
            passed(
                "flown",
                date!(2025 - 01 - 01),
                "circled",
                Passed::ProficiencyCheck,
            ),
            passed(
                "session",
                date!(2026 - 01 - 01),
                "s",
                Passed::ProficiencyCheck,
            ),
        ];
        let r = checked(circling_approach, REFERENCE, &entries, &assessments);
        assert_eq!(r.expiry, None);
    }

    #[test]
    fn a_rating_test_with_a_circling_approach_counts_for_12_months_to_the_day() {
        let on = date!(2025 - 09 - 02);
        let entries = [flight("e", on, vec![circled(ApproachType::Vor)])];
        let r = checked(
            circling_approach,
            REFERENCE,
            &entries,
            &[passed("t", on, "e", Passed::RatingTest)],
        );
        assert_eq!(r.expiry, Some(Expiry::On(date!(2026 - 09 - 02))));
        assert!(r.is_current());
        let r = checked(
            circling_approach,
            date!(2026 - 09 - 02),
            &entries,
            &[passed("t", on, "e", Passed::RatingTest)],
        );
        assert!(!r.is_current());

        // An endorsement test is not the flight test for the rating.
        let r = checked(
            circling_approach,
            REFERENCE,
            &entries,
            &[passed("t", on, "e", Passed::EndorsementTest)],
        );
        assert_eq!(r.expiry, None);
    }
}
