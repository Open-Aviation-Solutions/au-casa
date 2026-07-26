//! Design features requiring an endorsement under CASA Part 61.

use serde::{Deserialize, Serialize};

use super::category::AircraftCategory;

/// A design feature for which reg 61.755 requires a design feature
/// endorsement.
///
/// **One flat enum, not per-category enums.** The reg 61.755 table is
/// many-to-many rather than a tree: seven of the ten features belong to a
/// single category, but [`RetractableUndercarriage`](Self::RetractableUndercarriage)
/// spans four categories, [`PressurisationSystem`](Self::PressurisationSystem)
/// four, and [`GasTurbineEngine`](Self::GasTurbineEngine) all five. Splitting
/// per category would duplicate the shared features and fight the shape of
/// the regulation instead of modelling it.
///
/// Validity of a (category, feature) pair is enforced at the one construction
/// path — [`resolve_classification`](crate::aircraft::resolve_classification)
/// — rather than in the type structure. See
/// [`applicable_categories`](Self::applicable_categories) and
/// [`AircraftCategory::design_features`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DesignFeature {
    /// Aeroplane only.
    TailwheelUndercarriage,
    /// Aeroplane, helicopter, powered-lift aircraft, gyroplane.
    RetractableUndercarriage,
    /// Aeroplane only. Reg 61.755 qualifies this as a piston-engine feature.
    ManualPropellerPitchControl,
    /// All five categories.
    GasTurbineEngine,
    /// Aeroplane only.
    MultiEngineCentreLineThrust,
    /// Aeroplane, powered-lift aircraft, gyroplane, airship.
    PressurisationSystem,
    /// Aeroplane only. Distinct from the helicopter-only
    /// [`FloatAlightingGear`](Self::FloatAlightingGear).
    Floatplane,
    /// Aeroplane only.
    FloatingHull,
    /// Aeroplane only.
    SkiLandingGear,
    /// Helicopter only.
    FloatAlightingGear,
}

impl DesignFeature {
    /// The categories for which this feature requires an endorsement — the
    /// inverse of [`AircraftCategory::design_features`], and the reason the
    /// enum is flat.
    pub fn applicable_categories(self) -> &'static [AircraftCategory] {
        use AircraftCategory::*;
        match self {
            Self::TailwheelUndercarriage
            | Self::ManualPropellerPitchControl
            | Self::MultiEngineCentreLineThrust
            | Self::Floatplane
            | Self::FloatingHull
            | Self::SkiLandingGear => &[Aeroplane],
            Self::FloatAlightingGear => &[Helicopter],
            Self::RetractableUndercarriage => &[Aeroplane, Helicopter, PoweredLift, Gyroplane],
            Self::PressurisationSystem => &[Aeroplane, PoweredLift, Gyroplane, Airship],
            Self::GasTurbineEngine => &[Aeroplane, Helicopter, PoweredLift, Gyroplane, Airship],
        }
    }
}
