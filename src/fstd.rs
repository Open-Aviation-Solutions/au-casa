//! Recognition of a flight simulation training device under CASA Part 61.

use serde::{Deserialize, Serialize};

/// A State whose national aviation authority's device qualifications CASA
/// recognises.
///
/// The list is defined in CASR 1998 reg 61.010 (*recognised foreign State*).
/// Reg 61.047 lets CASA prescribe further countries by legislative instrument;
/// no such instrument is known to be in force, so this is a closed enum and a
/// future prescription needs a new variant.
///
/// The regulation groups the last fifteen under the heading "the following
/// EASA member States". That grouping is reproduced as written — note it lists
/// the United Kingdom, which is no longer an EASA member State; the
/// regulation's text is authoritative here, not the current EASA membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecognisedForeignState {
    Canada,
    HongKong,
    NewZealand,
    UnitedStatesOfAmerica,
    Belgium,
    CzechRepublic,
    Denmark,
    Finland,
    France,
    Germany,
    Ireland,
    Italy,
    Netherlands,
    Norway,
    Portugal,
    Spain,
    Sweden,
    Switzerland,
    UnitedKingdom,
}

/// The basis on which a device is a *flight simulation training device* for
/// CASA Part 61 purposes.
///
/// These are the five sub-types of the reg 61.010 definition:
///
/// > **flight simulation training device** means: (a) a qualified flight
/// > simulator; or (b) a qualified flight training device; or (c) a synthetic
/// > trainer that is approved under Civil Aviation Order 45.0; or (d) a device
/// > that meets the qualification standards prescribed by a legislative
/// > instrument under regulation 61.045; or (e) a device that is qualified
/// > (however described) by the national aviation authority of a recognised
/// > foreign State.
///
/// There is deliberately **no variant for an unrecognised device**. "Not
/// approved" is not one of the regulation's sub-types — it is the *absence* of
/// recognition, and is modelled as `Option::None`. A personal simulator is
/// perfectly loggable; it simply earns no credit.
///
/// This is a judgement recorded about a specific physical device, not
/// something derivable: a qualification is a certificate an authority issues
/// to one device. Unlike CASA aircraft classification, which is derived from a
/// Doc 8643 designator, there is no resolver here and no lookup table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FstdRecognition {
    /// Reg 61.010(a) — a flight simulator qualified under CASR Part 60.
    ///
    /// The CASR Dictionary defines *qualified flight simulator* as "a flight
    /// simulator that is qualified under Part 60 of CASR". Part 60 qualifies
    /// simulators at Level A, B, C or D (reg 60.020, table 60.020-1).
    QualifiedFlightSimulator,

    /// Reg 61.010(b) — a flight training device qualified under CASR Part 60.
    ///
    /// The CASR Dictionary defines *qualified flight training device* as "a
    /// flight training device that is qualified under Part 60 of CASR". Part
    /// 60 qualifies these at FAA Level 4–7 or EASA Level 1–3 (reg 60.020,
    /// table 60.020-2).
    QualifiedFlightTrainingDevice,

    /// Reg 61.010(c) — a synthetic trainer approved under Civil Aviation Order
    /// 45.0.
    ///
    /// **Historical only.** CAO 45.0 is no longer in force. CASA's AC 60-01
    /// v2.0 (July 2026) reproduces the 61.010 definition and notes: "Readers
    /// are reminded that Civil Aviation Order (CAO) 45.0, although mentioned
    /// in this definition, is no longer in force. Devices formerly approved
    /// under this CAO are now recognised through a regulation 61.045
    /// instrument." The regulation text still cites CAO 45.0, so the variant
    /// is kept for records predating the change; new records should use
    /// [`PrescribedUnderReg61045`](Self::PrescribedUnderReg61045).
    SyntheticTrainerCao45,

    /// Reg 61.010(d) — a device meeting qualification standards prescribed by
    /// a legislative instrument under reg 61.045.
    ///
    /// Reg 61.045 empowers CASA to "prescribe qualification standards for
    /// flight simulation training devices" by legislative instrument. This is
    /// now the live catch-all, including for devices formerly approved under
    /// CAO 45.0.
    PrescribedUnderReg61045,

    /// Reg 61.010(e) — a device qualified (however described) by the national
    /// aviation authority of a recognised foreign State.
    ForeignStateQualified(RecognisedForeignState),
}

/// Whether time logged on a device with this recognition counts toward Part 61
/// aeronautical experience and currency requirements.
///
/// All five sub-types of the reg 61.010 definition count — the regulation
/// draws no distinction between them, so the question is simply whether the
/// device was recognised at all. `None` (an unrecognised device) does not
/// count.
///
/// **This does not check dates, by design.** A qualification is time-bounded
/// (CASR 60.040 gives 12 months from certificate issue, and 60.050 allows
/// variation, cancellation or suspension), so recognition is only meaningful
/// as at a particular date. The intended usage is that a consumer records the
/// recognition that applied *at the time of the session* — checking
/// `DeviceQualification::is_in_force_on` against the session date when it
/// does so — and stores that snapshot on its own record. By the time this
/// function sees a recognition, the temporal question has already been
/// answered.
pub fn counts_for_part61(recognition: Option<&FstdRecognition>) -> bool {
    recognition.is_some()
}
