//! Layer A: the country-agnostic description of an aircraft type.

use serde::{Deserialize, Serialize};

/// What sort of airframe a type is — the first element of the ICAO Doc 8643
/// Aircraft Description Code.
///
/// `Glider` and `Airship` are not Doc 8643 ADC first-character values; they
/// are carried here because Part 61 needs to reach the airship category (reg
/// 61.015(e)) and registered sailplanes (reg 61.007(2)), and there is nowhere
/// else to express them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AirframeKind {
    /// `L` — landplane.
    LandPlane,
    /// `S` — seaplane.
    SeaPlane,
    /// `A` — amphibian.
    Amphibian,
    /// `H` — helicopter.
    Helicopter,
    /// `G` — gyrocopter.
    Gyrocopter,
    /// `T` — tiltrotor.
    TiltRotor,
    /// Not an ADC value — see the enum's own documentation.
    Glider,
    /// Not an ADC value — see the enum's own documentation.
    Airship,
}

/// The kind of powerplant a type has — the third element of the Doc 8643 ADC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EngineType {
    /// `P` — piston.
    Piston,
    /// `T` — turboprop / turboshaft.
    TurboProp,
    /// `J` — jet.
    Jet,
    /// No powerplant — a glider.
    None,
}

impl EngineType {
    /// Whether this powerplant is a gas turbine, and so attracts the reg
    /// 61.755 `gas turbine engine` design feature.
    pub fn is_gas_turbine(self) -> bool {
        matches!(self, Self::TurboProp | Self::Jet)
    }
}

/// The country-agnostic facts about an aircraft type, from which the CASA
/// classification is derived.
///
/// Deliberately narrower than the full Doc 8643 Aircraft Description Code:
/// the ADC's wake turbulence category is not carried, because no Part 61
/// classification rule depends on it. Add it when something needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AircraftDescription {
    pub airframe: AirframeKind,
    pub engine_count: u8,
    pub engine_type: EngineType,
}

impl AircraftDescription {
    pub fn new(airframe: AirframeKind, engine_count: u8, engine_type: EngineType) -> Self {
        Self {
            airframe,
            engine_count,
            engine_type,
        }
    }
}
