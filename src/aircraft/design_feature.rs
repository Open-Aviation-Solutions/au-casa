// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Design features requiring an endorsement under CASA Part 61.

use std::collections::BTreeSet;

use aircraft_type_descriptions::{AircraftDescription, EngineType};
use logbook_shared_kernel::AircraftCharacteristic;

/// A design feature for which reg 61.755 requires a design feature
/// endorsement.
///
/// **One flat enum, not per-category enums.** The reg 61.755 table is
/// many-to-many rather than a tree: retractable undercarriage is listed for
/// four categories, pressurisation system for four, and gas turbine engine
/// for all five. Which a category lists is
/// [`AircraftCategory::design_features`](super::AircraftCategory::design_features).
///
/// Crate-private: a host has them as `Casa::classify`'s `requires`, each by
/// its [`id`](Self::id).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesignFeature {
    TailwheelUndercarriage,
    RetractableUndercarriage,
    /// Reg 61.755 qualifies this as a piston-engine feature.
    ManualPropellerPitchControl,
    GasTurbineEngine,
    MultiEngineCentreLineThrust,
    PressurisationSystem,
    /// An aeroplane's float alighting gear (reg 61.755(2)(g)).
    Floatplane,
    FloatingHull,
    SkiLandingGear,
    /// A helicopter's (reg 61.755(3)(a)).
    FloatAlightingGear,
}

impl DesignFeature {
    /// Every design feature, each once, in reg 61.755's order: an
    /// aeroplane's (subregulation (2)), and then the one only a helicopter's
    /// list has (subregulation (3)(a)).
    pub(crate) const ALL: [Self; 10] = [
        Self::TailwheelUndercarriage,
        Self::RetractableUndercarriage,
        Self::ManualPropellerPitchControl,
        Self::GasTurbineEngine,
        Self::MultiEngineCentreLineThrust,
        Self::PressurisationSystem,
        Self::Floatplane,
        Self::FloatingHull,
        Self::SkiLandingGear,
        Self::FloatAlightingGear,
    ];

    /// Whether an aircraft of this description with these characteristics
    /// has the feature: gas turbine engine from the description's engine
    /// type, and every other from the characteristic recorded for it, as
    /// nothing is implied from the description. Manual propeller pitch
    /// control is reg 61.755(2)(c)'s only with a piston engine.
    pub(crate) fn is_on(
        self,
        description: AircraftDescription,
        characteristics: &BTreeSet<AircraftCharacteristic>,
    ) -> bool {
        use AircraftCharacteristic as C;
        let recorded = match self {
            Self::GasTurbineEngine => return description.engine_type.is_gas_turbine(),
            Self::TailwheelUndercarriage => C::TailwheelUndercarriage,
            Self::RetractableUndercarriage => C::RetractableUndercarriage,
            Self::ManualPropellerPitchControl if description.engine_type != EngineType::Piston => {
                return false
            }
            Self::ManualPropellerPitchControl => C::ManualPropellerPitchControl,
            Self::MultiEngineCentreLineThrust => C::MultiEngineCentreLineThrust,
            Self::PressurisationSystem => C::PressurisationSystem,
            Self::Floatplane | Self::FloatAlightingGear => C::FloatAlightingGear,
            Self::FloatingHull => C::FloatingHull,
            Self::SkiLandingGear => C::SkiLandingGear,
        };
        characteristics.contains(&recorded)
    }

    /// Its stable id, which a host may keep: pilot-logbook's stored
    /// spelling of the feature, so that what it stored reads as it is.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::TailwheelUndercarriage => "tailwheel-undercarriage",
            Self::RetractableUndercarriage => "retractable-undercarriage",
            Self::ManualPropellerPitchControl => "manual-propeller-pitch-control",
            Self::GasTurbineEngine => "gas-turbine-engine",
            Self::MultiEngineCentreLineThrust => "multi-engine-centre-line-thrust",
            Self::PressurisationSystem => "pressurisation-system",
            Self::Floatplane => "floatplane",
            Self::FloatingHull => "floating-hull",
            Self::SkiLandingGear => "ski-landing-gear",
            Self::FloatAlightingGear => "float-alighting-gear",
        }
    }

    /// Its name, as reg 61.755 words it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::TailwheelUndercarriage => "tailwheel undercarriage",
            Self::RetractableUndercarriage => "retractable undercarriage",
            Self::ManualPropellerPitchControl => "manual propeller pitch control (piston engine)",
            Self::GasTurbineEngine => "gas turbine engine",
            Self::MultiEngineCentreLineThrust => "multi-engine centre-line thrust",
            Self::PressurisationSystem => "pressurisation system",
            Self::Floatplane => "floatplane",
            Self::FloatingHull => "floating hull",
            Self::SkiLandingGear => "ski landing gear",
            Self::FloatAlightingGear => "float alighting gear",
        }
    }
}
