// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's rules for a pilot's logbook, through logbook-shared-kernel's
//! [`LicensingAuthority`], and CASA's information on an entry and on an
//! assessment.
//!
//! It says which entries count, what an aircraft type counts as, which
//! requirements it checks and the pilot's currency against them, CASA's own
//! totals, the questions CASA asks about an entry and an assessment, and
//! the problems with the answers.

mod assessment_information;
mod currency;
mod entry_information;
mod guards;
mod questions;
mod stored;
#[cfg(test)]
mod test_support;
mod totals;

pub use assessment_information::{
    AssessmentInformation, AssessmentInformationError, CheckOutcome, CheckPurpose, CheckType,
};
pub use entry_information::{EntryInformation, InformationError, SupervisionRoute};

use logbook_shared_kernel::{
    AircraftDescription, Assessment, AuthorityId, AuthoritySpecificInformation, Classification,
    CurrencyResult, DateRange, Entry, Figure, LicensingAuthority, Logbook, Problem, ProblemAt,
    ProblemKind, Questions, Record, Requirement, RequirementId, Subject,
};
use time::Date;

use crate::aircraft::{classify, AircraftClassRating};

/// CASA's rules, as the logbook calls them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Casa {
    id: AuthorityId,
}

impl Casa {
    pub fn new() -> Self {
        Self { id: casa() }
    }
}

/// `AU-CASA`.
pub(crate) fn casa() -> AuthorityId {
    AuthorityId::new("AU", "CASA").expect("AU is a State and CASA is not empty")
}

/// CASA's information on an entry, read as it is stored, without the
/// answers to questions that do not apply to the entry, so that no check
/// counts an answer to a question that no longer applies, however the record
/// was saved; `None` when the entry has none, or has information that cannot
/// be read as stored, those answers and all.
pub(crate) fn information(entry: &Entry) -> Option<EntryInformation> {
    let record = Record::Entry(entry);
    let stored = record.information_for(&casa())?;
    EntryInformation::read(stored).ok()?;
    let mut applicable = stored.clone();
    questions::ENTRY.retain_applicable_answers_for(record, &mut applicable);
    // Removing answers from information that reads leaves information that
    // reads.
    EntryInformation::read(&applicable).ok()
}

/// CASA's information on an assessment, read as it is stored; `None` when
/// the assessment has none, or has information that cannot be read.
pub(crate) fn assessment_information(assessment: &Assessment) -> Option<AssessmentInformation> {
    Record::Assessment(assessment)
        .information_for(&casa())
        .and_then(|information| AssessmentInformation::read(information).ok())
}

impl Default for Casa {
    fn default() -> Self {
        Self::new()
    }
}

impl LicensingAuthority for Casa {
    /// `AU-CASA`.
    fn id(&self) -> &AuthorityId {
        &self.id
    }

    /// A flight counts. A session counts only when CASA's information on it
    /// says it was in an "approved flight simulation training device" (reg
    /// 61.010); a session with no information, or with information that
    /// cannot be read, does not.
    fn counts(&self, entry: &Entry) -> bool {
        match entry.subject() {
            Subject::Flight { .. } => true,
            Subject::Session { .. } => information(entry)
                .is_some_and(|information| information.approved_fstd == Some(true)),
        }
    }

    /// The category (reg 61.015, or reg 61.007(2) for a registered
    /// sailplane), and the class (reg 61.020) where there is one, in Part
    /// 61's words.
    fn classify(&self, aircraft: &AircraftDescription) -> Option<Classification> {
        let (category, class) = classify(*aircraft);
        let mut counts_as = vec![category.to_string()];
        counts_as.extend(class.map(|class| class_name(class).to_owned()));
        Some(Classification {
            counts_as,
            cautions: Vec::new(),
        })
    }

    /// Seventeen: carrying a passenger by day and at night (reg 61.395(1),
    /// (2)), night VFR recency (reg 61.965), instrument recency (reg
    /// 61.870(2), (3) for each category of reg 61.015, (4) to (7), and reg
    /// 61.875), a flight review for each aeroplane class rating (reg
    /// 61.745) and the instrument proficiency check (reg 61.880). Each has a stable id and a name in its
    /// regulation's words.
    fn requirements(&self) -> Vec<Requirement> {
        currency::requirements()
    }

    /// Each of [`requirements`](LicensingAuthority::requirements)' results
    /// on `as_of`, in the same order, through the kernel's `run` and
    /// `run_assessments`.
    fn currency(&self, as_of: Date, logbook: &Logbook) -> Vec<(RequirementId, CurrencyResult)> {
        currency::currency(self, as_of, logbook)
    }

    /// Aeroplane time, single-engine and multi-engine aeroplane time,
    /// registered sailplane time, simulated flight time and solo flight
    /// time, of the entries that count. Solo flight time is the flights
    /// whose pilot was the sole occupant; no rule makes it count as pilot in
    /// command.
    fn totals(&self, logbook: &Logbook, range: DateRange) -> Vec<Figure> {
        totals::totals(self, logbook, range)
    }

    /// The questions for CASA's information on an entry, each key of
    /// [`EntryInformation`], or on an assessment, each key of
    /// [`AssessmentInformation`], at that format's version. Every entry is
    /// asked the same questions, each with the condition on which it
    /// applies, so that the host removes an answer to one that no longer
    /// does: whether the device was an approved flight simulation training
    /// device applies to a session, the two guidance operations for an entry with approaches,
    /// the route for a pilot in command under supervision, and whether the
    /// person was receiving flight training on the instructor's route. The
    /// DME or GNSS arrivals are asked whatever the flight rules, as reg
    /// 61.870 counts an instrument approach operation.
    fn questions_for(&self, record: Record) -> Questions {
        match record {
            Record::Entry(_) => questions::ENTRY.clone(),
            Record::Assessment(_) => questions::for_assessment(),
        }
    }

    /// A refusal at the information as a whole when it cannot be read
    /// ([`EntryInformation::read_to_save`],
    /// [`AssessmentInformation::read_to_save`]), so that the host does not
    /// stamp it as this version and save it: one that is not this format's,
    /// or of another version. Information with no version is information not
    /// saved yet, which the host then stamps; stored information that has
    /// lost its version is refused where it is read
    /// ([`EntryInformation::read`]), and so counts for nothing.
    ///
    /// And a refusal at `receiving_flight_training` when its answer is yes
    /// where the question applies, on the route of reg 61.095(3) for a pilot
    /// in command under supervision: "the person is not receiving flight
    /// training" (reg 61.095(3)(b)).
    fn problems_for(
        &self,
        record: Record,
        information: &AuthoritySpecificInformation,
    ) -> Vec<Problem> {
        match record {
            Record::Entry(_) => match EntryInformation::read_to_save(information) {
                Err(error) => vec![unreadable(error.to_string())],
                Ok(answers)
                    if answers.receiving_flight_training == Some(true)
                        && questions::ENTRY
                            .applicable_for(record, Some(information))
                            .iter()
                            .any(|question| {
                                question.key == entry_information::RECEIVING_FLIGHT_TRAINING
                            }) =>
                {
                    vec![Problem {
                        at: ProblemAt::Question(entry_information::RECEIVING_FLIGHT_TRAINING),
                        kind: ProblemKind::Refusal,
                        text: "Supervised by a flight instructor or flight examiner, the flight \
                               is time as pilot in command under supervision only if \"the \
                               person is not receiving flight training\""
                            .into(),
                        regulation: "61.095(3)(b)",
                    }]
                }
                Ok(_) => Vec::new(),
            },
            Record::Assessment(_) => AssessmentInformation::read_to_save(information)
                .err()
                .map(|error| unreadable(error.to_string()))
                .into_iter()
                .collect(),
        }
    }
}

/// A refusal of information that cannot be read, at the information as a
/// whole.
fn unreadable(text: String) -> Problem {
    Problem {
        at: ProblemAt::Information,
        kind: ProblemKind::Refusal,
        text,
        // A stored format's rule, not a regulation's.
        regulation: "",
    }
}

/// A class as reg 61.020(1) words it.
fn class_name(class: AircraftClassRating) -> &'static str {
    match class {
        AircraftClassRating::SingleEngineAeroplane => "single-engine aeroplane",
        AircraftClassRating::MultiEngineAeroplane => "multi-engine aeroplane",
        AircraftClassRating::SingleEngineHelicopter => "single-engine helicopter",
        AircraftClassRating::PoweredLiftAircraft => "powered-lift aircraft",
        AircraftClassRating::SingleEngineGyroplane => "single-engine gyroplane",
        AircraftClassRating::Airship => "airship",
    }
}
