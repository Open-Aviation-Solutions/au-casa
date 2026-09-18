//! Part 61 aircraft classification: the reg 61.755 table, Layer B derivation
//! per category, and the resolver's override and unknown-designator paths.

use au_casa::aircraft::designators::lookup;
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

// --- Task 0003: additional GA and glider designators ---------------------

/// Each newly seeded designator resolves to the expected category/class and
/// is still Provisional — none of the candidate sources have been read yet.
/// Note "PA28" is deliberately absent: it was retired from Doc 8643 in
/// favour of P28A/P28B/P28R/P28T/P28U around 1998, so it is not seeded here
/// at all — see `pa28_is_not_a_valid_designator` below.
#[rstest]
#[case(
    "C152",
    AircraftCategory::Aeroplane,
    Some(AircraftClassRating::SingleEngineAeroplane)
)]
#[case(
    "P28A",
    AircraftCategory::Aeroplane,
    Some(AircraftClassRating::SingleEngineAeroplane)
)]
#[case(
    "P28R",
    AircraftCategory::Aeroplane,
    Some(AircraftClassRating::SingleEngineAeroplane)
)]
#[case("AS21", AircraftCategory::RegisteredSailplane, None)]
#[case("DUOD", AircraftCategory::RegisteredSailplane, None)]
#[case("GLID", AircraftCategory::RegisteredSailplane, None)]
#[case("LS4", AircraftCategory::RegisteredSailplane, None)]
fn newly_seeded_designators_resolve(
    #[case] designator: &str,
    #[case] category: AircraftCategory,
    #[case] class: Option<AircraftClassRating>,
) {
    let result = resolve_classification(designator, None).unwrap();
    assert_eq!(result.category, category);
    assert_eq!(result.class, class);
    assert_eq!(
        result.confidence,
        Confidence::Provisional,
        "{designator} has not been checked against a TCDS"
    );
}

/// "PA28" was a real designator once, but Doc 8643 retired it around 1998 in
/// favour of a fixed/retractable/turbo split — it must stay unresolved
/// rather than being seeded as an alias for P28A, so a consumer storing the
/// stale string surfaces as "unknown, needs correcting" rather than silently
/// resolving to the wrong confidence trail.
#[test]
fn pa28_is_not_a_valid_designator() {
    let result = resolve_classification("PA28", None);
    assert!(matches!(
        result,
        Err(ClassificationError::UnknownDesignator(d)) if d == "PA28"
    ));
}

/// `P28A` and `P28R` share one type certificate (fixed vs retractable gear
/// is a Doc 8643 designator-level split, not something this table derives),
/// so they must classify identically, same guard as the Citabria pair above.
#[test]
fn p28a_and_p28r_classify_identically() {
    let fixed = resolve_classification("P28A", None).unwrap();
    let retractable = resolve_classification("P28R", None).unwrap();
    assert_eq!(fixed.category, retractable.category);
    assert_eq!(fixed.class, retractable.class);
}

/// A glider resolves to RegisteredSailplane with no class rating and no
/// design features (reg 61.755 defines none for it), regardless of which
/// seeded glider designator is used, whether or not that designator happens
/// to be a self-launching (engined) variant.
#[test]
fn gliders_have_no_class_and_no_design_features() {
    for designator in ["AS21", "DUOD", "GLID", "LS4"] {
        let result = resolve_classification(designator, None).unwrap();
        assert_eq!(result.category, AircraftCategory::RegisteredSailplane);
        assert_eq!(result.class, None);
        assert!(result.design_features.is_empty());
    }
}

/// AS21 and DUOD are specifically the self-launching motorglider variants
/// (ASK-21Mi, Duo Discus T) — the plain, unpowered types have no individual
/// Doc 8643 designator and fall under the generic GLID instead. The engine
/// facts must say so, even though it does not change the derived category
/// (Layer B's Glider -> RegisteredSailplane mapping ignores engine facts
/// entirely, so this can only be checked against Layer A's own table).
#[test]
fn self_launching_gliders_carry_an_engine() {
    for designator in ["AS21", "DUOD"] {
        let facts = lookup(designator).unwrap();
        assert_eq!(facts.description.engine_count, 1);
        assert_eq!(facts.description.engine_type, EngineType::Piston);
    }
}

/// GLID and LS4, by contrast, carry no engine at all.
#[test]
fn unpowered_gliders_carry_no_engine() {
    for designator in ["GLID", "LS4"] {
        let facts = lookup(designator).unwrap();
        assert_eq!(facts.description.engine_count, 0);
        assert_eq!(facts.description.engine_type, EngineType::None);
    }
}
