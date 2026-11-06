// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's authority-specific information on an assessment, as it is stored.

use logbook_shared_kernel::AuthoritySpecificInformation;
use serde_json::{Map, Value};
use thiserror::Error;

/// CASA's information on an assessment: what kind of check it was, its
/// outcome and its purpose. Every answer is optional here; which are asked,
/// and which are required, is for CASA's questions.
///
/// A stored format, versioned independently of
/// [`EntryInformation`](super::EntryInformation)'s, with every key and
/// spelling mapped by hand, as that one is. The host stores the version
/// beside the answers, under the kernel's reserved key
/// [`AuthoritySpecificInformation::VERSION_KEY`] (`_version`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssessmentInformation {
    /// `check_type`: which of the checks of regs 61.745(3) and (3A) it was.
    pub check_type: Option<CheckType>,
    /// `outcome`: whether the pilot passed it.
    pub outcome: Option<CheckOutcome>,
    /// `check_purpose`: which rating or endorsement a flight test was for,
    /// where reg 61.880(3) depends on it.
    pub check_purpose: Option<CheckPurpose>,
}

/// The checks that can be recorded against a flight, as regs 61.745(3) and
/// (3A) name them. Each is stored in those words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckType {
    /// `flight test` (reg 61.745(3)(a), (b)).
    FlightTest,
    /// `flight review` (reg 61.745(1); and reg 61.745(3)(d), "a flight
    /// review for a pilot type rating").
    FlightReview,
    /// `operator proficiency check` (reg 61.745(3)(e)(i)).
    OperatorProficiencyCheck,
    /// `instrument proficiency check` (reg 61.745(3A)(a)).
    InstrumentProficiencyCheck,
    /// `night vision imaging system proficiency check` (reg 61.745(3A)(b)).
    NightVisionImagingSystemProficiencyCheck,
    /// `aerial application proficiency check` (reg 61.745(3A)(c)).
    AerialApplicationProficiencyCheck,
    /// `instructor proficiency check` (reg 61.745(3A)(d)).
    InstructorProficiencyCheck,
    /// `examiner proficiency check` (reg 61.745(3A)(e)).
    ExaminerProficiencyCheck,
}

impl CheckType {
    /// Its stored spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FlightTest => "flight test",
            Self::FlightReview => "flight review",
            Self::OperatorProficiencyCheck => "operator proficiency check",
            Self::InstrumentProficiencyCheck => "instrument proficiency check",
            Self::NightVisionImagingSystemProficiencyCheck => {
                "night vision imaging system proficiency check"
            }
            Self::AerialApplicationProficiencyCheck => "aerial application proficiency check",
            Self::InstructorProficiencyCheck => "instructor proficiency check",
            Self::ExaminerProficiencyCheck => "examiner proficiency check",
        }
    }

    /// The check a stored spelling names; `None` for any other.
    pub fn parse(spelling: &str) -> Option<Self> {
        match spelling {
            "flight test" => Some(Self::FlightTest),
            "flight review" => Some(Self::FlightReview),
            "operator proficiency check" => Some(Self::OperatorProficiencyCheck),
            "instrument proficiency check" => Some(Self::InstrumentProficiencyCheck),
            "night vision imaging system proficiency check" => {
                Some(Self::NightVisionImagingSystemProficiencyCheck)
            }
            "aerial application proficiency check" => Some(Self::AerialApplicationProficiencyCheck),
            "instructor proficiency check" => Some(Self::InstructorProficiencyCheck),
            "examiner proficiency check" => Some(Self::ExaminerProficiencyCheck),
            _ => None,
        }
    }
}

/// Whether the pilot passed the check ("passes", "successfully completes",
/// reg 61.745(3)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckOutcome {
    /// `passed`.
    Passed,
    /// `failed`.
    Failed,
}

impl CheckOutcome {
    /// Its stored spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }

    /// The outcome a stored spelling names; `None` for any other.
    pub fn parse(spelling: &str) -> Option<Self> {
        match spelling {
            "passed" => Some(Self::Passed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// What a flight test was for, of the purposes reg 61.880(3) names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckPurpose {
    /// `instrument rating`: "the flight test for the instrument rating" (reg
    /// 61.880(3)(a)).
    InstrumentRating,
    /// `instrument endorsement`: "the flight test for an instrument
    /// endorsement" (reg 61.880(3)(b)).
    InstrumentEndorsement,
}

impl CheckPurpose {
    /// Its stored spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InstrumentRating => "instrument rating",
            Self::InstrumentEndorsement => "instrument endorsement",
        }
    }

    /// The purpose a stored spelling names; `None` for any other.
    pub fn parse(spelling: &str) -> Option<Self> {
        match spelling {
            "instrument rating" => Some(Self::InstrumentRating),
            "instrument endorsement" => Some(Self::InstrumentEndorsement),
            _ => None,
        }
    }
}

/// Why stored information cannot be read as [`AssessmentInformation`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssessmentInformationError {
    #[error("CASA's information on an assessment is not a JSON object")]
    NotAnObject,
    #[error(
        "CASA's information on an assessment is not of version {}",
        AssessmentInformation::VERSION
    )]
    UnsupportedVersion,
    #[error("CASA's information on an assessment has no key {0:?}")]
    UnknownKey(String),
    #[error("CASA's information on an assessment holds no answer of its kind under {0:?}")]
    InvalidAnswer(&'static str),
}

const CHECK_TYPE: &str = "check_type";
const OUTCOME: &str = "outcome";
const CHECK_PURPOSE: &str = "check_purpose";

/// Every key of the format, in the order
/// [`AssessmentInformation::to_information`] writes them.
const KEYS: [&str; 3] = [CHECK_TYPE, OUTCOME, CHECK_PURPOSE];

impl AssessmentInformation {
    /// The version of this format.
    pub const VERSION: u32 = 1;

    /// Read stored information, which carries its version: it must be
    /// [`Self::VERSION`]. A key that is not this format's, or a spelling
    /// that is not one of its key's, is refused.
    pub fn read(
        information: &AuthoritySpecificInformation,
    ) -> Result<Self, AssessmentInformationError> {
        Self::read_with(information, true)
    }

    /// Read information about to be saved, as [`Self::read`] does, except
    /// that its version may be absent, as it is on information not saved
    /// yet (the host stamps it after checking it).
    pub fn read_to_save(
        information: &AuthoritySpecificInformation,
    ) -> Result<Self, AssessmentInformationError> {
        Self::read_with(information, false)
    }

    fn read_with(
        information: &AuthoritySpecificInformation,
        stored: bool,
    ) -> Result<Self, AssessmentInformationError> {
        let answers = information
            .value()
            .as_object()
            .ok_or(AssessmentInformationError::NotAnObject)?;
        let versioned = stored || answers.contains_key(AuthoritySpecificInformation::VERSION_KEY);
        if versioned && information.version() != Some(Self::VERSION) {
            return Err(AssessmentInformationError::UnsupportedVersion);
        }
        if let Some(key) = answers.keys().find(|key| {
            key.as_str() != AuthoritySpecificInformation::VERSION_KEY
                && !KEYS.contains(&key.as_str())
        }) {
            return Err(AssessmentInformationError::UnknownKey(key.clone()));
        }
        Ok(Self {
            check_type: choice(answers, CHECK_TYPE, CheckType::parse)?,
            outcome: choice(answers, OUTCOME, CheckOutcome::parse)?,
            check_purpose: choice(answers, CHECK_PURPOSE, CheckPurpose::parse)?,
        })
    }

    /// The answers as they are stored, without a version: the host writes
    /// that ([`Questions::stamp`](logbook_shared_kernel::Questions::stamp)).
    /// An answer not given has no key.
    pub fn to_information(&self) -> AuthoritySpecificInformation {
        // In the order of `KEYS`.
        let values: [Option<&str>; KEYS.len()] = [
            self.check_type.map(CheckType::as_str),
            self.outcome.map(CheckOutcome::as_str),
            self.check_purpose.map(CheckPurpose::as_str),
        ];
        let answers: Map<String, Value> = KEYS
            .into_iter()
            .zip(values)
            .filter_map(|(key, value)| Some((key.to_owned(), value?.into())))
            .collect();
        AuthoritySpecificInformation::new(Value::Object(answers))
    }
}

/// The choice under `key`, if there is one, read by `parse`, which gives
/// `None` for a spelling that is not one of the key's.
fn choice<T>(
    answers: &Map<String, Value>,
    key: &'static str,
    parse: fn(&str) -> Option<T>,
) -> Result<Option<T>, AssessmentInformationError> {
    answers
        .get(key)
        .map(|value| {
            value
                .as_str()
                .and_then(parse)
                .ok_or(AssessmentInformationError::InvalidAnswer(key))
        })
        .transpose()
}
