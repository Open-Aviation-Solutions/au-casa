// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's rules for a pilot's logbook, through logbook-shared-kernel's
//! [`LicensingAuthority`], and CASA's information on an entry and on an
//! assessment.
//!
//! So far it says which entries count, what an aircraft type counts as,
//! which requirements it checks and the pilot's currency against them, and
//! CASA's own totals. It asks no questions yet; of the problems, it finds
//! only information it cannot read. The rest follow (au-casa#40).

mod assessment_information;
mod currency;
mod entry_information;
mod guards;
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

/// CASA's information on an entry, read as it is stored; `None` when the
/// entry has none, or has information that cannot be read.
pub(crate) fn information(entry: &Entry) -> Option<EntryInformation> {
    read_information(Record::Entry(entry), EntryInformation::read)
}

/// CASA's information on an assessment, read as it is stored; `None` when
/// the assessment has none, or has information that cannot be read.
pub(crate) fn assessment_information(assessment: &Assessment) -> Option<AssessmentInformation> {
    read_information(Record::Assessment(assessment), AssessmentInformation::read)
}

/// CASA's information on a record, read by `read`; `None` when there is
/// none, or it cannot be read.
fn read_information<T, E>(
    record: Record,
    read: fn(&AuthoritySpecificInformation) -> Result<T, E>,
) -> Option<T> {
    record
        .information_for(&casa())
        .and_then(|information| read(information).ok())
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

    /// Twelve: carrying a passenger by day and at night (reg 61.395(1),
    /// (2)), night VFR recency (reg 61.965), instrument recency (reg
    /// 61.870(2), (4) to (7), and reg 61.875), a flight review for each
    /// aeroplane class rating (reg 61.745) and the instrument proficiency
    /// check (reg 61.880). Each has a stable id and a name in its
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

    /// None yet, at the version of CASA's information on the kind of
    /// record: [`EntryInformation::VERSION`] or
    /// [`AssessmentInformation::VERSION`].
    fn questions_for(&self, record: Record) -> Questions {
        let version = match record {
            Record::Entry(_) => EntryInformation::VERSION,
            Record::Assessment(_) => AssessmentInformation::VERSION,
        };
        Questions::new(version, Vec::new())
    }

    /// So far, only that CASA's information on an entry or an assessment
    /// cannot be read ([`EntryInformation::read_to_save`],
    /// [`AssessmentInformation::read_to_save`]): a refusal, at the
    /// information as a whole, so that the host does not stamp it as this
    /// version and save it. Its answers are checked against each other later
    /// (au-casa#40).
    fn problems_for(
        &self,
        record: Record,
        information: &AuthoritySpecificInformation,
    ) -> Vec<Problem> {
        let unreadable = match record {
            Record::Entry(_) => EntryInformation::read_to_save(information)
                .err()
                .map(|error| error.to_string()),
            Record::Assessment(_) => AssessmentInformation::read_to_save(information)
                .err()
                .map(|error| error.to_string()),
        };
        unreadable
            .map(|text| Problem {
                at: ProblemAt::Information,
                kind: ProblemKind::Refusal,
                text,
                // A stored format's rule, not a regulation's.
                regulation: "",
            })
            .into_iter()
            .collect()
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
