// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's rules for a pilot's logbook, through logbook-shared-kernel's
//! [`LicensingAuthority`], and CASA's information on an entry.
//!
//! So far it says which entries count and what an aircraft type counts as.
//! It asks no questions and checks no requirements yet, and has no totals
//! of its own; of the problems, it finds only information it cannot read.
//! Some of its currency checks are written, but `currency` does not run
//! them yet. The rest follow (au-casa#29).

// Run by `currency` once the requirements are named (au-casa#39).
#[cfg_attr(not(test), expect(dead_code))]
mod currency;
mod entry_information;
#[cfg_attr(not(test), expect(dead_code))]
mod guards;
#[cfg(test)]
mod test_support;

pub use entry_information::{EntryInformation, InformationError, SupervisionRoute};

use logbook_shared_kernel::{
    AircraftDescription, AuthorityId, AuthoritySpecificInformation, Classification, CurrencyResult,
    DateRange, Entry, Figure, LicensingAuthority, Logbook, Problem, ProblemAt, ProblemKind,
    Questions, Record, Requirement, RequirementId, Subject,
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
fn casa() -> AuthorityId {
    AuthorityId::new("AU", "CASA").expect("AU is a State and CASA is not empty")
}

/// CASA's information on an entry, read as it is stored; `None` when the
/// entry has none, or has information that cannot be read.
pub(crate) fn information(entry: &Entry) -> Option<EntryInformation> {
    Record::Entry(entry)
        .information_for(&casa())
        .and_then(|information| EntryInformation::read(information).ok())
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

    fn requirements(&self) -> Vec<Requirement> {
        Vec::new()
    }

    fn currency(&self, _as_of: Date, _logbook: &Logbook) -> Vec<(RequirementId, CurrencyResult)> {
        Vec::new()
    }

    fn totals(&self, _logbook: &Logbook, _range: DateRange) -> Vec<Figure> {
        Vec::new()
    }

    /// None yet, at version 1 of CASA's information on either kind of
    /// record.
    fn questions_for(&self, record: Record) -> Questions {
        let version = match record {
            Record::Entry(_) => EntryInformation::VERSION,
            Record::Assessment(_) => 1,
        };
        Questions::new(version, Vec::new())
    }

    /// So far, only that CASA's information on an entry cannot be read
    /// ([`EntryInformation::read_to_save`]): a refusal, at the information
    /// as a whole, so that the host does not stamp it as this version and
    /// save it. Its answers are checked against each other later
    /// (au-casa#40).
    fn problems_for(
        &self,
        record: Record,
        information: &AuthoritySpecificInformation,
    ) -> Vec<Problem> {
        match record {
            Record::Entry(_) => EntryInformation::read_to_save(information)
                .err()
                .map(|error| Problem {
                    at: ProblemAt::Information,
                    kind: ProblemKind::Refusal,
                    text: error.to_string(),
                    // A stored format's rule, not a regulation's.
                    regulation: "",
                })
                .into_iter()
                .collect(),
            Record::Assessment(_) => Vec::new(),
        }
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
