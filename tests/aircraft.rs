//! Part 61 aircraft classification: the reg 61.755 table, Layer B derivation
//! per category, and the resolver's override and unknown-designator paths.

use au_casa::{
    classify, resolve_classification, AircraftCategory, AircraftClassRating, AircraftDescription,
    AirframeKind, ClassificationError, ClassificationOverride, Confidence, DesignFeature,
    EngineType,
};
use rstest::rstest;

fn description(
    airframe: AirframeKind,
    engine_count: u8,
    engine_type: EngineType,
) -> AircraftDescription {
    AircraftDescription::new(airframe, engine_count, engine_type)
}

// --- reg 61.755 design feature table ------------------------------------

/// Each category's feature list, verbatim from the reg 61.755 table.
#[rstest]
#[case(AircraftCategory::Aeroplane, 9)]
#[case(AircraftCategory::Helicopter, 3)]
#[case(AircraftCategory::PoweredLift, 3)]
#[case(AircraftCategory::Gyroplane, 3)]
#[case(AircraftCategory::Airship, 2)]
#[case(AircraftCategory::RegisteredSailplane, 0)]
fn design_feature_table_sizes(#[case] category: AircraftCategory, #[case] expected: usize) {
    assert_eq!(category.design_features().len(), expected);
}

#[test]
fn aeroplane_features_match_the_regulation() {
    let features = AircraftCategory::Aeroplane.design_features();
    for expected in [
        DesignFeature::TailwheelUndercarriage,
        DesignFeature::RetractableUndercarriage,
        DesignFeature::ManualPropellerPitchControl,
        DesignFeature::GasTurbineEngine,
        DesignFeature::MultiEngineCentreLineThrust,
        DesignFeature::PressurisationSystem,
        DesignFeature::Floatplane,
        DesignFeature::FloatingHull,
        DesignFeature::SkiLandingGear,
    ] {
        assert!(features.contains(&expected), "missing {expected:?}");
    }
}

/// A helicopter must not be assignable an aeroplane-only feature — the flaw
/// in the old flat, unscoped enum.
#[test]
fn helicopter_does_not_allow_aeroplane_only_features() {
    assert!(!AircraftCategory::Helicopter.allows(DesignFeature::Floatplane));
    assert!(!AircraftCategory::Helicopter.allows(DesignFeature::TailwheelUndercarriage));
    assert!(AircraftCategory::Helicopter.allows(DesignFeature::FloatAlightingGear));
}

/// `design_features` and `applicable_categories` must agree in both
/// directions for all 10 features and all 6 categories.
#[test]
fn feature_and_category_tables_are_mutual_inverses() {
    use AircraftCategory::*;
    use DesignFeature::*;
    let categories = [
        Aeroplane,
        Helicopter,
        PoweredLift,
        Gyroplane,
        Airship,
        RegisteredSailplane,
    ];
    let features = [
        TailwheelUndercarriage,
        RetractableUndercarriage,
        ManualPropellerPitchControl,
        GasTurbineEngine,
        MultiEngineCentreLineThrust,
        PressurisationSystem,
        Floatplane,
        FloatingHull,
        SkiLandingGear,
        FloatAlightingGear,
    ];
    for category in categories {
        for feature in features {
            assert_eq!(
                category.allows(feature),
                feature.applicable_categories().contains(&category),
                "{category:?} vs {feature:?} disagree",
            );
        }
    }
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
    assert_eq!(classify(description), (category, class));
}

/// Reg 61.020 lists no multi-engine helicopter class — those are type-rated.
#[test]
fn multi_engine_helicopter_has_no_class() {
    let (category, class) = classify(description(
        AirframeKind::Helicopter,
        2,
        EngineType::TurboProp,
    ));
    assert_eq!(category, AircraftCategory::Helicopter);
    assert_eq!(class, None);
}

// --- The resolver -------------------------------------------------------

#[test]
fn catalogued_designator_resolves() {
    let result = resolve_classification("C172", None).unwrap();
    assert_eq!(result.category, AircraftCategory::Aeroplane);
    assert_eq!(
        result.class,
        Some(AircraftClassRating::SingleEngineAeroplane)
    );
    assert!(result.design_features.is_empty());
    assert_eq!(result.confidence, Confidence::Confirmed);
    assert!(result.source.is_some());
}

#[test]
fn twin_resolves_to_multi_engine_aeroplane() {
    let result = resolve_classification("PA44", None).unwrap();
    assert_eq!(
        result.class,
        Some(AircraftClassRating::MultiEngineAeroplane)
    );
}

#[test]
fn uncatalogued_designator_without_override_is_an_error() {
    let result = resolve_classification("ZZZZ", None);
    assert!(matches!(
        result,
        Err(ClassificationError::UnknownDesignator(d)) if d == "ZZZZ"
    ));
}

#[test]
fn uncatalogued_designator_falls_back_to_the_override() {
    let over = ClassificationOverride {
        category: Some(AircraftCategory::Aeroplane),
        class: Some(AircraftClassRating::SingleEngineAeroplane),
        ..Default::default()
    };
    let result = resolve_classification("ZZZZ", Some(&over)).unwrap();
    assert_eq!(result.category, AircraftCategory::Aeroplane);
    assert_eq!(result.confidence, Confidence::Overridden);
    assert_eq!(result.source, None);
}

/// The routine case: per-airframe undercarriage differs from the type default.
#[test]
fn design_feature_override_is_accepted() {
    let over = ClassificationOverride {
        design_features: Some(vec![DesignFeature::TailwheelUndercarriage]),
        ..Default::default()
    };
    let result = resolve_classification("PA25", Some(&over)).unwrap();
    assert_eq!(
        result.design_features,
        vec![DesignFeature::TailwheelUndercarriage]
    );
}

#[test]
fn design_feature_override_invalid_for_category_is_rejected() {
    let over = ClassificationOverride {
        design_features: Some(vec![DesignFeature::Floatplane]),
        ..Default::default()
    };
    // R44 resolves to Helicopter, for which floatplane is not a reg 61.755
    // feature — float alighting gear is.
    let result = resolve_classification("R44", Some(&over));
    assert!(matches!(
        result,
        Err(ClassificationError::FeatureNotValidForCategory {
            feature: DesignFeature::Floatplane,
            category: AircraftCategory::Helicopter,
        })
    ));
}

/// One of only two features derivable from the description alone — the rest
/// of the reg 61.755 table varies per airframe and must be overridden.
#[test]
fn gas_turbine_feature_is_derived_from_the_engine_type() {
    let turboprop = resolve_classification("C208", None).unwrap();
    assert_eq!(
        turboprop.design_features,
        vec![DesignFeature::GasTurbineEngine]
    );

    let piston = resolve_classification("C172", None).unwrap();
    assert!(piston.design_features.is_empty());
}

/// A category override must not leave behind a class or features derived
/// under the old category.
#[test]
fn category_override_clears_the_derived_class() {
    let over = ClassificationOverride {
        category: Some(AircraftCategory::Helicopter),
        ..Default::default()
    };
    // C208 derives an aeroplane class and a gas turbine feature; gas turbine
    // survives (it applies to helicopters too), the aeroplane class does not.
    let result = resolve_classification("C208", Some(&over)).unwrap();
    assert_eq!(result.category, AircraftCategory::Helicopter);
    assert_eq!(result.class, None);
    assert_eq!(
        result.design_features,
        vec![DesignFeature::GasTurbineEngine]
    );
}

/// The Citabria is split across two Doc 8643 designators, and CH7A is also
/// shared with the Aeronca 7AC Champion. Neither ambiguity can change the
/// Part 61 answer, because every model under both is a single piston
/// landplane — assert that, so a future edit cannot quietly diverge them.
#[test]
fn both_citabria_designators_classify_identically() {
    let aurora = resolve_classification("CH7A", None).unwrap();
    let adventure = resolve_classification("CH7B", None).unwrap();

    for result in [&aurora, &adventure] {
        assert_eq!(result.category, AircraftCategory::Aeroplane);
        assert_eq!(
            result.class,
            Some(AircraftClassRating::SingleEngineAeroplane)
        );
        // Piston, so no gas turbine feature; undercarriage is per-airframe.
        assert!(result.design_features.is_empty());
    }
    assert_eq!(aurora.category, adventure.category);
    assert_eq!(aurora.class, adventure.class);
}

/// Rows checked against a type certificate data sheet say so, and cite the
/// sheet by number and revision. The flag is only worth carrying if it is
/// applied honestly, so the rows nobody has checked must still say so.
#[test]
fn confidence_reflects_whether_a_tcds_was_actually_read() {
    for (designator, sheet) in [
        ("CH7A", "A-759"),
        ("CH7B", "A-759"),
        ("C172", "3A12"),
        ("PA34", "A7SO"),
        ("PA44", "IM.A.232"),
        ("C208", "A37CE"),
    ] {
        let result = resolve_classification(designator, None).unwrap();
        assert_eq!(
            result.confidence,
            Confidence::Confirmed,
            "{designator} should be confirmed"
        );
        assert!(
            result.source.unwrap().contains(sheet),
            "{designator} should cite {sheet}"
        );
    }

    // PA25's sheet could not be obtained and R44's is behind a login, so
    // neither has been read by anyone. They stay Provisional.
    for designator in ["PA25", "R44"] {
        let result = resolve_classification(designator, None).unwrap();
        assert_eq!(
            result.confidence,
            Confidence::Provisional,
            "{designator} has not been checked against a TCDS"
        );
    }
}
