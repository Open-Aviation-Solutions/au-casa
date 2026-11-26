// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The passed instrument flight tests and proficiency checks, with what was
//! flown on them, for the limits that turn on what a test or check included
//! (regs 61.860(2) and (3), 61.900(2)).

use std::collections::BTreeMap;

use icao_shared_kernel::Capacity;
use logbook_shared_kernel::authority::window::{add_months_clamped, months_on};
use logbook_shared_kernel::{
    Assessment, AssessmentAccumulator, AssessmentId, CurrencyResult, Entry, EntryId, Expiry,
    SubjectId,
};
use time::Date;

use super::instrument_approaches::is_3d;
pub(super) use super::instrument_proficiency_check::Basis;
use super::instrument_proficiency_check::{passed_basis, qualifies};
use crate::logbook::{assessment_information, stored_assessment_information};

/// What a flight recorded, over every entry for it that counts.
#[derive(Debug, Clone, Copy)]
pub(super) struct Flown {
    /// An approach on one of its entries ended in a circling manoeuvre.
    pub(super) circling: bool,
    /// An approach on one of its entries was a 3D instrument approach
    /// operation.
    pub(super) three_d: bool,
    /// No entry for it was logged as co-pilot: a single-pilot operation, as
    /// reg 61.875's check reads one, which stands in for reg 61.860(2)'s
    /// "single-pilot aircraft" (see `INSTRUCTIONS.md`).
    pub(super) single_pilot: bool,
}

#[derive(Debug)]
pub(super) struct Pass {
    pub(super) id: AssessmentId,
    pub(super) on: Date,
    /// What it was, of reg 61.880(3)'s kinds.
    pub(super) basis: Basis,
    /// What its flight recorded, `None` when the entry it names is not a
    /// flight among those that count: one in a flight simulation training
    /// device is pilot-logbook#278's.
    pub(super) flown: Option<Flown>,
}

/// The passed instrument flight tests and proficiency checks up to the
/// reference date, of reg 61.880(3)'s kinds, each with what its flight
/// recorded. An assessment's information is read with the entry it names,
/// from every entry of the logbook, or as stored when it names none, as for
/// reg 61.880; what was flown is read only from the entries that count and
/// record a flight. A failed check is not kept: "the most recent instrument
/// proficiency check" is the latest passed one (the owner's decision on
/// au-casa#116).
pub(super) struct AssessedFlights<'a> {
    pub(super) reference: Date,
    logbook: BTreeMap<&'a EntryId, &'a Entry>,
    /// The flight each entry that counts and records a flight is for.
    flights: BTreeMap<EntryId, SubjectId>,
    /// What each of those flights recorded.
    flown: BTreeMap<SubjectId, Flown>,
    /// The dates of the flight tests passed for the instrument rating, which
    /// decide whether an endorsement test counts as a proficiency check.
    rating_tests: Vec<Date>,
    passes: Vec<Pass>,
}

impl<'a> AssessedFlights<'a> {
    /// Over `entries`, the entries [`Casa::counts`](crate::Casa) lets
    /// through, and every entry of the `logbook`, to read an assessment's
    /// information with its entry. An entry dated after the reference has
    /// not happened yet, and is left out.
    pub(super) fn new(
        reference: Date,
        entries: impl IntoIterator<Item = &'a Entry>,
        logbook: &'a [Entry],
    ) -> Self {
        let mut flights = BTreeMap::new();
        let mut flown: BTreeMap<SubjectId, Flown> = BTreeMap::new();
        for entry in entries {
            let subject = entry.subject().id();
            if !matches!(subject, SubjectId::Flight(_)) || entry.date() > reference {
                continue;
            }
            let approaches = entry.approaches();
            let flight = flown.entry(subject).or_insert(Flown {
                circling: false,
                three_d: false,
                single_pilot: true,
            });
            flight.circling |= approaches.iter().any(|approach| approach.circled());
            flight.three_d |= approaches.iter().any(is_3d);
            flight.single_pilot &= entry.capacity() != Capacity::CoPilot;
            flights.insert(entry.id().clone(), subject);
        }
        Self {
            reference,
            logbook: logbook.iter().map(|entry| (entry.id(), entry)).collect(),
            flights,
            flown,
            rating_tests: Vec::new(),
            passes: Vec::new(),
        }
    }

    /// The passes `qualifies` accepts, earliest first, the same day's in the
    /// order of their ids, so that nothing depends on the order assessments
    /// arrive in.
    pub(super) fn passes(&self, qualifies: impl Fn(&Pass) -> bool) -> Vec<&Pass> {
        let mut passes: Vec<&Pass> = self.passes.iter().filter(|pass| qualifies(pass)).collect();
        passes.sort_by(|a, b| a.on.cmp(&b.on).then_with(|| a.id.cmp(&b.id)));
        passes
    }

    /// "The holder's most recent instrument proficiency check": of the
    /// passes reg 61.880's check counts as one (reg 61.880(3)(a), (b), (c)
    /// and (e); an operator proficiency check, regs 61.860(4) and 61.900(3)),
    /// wherever flown, the first that `included` on the latest day any was
    /// passed, as two on one day are taken together. `None` when none on
    /// that day did, or none was passed. One whose flight au-casa cannot
    /// read (a session, or an entry not in the logbook) included nothing,
    /// so it ends the most recent check's privilege until pilot-logbook#278.
    pub(super) fn most_recent_check(&self, included: impl Fn(&Flown) -> bool) -> Option<&Pass> {
        let checks = self.passes(|pass| qualifies(pass.basis, pass.on, &self.rating_tests));
        let latest = checks.last()?.on;
        checks
            .into_iter()
            .filter(|check| check.on == latest)
            .find(|check| check.flown.as_ref().is_some_and(&included))
    }

    /// Met for good by `pass`.
    pub(super) fn never(&self, pass: &Pass) -> CurrencyResult {
        CurrencyResult {
            expiry: Some(Expiry::Never),
            qualifying_assessments: vec![pass.id.clone()],
            found: 1,
            ..self.within(0, &[])
        }
    }

    /// Met while the latest of `passes` is "within the previous `months`
    /// months", calendar months to the day; those within them on the
    /// reference date are reported.
    pub(super) fn within(&self, months: u32, passes: &[&Pass]) -> CurrencyResult {
        let months = i32::try_from(months).unwrap_or(i32::MAX);
        let ends = |pass: &Pass| months_on(pass.on, months).unwrap_or(Date::MAX);
        let shown: Vec<AssessmentId> = passes
            .iter()
            .filter(|pass| ends(pass) > self.reference)
            .map(|pass| pass.id.clone())
            .collect();
        let look_back_days = add_months_clamped(self.reference, -months)
            .map_or(0, |from| (self.reference - from).whole_days());
        CurrencyResult {
            expiry: passes.last().map(|pass| Expiry::On(ends(pass))),
            reference: self.reference,
            qualifying: Vec::new(),
            found: shown.len(),
            qualifying_assessments: shown,
            required: 1,
            look_back_days,
        }
    }
}

impl AssessmentAccumulator for AssessedFlights<'_> {
    fn accept_assessment(&mut self, assessment: &Assessment) {
        // An assessment dated after the reference has not happened yet.
        if assessment.date > self.reference {
            return;
        }
        let information = match self.logbook.get(&assessment.entry) {
            Some(named) => assessment_information(assessment, named),
            None => stored_assessment_information(assessment),
        };
        let Some(basis) = information.as_ref().and_then(passed_basis) else {
            return;
        };
        if basis == Basis::RatingTest {
            self.rating_tests.push(assessment.date);
        }
        let flown = self
            .flights
            .get(&assessment.entry)
            .and_then(|flight| self.flown.get(flight))
            .copied();
        self.passes.push(Pass {
            id: assessment.id.clone(),
            on: assessment.date,
            basis,
            flown,
        });
    }
}

/// For the checks' tests: the result of `check` over `entries`, all of
/// which count, and `assessments`, on `reference`.
#[cfg(test)]
pub(super) fn checked(
    check: fn(&AssessedFlights) -> CurrencyResult,
    reference: Date,
    entries: &[Entry],
    assessments: &[Assessment],
) -> CurrencyResult {
    let mut assessed = AssessedFlights::new(reference, entries, entries);
    for assessment in assessments {
        assessed.accept_assessment(assessment);
    }
    check(&assessed)
}

/// For the checks' tests: a flight of its own for each entry id, so that
/// entries are for one flight only when a test says so.
#[cfg(test)]
pub(super) fn flight_of(id: &str) -> u128 {
    id.bytes().fold(1, |flight: u128, byte| {
        flight.wrapping_mul(257) + u128::from(byte)
    })
}

/// For the checks' tests: a passed check of this type, or a flight test
/// passed for this purpose, in entry `entry`.
#[cfg(test)]
pub(super) fn passed(id: &str, on: Date, entry: &str, basis: Basis) -> Assessment {
    use crate::logbook::test_support::{assessed, flight_test};
    use crate::logbook::{AssessmentInformation, CheckOutcome, CheckPurpose, CheckType};
    let information = match basis {
        Basis::RatingTest => flight_test(&[(CheckPurpose::InstrumentRating, CheckOutcome::Passed)]),
        Basis::EndorsementTest => {
            flight_test(&[(CheckPurpose::InstrumentEndorsement, CheckOutcome::Passed)])
        }
        Basis::ProficiencyCheck | Basis::OperatorCheck => AssessmentInformation {
            check_type: Some(if basis == Basis::ProficiencyCheck {
                CheckType::InstrumentProficiencyCheck
            } else {
                CheckType::OperatorProficiencyCheck
            }),
            outcome: Some(CheckOutcome::Passed),
            check_purposes: Vec::new(),
        },
    };
    assessed(id, on, entry, information)
}
