//! CASA Part 61 aircraft classification, derived from an ICAO Doc 8643 type
//! designator.
//!
//! Two layers:
//!
//! - **Layer A** — `designator -> `[`AircraftDescription`]: country-agnostic
//!   facts about the type, compiled independently of Doc 8643 (see
//!   [`designators`]).
//! - **Layer B** — [`AircraftDescription`]` -> `[`CasaAircraftClassification`]:
//!   the Part 61 reading of those facts (regs 61.015, 61.020, 61.755).
//!
//! Resolution is a pure function, not a stored aggregate. Nothing here has an
//! identity, an `aircraft_id`, or a lifecycle; per-airframe overrides are
//! stored by whichever consumer owns them and passed back in.

pub mod category;
pub mod class_rating;
pub mod description;
pub mod design_feature;
pub mod designators;

pub use category::AircraftCategory;
pub use class_rating::AircraftClassRating;
pub use description::{AircraftDescription, AirframeKind, EngineType};
pub use design_feature::DesignFeature;
pub use designators::Confidence;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Everything that can go wrong resolving a classification.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClassificationError {
    /// The designator is not in the compiled table and the override did not
    /// supply a category to fall back on.
    #[error(
        "aircraft type '{0}' is not in the compiled designator table; supply \
         a category override to classify it"
    )]
    UnknownDesignator(String),

    /// An override named a design feature that does not require an
    /// endorsement for the resolved category (reg 61.755).
    #[error("design feature {feature:?} does not apply to category {category:?} (reg 61.755)")]
    FeatureNotValidForCategory {
        feature: DesignFeature,
        category: AircraftCategory,
    },
}

/// The Part 61 classification of an aircraft type.
///
/// A value object: no identity, no lifecycle, nothing persisted. Consumers
/// store their own per-airframe overrides keyed by their own aircraft id and
/// call [`resolve_classification`] to get the effective classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CasaAircraftClassification {
    /// Reg 61.015 (or reg 61.007(2) for a registered sailplane).
    pub category: AircraftCategory,
    /// Reg 61.020. `None` where the regulation defines no class — a
    /// registered sailplane, or a multi-engine helicopter or gyroplane
    /// (type-rated rather than class-rated).
    pub class: Option<AircraftClassRating>,
    /// Reg 61.755 features derivable from the description alone. Always a
    /// *default* pending a consumer's override: undercarriage configuration,
    /// float fitment and propeller controls vary between airframes of the
    /// same type.
    pub design_features: Vec<DesignFeature>,
    /// How far to trust this classification. Carried once for the whole
    /// value, not per field: the uncertainty originates entirely in the Layer
    /// A lookup, and Layer B is deterministic given a correct description.
    pub confidence: Confidence,
    /// Where the Layer A facts came from. `None` when the classification came
    /// wholly from an override.
    pub source: Option<&'static str>,
}

/// A consumer-supplied correction to the derived classification.
///
/// Needed routinely rather than exceptionally — `design_features` especially,
/// since two airframes of the same type designator can differ on tailwheel
/// versus tricycle undercarriage, floats versus wheels, or retractable versus
/// fixed gear. Storage of an override belongs to the consuming application,
/// keyed by its own aircraft id; this package never sees one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassificationOverride {
    pub category: Option<AircraftCategory>,
    pub class: Option<AircraftClassRating>,
    pub design_features: Option<Vec<DesignFeature>>,
}

/// Layer B: read a description under Part 61.
///
/// Deterministic given a correct description, and public so each category's
/// derivation can be exercised without inventing a designator for it.
pub fn classify(
    description: AircraftDescription,
) -> (AircraftCategory, Option<AircraftClassRating>) {
    let category = match description.airframe {
        AirframeKind::LandPlane | AirframeKind::SeaPlane | AirframeKind::Amphibian => {
            AircraftCategory::Aeroplane
        }
        AirframeKind::Helicopter => AircraftCategory::Helicopter,
        AirframeKind::Gyrocopter => AircraftCategory::Gyroplane,
        AirframeKind::TiltRotor => AircraftCategory::PoweredLift,
        AirframeKind::Airship => AircraftCategory::Airship,
        AirframeKind::Glider => AircraftCategory::RegisteredSailplane,
    };

    let single = description.engine_count <= 1;
    let class = match category {
        AircraftCategory::Aeroplane if single => Some(AircraftClassRating::SingleEngineAeroplane),
        AircraftCategory::Aeroplane => Some(AircraftClassRating::MultiEngineAeroplane),
        // Reg 61.020 lists only *single-engine* helicopter and gyroplane
        // classes; multi-engine ones are type-rated, so they have no class.
        AircraftCategory::Helicopter if single => Some(AircraftClassRating::SingleEngineHelicopter),
        AircraftCategory::Gyroplane if single => Some(AircraftClassRating::SingleEngineGyroplane),
        AircraftCategory::PoweredLift => Some(AircraftClassRating::PoweredLiftAircraft),
        AircraftCategory::Airship => Some(AircraftClassRating::Airship),
        _ => None,
    };

    (category, class)
}

/// Design features derivable from the description alone.
///
/// Only two are: a gas turbine powerplant, and float fitment implied by a
/// seaplane or amphibian airframe. Everything else in the reg 61.755 table
/// varies per airframe and must come from an override.
fn derived_features(
    description: AircraftDescription,
    category: AircraftCategory,
) -> Vec<DesignFeature> {
    let mut features = Vec::new();

    if description.engine_type.is_gas_turbine() && category.allows(DesignFeature::GasTurbineEngine)
    {
        features.push(DesignFeature::GasTurbineEngine);
    }

    if matches!(
        description.airframe,
        AirframeKind::SeaPlane | AirframeKind::Amphibian
    ) {
        // Floatplane and floating hull are indistinguishable from the
        // description alone; the more common fitment is the default and an
        // override corrects it.
        match category {
            AircraftCategory::Aeroplane => features.push(DesignFeature::Floatplane),
            AircraftCategory::Helicopter => features.push(DesignFeature::FloatAlightingGear),
            _ => {}
        }
    }

    features
}

/// Resolve the effective Part 61 classification of an aircraft type.
///
/// The only public way to build a [`CasaAircraftClassification`], and so the
/// one place (category, design feature) validity is enforced — an override
/// naming a feature that does not apply to the resolved category is rejected
/// here rather than being made unrepresentable in the type structure.
///
/// An uncatalogued designator is not guessed at: it is an error unless the
/// override supplies a category to classify by instead.
pub fn resolve_classification(
    designator: &str,
    classification_override: Option<&ClassificationOverride>,
) -> Result<CasaAircraftClassification, ClassificationError> {
    let facts = designators::lookup(designator);

    let (mut category, mut class, mut design_features, confidence, source) = match facts {
        Some(facts) => {
            let (category, class) = classify(facts.description);
            let features = derived_features(facts.description, category);
            (
                category,
                class,
                features,
                facts.confidence,
                Some(facts.source),
            )
        }
        None => {
            let category = classification_override
                .and_then(|o| o.category)
                .ok_or_else(|| ClassificationError::UnknownDesignator(designator.to_string()))?;
            (category, None, Vec::new(), Confidence::Overridden, None)
        }
    };

    if let Some(over) = classification_override {
        if let Some(overridden) = over.category {
            category = overridden;
            // A category override invalidates a class derived under the old
            // category; the override must restate it.
            class = None;
            design_features.retain(|f| category.allows(*f));
        }
        if let Some(overridden) = over.class {
            class = Some(overridden);
        }
        if let Some(features) = &over.design_features {
            for feature in features {
                if !category.allows(*feature) {
                    return Err(ClassificationError::FeatureNotValidForCategory {
                        feature: *feature,
                        category,
                    });
                }
            }
            design_features = features.clone();
        }
    }

    Ok(CasaAircraftClassification {
        category,
        class,
        design_features,
        confidence,
        source,
    })
}
