// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA (Australia) national aviation regulatory package.
//!
//! Resolves national (CASA Part 61) facts that the ICAO-universal kernel
//! deliberately does not carry. Stateless: pure functions and value objects,
//! with no persistence and no identity-keyed storage.
//!
//! - [`aircraft`] — CASA category, class rating and design features, derived
//!   from an ICAO Doc 8643 type designator (task `0001`).
//! - [`fstd`] — recognition of a flight simulation training device under reg
//!   61.010, and whether time on it counts for Part 61 (task `0002`).
//! - [`licence`] — the Part 61 term (licence and category rating) for each of
//!   the kernel's Annex 1 titles of licence, or that CASA does not issue it.

pub mod aircraft;
pub mod fstd;
pub mod licence;

pub use aircraft::{
    classify, resolve_classification, AircraftCategory, AircraftClassRating, AircraftDescription,
    AirframeKind, CasaAircraftClassification, ClassificationError, ClassificationOverride,
    Confidence, DesignFeature, EngineType,
};
pub use fstd::{counts_for_part61, FstdRecognition, RecognisedForeignState};
pub use licence::{part61_term, FlightCrewLicence, Part61Term};
