// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's authority-specific information on a logbook entry, as it is
//! stored.

use logbook_shared_kernel::AuthoritySpecificInformation;
use serde_json::{Map, Value};
use thiserror::Error;

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
    /// `azimuth_guidance_operation`: whether one of the entry's approaches
    /// was an "azimuth guidance operation" (reg 61.010; reg 61.870(6)).
    pub azimuth_guidance_operation: Option<bool>,
    /// `course_deviation_indicator_operation`: whether one of the entry's
    /// approaches was a "course deviation indicator operation" (reg 61.010;
    /// reg 61.870(7)).
    pub course_deviation_indicator_operation: Option<bool>,
    /// `dme_or_gnss_arrivals`: how many instrument approach operations were
    /// conducted "using distance measuring equipment or a global navigation
    /// satellite system" (Table 61.935, item 20, "Approach – DME or GNSS
    /// arrival procedure"). Stored as a JSON integer.
    pub dme_or_gnss_arrivals: Option<u32>,
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

const APPROVED_FSTD: &str = "approved_fstd";
const ROUTE: &str = "route";
const RECEIVING_FLIGHT_TRAINING: &str = "receiving_flight_training";
const AZIMUTH_GUIDANCE_OPERATION: &str = "azimuth_guidance_operation";
const COURSE_DEVIATION_INDICATOR_OPERATION: &str = "course_deviation_indicator_operation";
const DME_OR_GNSS_ARRIVALS: &str = "dme_or_gnss_arrivals";

impl EntryInformation {
    /// The version of this format.
    pub const VERSION: u32 = 1;

    /// Read stored information. Its version must be [`Self::VERSION`], or
    /// absent, as it is on information not saved yet. A key that is not
    /// this format's, or an answer not of its key's kind, is refused, as is
    /// a count that is not a whole number (`2.0` is not one).
    pub fn read(information: &AuthoritySpecificInformation) -> Result<Self, InformationError> {
        let answers = information
            .value()
            .as_object()
            .ok_or(InformationError::NotAnObject)?;
        if answers.contains_key(AuthoritySpecificInformation::VERSION_KEY)
            && information.version() != Some(Self::VERSION)
        {
            return Err(InformationError::UnsupportedVersion);
        }
        if let Some(key) = answers.keys().find(|key| {
            ![
                AuthoritySpecificInformation::VERSION_KEY,
                APPROVED_FSTD,
                ROUTE,
                RECEIVING_FLIGHT_TRAINING,
                AZIMUTH_GUIDANCE_OPERATION,
                COURSE_DEVIATION_INDICATOR_OPERATION,
                DME_OR_GNSS_ARRIVALS,
            ]
            .contains(&key.as_str())
        }) {
            return Err(InformationError::UnknownKey(key.clone()));
        }
        let yes_no = |key| read(answers, key, Value::as_bool);
        Ok(Self {
            approved_fstd: yes_no(APPROVED_FSTD)?,
            route: read(answers, ROUTE, |value| {
                value.as_str().and_then(SupervisionRoute::parse)
            })?,
            receiving_flight_training: yes_no(RECEIVING_FLIGHT_TRAINING)?,
            azimuth_guidance_operation: yes_no(AZIMUTH_GUIDANCE_OPERATION)?,
            course_deviation_indicator_operation: yes_no(COURSE_DEVIATION_INDICATOR_OPERATION)?,
            dme_or_gnss_arrivals: read(answers, DME_OR_GNSS_ARRIVALS, |value| {
                value.as_u64().and_then(|count| u32::try_from(count).ok())
            })?,
        })
    }

    /// The answers as they are stored, without a version: the host writes
    /// that ([`Questions::stamp`](logbook_shared_kernel::Questions::stamp)).
    /// An answer not given has no key.
    pub fn to_information(&self) -> AuthoritySpecificInformation {
        let mut answers = Map::new();
        let mut write = |key: &str, value: Option<Value>| {
            if let Some(value) = value {
                answers.insert(key.into(), value);
            }
        };
        write(APPROVED_FSTD, self.approved_fstd.map(Value::from));
        write(ROUTE, self.route.map(|route| route.as_str().into()));
        write(
            RECEIVING_FLIGHT_TRAINING,
            self.receiving_flight_training.map(Value::from),
        );
        write(
            AZIMUTH_GUIDANCE_OPERATION,
            self.azimuth_guidance_operation.map(Value::from),
        );
        write(
            COURSE_DEVIATION_INDICATOR_OPERATION,
            self.course_deviation_indicator_operation.map(Value::from),
        );
        write(
            DME_OR_GNSS_ARRIVALS,
            self.dme_or_gnss_arrivals.map(Value::from),
        );
        AuthoritySpecificInformation::new(Value::Object(answers))
    }
}

/// The answer under `key`, if there is one, read by `answer`, which gives
/// `None` for a value that is not an answer of the key's kind.
fn read<T>(
    answers: &Map<String, Value>,
    key: &'static str,
    answer: impl Fn(&Value) -> Option<T>,
) -> Result<Option<T>, InformationError> {
    answers
        .get(key)
        .map(|value| answer(value).ok_or(InformationError::InvalidAnswer(key)))
        .transpose()
}
