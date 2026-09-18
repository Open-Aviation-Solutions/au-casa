//! The compiled `designator -> AircraftDescription` table.
//!
//! # Where the data comes from — and where it does not
//!
//! **No ICAO Doc 8643 data is vendored here, and Doc 8643 is not the source
//! of any fact in this table.** Doc 8643 is sold by ICAO and is not freely
//! redistributable.
//!
//! Two different things are easy to conflate, so to be explicit:
//!
//! - The **designator strings** (`"C172"`, `"CH7A"`) are used only as lookup
//!   keys — short public identifiers, in the same way an airport code is.
//!   Comments naming which models a designator covers are orientation for a
//!   reader, not transcribed Doc 8643 content.
//! - The **facts** each row asserts — airframe kind, engine count, engine
//!   type — come from the type certificate data sheet named in that row's
//!   `source`, which is a free publication of the FAA or EASA. Nothing is
//!   copied from Doc 8643 or from any mirror of it.
//!
//! A row is [`Confidence::Confirmed`] only when someone has actually read
//! the cited TCDS. Rows still carrying [`Confidence::Provisional`] were
//! compiled from manufacturer model information and are waiting for that
//! check — the flag is meaningful, so do not promote a row without doing it.
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

const TCDS_3A12: &str = "FAA Type Certificate Data Sheet 3A12 rev 80 (26 May 2010)";
const TCDS_A7SO: &str = "FAA Type Certificate Data Sheet A7SO rev 24 (9 Apr 2019)";
const TCDS_A37CE: &str = "FAA Type Certificate Data Sheet A37CE rev 22 (31 Oct 2017)";
const TCDS_A759: &str = "FAA Type Certificate Data Sheet A-759 rev 73 (9 Feb 2011)";
const TCDS_EASA_A232: &str = "EASA Type-Certificate Data Sheet IM.A.232 issue 03 (10 Feb 2017)";

// Candidate sources for the rows below, identified this session by public
// search rather than by opening the document — see task 0003. Each one
// names the right certificate but not yet a checked revision/date, so every
// row citing one of these stays Provisional until someone reads the actual
// sheet and promotes it, same as PA25/R44 above.
const TCDS_3A19_CANDIDATE: &str =
    "FAA Type Certificate Data Sheet 3A19 (Cessna 150/152) — revision not yet checked";
const TCDS_2A13_CANDIDATE: &str =
    "FAA Type Certificate Data Sheet 2A13 (Piper PA-28 family) — revision not yet checked";
const EASA_A221_CANDIDATE: &str =
    "EASA Type-Certificate Data Sheet EASA.A.221 (Schleicher ASK 21) — not yet confirmed this covers the base ASK 21 rather than only the ASK 21 B";
const EASA_A025_CANDIDATE: &str =
    "EASA Type-Certificate Data Sheet EASA.A.025 (Schempp-Hirth Duo Discus) — not yet confirmed this is the unpowered variant, distinct from the self-launching Duo Discus T (EASA.A.074)";
const LS4_CANDIDATE: &str =
    "LBA type certificate 345 (Rolladen-Schneider LS4) — exact EASA TCDS reference not yet found";

/// Look up the compiled facts for a Doc 8643 type designator.
///
/// Returns `None` for an uncatalogued designator rather than guessing.
pub fn lookup(designator: &str) -> Option<DesignatorFacts> {
    let (airframe, engine_count, engine_type, confidence, source) = match designator {
        // Cessna 172 — single piston landplane.
        // TCDS 3A12 I: Continental O-300-A/-B; later models list Lycoming
        // O-320/O-360/IO-360. One reciprocating engine throughout the series,
        // 172 through 172S. Float variants are approved (172A onward are
        // "PCL-SM"), which is per-airframe and so an override, not derived.
        "C172" => (
            AirframeKind::LandPlane,
            1,
            EngineType::Piston,
            Confidence::Confirmed,
            TCDS_3A12,
        ),
        // Piper PA-25 Pawnee — single piston landplane, tailwheel. The
        // tailwheel design feature is not derived here: undercarriage
        // configuration varies per airframe, so it is an override.
        //
        // Still Provisional: the Pawnee's TCDS could not be obtained. Note
        // it is *not* 1A11, a plausible-looking guess that turns out to be
        // wrong — the Pawnee is certificated in the restricted category
        // under a different sheet. Confirm the number before promoting.
        "PA25" => (
            AirframeKind::LandPlane,
            1,
            EngineType::Piston,
            Confidence::Provisional,
            MANUFACTURER_MODEL,
        ),
        // Piper PA-34 Seneca — twin piston landplane.
        // TCDS A7SO I: one Lycoming IO-360-C1E6 and one LIO-360-C1E6
        // (counter-rotating), both reciprocating. Covers PA-34-200, -200T
        // and -220T.
        "PA34" => (
            AirframeKind::LandPlane,
            2,
            EngineType::Piston,
            Confidence::Confirmed,
            TCDS_A7SO,
        ),
        // Piper PA-44 Seminole — twin piston landplane.
        // EASA IM.A.232 A.III.2 describes it outright: "Twin engine
        // reciprocating, all-metal, four-place, unpressurized, low wing
        // aeroplane, retractable tricycle landing gear." Engines are
        // Lycoming O-360/LO-360. The EASA sheet is used because it states
        // the configuration explicitly; the FAA equivalent is A19SO.
        "PA44" => (
            AirframeKind::LandPlane,
            2,
            EngineType::Piston,
            Confidence::Confirmed,
            TCDS_EASA_A232,
        ),
        // Cessna 208 Caravan — single turboprop landplane.
        // TCDS A37CE I: one Pratt & Whitney Canada PT6A-114 or PT6A-114A
        // Turbo Prop. Approved as both landplane ("11 PCLM") and seaplane
        // ("11 PCSM"); floats are per-airframe, so an override.
        "C208" => (
            AirframeKind::LandPlane,
            1,
            EngineType::TurboProp,
            Confidence::Confirmed,
            TCDS_A37CE,
        ),
        // Robinson R44 — single piston helicopter.
        //
        // Still Provisional: TC H11NM is the right certificate, but the data
        // sheet itself was not obtainable free of a login, so nobody has
        // read it. The Lycoming O-540 fitment is manufacturer information.
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
        // Cessna 152 — single piston landplane, Lycoming O-235.
        // Shares TCDS 3A19 with the Cessna 150. Still Provisional: identified
        // by public search this session, not read from the actual sheet.
        "C152" => (
            AirframeKind::LandPlane,
            1,
            EngineType::Piston,
            Confidence::Provisional,
            TCDS_3A19_CANDIDATE,
        ),
        // Piper PA-28 — fixed-gear Cherokee/Warrior/Archer family, single
        // piston landplane. TC 2A13 spans PA-28-140 through PA-28-236.
        // Undercarriage is not derived here — see PA25/CH7A above; a fixed
        // vs retractable split already exists at the designator level
        // between this row and P28R, but that is per Doc 8643's own
        // designator assignment, not something this table infers.
        "PA28" => (
            AirframeKind::LandPlane,
            1,
            EngineType::Piston,
            Confidence::Provisional,
            TCDS_2A13_CANDIDATE,
        ),
        // Piper PA-28R — retractable-gear Arrow variants (PA-28R-180, -200,
        // -201 etc.), single piston landplane. Same TC 2A13 as PA28.
        "P28R" => (
            AirframeKind::LandPlane,
            1,
            EngineType::Piston,
            Confidence::Provisional,
            TCDS_2A13_CANDIDATE,
        ),
        // Schleicher ASK 21 — two-seat glider, no powerplant.
        "AS21" => (
            AirframeKind::Glider,
            0,
            EngineType::None,
            Confidence::Provisional,
            EASA_A221_CANDIDATE,
        ),
        // Schempp-Hirth Duo Discus — two-seat glider, no powerplant. The
        // self-launching "Duo Discus T" is a separate Doc 8643 designator
        // (EASA.A.074), not this row — same kind of split as CH7A/CH7B.
        "DUOD" => (
            AirframeKind::Glider,
            0,
            EngineType::None,
            Confidence::Provisional,
            EASA_A025_CANDIDATE,
        ),
        // Rolladen-Schneider LS4 — single-seat glider, no powerplant.
        "LS4" => (
            AirframeKind::Glider,
            0,
            EngineType::None,
            Confidence::Provisional,
            LS4_CANDIDATE,
        ),
        _ => return None,
    };

    Some(DesignatorFacts {
        description: AircraftDescription::new(airframe, engine_count, engine_type),
        confidence,
        source,
    })
}
