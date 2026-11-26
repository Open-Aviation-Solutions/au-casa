// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.860(2) and (3): the limits on an instrument rating that turn on
//! what its flight test or a proficiency check included.

use logbook_shared_kernel::CurrencyResult;

use super::assessed_flights::{AssessedFlights, Basis, Pass};

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
/// logged as co-pilot. That stands in for the regulation's "single-pilot
/// aircraft", an aircraft certificated for one pilot, which nothing recorded
/// gives (the owner, on au-casa#116). The regulation sets no period. The
/// latest is reported.
pub(super) fn single_pilot_operation(assessed: &AssessedFlights) -> CurrencyResult {
    let passes = assessed.passes(|pass| {
        matches!(pass.basis, Basis::RatingTest | Basis::ProficiencyCheck)
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
/// (b) holds, with no end, while the most recent check
/// ([`AssessedFlights::most_recent_check`], an operator proficiency check
/// among them, reg 61.860(4)) recorded an approach that ended in a circling
/// manoeuvre on its flight; otherwise, (a) while a passed flight test for
/// the rating whose flight recorded one is within the previous 12 months,
/// to the day.
pub(super) fn circling_approach(assessed: &AssessedFlights) -> CurrencyResult {
    if let Some(check) = assessed.most_recent_check(|flown| flown.circling) {
        return assessed.never(check);
    }
    let tests = assessed.passes(|pass: &Pass| {
        pass.basis == Basis::RatingTest && pass.flown.is_some_and(|flown| flown.circling)
    });
    assessed.within(CIRCLING_TEST_MONTHS, &tests)
}

#[cfg(test)]
mod tests {
    use icao_shared_kernel::Capacity;
    use logbook_shared_kernel::{Approach, ApproachType, AssessmentId, Entry, EntryParts, Expiry};
    use time::macros::date;
    use time::Date;

    use super::super::assessed_flights::{checked, flight_of, passed, Basis};
    use super::*;
    use crate::logbook::test_support::{aeroplane, approach, circled, entry, on_flight, simulated};

    const REFERENCE: Date = date!(2026 - 09 - 01);

    fn flight(id: &str, on: Date, approaches: Vec<Approach>) -> Entry {
        entry(on_flight(
            flight_of(id),
            EntryParts {
                approaches,
                ..aeroplane(id, on)
            },
        ))
    }

    #[test]
    fn a_single_pilot_rating_test_or_proficiency_check_is_met_for_good() {
        let on = date!(2010 - 01 - 05);
        let entries = [flight("e", on, Vec::new())];
        for kind in [Basis::RatingTest, Basis::ProficiencyCheck] {
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
            passed("op", on, "e", Basis::OperatorCheck),
            passed("end", on, "e", Basis::EndorsementTest),
            passed("ipc", on, "c", Basis::ProficiencyCheck),
            // Dated after the reference: not yet passed.
            passed("later", date!(2026 - 09 - 02), "e", Basis::RatingTest),
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
        let earlier = || {
            passed(
                "old",
                date!(2020 - 01 - 01),
                "circled",
                Basis::OperatorCheck,
            )
        };
        let r = checked(circling_approach, REFERENCE, &entries, &[earlier()]);
        assert_eq!(r.expiry, Some(Expiry::Never));
        assert_eq!(r.qualifying_assessments, [AssessmentId::new("old")]);

        // A later check with no circling approach replaces it, in whichever
        // order they arrive.
        let later = || {
            passed(
                "new",
                date!(2026 - 01 - 01),
                "straight",
                Basis::ProficiencyCheck,
            )
        };
        for assessments in [[later(), earlier()], [earlier(), later()]] {
            let r = checked(circling_approach, REFERENCE, &entries, &assessments);
            assert_eq!(r.expiry, None);
        }
    }

    /// Two checks on the latest day count if either included one, whatever
    /// their ids.
    #[test]
    fn of_two_checks_on_the_latest_day_either_can_hold_it() {
        let on = date!(2026 - 01 - 01);
        let entries = [
            flight("circled", on, vec![circled(ApproachType::Vor)]),
            flight("straight", on, vec![approach(ApproachType::Vor)]),
        ];
        for (circling, straight) in [("a", "b"), ("b", "a")] {
            let assessments = [
                passed(circling, on, "circled", Basis::ProficiencyCheck),
                passed(straight, on, "straight", Basis::ProficiencyCheck),
            ];
            let r = checked(circling_approach, REFERENCE, &entries, &assessments);
            assert_eq!(r.expiry, Some(Expiry::Never));
            assert_eq!(r.qualifying_assessments, [AssessmentId::new(circling)]);
        }
    }

    /// A rating test is a proficiency check (reg 61.880(3)(a)), so a later
    /// one with no circling approach ends (b) for an earlier check's; it
    /// then counts under (a) only if it had one.
    #[test]
    fn a_later_rating_test_is_the_most_recent_check() {
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
        let assessments = [
            passed(
                "ipc",
                date!(2020 - 01 - 01),
                "circled",
                Basis::ProficiencyCheck,
            ),
            passed("test", date!(2026 - 01 - 01), "straight", Basis::RatingTest),
        ];
        let r = checked(circling_approach, REFERENCE, &entries, &assessments);
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
                Basis::ProficiencyCheck,
            ),
            passed(
                "session",
                date!(2026 - 01 - 01),
                "s",
                Basis::ProficiencyCheck,
            ),
        ];
        let r = checked(circling_approach, REFERENCE, &entries, &assessments);
        assert_eq!(r.expiry, None);
    }

    /// Under (a), once a later check with no circling approach is the most
    /// recent.
    #[test]
    fn a_rating_test_with_a_circling_approach_counts_for_12_months_to_the_day() {
        let on = date!(2025 - 09 - 02);
        let entries = [
            flight("e", on, vec![circled(ApproachType::Vor)]),
            flight("straight", date!(2026 - 01 - 01), Vec::new()),
        ];
        let assessments = |test| {
            [
                passed("t", on, "e", test),
                passed(
                    "ipc",
                    date!(2026 - 01 - 01),
                    "straight",
                    Basis::ProficiencyCheck,
                ),
            ]
        };
        let r = checked(
            circling_approach,
            REFERENCE,
            &entries,
            &assessments(Basis::RatingTest),
        );
        assert_eq!(r.expiry, Some(Expiry::On(date!(2026 - 09 - 02))));
        assert!(r.is_current());
        let r = checked(
            circling_approach,
            date!(2026 - 09 - 02),
            &entries,
            &assessments(Basis::RatingTest),
        );
        assert!(!r.is_current());

        // An endorsement test is not the flight test for the rating.
        let r = checked(
            circling_approach,
            REFERENCE,
            &entries,
            &assessments(Basis::EndorsementTest),
        );
        assert_eq!(r.expiry, None);
    }

    /// An entry for the flight dated after the reference has not happened,
    /// so its co-pilot capacity does not change the flight.
    #[test]
    fn an_entry_after_the_reference_does_not_change_the_flight() {
        let on = date!(2026 - 01 - 05);
        let later = entry(on_flight(
            flight_of("e"),
            EntryParts {
                capacity: Capacity::CoPilot,
                ..aeroplane("later", date!(2026 - 09 - 02))
            },
        ));
        let entries = [flight("e", on, Vec::new()), later];
        let r = checked(
            single_pilot_operation,
            REFERENCE,
            &entries,
            &[passed("ipc", on, "e", Basis::ProficiencyCheck)],
        );
        assert_eq!(r.expiry, Some(Expiry::Never));
    }

    /// A circling approach on another entry for the check's flight is the
    /// flight's.
    #[test]
    fn the_approaches_are_the_flights() {
        let on = date!(2026 - 01 - 05);
        let other = entry(on_flight(
            flight_of("e"),
            EntryParts {
                approaches: vec![circled(ApproachType::Vor)],
                ..aeroplane("other", on)
            },
        ));
        let entries = [flight("e", on, Vec::new()), other];
        let r = checked(
            circling_approach,
            REFERENCE,
            &entries,
            &[passed("ipc", on, "e", Basis::ProficiencyCheck)],
        );
        assert_eq!(r.expiry, Some(Expiry::Never));
    }
}
