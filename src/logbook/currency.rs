// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's currency checks, one module for each regulation, written over the
//! kernel's [`EventWindow`](logbook_shared_kernel::EventWindow), and the
//! requirements they answer. Each sees only the entries
//! [`Casa::counts`](crate::Casa) lets through, so none repeats that gate. A
//! check over assessments ([`AssessmentCurrencyCheck`]) that reads their
//! aircraft is given the entries it reads it from.

mod flight_review;
mod guidance_operations;
mod instrument_approaches;
mod instrument_approaches_2d;
mod instrument_approaches_3d;
mod instrument_proficiency_check;
mod night_vfr;
mod passenger_day;
mod passenger_night;
mod single_pilot_ifr;

use logbook_shared_kernel::{
    run, run_assessments, AssessmentAccumulator, AssessmentCurrencyCheck, CurrencyCheck,
    CurrencyResult, EntryAccumulator, LicensingAuthority, Logbook, Requirement, RequirementId,
};
use time::Date;

use crate::aircraft::AircraftClassRating;
use crate::logbook::Casa;

use flight_review::FlightReview;
use guidance_operations::GuidanceOperations;
use instrument_approaches::InstrumentApproaches;
use instrument_approaches_2d::InstrumentApproaches2D;
use instrument_approaches_3d::InstrumentApproaches3D;
use instrument_proficiency_check::InstrumentProficiencyCheck;
use night_vfr::NightVfr;
use passenger_day::PassengerDay;
use passenger_night::PassengerNight;
use single_pilot_ifr::SinglePilotIfr;

/// Each requirement CASA's rules check: its id, its name in the
/// regulation's words and the regulation that sets it, in the order they
/// are reported. Aeroplane recency comes first, then instrument recency,
/// then the flight reviews and the instrument proficiency check, as
/// pilot-logbook listed them; the two guidance operations follow reg
/// 61.870(5).
///
/// The ids are pilot-logbook's keys for the same requirements (its
/// `wording.ts`): the regulation, and for a flight review its class rating
/// too. They are stable: a host may keep them.
const REQUIREMENTS: [(&str, &str, &str); 12] = [
    (
        "61.395(1)",
        "Carrying a passenger by day in an aeroplane",
        "reg 61.395(1)",
    ),
    (
        "61.395(2)",
        "Carrying a passenger at night in an aeroplane",
        "reg 61.395(2)",
    ),
    (
        "61.965",
        "Recent experience for the night VFR rating in an aeroplane",
        "reg 61.965",
    ),
    (
        "61.870(2)",
        "3 instrument approach operations",
        "reg 61.870(2)",
    ),
    (
        "61.870(4)",
        "2D instrument approach operation",
        "reg 61.870(4)",
    ),
    (
        "61.870(5)",
        "3D instrument approach operation",
        "reg 61.870(5)",
    ),
    ("61.870(6)", "Azimuth guidance operation", "reg 61.870(6)"),
    (
        "61.870(7)",
        "Course deviation indicator operation",
        "reg 61.870(7)",
    ),
    (
        "61.875",
        "Flight or simulated flight under the IFR in a single-pilot operation",
        "reg 61.875",
    ),
    (
        "61.745|single-engine-aeroplane",
        "Flight review for the single-engine aeroplane class rating",
        "reg 61.745",
    ),
    (
        "61.745|multi-engine-aeroplane",
        "Flight review for the multi-engine aeroplane class rating",
        "reg 61.745",
    ),
    ("61.880", "Instrument proficiency check", "reg 61.880"),
];

/// The requirements, in [`REQUIREMENTS`]' order.
pub(super) fn requirements() -> Vec<Requirement> {
    REQUIREMENTS
        .iter()
        .map(|&(id, name, regulation)| Requirement {
            id: RequirementId::new(id),
            name: name.to_owned(),
            regulation,
        })
        .collect()
}

/// Each requirement's result on `as_of`, in [`REQUIREMENTS`]' order.
///
/// The checks over entries are run together, in one pass of the kernel's
/// `run`, which applies [`Casa::counts`]. The flight reviews read an
/// assessment's aircraft from the entry for its flight, which `run` does
/// not reach, so they are given the entries `counts` lets through. The
/// instrument proficiency check reads no aircraft, and is given none.
pub(super) fn currency(
    casa: &Casa,
    as_of: Date,
    logbook: &Logbook,
) -> Vec<(RequirementId, CurrencyResult)> {
    let mut passenger_day = PassengerDay::new(as_of);
    let mut passenger_night = PassengerNight::new(as_of);
    let mut night_vfr = NightVfr::new(as_of);
    let mut approaches = InstrumentApproaches::new(as_of);
    let mut approaches_2d = InstrumentApproaches2D::new(as_of);
    let mut approaches_3d = InstrumentApproaches3D::new(as_of);
    let mut azimuth_guidance = GuidanceOperations::azimuth_guidance(as_of);
    let mut course_deviation_indicator = GuidanceOperations::course_deviation_indicator(as_of);
    let mut single_pilot_ifr = SinglePilotIfr::new(as_of);
    run(
        casa,
        &logbook.entries,
        &mut [
            &mut passenger_day as &mut dyn EntryAccumulator,
            &mut passenger_night,
            &mut night_vfr,
            &mut approaches,
            &mut approaches_2d,
            &mut approaches_3d,
            &mut azimuth_guidance,
            &mut course_deviation_indicator,
            &mut single_pilot_ifr,
        ],
    );

    let counted = || logbook.entries.iter().filter(|entry| casa.counts(entry));
    let mut single_engine_review =
        FlightReview::new(as_of, AircraftClassRating::SingleEngineAeroplane, counted());
    let mut multi_engine_review =
        FlightReview::new(as_of, AircraftClassRating::MultiEngineAeroplane, counted());
    let mut proficiency_check = InstrumentProficiencyCheck::new(as_of);
    run_assessments(
        &logbook.assessments,
        &mut [
            &mut single_engine_review as &mut dyn AssessmentAccumulator,
            &mut multi_engine_review,
            &mut proficiency_check,
        ],
    );

    // As long as `REQUIREMENTS`, so that a requirement added there is a
    // compile error here until its check is.
    let results: [CurrencyResult; REQUIREMENTS.len()] = [
        passenger_day.result(),
        passenger_night.result(),
        night_vfr.result(),
        approaches.result(),
        approaches_2d.result(),
        approaches_3d.result(),
        azimuth_guidance.result(),
        course_deviation_indicator.result(),
        single_pilot_ifr.result(),
        single_engine_review.result(),
        multi_engine_review.result(),
        proficiency_check.result(),
    ];
    REQUIREMENTS
        .iter()
        .zip(results)
        .map(|(&(id, _, _), result)| (RequirementId::new(id), result))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use logbook_shared_kernel::{ApproachType, Assessment, EntryParts};
    use time::macros::date;

    use super::*;
    use crate::logbook::test_support::{
        aeroplane, approach, assessed, day, entry, informed, night, on_flight, simulated,
    };
    use crate::logbook::{AssessmentInformation, CheckOutcome, CheckType, EntryInformation};

    const REFERENCE: Date = date!(2024 - 09 - 01);
    const ON: Date = date!(2024 - 08 - 20);

    /// A passed check of this type on flight 1.
    fn passed(id: &str, check_type: CheckType) -> Assessment {
        assessed(
            id,
            ON,
            1,
            AssessmentInformation {
                check_type: Some(check_type),
                outcome: Some(CheckOutcome::Passed),
                check_purpose: None,
            },
        )
    }

    #[test]
    fn every_requirement_is_answered_once_in_the_order_listed() {
        let ids: Vec<RequirementId> = requirements().into_iter().map(|r| r.id).collect();
        let answered: Vec<RequirementId> = currency(&Casa::new(), REFERENCE, &Logbook::default())
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(answered, ids);
        assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len());
    }

    #[test]
    fn each_requirement_is_answered_by_its_own_check_over_the_entries_that_count() {
        // Flight 1, a single-engine aeroplane: 3 take-offs and landings by
        // day, and an ILS flown as an azimuth guidance operation.
        let flight = informed(
            EntryInformation {
                azimuth_guidance_operation: Some(true),
                ..EntryInformation::default()
            },
            EntryParts {
                take_offs: day(3),
                landings: day(3),
                approaches: vec![approach(ApproachType::Ils)],
                ..on_flight(1, aeroplane("e", ON))
            },
        );
        // A session that does not count, in no approved device, with what
        // would make the night checks current.
        let session = EntryParts {
            take_offs: night(3),
            landings: night(3),
            ..simulated("s", ON, "C172")
        };
        let logbook = Logbook {
            entries: vec![entry(flight), entry(session)],
            assessments: vec![
                passed("review", CheckType::FlightReview),
                passed("ipc", CheckType::InstrumentProficiencyCheck),
            ],
        };
        let current: Vec<String> = currency(&Casa::new(), REFERENCE, &logbook)
            .into_iter()
            .filter(|(_, result)| result.is_current())
            .map(|(id, _)| id.as_str().to_owned())
            .collect();
        assert_eq!(
            current,
            [
                "61.395(1)",
                "61.870(5)",
                "61.870(6)",
                "61.745|single-engine-aeroplane",
                "61.880",
            ]
        );
    }
}
