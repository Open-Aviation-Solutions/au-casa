// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's authority-specific information on a logbook entry, as it is
//! stored.

use logbook_shared_kernel::AuthoritySpecificInformation;
use serde_json::Value;
use thiserror::Error;

use super::stored::{self, Unreadable};

/// CASA's information on an entry: the answers the logbook's common entry
/// cannot hold, each under its key. Every answer is optional here; which
/// are asked, and which are required, is for CASA's questions.
///
/// A stored format. Every key and spelling is mapped by hand, not derived,
/// so that renaming a field is not a change to stored records; a change to
/// the format is breaking, and ships with a migration of the stored
/// information. The host stores the version beside the answers, under the
/// kernel's reserved key [`AuthoritySpecificInformation::VERSION_KEY`]
/// (`_version`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntryInformation {
    /// `approved_fstd`: whether a session was in an "approved flight
    /// simulation training device" for the purpose (reg 61.010).
    pub approved_fstd: Option<bool>,
    /// `route`: which of subregulations 61.095(2) and (3) applies to a
    /// flight logged as pilot in command under supervision (reg
    /// 61.095(1)(c)).
    pub route: Option<SupervisionRoute>,
    /// `receiving_flight_training`: whether the person was "receiving flight
    /// training" on the instructor's route (reg 61.095(3)(b)).
    pub receiving_flight_training: Option<bool>,
    /// `azimuth_guidance_operation`: whether one of the entry's approaches,
    /// or DME or GNSS arrivals, was an "azimuth guidance operation" (reg
    /// 61.010; reg 61.870(6)).
    pub azimuth_guidance_operation: Option<bool>,
    /// `course_deviation_indicator_operation`: whether one of the entry's
    /// approaches, or DME or GNSS arrivals, was a "course deviation
    /// indicator operation" (reg 61.010; reg 61.870(7)).
    pub course_deviation_indicator_operation: Option<bool>,
    /// `dme_or_gnss_arrivals`: how many instrument approach operations were
    /// conducted "using distance measuring equipment or a global navigation
    /// satellite system" (Table 61.935, item 20, "Approach – DME or GNSS
    /// arrival procedure"). Stored as a JSON integer, as the kernel's
    /// `Answer::Count` is.
    pub dme_or_gnss_arrivals: Option<u64>,
}

/// Which subregulation makes a flight time as pilot in command under
/// supervision (reg 61.095(1)(c)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupervisionRoute {
    /// `operator`: "the flight is conducted by an operator that has training
    /// and checking responsibilities", and its pilot in command is
    /// authorised "to conduct the supervision of the person" (reg
    /// 61.095(2)).
    Operator,
    /// `instructor`: "the person is supervised by a flight instructor or
    /// flight examiner" (reg 61.095(3)).
    Instructor,
}

impl SupervisionRoute {
    /// Its stored spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::Instructor => "instructor",
        }
    }

    /// The route a stored spelling names; `None` for any other.
    pub fn parse(spelling: &str) -> Option<Self> {
        match spelling {
            "operator" => Some(Self::Operator),
            "instructor" => Some(Self::Instructor),
            _ => None,
        }
    }
}

/// Why stored information cannot be read as [`EntryInformation`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InformationError {
    #[error("CASA's information on an entry is not a JSON object")]
    NotAnObject,
    #[error(
        "CASA's information on an entry is not of version {}",
        EntryInformation::VERSION
    )]
    UnsupportedVersion,
    #[error("CASA's information on an entry has no key {0:?}")]
    UnknownKey(String),
    #[error("CASA's information on an entry holds no answer of its kind under {0:?}")]
    InvalidAnswer(&'static str),
}

impl InformationError {
    fn from_unreadable(unreadable: Unreadable) -> Self {
        match unreadable {
            Unreadable::NotAnObject => Self::NotAnObject,
            Unreadable::UnsupportedVersion => Self::UnsupportedVersion,
            Unreadable::UnknownKey(key) => Self::UnknownKey(key),
            Unreadable::InvalidAnswer(key) => Self::InvalidAnswer(key),
        }
    }
}

pub(super) const APPROVED_FSTD: &str = "approved_fstd";
pub(super) const ROUTE: &str = "route";
pub(super) const RECEIVING_FLIGHT_TRAINING: &str = "receiving_flight_training";
pub(super) const AZIMUTH_GUIDANCE_OPERATION: &str = "azimuth_guidance_operation";
pub(super) const COURSE_DEVIATION_INDICATOR_OPERATION: &str =
    "course_deviation_indicator_operation";
pub(super) const DME_OR_GNSS_ARRIVALS: &str = "dme_or_gnss_arrivals";

/// Every key of the format, in the order [`EntryInformation::to_information`]
/// writes them: the one list both reading and writing go by.
const KEYS: [&str; 6] = [
    APPROVED_FSTD,
    ROUTE,
    RECEIVING_FLIGHT_TRAINING,
    AZIMUTH_GUIDANCE_OPERATION,
    COURSE_DEVIATION_INDICATOR_OPERATION,
    DME_OR_GNSS_ARRIVALS,
];

impl EntryInformation {
    /// The version of this format.
    pub const VERSION: u32 = 1;

    /// Read stored information, which carries its version: it must be
    /// [`Self::VERSION`]. A key that is not this format's, or an answer not
    /// of its key's kind, is refused, as is a count that is not a whole
    /// number (`2.0` is not one); a `null` is no answer, as an absent key
    /// is.
    pub fn read(information: &AuthoritySpecificInformation) -> Result<Self, InformationError> {
        Self::read_with(information, true)
    }

    /// Read information about to be saved, as [`Self::read`] does, except
    /// that its version may be absent, as it is on information not saved
    /// yet (the host stamps it after checking it).
    pub fn read_to_save(
        information: &AuthoritySpecificInformation,
    ) -> Result<Self, InformationError> {
        Self::read_with(information, false)
    }

    fn read_with(
        information: &AuthoritySpecificInformation,
        stored: bool,
    ) -> Result<Self, InformationError> {
        Self::read_answers(information, stored).map_err(InformationError::from_unreadable)
    }

    fn read_answers(
        information: &AuthoritySpecificInformation,
        stored: bool,
    ) -> Result<Self, Unreadable> {
        let answers = stored::answers(information, Self::VERSION, &KEYS, stored)?;
        let yes_no = |key| stored::answer(answers, key, Value::as_bool);
        Ok(Self {
            approved_fstd: yes_no(APPROVED_FSTD)?,
            route: stored::answer(answers, ROUTE, |value| {
                value.as_str().and_then(SupervisionRoute::parse)
            })?,
            receiving_flight_training: yes_no(RECEIVING_FLIGHT_TRAINING)?,
            azimuth_guidance_operation: yes_no(AZIMUTH_GUIDANCE_OPERATION)?,
            course_deviation_indicator_operation: yes_no(COURSE_DEVIATION_INDICATOR_OPERATION)?,
            dme_or_gnss_arrivals: stored::answer(answers, DME_OR_GNSS_ARRIVALS, Value::as_u64)?,
        })
    }

    /// The answers as they are stored, without a version: the host writes
    /// that ([`Questions::stamp`](logbook_shared_kernel::Questions::stamp)).
    /// An answer not given has no key.
    pub fn to_information(&self) -> AuthoritySpecificInformation {
        // In the order of `KEYS`.
        let values: [Option<Value>; KEYS.len()] = [
            self.approved_fstd.map(Value::from),
            self.route.map(|route| route.as_str().into()),
            self.receiving_flight_training.map(Value::from),
            self.azimuth_guidance_operation.map(Value::from),
            self.course_deviation_indicator_operation.map(Value::from),
            self.dme_or_gnss_arrivals.map(Value::from),
        ];
        stored::information(KEYS.into_iter().zip(values))
    }
}
