// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA Part 61 aircraft classification, from the description of an
//! aircraft's type and its airframe's characteristics.
//!
//! Two layers:
//!
//! - **Layer A** — `designator -> `[`AircraftDescription`]: country-agnostic
//!   facts about the type, from the `aircraft-type-descriptions` crate, which
//!   holds the table and cites each row's sources. [`AircraftDescription`],
//!   [`AirframeKind`], [`EngineType`](aircraft_type_descriptions::EngineType) and
//!   [`Confidence`](aircraft_type_descriptions::Confidence) are that crate's
//!   types, the vocabulary shared with sibling country crates. They are named
//!   from that crate, not from here; a consumer takes it by a caret range
//!   compatible with au-casa's own dependency on it (the same 0.x minor; see
//!   `Cargo.toml`), so that Cargo builds one copy.
//! - **Layer B** — the Part 61 reading of those facts and of the
//!   characteristics recorded for the airframe (logbook-shared-kernel's
//!   [`AircraftCharacteristic`]): the category and class ([`classify`], regs
//!   61.015 and 61.020) and the design features that require an endorsement
//!   (reg 61.755). A host has them through `Casa`'s `classify`.
//!
//! Resolution is a pure function, not a stored aggregate. Nothing here has an
//! identity, an `aircraft_id`, or a lifecycle: the characteristics are
//! recorded by whichever consumer owns the aircraft and passed in.
//!
//! The design features are not exported: a host has them through `Casa`.
//!
//! ```compile_fail
//! use au_casa::aircraft::DesignFeature;
//! ```

pub mod category;
pub mod class_rating;
mod design_feature;

pub use category::AircraftCategory;
pub use class_rating::AircraftClassRating;
pub(crate) use design_feature::DesignFeature;

use std::collections::BTreeSet;

use aircraft_type_descriptions::{AircraftDescription, AirframeKind};
use logbook_shared_kernel::AircraftCharacteristic;

/// Layer B: the category (reg 61.015, or reg 61.007(2) for a registered
/// sailplane) and the class (reg 61.020) of an aircraft of this description
/// with these characteristics.
///
/// An aeroplane with two or more engines is in the multi-engine class unless
/// [`AircraftCharacteristic::MultiEngineCentreLineThrust`] is recorded for
/// it: reg 61.020(2)(a) puts a multi-engine centre-line thrust aeroplane in
/// the single-engine class. Its other characteristics do not change the
/// class.
pub fn classify(
    description: AircraftDescription,
    characteristics: &BTreeSet<AircraftCharacteristic>,
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
    let centre_line_thrust =
        characteristics.contains(&AircraftCharacteristic::MultiEngineCentreLineThrust);
    let class = match category {
        AircraftCategory::Aeroplane if single || centre_line_thrust => {
            Some(AircraftClassRating::SingleEngineAeroplane)
        }
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

/// The design features on an aircraft of this description and these
/// characteristics that require a design feature endorsement (reg 61.755),
/// in the regulation's order for its category. Gas turbine engine is read
/// from the description's engine type; every other is recorded, and a
/// characteristic the category's list does not have requires nothing.
pub(crate) fn design_features(
    description: AircraftDescription,
    characteristics: &BTreeSet<AircraftCharacteristic>,
) -> Vec<DesignFeature> {
    let (category, _) = classify(description, characteristics);
    category
        .design_features()
        .iter()
        .copied()
        .filter(|feature| feature.is_on(description, characteristics))
        .collect()
}

/// Whether the aircraft operates on water (a seaplane or an amphibian) with
/// neither float alighting gear nor a floating hull recorded. Its
/// description cannot say which it has, so nothing is required for either
/// until one is recorded.
pub(crate) fn lacks_water_alighting_gear(
    description: AircraftDescription,
    characteristics: &BTreeSet<AircraftCharacteristic>,
) -> bool {
    matches!(
        description.airframe,
        AirframeKind::SeaPlane | AirframeKind::Amphibian
    ) && !characteristics.contains(&AircraftCharacteristic::FloatAlightingGear)
        && !characteristics.contains(&AircraftCharacteristic::FloatingHull)
}
