//! The compiled `designator -> AircraftDescription` table.
//!
//! **No ICAO Doc 8643 data is vendored here.** Doc 8643 is sold by ICAO and
//! is not freely redistributable, so this table is compiled independently:
//! each row records only the four facts a Part 61 classification actually
//! needs (airframe kind, engine count, engine type), with a source citation
//! and a confidence flag.
//!
//! The table grows on demand rather than attempting to mirror Doc 8643. An
//! uncatalogued designator returns nothing and the caller falls back to its
//! own override — see
//! [`resolve_classification`](super::resolve_classification).

use serde::{Deserialize, Serialize};

use super::description::{AircraftDescription, AirframeKind, EngineType};

/// How much weight a compiled table row carries.
///
/// Confidence belongs to the *row*, not to the derived category or class:
/// once the description is right, the Part 61 derivation from it is
/// deterministic. So one flag covers the whole classification rather than
/// each field carrying its own copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    /// Checked against a type certificate data sheet.
    Confirmed,
    /// Compiled from manufacturer model information, not yet checked against
    /// a type certificate data sheet.
    Provisional,
    /// Not derived from the table at all — supplied wholly by a consumer's
    /// override.
    Overridden,
}

/// One compiled row: the facts, where they came from, and how far to trust
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesignatorFacts {
    pub description: AircraftDescription,
    pub confidence: Confidence,
    pub source: &'static str,
}

const MANUFACTURER_MODEL: &str =
    "Manufacturer model series; pending verification against an FAA/EASA type \
     certificate data sheet";

const TCDS_A759: &str = "FAA Type Certificate Data Sheet A-759 rev 73 (9 Feb 2011)";

/// Look up the compiled facts for a Doc 8643 type designator.
///
/// Returns `None` for an uncatalogued designator rather than guessing.
pub fn lookup(designator: &str) -> Option<DesignatorFacts> {
    let (airframe, engine_count, engine_type, confidence, source) = match designator {
        // Cessna 172 — single piston landplane.
        "C172" => (
            AirframeKind::LandPlane,
            1,
            EngineType::Piston,
            Confidence::Provisional,
            MANUFACTURER_MODEL,
        ),
        // Piper PA-25 Pawnee — single piston landplane, tailwheel. The
        // tailwheel design feature is not derived here: undercarriage
        // configuration varies per airframe, so it is an override.
        "PA25" => (
            AirframeKind::LandPlane,
            1,
            EngineType::Piston,
            Confidence::Provisional,
            MANUFACTURER_MODEL,
        ),
        // Piper PA-34 Seneca — twin piston landplane.
        "PA34" => (
            AirframeKind::LandPlane,
            2,
            EngineType::Piston,
            Confidence::Provisional,
            MANUFACTURER_MODEL,
        ),
        // Piper PA-44 Seminole — twin piston landplane.
        "PA44" => (
            AirframeKind::LandPlane,
            2,
            EngineType::Piston,
            Confidence::Provisional,
            MANUFACTURER_MODEL,
        ),
        // Cessna 208 Caravan — single turboprop landplane.
        "C208" => (
            AirframeKind::LandPlane,
            1,
            EngineType::TurboProp,
            Confidence::Provisional,
            MANUFACTURER_MODEL,
        ),
        // Robinson R44 — single piston helicopter.
        "R44" => (
            AirframeKind::Helicopter,
            1,
            EngineType::Piston,
            Confidence::Provisional,
            MANUFACTURER_MODEL,
        ),
        // Citabria, 7ECA series — single piston landplane.
        // TCDS A-759 XVIII: Continental O-200-A, one reciprocating engine.
        //
        // Doc 8643 splits the Citabria across two designators rather than
        // one, so a logbook row reading only "Citabria" is ambiguous between
        // this and CH7B; the model number decides. CH7A additionally covers
        // the Aeronca 7AC Champion, a different aeroplane sharing the
        // designator (TCDS A-759 I: Continental A-65-8).
        //
        // Neither ambiguity affects the classification: every model under
        // both designators is a single-engine piston landplane, so all of
        // them resolve to Aeroplane / SingleEngineAeroplane either way.
        //
        // Undercarriage is not derived here, for the same reason as PA25 —
        // and this family shows why. A-759 approves the 7ECA on floats
        // (item 204(c)) and the 7GCBC on floats *or* skis (items 203(k),
        // 203(l)), so Floatplane and SkiLandingGear are properties of an
        // individual airframe, not of the designator.
        "CH7A" => (
            AirframeKind::LandPlane,
            1,
            EngineType::Piston,
            Confidence::Confirmed,
            TCDS_A759,
        ),
        // Citabria, 7GCAA/7GCBC/7KCAB series — single piston landplane.
        // TCDS A-759 XIX and XX: Lycoming O-320-A2B/C2B/A2D; XXI: Lycoming
        // IO-320-E2A or AEIO-320-E2B. One reciprocating engine throughout.
        // See CH7A above on the split.
        "CH7B" => (
            AirframeKind::LandPlane,
            1,
            EngineType::Piston,
            Confidence::Confirmed,
            TCDS_A759,
        ),
        _ => return None,
    };

    Some(DesignatorFacts {
        description: AircraftDescription::new(airframe, engine_count, engine_type),
        confidence,
        source,
    })
}
