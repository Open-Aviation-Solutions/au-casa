// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's authority-specific information on an assessment, as it is stored.

use logbook_shared_kernel::{Assessment, AuthoritySpecificInformation, Record};
use serde_json::{Map, Value};
use thiserror::Error;

use super::questions;
use super::stored::{self, Unreadable};

/// CASA's information on an assessment: what kind of check it was, and its
/// outcome or, for a flight test, its purposes, each with its own outcome.
/// Every answer is optional here; which are asked, and which are required,
/// is for CASA's questions.
///
/// A stored format, versioned independently of
/// [`EntryInformation`](super::EntryInformation)'s, with every key and
/// spelling mapped by hand, as that one is. The host stores the version
/// beside the answers, under the kernel's reserved key
/// [`AuthoritySpecificInformation::VERSION_KEY`] (`_version`). Every stored
/// value is an identifier, and its wording is its choice's label, so a
/// stored value does not change when its wording does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssessmentInformation {
    /// `check_type`: which of the checks of regs 61.745(3) and (3A) it was.
    pub check_type: Option<CheckType>,
    /// `outcome`: whether the pilot passed it. A flight test has none of its
    /// own, only its purposes' outcomes: the question does not apply to
    /// one, so the host removes such an answer on a save, and the checks
    /// read none ([`applicable`]).
    pub outcome: Option<CheckOutcome>,
    /// `check_purposes`: what a flight test was for, each purpose with its
    /// outcome; it applies to a flight test only, as `outcome` does not.
    /// Empty is no answer.
    pub check_purposes: Vec<AssessedPurpose>,
}

/// The checks that can be recorded against a flight, as regs 61.745(3) and
/// (3A) name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckType {
    /// `flight_test` (reg 61.745(3)(a), (b)).
    FlightTest,
    /// `flight_review` (reg 61.745(1); and reg 61.745(3)(d), "a flight
    /// review for a pilot type rating").
    FlightReview,
    /// `operator_proficiency_check` (reg 61.745(3)(e)(i)).
    OperatorProficiencyCheck,
    /// `instrument_proficiency_check` (reg 61.745(3A)(a)).
    InstrumentProficiencyCheck,
    /// `night_vision_imaging_system_proficiency_check` (reg 61.745(3A)(b)).
    NightVisionImagingSystemProficiencyCheck,
    /// `aerial_application_proficiency_check` (reg 61.745(3A)(c)).
    AerialApplicationProficiencyCheck,
    /// `instructor_proficiency_check` (reg 61.745(3A)(d)).
    InstructorProficiencyCheck,
    /// `examiner_proficiency_check` (reg 61.745(3A)(e)).
    ExaminerProficiencyCheck,
}

impl CheckType {
    /// Every check, in the order CASA's question offers them.
    pub const ALL: [Self; 8] = [
        Self::FlightTest,
        Self::FlightReview,
        Self::OperatorProficiencyCheck,
        Self::InstrumentProficiencyCheck,
        Self::NightVisionImagingSystemProficiencyCheck,
        Self::AerialApplicationProficiencyCheck,
        Self::InstructorProficiencyCheck,
        Self::ExaminerProficiencyCheck,
    ];

    /// Its stored spelling, and its name in regs 61.745(3) and (3A)'s words.
    fn spelling(self) -> (&'static str, &'static str) {
        match self {
            Self::FlightTest => ("flight_test", "flight test"),
            Self::FlightReview => ("flight_review", "flight review"),
            Self::OperatorProficiencyCheck => {
                ("operator_proficiency_check", "operator proficiency check")
            }
            Self::InstrumentProficiencyCheck => (
                "instrument_proficiency_check",
                "instrument proficiency check",
            ),
            Self::NightVisionImagingSystemProficiencyCheck => (
                "night_vision_imaging_system_proficiency_check",
                "night vision imaging system proficiency check",
            ),
            Self::AerialApplicationProficiencyCheck => (
                "aerial_application_proficiency_check",
                "aerial application proficiency check",
            ),
            Self::InstructorProficiencyCheck => (
                "instructor_proficiency_check",
                "instructor proficiency check",
            ),
            Self::ExaminerProficiencyCheck => {
                ("examiner_proficiency_check", "examiner proficiency check")
            }
        }
    }

    /// Its stored spelling.
    pub fn as_str(self) -> &'static str {
        self.spelling().0
    }

    /// Its name, in regs 61.745(3) and (3A)'s words.
    pub fn label(self) -> &'static str {
        self.spelling().1
    }

    /// The check a stored spelling names; `None` for any other.
    pub fn parse(spelling: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|check| check.as_str() == spelling)
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
    /// Its stored spelling, which is also its wording.
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

/// The kinds of grant a flight test can be for. The kind is stored with
/// each purpose, since the difference between a licence and a rating
/// matters to the pilot; class and operational ratings are kept apart, as
/// reg 61.745(3)(a) and (b) treat them differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PurposeKind {
    /// `licence`: a pilot licence (reg 61.010). The aircraft category
    /// rating granted with it is implied (reg 61.155(2)(b)), not listed.
    Licence,
    /// `category_rating`: an aircraft category rating for an existing pilot
    /// licence (reg 61.730(3)).
    CategoryRating,
    /// `class_rating`: an aircraft class rating (reg 61.750).
    ClassRating,
    /// `operational_rating`: an operational rating (reg 61.010).
    OperationalRating,
    /// `operational_endorsement`: an operational endorsement (reg 61.010),
    /// such as the instrument endorsement. Not any endorsement: reg
    /// 61.745(3)(c) has a design feature endorsement's flight training
    /// completed, not a flight test passed, so it is no flight test's
    /// purpose.
    OperationalEndorsement,
}

impl PurposeKind {
    /// Every kind, in the order CASA's question offers them.
    pub const ALL: [Self; 5] = [
        Self::Licence,
        Self::CategoryRating,
        Self::ClassRating,
        Self::OperationalRating,
        Self::OperationalEndorsement,
    ];

    /// Its stored spelling, and its name in Part 61's words.
    fn spelling(self) -> (&'static str, &'static str) {
        match self {
            Self::Licence => ("licence", "pilot licence"),
            Self::CategoryRating => ("category_rating", "aircraft category rating"),
            Self::ClassRating => ("class_rating", "aircraft class rating"),
            Self::OperationalRating => ("operational_rating", "operational rating"),
            Self::OperationalEndorsement => ("operational_endorsement", "operational endorsement"),
        }
    }

    /// Its stored spelling.
    pub fn as_str(self) -> &'static str {
        self.spelling().0
    }

    /// Its name, in Part 61's words.
    pub fn label(self) -> &'static str {
        self.spelling().1
    }
}

/// What a flight test ("a test ... for the licence, rating or endorsement",
/// reg 61.010) was for. Type ratings, and endorsements other than the
/// instrument endorsement, are left out until they are needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckPurpose {
    /// The pilot licences, in reg 61.010's order.
    AirTransportPilotLicence,
    CommercialPilotLicence,
    MultiCrewPilotLicence,
    PrivatePilotLicence,
    RecreationalPilotLicence,
    /// The category ratings, one for each category of reg 61.015, in its
    /// order.
    AeroplaneCategoryRating,
    HelicopterCategoryRating,
    PoweredLiftAircraftCategoryRating,
    GyroplaneCategoryRating,
    AirshipCategoryRating,
    /// The aircraft class ratings of Division 61.L.3, one for each class of
    /// reg 61.020(1)
    /// ([`AircraftClassRating`](crate::aircraft::AircraftClassRating)), in
    /// its order, each named as Part 61 names one ("single-engine aeroplane
    /// class rating", reg 61.375(4)).
    SingleEngineAeroplaneClassRating,
    MultiEngineAeroplaneClassRating,
    SingleEngineHelicopterClassRating,
    PoweredLiftAircraftClassRating,
    SingleEngineGyroplaneClassRating,
    AirshipClassRating,
    /// The operational ratings, as reg 61.010 defines them, in its order.
    AerialApplicationRating,
    ExaminerRating,
    InstructorRating,
    /// "The flight test for the instrument rating" (reg 61.880(3)(a)).
    InstrumentRating,
    LowLevelRating,
    NightVfrRating,
    NightVisionImagingSystemRating,
    PrivateInstrumentRating,
    /// "The flight test for an instrument endorsement" (reg 61.880(3)(b)),
    /// one of the kinds of reg 61.890.
    InstrumentEndorsement,
}

impl CheckPurpose {
    /// Every purpose, kind by kind in the order of [`PurposeKind::ALL`], and
    /// within a kind in its regulation's order: the order CASA's question
    /// offers them in, and they are stored in. The stored order is part of
    /// the format (version 2), as the kernel's `Answer::Choices` accepts
    /// only that order, so reordering this list is a breaking change to the
    /// format, with a new version and a migration.
    pub const ALL: [Self; 25] = [
        Self::AirTransportPilotLicence,
        Self::CommercialPilotLicence,
        Self::MultiCrewPilotLicence,
        Self::PrivatePilotLicence,
        Self::RecreationalPilotLicence,
        Self::AeroplaneCategoryRating,
        Self::HelicopterCategoryRating,
        Self::PoweredLiftAircraftCategoryRating,
        Self::GyroplaneCategoryRating,
        Self::AirshipCategoryRating,
        Self::SingleEngineAeroplaneClassRating,
        Self::MultiEngineAeroplaneClassRating,
        Self::SingleEngineHelicopterClassRating,
        Self::PoweredLiftAircraftClassRating,
        Self::SingleEngineGyroplaneClassRating,
        Self::AirshipClassRating,
        Self::AerialApplicationRating,
        Self::ExaminerRating,
        Self::InstructorRating,
        Self::InstrumentRating,
        Self::LowLevelRating,
        Self::NightVfrRating,
        Self::NightVisionImagingSystemRating,
        Self::PrivateInstrumentRating,
        Self::InstrumentEndorsement,
    ];

    /// Its kind, its stored spelling, its name in Part 61's words, and the
    /// regulation that names it.
    fn spelling(self) -> (PurposeKind, &'static str, &'static str, &'static str) {
        use PurposeKind::*;
        // Reg 61.010 defines the pilot licences and the operational
        // ratings; reg 61.730(3) grants a category rating for an existing
        // licence, and reg 61.750 a class rating; reg 61.890 sets out the
        // kinds of instrument endorsement.
        match self {
            Self::RecreationalPilotLicence => (
                Licence,
                "recreational_pilot_licence",
                "recreational pilot licence",
                "61.010",
            ),
            Self::PrivatePilotLicence => (
                Licence,
                "private_pilot_licence",
                "private pilot licence",
                "61.010",
            ),
            Self::CommercialPilotLicence => (
                Licence,
                "commercial_pilot_licence",
                "commercial pilot licence",
                "61.010",
            ),
            Self::MultiCrewPilotLicence => (
                Licence,
                "multi_crew_pilot_licence",
                "multi-crew pilot licence",
                "61.010",
            ),
            Self::AirTransportPilotLicence => (
                Licence,
                "air_transport_pilot_licence",
                "air transport pilot licence",
                "61.010",
            ),
            Self::AeroplaneCategoryRating => (
                CategoryRating,
                "aeroplane_category_rating",
                "aeroplane category rating",
                "61.730(3)",
            ),
            Self::HelicopterCategoryRating => (
                CategoryRating,
                "helicopter_category_rating",
                "helicopter category rating",
                "61.730(3)",
            ),
            Self::PoweredLiftAircraftCategoryRating => (
                CategoryRating,
                "powered_lift_aircraft_category_rating",
                "powered-lift aircraft category rating",
                "61.730(3)",
            ),
            Self::GyroplaneCategoryRating => (
                CategoryRating,
                "gyroplane_category_rating",
                "gyroplane category rating",
                "61.730(3)",
            ),
            Self::AirshipCategoryRating => (
                CategoryRating,
                "airship_category_rating",
                "airship category rating",
                "61.730(3)",
            ),
            Self::SingleEngineAeroplaneClassRating => (
                ClassRating,
                "single_engine_aeroplane_class_rating",
                "single-engine aeroplane class rating",
                "61.750",
            ),
            Self::MultiEngineAeroplaneClassRating => (
                ClassRating,
                "multi_engine_aeroplane_class_rating",
                "multi-engine aeroplane class rating",
                "61.750",
            ),
            Self::SingleEngineHelicopterClassRating => (
                ClassRating,
                "single_engine_helicopter_class_rating",
                "single-engine helicopter class rating",
                "61.750",
            ),
            Self::PoweredLiftAircraftClassRating => (
                ClassRating,
                "powered_lift_aircraft_class_rating",
                "powered-lift aircraft class rating",
                "61.750",
            ),
            Self::SingleEngineGyroplaneClassRating => (
                ClassRating,
                "single_engine_gyroplane_class_rating",
                "single-engine gyroplane class rating",
                "61.750",
            ),
            Self::AirshipClassRating => (
                ClassRating,
                "airship_class_rating",
                "airship class rating",
                "61.750",
            ),
            Self::AerialApplicationRating => (
                OperationalRating,
                "aerial_application_rating",
                "aerial application rating",
                "61.010",
            ),
            Self::ExaminerRating => (
                OperationalRating,
                "examiner_rating",
                "examiner rating",
                "61.010",
            ),
            Self::InstructorRating => (
                OperationalRating,
                "instructor_rating",
                "instructor rating",
                "61.010",
            ),
            Self::InstrumentRating => (
                OperationalRating,
                "instrument_rating",
                "instrument rating",
                "61.010",
            ),
            Self::LowLevelRating => (
                OperationalRating,
                "low_level_rating",
                "low-level rating",
                "61.010",
            ),
            Self::NightVfrRating => (
                OperationalRating,
                "night_vfr_rating",
                "night VFR rating",
                "61.010",
            ),
            Self::NightVisionImagingSystemRating => (
                OperationalRating,
                "night_vision_imaging_system_rating",
                "night vision imaging system rating",
                "61.010",
            ),
            Self::PrivateInstrumentRating => (
                OperationalRating,
                "private_instrument_rating",
                "private instrument rating",
                "61.010",
            ),
            Self::InstrumentEndorsement => (
                OperationalEndorsement,
                "instrument_endorsement",
                "instrument endorsement",
                "61.890",
            ),
        }
    }

    /// Its kind.
    pub fn kind(self) -> PurposeKind {
        self.spelling().0
    }

    /// Its stored spelling.
    pub fn as_str(self) -> &'static str {
        self.spelling().1
    }

    /// Its name, in Part 61's words.
    pub fn label(self) -> &'static str {
        self.spelling().2
    }

    /// The regulation that names it.
    pub fn regulation(self) -> &'static str {
        self.spelling().3
    }

    /// Where in [`Self::ALL`] the purpose a stored kind and spelling name
    /// is.
    fn position(kind: &str, spelling: &str) -> Option<usize> {
        Self::ALL
            .iter()
            .position(|purpose| purpose.kind().as_str() == kind && purpose.as_str() == spelling)
    }
}

/// One purpose of a flight test, with its own outcome: one flight test can
/// be for more than one grant (regs 61.155(2)(b) and 61.175), and passed
/// for one and failed for another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssessedPurpose {
    pub purpose: CheckPurpose,
    pub outcome: CheckOutcome,
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

impl AssessmentInformationError {
    fn from_unreadable(unreadable: Unreadable) -> Self {
        match unreadable {
            Unreadable::NotAnObject => Self::NotAnObject,
            Unreadable::UnsupportedVersion => Self::UnsupportedVersion,
            Unreadable::UnknownKey(key) => Self::UnknownKey(key),
            Unreadable::InvalidAnswer(key) => Self::InvalidAnswer(key),
        }
    }
}

pub(super) const CHECK_TYPE: &str = "check_type";
pub(super) const OUTCOME: &str = "outcome";
pub(super) const CHECK_PURPOSES: &str = "check_purposes";

/// Every key of the format, in the order
/// [`AssessmentInformation::to_information`] writes them.
const KEYS: [&str; 3] = [CHECK_TYPE, OUTCOME, CHECK_PURPOSES];

impl AssessmentInformation {
    /// The version of this format.
    pub const VERSION: u32 = 2;

    /// Read stored information, which carries its version: it must be
    /// [`Self::VERSION`]. A key that is not this format's, or a spelling
    /// that is not one of its key's, is refused; a `null` is no answer, as
    /// an absent key is. The purposes are read as the kernel's
    /// `Answer::Choices` accepts them: a list of at least one
    /// `{"kind", "value", "answer"}`, with no other key, in the order of
    /// [`CheckPurpose::ALL`] and none listed twice. An empty list is
    /// refused, as no purposes are stored as no key. An answer to a question
    /// that does not apply (a flight test's own outcome, another check's
    /// purposes) is read here as stored; the checks read the information
    /// without those answers ([`applicable`]), as the host removes them on
    /// a save.
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
        Self::read_answers(information, stored).map_err(AssessmentInformationError::from_unreadable)
    }

    fn read_answers(
        information: &AuthoritySpecificInformation,
        stored: bool,
    ) -> Result<Self, Unreadable> {
        let answers = stored::answers(information, Self::VERSION, &KEYS, stored)?;
        Ok(Self {
            check_type: choice(answers, CHECK_TYPE, CheckType::parse)?,
            outcome: choice(answers, OUTCOME, CheckOutcome::parse)?,
            check_purposes: stored::answer(answers, CHECK_PURPOSES, purposes)?.unwrap_or_default(),
        })
    }

    /// The answers as they are stored, without a version: the host writes
    /// that ([`Questions::stamp`](logbook_shared_kernel::Questions::stamp)).
    /// An answer not given has no key, and no purposes are none given. The
    /// purposes are written in the order of [`CheckPurpose::ALL`], whatever
    /// order they are held in, and one held twice is written once, with its
    /// first outcome, so that what is written reads.
    #[cfg(test)]
    pub fn to_information(&self) -> AuthoritySpecificInformation {
        let purposes: Vec<Value> = CheckPurpose::ALL
            .into_iter()
            .filter_map(|purpose| {
                let assessed = self
                    .check_purposes
                    .iter()
                    .find(|assessed| assessed.purpose == purpose)?;
                Some(serde_json::json!({
                    "kind": purpose.kind().as_str(),
                    "value": purpose.as_str(),
                    "answer": assessed.outcome.as_str(),
                }))
            })
            .collect();
        // In the order of `KEYS`.
        let values: [Option<Value>; KEYS.len()] = [
            self.check_type.map(|check| check.as_str().into()),
            self.outcome.map(|outcome| outcome.as_str().into()),
            (!purposes.is_empty()).then(|| purposes.into()),
        ];
        stored::information(KEYS.into_iter().zip(values))
    }
}

/// The choice under `key`, read by `parse`, which gives `None` for a
/// spelling that is not one of the key's.
fn choice<T>(
    answers: &Map<String, Value>,
    key: &'static str,
    parse: fn(&str) -> Option<T>,
) -> Result<Option<T>, Unreadable> {
    stored::answer(answers, key, |value| value.as_str().and_then(parse))
}

/// The purposes a stored list holds; `None` when it is not a list of at
/// least one purpose, each with its outcome and no other key, in the order
/// of [`CheckPurpose::ALL`] with none twice.
fn purposes(value: &Value) -> Option<Vec<AssessedPurpose>> {
    let mut previous: Option<usize> = None;
    let mut purposes = Vec::new();
    for item in value.as_array()? {
        let item = item.as_object().filter(|item| item.len() == 3)?;
        let field = |key| item.get(key).and_then(Value::as_str);
        let position = CheckPurpose::position(field("kind")?, field("value")?)?;
        let purpose = CheckPurpose::ALL[position];
        let outcome = CheckOutcome::parse(field("answer")?)?;
        if previous.is_some_and(|previous| position <= previous) {
            return None;
        }
        previous = Some(position);
        purposes.push(AssessedPurpose { purpose, outcome });
    }
    (!purposes.is_empty()).then_some(purposes)
}

/// Stored information without the answers to questions that do not apply
/// to the assessment (a flight test's own outcome, another check's
/// purposes), as the host removes them on a save.
pub(crate) fn applicable(
    information: &AuthoritySpecificInformation,
    assessment: &Assessment,
) -> AuthoritySpecificInformation {
    let mut applicable = information.clone();
    questions::ASSESSMENT
        .retain_applicable_answers_for(Record::Assessment(assessment), &mut applicable);
    applicable
}
