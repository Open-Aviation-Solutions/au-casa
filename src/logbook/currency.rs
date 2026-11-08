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

use crate::aircraft::{AircraftCategory, AircraftClassRating};
use crate::logbook::{class_name, Casa};

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

/// A requirement CASA's rules check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CasaRequirement {
    PassengerDay,
    PassengerNight,
    NightVfr,
    InstrumentApproaches,
    /// Reg 61.870(3)'s instrument approach operation in this category.
    InstrumentApproachesInCategory(AircraftCategory),
    InstrumentApproaches2D,
    InstrumentApproaches3D,
    AzimuthGuidance,
    CourseDeviationIndicator,
    SinglePilotIfr,
    /// A flight review for this aircraft class rating.
    FlightReview(AircraftClassRating),
    InstrumentProficiencyCheck,
}

/// Reg 61.015's categories of aircraft, in its order, each with a
/// requirement of reg 61.870(3). A registered sailplane is not one.
const CATEGORIES: [AircraftCategory; 5] = [
    AircraftCategory::Aeroplane,
    AircraftCategory::Helicopter,
    AircraftCategory::PoweredLift,
    AircraftCategory::Gyroplane,
    AircraftCategory::Airship,
];

/// The requirements, in the order they are reported. Aeroplane recency
/// comes first, then instrument recency, then the flight reviews and the
/// instrument proficiency check, as pilot-logbook listed them; instrument
/// recency is in reg 61.870's order, with a requirement of its subregulation
/// (3) for each category in reg 61.015's.
const REQUIREMENTS: [CasaRequirement; 17] = [
    CasaRequirement::PassengerDay,
    CasaRequirement::PassengerNight,
    CasaRequirement::NightVfr,
    CasaRequirement::InstrumentApproaches,
    CasaRequirement::InstrumentApproachesInCategory(CATEGORIES[0]),
    CasaRequirement::InstrumentApproachesInCategory(CATEGORIES[1]),
    CasaRequirement::InstrumentApproachesInCategory(CATEGORIES[2]),
    CasaRequirement::InstrumentApproachesInCategory(CATEGORIES[3]),
    CasaRequirement::InstrumentApproachesInCategory(CATEGORIES[4]),
    CasaRequirement::InstrumentApproaches2D,
    CasaRequirement::InstrumentApproaches3D,
    CasaRequirement::AzimuthGuidance,
    CasaRequirement::CourseDeviationIndicator,
    CasaRequirement::SinglePilotIfr,
    CasaRequirement::FlightReview(AircraftClassRating::SingleEngineAeroplane),
    CasaRequirement::FlightReview(AircraftClassRating::MultiEngineAeroplane),
    CasaRequirement::InstrumentProficiencyCheck,
];

impl CasaRequirement {
    /// The regulation that sets it, bare (`61.395(1)`).
    fn regulation(self) -> &'static str {
        match self {
            Self::PassengerDay => "61.395(1)",
            Self::PassengerNight => "61.395(2)",
            Self::NightVfr => "61.965",
            Self::InstrumentApproaches => "61.870(2)",
            Self::InstrumentApproachesInCategory(_) => "61.870(3)",
            Self::InstrumentApproaches2D => "61.870(4)",
            Self::InstrumentApproaches3D => "61.870(5)",
            Self::AzimuthGuidance => "61.870(6)",
            Self::CourseDeviationIndicator => "61.870(7)",
            Self::SinglePilotIfr => "61.875",
            Self::FlightReview(_) => "61.745",
            Self::InstrumentProficiencyCheck => "61.880",
        }
    }

    /// Its id: pilot-logbook's key for it (its `wording.ts`), the
    /// regulation and, for a flight review, `|` and the class rating's
    /// stored name (`61.745|single-engine-aeroplane`); for reg 61.870(3),
    /// `|` and the category's stored name (`61.870(3)|aeroplane`). Stable:
    /// a host may keep it.
    fn id(self) -> RequirementId {
        let qualified = |name: serde_json::Value| {
            let name = name.as_str().expect("a stored name is a string");
            RequirementId::new(format!("{}|{name}", self.regulation()))
        };
        match self {
            Self::FlightReview(class_rating) => qualified(
                serde_json::to_value(class_rating).expect("a class rating serialises to its name"),
            ),
            Self::InstrumentApproachesInCategory(category) => qualified(
                serde_json::to_value(category).expect("a category serialises to its name"),
            ),
            _ => RequirementId::new(self.regulation()),
        }
    }

    /// Its name, in its regulation's words.
    fn name(self) -> String {
        match self {
            Self::PassengerDay => "Carrying a passenger by day in an aeroplane".to_owned(),
            Self::PassengerNight => "Carrying a passenger at night in an aeroplane".to_owned(),
            Self::NightVfr => {
                "Recent experience for the night VFR rating in an aeroplane".to_owned()
            }
            Self::InstrumentApproaches => "At least 3 instrument approach operations".to_owned(),
            Self::InstrumentApproachesInCategory(category) => {
                format!("At least one instrument approach operation for the {category} category")
            }
            Self::InstrumentApproaches2D => "2D instrument approach operation".to_owned(),
            Self::InstrumentApproaches3D => "3D instrument approach operation".to_owned(),
            Self::AzimuthGuidance => "Azimuth guidance operation".to_owned(),
            Self::CourseDeviationIndicator => "Course deviation indicator operation".to_owned(),
            Self::SinglePilotIfr => {
                "Flight or simulated flight under the IFR in a single-pilot operation".to_owned()
            }
            Self::FlightReview(class_rating) => format!(
                "Flight review for the {} class rating",
                class_name(class_rating)
            ),
            Self::InstrumentProficiencyCheck => "Instrument proficiency check".to_owned(),
        }
    }
}

/// The requirements, in [`REQUIREMENTS`]' order.
pub(super) fn requirements() -> Vec<Requirement> {
    REQUIREMENTS
        .iter()
        .map(|&requirement| Requirement {
            id: requirement.id(),
            name: requirement.name(),
            regulation: requirement.regulation(),
        })
        .collect()
}

/// Each requirement's result on `as_of`, each paired with its requirement
/// where its check is run, in [`REQUIREMENTS`]' order (a test holds the two
/// to the same order).
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
    let mut in_category =
        CATEGORIES.map(|category| InstrumentApproaches::in_category(as_of, category));
    let mut approaches_2d = InstrumentApproaches2D::new(as_of);
    let mut approaches_3d = InstrumentApproaches3D::new(as_of);
    let mut azimuth_guidance = GuidanceOperations::azimuth_guidance(as_of);
    let mut course_deviation_indicator = GuidanceOperations::course_deviation_indicator(as_of);
    let mut single_pilot_ifr = SinglePilotIfr::new(as_of);
    let mut accumulators: Vec<&mut dyn EntryAccumulator> = vec![
        &mut passenger_day,
        &mut passenger_night,
        &mut night_vfr,
        &mut approaches,
        &mut approaches_2d,
        &mut approaches_3d,
        &mut azimuth_guidance,
        &mut course_deviation_indicator,
        &mut single_pilot_ifr,
    ];
    accumulators.extend(
        in_category
            .iter_mut()
            .map(|check| check as &mut dyn EntryAccumulator),
    );
    run(casa, &logbook.entries, &mut accumulators);

    let single_engine = AircraftClassRating::SingleEngineAeroplane;
    let multi_engine = AircraftClassRating::MultiEngineAeroplane;
    let counted = || logbook.entries.iter().filter(|entry| casa.counts(entry));
    let mut single_engine_review = FlightReview::new(as_of, single_engine, counted());
    let mut multi_engine_review = FlightReview::new(as_of, multi_engine, counted());
    let mut proficiency_check = InstrumentProficiencyCheck::new(as_of);
    run_assessments(
        &logbook.assessments,
        &mut [
            &mut single_engine_review as &mut dyn AssessmentAccumulator,
            &mut multi_engine_review,
            &mut proficiency_check,
        ],
    );

    let in_category = CATEGORIES
        .iter()
        .zip(&in_category)
        .map(|(&category, check)| {
            (
                CasaRequirement::InstrumentApproachesInCategory(category),
                check.result(),
            )
        });

    [
        (CasaRequirement::PassengerDay, passenger_day.result()),
        (CasaRequirement::PassengerNight, passenger_night.result()),
        (CasaRequirement::NightVfr, night_vfr.result()),
        (CasaRequirement::InstrumentApproaches, approaches.result()),
    ]
    .into_iter()
    .chain(in_category)
    .chain([
        (
            CasaRequirement::InstrumentApproaches2D,
            approaches_2d.result(),
        ),
        (
            CasaRequirement::InstrumentApproaches3D,
            approaches_3d.result(),
        ),
        (CasaRequirement::AzimuthGuidance, azimuth_guidance.result()),
        (
            CasaRequirement::CourseDeviationIndicator,
            course_deviation_indicator.result(),
        ),
        (CasaRequirement::SinglePilotIfr, single_pilot_ifr.result()),
        (
            CasaRequirement::FlightReview(single_engine),
            single_engine_review.result(),
        ),
        (
            CasaRequirement::FlightReview(multi_engine),
            multi_engine_review.result(),
        ),
        (
            CasaRequirement::InstrumentProficiencyCheck,
            proficiency_check.result(),
        ),
    ])
    .map(|(requirement, result)| (requirement.id(), result))
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

    /// `currency` pairs each result with its requirement where it runs the
    /// check, apart from `REQUIREMENTS`, so this holds the two lists to the
    /// same requirements in the same order.
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
                "61.870(3)|aeroplane",
                "61.870(5)",
                "61.870(6)",
                "61.745|single-engine-aeroplane",
                "61.880",
            ]
        );
    }
}
