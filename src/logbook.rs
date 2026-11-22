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
//!
//! CASA's information on an entry and on an assessment is not exported: a
//! host reads it through [`LicensingAuthority`], and these keep it so. The
//! migration of stored information on an entry
//! ([`migrate_entry_information_from_version_1`]) takes and gives the
//! kernel's [`AuthoritySpecificInformation`].
//!
//! ```compile_fail
//! use au_casa::logbook::EntryInformation;
//! ```
//!
//! ```compile_fail
//! use au_casa::logbook::AssessmentInformation;
//! ```

mod assessment_information;
mod currency;
mod entry_information;
mod guards;
mod licence_titles;
mod questions;
mod stored;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
mod totals;

pub(crate) use assessment_information::{
    AssessedPurpose, AssessmentInformation, CheckOutcome, CheckPurpose, CheckType, PurposeKind,
};
pub use entry_information::EntryMigrationError;
pub(crate) use entry_information::{EntryInformation, SupervisionRoute};

use std::collections::BTreeSet;

use icao_shared_kernel::MinimumAges;
use logbook_shared_kernel::{
    AircraftCharacteristic, AircraftDescription, Assessment, AuthorityId,
    AuthoritySpecificInformation, AuthorityTotal, Classification, CurrencyResult, DateRange, Entry,
    LicenceTitleTerm, LicensingAuthority, Logbook, MigrationRefused, Problem, ProblemAt,
    ProblemKind, Questions, Record, Requirement, RequirementId, Subject, Term, TermId,
};
use time::Date;

use crate::aircraft::{
    classify, design_features, lacks_water_alighting_gear, AircraftCategory, AircraftClassRating,
};

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

/// CASA's information on an assessment, read as it is stored, without the
/// answers to questions that do not apply to it (a flight test's own
/// outcome, another check's purposes), as for an entry; `None` when the
/// assessment has none, or has information that cannot be read as stored.
pub(crate) fn assessment_information(assessment: &Assessment) -> Option<AssessmentInformation> {
    let stored = Record::Assessment(assessment).information_for(&casa())?;
    AssessmentInformation::read(stored).ok()?;
    // Removing answers from information that reads leaves information that
    // reads.
    AssessmentInformation::read(&assessment_information::applicable(stored, assessment)).ok()
}

/// Rewrite one entry's stored CASA information of version 1 as version 2,
/// for the migration of a host's stored records: restamped as version 2,
/// with every answer kept, since version 2 only adds `cross_country_flight`,
/// which no version 1 information holds. Information of version 2 already
/// is given back as it is, once it reads, so the migration can be run
/// again. Nothing in it needs guessing, so it refuses only information
/// that reads as neither version.
///
/// Superseded by [`LicensingAuthority::migrate_information`] on [`Casa`],
/// which migrates this and says why it refuses by kind. It stays public in
/// this release for hosts that call it, and goes in au-casa's next breaking
/// release.
pub fn migrate_entry_information_from_version_1(
    information: &AuthoritySpecificInformation,
) -> Result<AuthoritySpecificInformation, EntryMigrationError> {
    entry_information::migrate_from_version_1(information)
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
    /// sailplane), and the class (reg 61.020) where there is one, in that
    /// order; and the design features that require a design feature
    /// endorsement (reg 61.755), in its order for the category. Each is in
    /// Part 61's words, with a stable id.
    ///
    /// A multi-engine aeroplane with centre-line thrust recorded is in the
    /// single-engine class (reg 61.020(2)(a)). The caller passes the
    /// characteristics: a flight's, or, for a session, the
    /// `Simulated::characteristics` of the configuration its device
    /// represented. Gas turbine engine is read from the description; every
    /// other design feature from the characteristic recorded for it. A seaplane or amphibian with neither
    /// float alighting gear nor a floating hull recorded is cautioned to
    /// record the one it has, and nothing is required for either: nothing
    /// is implied from the description.
    fn classify(
        &self,
        aircraft: &AircraftDescription,
        characteristics: &BTreeSet<AircraftCharacteristic>,
    ) -> Option<Classification> {
        let (category, class) = classify(*aircraft, characteristics);
        let mut counts_as = vec![term(category_id(category), category.to_string())];
        counts_as.extend(class.map(|class| term(class_id(class), class_name(class))));
        let requires = design_features(category, *aircraft, characteristics)
            .into_iter()
            .map(|feature| term(feature.id(), feature.name()))
            .collect();
        let cautions = lacks_water_alighting_gear(*aircraft, characteristics)
            .then(|| {
                "It operates on water: record whether it has float alighting gear or a \
                 floating hull, as each requires its own design feature endorsement \
                 (reg 61.755(2)(g), (h))"
                    .to_owned()
            })
            .into_iter()
            .collect();
        Some(Classification {
            counts_as,
            requires,
            cautions,
        })
    }

    /// Annex 1's titles CASA issues, each under its Part 61 name (every one
    /// but the free balloon licence, which CASA does not issue), and then
    /// the five national titles, each with its licence's minimum age (regs
    /// 61.475(1), 61.515(1) and 61.580(1)). Each has a stable id.
    fn licence_titles(&self) -> Vec<LicenceTitleTerm> {
        licence_titles::licence_titles()
    }

    /// Annex 1's, as CASA's are the same (regs 61.515(1), 61.580(1),
    /// 61.655(1), 61.700(1) and 61.1540(1)).
    fn minimum_ages(&self) -> MinimumAges {
        licence_titles::minimum_ages()
    }

    /// Twenty-six: carrying a passenger by day and at night (reg 61.395(1),
    /// (2)) and night VFR recency (reg 61.965(a)), each a requirement for
    /// its take-offs and one for its landings, instrument recency (reg
    /// 61.870(2), (3) for each category of reg 61.015, (4) to (7), and reg
    /// 61.875), a flight review for each aeroplane class rating (reg
    /// 61.745) and an instrument proficiency check for each category and for
    /// multi-engine aeroplanes and helicopters (reg 61.880). Each has a
    /// stable id and a name in its regulation's words.
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
    /// registered sailplane time, simulated flight time, solo flight time
    /// and cross-country flight time, of the entries that count. Solo flight
    /// time is the flights whose pilot was the sole occupant; no rule makes
    /// it count as pilot in command. Cross-country flight time is the
    /// flights CASA's information answers were cross-country flights (reg
    /// 61.010), whatever the capacity; an unanswered flight is not one.
    ///
    /// Each has a stable id. Aeroplane time's is the aeroplane category's,
    /// and each class's is its class's, as `classify` gives them; the two
    /// classes are part of aeroplane time, which comes before them.
    fn totals(&self, logbook: &Logbook, range: DateRange) -> Vec<AuthorityTotal> {
        totals::totals(self, logbook, range)
    }

    /// The questions for CASA's information on an entry, each key of
    /// `EntryInformation`, or on an assessment, each key of
    /// `AssessmentInformation`, at that format's version. Every entry is
    /// asked the same questions, each with the condition on which it
    /// applies, so that the host removes an answer to one that no longer
    /// does: whether the device was an approved flight simulation training
    /// device applies to a session, whether it was a cross-country flight
    /// to a flight, the two guidance operations to an entry
    /// with an approach or a DME or GNSS arrival, the route to a pilot in
    /// command under supervision, and whether the person was receiving
    /// flight training on the instructor's route. The DME or GNSS arrivals
    /// question excludes operations also recorded in the entry's approaches,
    /// as both count towards reg 61.870(2) and a count cannot be
    /// de-duplicated against them, so do not reword it back. They
    /// are asked whatever the flight rules, as reg 61.870 counts an
    /// instrument approach operation, and before the guidance operations,
    /// whose condition reads their answer.
    fn questions_for(&self, record: Record) -> Questions {
        match record {
            Record::Entry(_) => questions::ENTRY.clone(),
            Record::Assessment(_) => questions::ASSESSMENT.clone(),
        }
    }

    /// An entry's information of version 1 is rewritten as version 2
    /// ([`migrate_entry_information_from_version_1`]), and refused as not
    /// readable when it does not read as version 1. Information of the
    /// current version, 2, of an entry or of an assessment, gives `Ok(None)`
    /// on its version alone, without its answers read: that is not a check of
    /// them, which is `problems_for`'s. No older version of an assessment is
    /// kept, so one is refused as too old. A version newer than the current
    /// is refused as newer; information with no version or that is not a
    /// JSON object, as not readable.
    fn migrate_information(
        &self,
        record: Record,
        information: &AuthoritySpecificInformation,
    ) -> Result<Option<AuthoritySpecificInformation>, MigrationRefused> {
        let current = self.questions_for(record).version();
        let too_old = |version: u32| {
            MigrationRefused::TooOld(format!(
                "the information is of version {version}, which these rules have no migration from, to version {current}"
            ))
        };
        match (information.version(), record) {
            (None, _) => Err(MigrationRefused::Unreadable(format!(
                "the information holds no version: a JSON object with \"{}\" a whole number is expected",
                AuthoritySpecificInformation::VERSION_KEY
            ))),
            (Some(version), _) if version > current => Err(MigrationRefused::Newer(format!(
                "the information is of version {version}, newer than version {current}, which these rules write"
            ))),
            (Some(version), Record::Assessment(_)) if version == current => Ok(None),
            (Some(version), Record::Assessment(_)) => Err(too_old(version)),
            (Some(version), Record::Entry(_)) if version == current => Ok(None),
            (Some(0), Record::Entry(_)) => Err(too_old(0)),
            (Some(_), Record::Entry(_)) => {
                migrate_entry_information_from_version_1(information)
                    .map(Some)
                    .map_err(|error| MigrationRefused::Unreadable(error.to_string()))
            }
        }
    }

    /// A refusal at the information as a whole when it cannot be read
    /// (`EntryInformation::read_to_save`,
    /// `AssessmentInformation::read_to_save`), so that the host does not
    /// stamp it as this version and save it: one that is not this format's,
    /// or of another version. Information with no version is information not
    /// saved yet, which the host then stamps; stored information that has
    /// lost its version is refused where it is read
    /// (`EntryInformation::read`), and so counts for nothing.
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

/// One of CASA's terms.
fn term(id: &str, name: impl Into<String>) -> Term {
    Term {
        id: TermId::new(id),
        name: name.into(),
    }
}

/// A category's stable id, which a host may keep: its stored name, as a
/// requirement of reg 61.870(3) has it.
pub(crate) fn category_id(category: AircraftCategory) -> &'static str {
    match category {
        AircraftCategory::Aeroplane => "aeroplane",
        AircraftCategory::Helicopter => "helicopter",
        AircraftCategory::PoweredLift => "powered-lift",
        AircraftCategory::Gyroplane => "gyroplane",
        AircraftCategory::Airship => "airship",
        AircraftCategory::RegisteredSailplane => "registered-sailplane",
    }
}

/// A class's stable id, which a host may keep: its stored name, as a flight
/// review's requirement has it, but for the airship class, whose stored name
/// is the airship category's id.
pub(crate) fn class_id(class: AircraftClassRating) -> &'static str {
    match class {
        AircraftClassRating::SingleEngineAeroplane => "single-engine-aeroplane",
        AircraftClassRating::MultiEngineAeroplane => "multi-engine-aeroplane",
        AircraftClassRating::SingleEngineHelicopter => "single-engine-helicopter",
        AircraftClassRating::PoweredLiftAircraft => "powered-lift-aircraft",
        AircraftClassRating::SingleEngineGyroplane => "single-engine-gyroplane",
        AircraftClassRating::Airship => "airship-class",
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
