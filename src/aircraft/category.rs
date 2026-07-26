//! Aircraft categories under CASA Part 61, and the design features each may
//! carry.

use serde::{Deserialize, Serialize};

use super::design_feature::DesignFeature;

/// A category of aircraft for Part 61 purposes.
///
/// Reg 61.015 lists exactly five: *"Each of the following is a category of
/// aircraft: (a) aeroplane; (b) helicopter; (c) powered-lift aircraft;
/// (d) gyroplane; (e) airship."*
///
/// [`RegisteredSailplane`](Self::RegisteredSailplane) is a sixth value here
/// but is **not** a reg 61.015 category — see its own documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AircraftCategory {
    /// Reg 61.015(a).
    Aeroplane,
    /// Reg 61.015(b).
    Helicopter,
    /// Reg 61.015(c).
    PoweredLift,
    /// Reg 61.015(d).
    Gyroplane,
    /// Reg 61.015(e).
    Airship,
    /// **Not a reg 61.015 category.** Part 61 reaches registered sailplanes
    /// through reg 61.007(2) instead — *"The Part applies also to flight in a
    /// glider that is a registered sailplane"* — an applicability extension,
    /// not a category.
    ///
    /// Kept as a value because consuming code branches on it, but it has no
    /// class rating under reg 61.020 and no design features under reg 61.755.
    RegisteredSailplane,
}

impl AircraftCategory {
    /// The design features that require an endorsement for this category,
    /// per the reg 61.755 table.
    ///
    /// Empty for [`RegisteredSailplane`](Self::RegisteredSailplane), which
    /// does not appear in that table at all.
    pub fn design_features(self) -> &'static [DesignFeature] {
        use DesignFeature::*;
        match self {
            Self::Aeroplane => &[
                TailwheelUndercarriage,
                RetractableUndercarriage,
                ManualPropellerPitchControl,
                GasTurbineEngine,
                MultiEngineCentreLineThrust,
                PressurisationSystem,
                Floatplane,
                FloatingHull,
                SkiLandingGear,
            ],
            Self::Helicopter => &[
                FloatAlightingGear,
                RetractableUndercarriage,
                GasTurbineEngine,
            ],
            Self::PoweredLift => &[
                RetractableUndercarriage,
                PressurisationSystem,
                GasTurbineEngine,
            ],
            Self::Gyroplane => &[
                RetractableUndercarriage,
                PressurisationSystem,
                GasTurbineEngine,
            ],
            Self::Airship => &[PressurisationSystem, GasTurbineEngine],
            Self::RegisteredSailplane => &[],
        }
    }

    /// Whether `feature` requires an endorsement for this category.
    pub fn allows(self, feature: DesignFeature) -> bool {
        self.design_features().contains(&feature)
    }
}
