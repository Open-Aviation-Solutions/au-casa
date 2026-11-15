// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Part 61 aircraft classification: the category and class derivation of
//! regs 61.015 and 61.020, from a description and the characteristics
//! recorded for the airframe. What an aircraft requires (reg 61.755) is
//! `Casa::classify`'s, and is tested with it.

use std::collections::BTreeSet;

use aircraft_type_descriptions::{AircraftDescription, AirframeKind, EngineType};
use au_casa::{classify, AircraftCategory, AircraftClassRating};
use logbook_shared_kernel::AircraftCharacteristic;
use rstest::rstest;

fn description(
    airframe: AirframeKind,
    engine_count: u8,
    engine_type: EngineType,
) -> AircraftDescription {
    AircraftDescription::new(airframe, engine_count, engine_type)
}

// --- Layer B: one derivation per category -------------------------------

#[rstest]
#[case(
    description(AirframeKind::LandPlane, 1, EngineType::Piston),
    AircraftCategory::Aeroplane,
    Some(AircraftClassRating::SingleEngineAeroplane)
)]
#[case(
    description(AirframeKind::LandPlane, 2, EngineType::Piston),
    AircraftCategory::Aeroplane,
    Some(AircraftClassRating::MultiEngineAeroplane)
)]
#[case(
    description(AirframeKind::Helicopter, 1, EngineType::TurboProp),
    AircraftCategory::Helicopter,
    Some(AircraftClassRating::SingleEngineHelicopter)
)]
#[case(
    description(AirframeKind::TiltRotor, 2, EngineType::TurboProp),
    AircraftCategory::PoweredLift,
    Some(AircraftClassRating::PoweredLiftAircraft)
)]
#[case(
    description(AirframeKind::Gyrocopter, 1, EngineType::Piston),
    AircraftCategory::Gyroplane,
    Some(AircraftClassRating::SingleEngineGyroplane)
)]
#[case(
    description(AirframeKind::Airship, 1, EngineType::Piston),
    AircraftCategory::Airship,
    Some(AircraftClassRating::Airship)
)]
#[case(
    description(AirframeKind::Glider, 0, EngineType::None),
    AircraftCategory::RegisteredSailplane,
    None
)]
fn category_and_class_derivation(
    #[case] description: AircraftDescription,
    #[case] category: AircraftCategory,
    #[case] class: Option<AircraftClassRating>,
) {
    assert_eq!(classify(description, &BTreeSet::new()), (category, class));
}

/// Reg 61.020 lists no multi-engine helicopter class — those are type-rated.
#[test]
fn multi_engine_helicopter_has_no_class() {
    let (category, class) = classify(
        description(AirframeKind::Helicopter, 2, EngineType::TurboProp),
        &BTreeSet::new(),
    );
    assert_eq!(category, AircraftCategory::Helicopter);
    assert_eq!(class, None);
}

/// Reg 61.020(2)(a): a multi-engine centre-line thrust aeroplane is in the
/// single-engine class, once the characteristic is recorded.
#[test]
fn a_centre_line_thrust_aeroplane_is_in_the_single_engine_class() {
    let twin = description(AirframeKind::LandPlane, 2, EngineType::Piston);
    let recorded = BTreeSet::from([AircraftCharacteristic::MultiEngineCentreLineThrust]);
    assert_eq!(
        classify(twin, &recorded),
        (
            AircraftCategory::Aeroplane,
            Some(AircraftClassRating::SingleEngineAeroplane)
        )
    );
    // Any other characteristic leaves it multi-engine.
    let other = BTreeSet::from([AircraftCharacteristic::RetractableUndercarriage]);
    assert_eq!(
        classify(twin, &other).1,
        Some(AircraftClassRating::MultiEngineAeroplane)
    );
}
