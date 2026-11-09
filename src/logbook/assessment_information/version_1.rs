// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Version 1 of CASA's information on an assessment, read only to migrate
//! it: the check type and outcome as phrases, and `check_purpose`, one of
//! the two purposes reg 61.880(3) names, beside the flight test's outcome.

use logbook_shared_kernel::AuthoritySpecificInformation;
use thiserror::Error;

use super::{
    AssessedPurpose, AssessmentInformation, AssessmentInformationError, CheckOutcome, CheckPurpose,
    CheckType,
};
use crate::logbook::stored::{self, Unreadable};

const VERSION: u32 = 1;
const CHECK_PURPOSE: &str = "check_purpose";
const KEYS: [&str; 3] = [super::CHECK_TYPE, super::OUTCOME, CHECK_PURPOSE];

/// Why stored information cannot be migrated from version 1.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MigrationError {
    #[error("CASA's information on an assessment is not a JSON object")]
    NotAnObject,
    #[error("CASA's information on an assessment is not of version 1 or 2")]
    NotVersion1,
    /// Information of version 2 already, which cannot be read.
    #[error(transparent)]
    Unreadable(AssessmentInformationError),
    #[error("CASA's information on an assessment of version 1 has no key {0:?}")]
    UnknownKey(String),
    #[error(
        "CASA's information on an assessment of version 1 holds no answer of its kind under {0:?}"
    )]
    InvalidAnswer(&'static str),
    #[error("the flight test records no purpose, which cannot be guessed: fill in its purposes")]
    NoPurpose,
    #[error(
        "the flight test records no outcome for its purpose, which cannot be guessed: fill in \
         its purposes"
    )]
    NoOutcome,
}

impl From<Unreadable> for MigrationError {
    fn from(unreadable: Unreadable) -> Self {
        match unreadable {
            Unreadable::NotAnObject => Self::NotAnObject,
            Unreadable::UnsupportedVersion => Self::NotVersion1,
            Unreadable::UnknownKey(key) => Self::UnknownKey(key),
            Unreadable::InvalidAnswer(key) => Self::InvalidAnswer(key),
        }
    }
}

/// Stored information of version 1, as the current format holds it.
pub(super) fn migrate(
    information: &AuthoritySpecificInformation,
) -> Result<AssessmentInformation, MigrationError> {
    let answers = stored::answers(information, VERSION, &KEYS, true)?;
    let check_type = super::choice(answers, super::CHECK_TYPE, check_type)?;
    let outcome = super::choice(answers, super::OUTCOME, CheckOutcome::parse)?;
    let purpose = super::choice(answers, CHECK_PURPOSE, check_purpose)?;
    if check_type != Some(CheckType::FlightTest) {
        return Ok(AssessmentInformation {
            check_type,
            outcome,
            check_purposes: Vec::new(),
        });
    }
    let purpose = purpose.ok_or(MigrationError::NoPurpose)?;
    let outcome = outcome.ok_or(MigrationError::NoOutcome)?;
    Ok(AssessmentInformation {
        check_type,
        outcome: None,
        check_purposes: vec![AssessedPurpose { purpose, outcome }],
    })
}

/// A check type as version 1 spelt it, in regs 61.745(3) and (3A)'s words.
/// Spelt out here, not read from the labels, so that a label can change.
fn check_type(phrase: &str) -> Option<CheckType> {
    use CheckType::*;
    match phrase {
        "flight test" => Some(FlightTest),
        "flight review" => Some(FlightReview),
        "operator proficiency check" => Some(OperatorProficiencyCheck),
        "instrument proficiency check" => Some(InstrumentProficiencyCheck),
        "night vision imaging system proficiency check" => {
            Some(NightVisionImagingSystemProficiencyCheck)
        }
        "aerial application proficiency check" => Some(AerialApplicationProficiencyCheck),
        "instructor proficiency check" => Some(InstructorProficiencyCheck),
        "examiner proficiency check" => Some(ExaminerProficiencyCheck),
        _ => None,
    }
}

/// A purpose as version 1 spelt it: only the two of reg 61.880(3).
fn check_purpose(phrase: &str) -> Option<CheckPurpose> {
    match phrase {
        "instrument rating" => Some(CheckPurpose::InstrumentRating),
        "instrument endorsement" => Some(CheckPurpose::InstrumentEndorsement),
        _ => None,
    }
}
