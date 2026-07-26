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

pub mod aircraft;
pub mod fstd;

pub use aircraft::{
    classify, resolve_classification, AircraftCategory, AircraftClassRating, AircraftDescription,
    AirframeKind, CasaAircraftClassification, ClassificationError, ClassificationOverride,
    Confidence, DesignFeature, EngineType,
};
pub use fstd::{counts_for_part61, FstdRecognition, RecognisedForeignState};
