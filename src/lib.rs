//! CASA (Australia) national aviation regulatory package.
//!
//! Resolves national (CASA Part 61) facts that the ICAO-universal kernel
//! deliberately does not carry. Stateless: pure functions and value objects,
//! with no persistence and no identity-keyed storage.
//!
//! - [`fstd`] — recognition of a flight simulation training device under reg
//!   61.010, and whether time on it counts for Part 61 (task `0002`).
//!
//! Aircraft classification (CASA category, class and design features, derived
//! from an ICAO Doc 8643 type designator) is still in design — see
//! `tasks/0001-aircraft-classification.md`.

pub mod fstd;

pub use fstd::{counts_for_part61, FstdRecognition, RecognisedForeignState};
