//! Layer A: the country-agnostic description of an aircraft type.
//!
//! # Why country-agnostic types live in a national package
//!
//! Nothing in this module is Australian. `AircraftDescription` is the ICAO
//! Doc 8643 Aircraft Description Code concept, and the
//! [`designators`](super::designators) table that produces it would be
//! identical in a New Zealand or United Kingdom package. Only Layer B —
//! [`classify`](super::classify), and the `AircraftCategory` /
//! `AircraftClassRating` / `DesignFeature` types it produces — is CASA
//! regulatory interpretation.
//!
//! It stays here deliberately: `au-casa` is the only national package that
//! exists, so extracting a shared crate now would be building the extension
//! rather than leaving room for it. The module boundary is already the seam,
//! so the move stays cheap. A separate `icao-type-designators` crate is the
//! likely home rather than `icao-shared-kernel`, whose thin-hub discipline
//! covers aggregates and repository protocols — reference data is neither.
//!
//! **If it is ever extracted, restore the wake turbulence category.** Layer A
//! is nominally universal but has been narrowed by one country's needs in
//! three places, and the first is the one that would make a shared table
//! quietly wrong for other consumers:
//!
//! - `AircraftDescription` omits WTC, because no Part 61 classification rule
//!   depends on it — see the struct's own note. Task `0001`'s design included
//!   it; the implementation dropped it.
//! - [`AirframeKind`] carries `Glider` and `Airship`, which are not ADC
//!   values, to reach reg 61.007(2) and reg 61.015(e).
//! - [`EngineType::is_gas_turbine`] is defined by what reg 61.755 needs.
//!
//! The omission is invisible to a consumer; the two additions are harmless to
//! anyone not using them.

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
