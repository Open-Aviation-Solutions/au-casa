//! Aircraft class ratings under CASA Part 61.

use serde::{Deserialize, Serialize};

/// A class of aircraft for Part 61 purposes.
///
/// Reg 61.020 lists exactly these six: single-engine aeroplane, multi-engine
/// aeroplane, single-engine helicopter, powered-lift aircraft, single-engine
/// gyroplane, airship.
///
/// Not every aircraft has one. Multi-engine helicopters and gyroplanes are
/// absent from the regulation's list (they are type-rated instead), and a
/// registered sailplane has no class rating at all — hence
/// [`CasaAircraftClassification::class`](crate::aircraft::CasaAircraftClassification::class)
/// being optional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AircraftClassRating {
    SingleEngineAeroplane,
    MultiEngineAeroplane,
    SingleEngineHelicopter,
    PoweredLiftAircraft,
    SingleEngineGyroplane,
    Airship,
}
