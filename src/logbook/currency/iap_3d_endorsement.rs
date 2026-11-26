// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reg 61.900(2): the limit on an IAP 3D instrument endorsement that turns
//! on what a flight test or proficiency check included.

use logbook_shared_kernel::CurrencyResult;

use super::assessed_flights::{AssessedFlights, Pass};

/// "Within the previous 15 months" (reg 61.900(2)(b)(ii)), in calendar
/// months.
const DEMONSTRATED_MONTHS: u32 = 15;

/// Reg 61.900(2): "the holder of an IAP 3D instrument endorsement is
/// authorised to conduct a 3D instrument approach operation only if: (a) the
/// holder passed the flight test for the endorsement within the previous 12
/// months; or (b) either: (i) in a case in which the holder's most recent
/// instrument proficiency check included a 3D instrument approach
/// operation—during the check the holder demonstrated, to a person
/// mentioned in subregulation (4), the holder's competence in the conduct
/// of a 3D instrument approach operation; or (ii) in any other case—the
/// holder has, within the previous 15 months, demonstrated, to a person
/// mentioned in subregulation (4), the holder's competence in the conduct
/// of a 3D instrument approach operation; or (c)" an approved operator's
/// training and checking system.
///
/// (b)(i) holds, with no end, while the most recent check
/// ([`AssessedFlights::most_recent_check`], an operator proficiency check
/// among them, reg 61.900(3)) recorded a 3D instrument approach operation
/// on its flight; otherwise (b)(ii) while a passed instrument or operator
/// proficiency check (reg 61.900(3) makes the latter one "for paragraph
/// (2)(b)"), or a flight test passed for the instrument rating or an
/// instrument endorsement, whose flight recorded one is within the previous
/// 15 months, to the day. Each is conducted by an examiner, one of
/// subregulation (4)'s persons; a flight review, conducted by an
/// instructor, is not. A demonstration not recorded as an assessment is not
/// read.
///
/// (a) is not checked: which endorsement a flight test was for is not
/// recorded (au-casa#123), and (b)(ii) already counts a passed flight test
/// whose flight recorded a 3D approach. (c) is not recorded.
pub(super) fn three_d_approach(assessed: &AssessedFlights) -> CurrencyResult {
    if let Some(check) = assessed.most_recent_check(|flown| flown.three_d) {
        return assessed.never(check);
    }
    let demonstrated = assessed.passes(|pass: &Pass| pass.flown.is_some_and(|flown| flown.three_d));
    assessed.within(DEMONSTRATED_MONTHS, &demonstrated)
}

#[cfg(test)]
mod tests {
    use logbook_shared_kernel::{ApproachType, Entry, EntryParts, Expiry};
    use time::macros::date;
    use time::Date;

    use super::super::assessed_flights::{checked, flight_of, passed, Basis};
    use super::*;
    use crate::logbook::test_support::{aeroplane, approach, entry, on_flight};

    const REFERENCE: Date = date!(2026 - 09 - 01);
    const ILS_ON: Date = date!(2025 - 06 - 02);
    const VOR_ON: Date = date!(2026 - 01 - 01);

    fn flight(id: &str, on: Date, procedure: ApproachType) -> Entry {
        entry(on_flight(
            flight_of(id),
            EntryParts {
                approaches: vec![approach(procedure)],
                ..aeroplane(id, on)
            },
        ))
    }

    /// An ILS on `ILS_ON`, a VOR on `VOR_ON`, and an ILS dated after the
    /// reference, which has not happened yet.
    fn flights() -> [Entry; 3] {
        [
            flight("ils", ILS_ON, ApproachType::Ils),
            flight("vor", VOR_ON, ApproachType::Vor),
            flight("later", date!(2026 - 09 - 02), ApproachType::Ils),
        ]
    }

    #[test]
    fn the_most_recent_check_with_a_3d_approach_holds_until_the_next() {
        let on = date!(2020 - 01 - 01);
        let entries = [flight("e", on, ApproachType::Ils)];
        let r = checked(
            three_d_approach,
            REFERENCE,
            &entries,
            &[passed("opc", on, "e", Basis::OperatorCheck)],
        );
        assert_eq!(r.expiry, Some(Expiry::Never));
    }

    /// An ILS on a test or check, any of them an operator proficiency check
    /// (reg 61.900(3)), and a later check with none.
    #[test]
    fn otherwise_a_test_or_check_with_one_counts_for_15_months_to_the_day() {
        for basis in [
            Basis::ProficiencyCheck,
            Basis::OperatorCheck,
            Basis::RatingTest,
            Basis::EndorsementTest,
        ] {
            let assessments = [
                passed("ils", ILS_ON, "ils", basis),
                passed("vor", VOR_ON, "vor", Basis::ProficiencyCheck),
            ];
            let r = checked(three_d_approach, REFERENCE, &flights(), &assessments);
            assert_eq!(
                r.expiry,
                Some(Expiry::On(date!(2026 - 09 - 02))),
                "{basis:?}"
            );
            assert!(r.is_current());
            let r = checked(
                three_d_approach,
                date!(2026 - 09 - 02),
                &flights(),
                &assessments,
            );
            assert!(!r.is_current(), "{basis:?}");
        }
    }

    /// A check dated after the reference is not the most recent one, and
    /// its flight's approach is not counted.
    #[test]
    fn a_check_after_the_reference_does_not_count() {
        let assessments = [
            passed("vor", VOR_ON, "vor", Basis::ProficiencyCheck),
            passed(
                "later",
                date!(2026 - 09 - 02),
                "later",
                Basis::ProficiencyCheck,
            ),
        ];
        let r = checked(three_d_approach, REFERENCE, &flights(), &assessments);
        assert_eq!(r.expiry, None);
    }
}
