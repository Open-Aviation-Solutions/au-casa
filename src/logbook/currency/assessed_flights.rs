// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The passed instrument flight tests and proficiency checks, with what was
//! flown on them, for the limits that turn on what a test or check included
//! (regs 61.860(2) and (3), 61.900(2)).

use std::collections::{BTreeMap, BTreeSet};

use icao_shared_kernel::Capacity;
use logbook_shared_kernel::authority::window::{add_months_clamped, months_on};
use logbook_shared_kernel::{
    Assessment, AssessmentAccumulator, AssessmentId, CurrencyResult, Entry, EntryId, Expiry,
    SubjectId,
};
use time::Date;

use super::instrument_approaches::is_3d;
use crate::logbook::{
    assessment_information, stored_assessment_information, AssessedPurpose, CheckOutcome,
    CheckPurpose, CheckType,
};

/// What a passed assessment was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Passed {
    /// A flight test passed for the instrument rating, "the flight test for
    /// the rating" (regs 61.860(2)(a) and (3)(a)), whatever else it was for.
    RatingTest,
    /// A flight test passed for an instrument endorsement, and not the
    /// rating.
    EndorsementTest,
    /// An instrument proficiency check.
    ProficiencyCheck,
    /// An operator proficiency check, which counts as an instrument
    /// proficiency check for regs 61.860(3)(b) and 61.900(2)(b) (regs
    /// 61.860(4), 61.900(3)); that it covered IFR operations, and its
    /// examiner, are not recorded, as for reg 61.880.
    OperatorCheck,
}

/// What its flight recorded, read from the entry it names.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Flown {
    /// An approach on the entry ended in a circling manoeuvre.
    pub(super) circling: bool,
    /// An approach on the entry was a 3D instrument approach operation.
    pub(super) three_d: bool,
    /// No entry for its flight was logged as co-pilot: a single-pilot
    /// operation, as reg 61.875's check reads one.
    pub(super) single_pilot: bool,
}

#[derive(Debug)]
pub(super) struct Pass {
    pub(super) id: AssessmentId,
    pub(super) on: Date,
    pub(super) passed: Passed,
    /// What was flown, `None` when its entry is not a flight among those
    /// that count: one in a flight simulation training device is
    /// pilot-logbook#278's.
    pub(super) flown: Option<Flown>,
}

/// The passed instrument flight tests and proficiency checks up to the
/// reference date, each with what its flight recorded. An assessment's
/// information is read with the entry it names, from every entry of the
/// logbook, or as stored when it names none, as for reg 61.880; what was
/// flown is read only from an entry that counts and records a flight. A
/// failed check is not kept: "the most recent instrument proficiency check"
/// is the latest passed one (decided on au-casa#116), and reg 61.880's check
/// already ends at a failure.
pub(super) struct AssessedFlights<'a> {
    pub(super) reference: Date,
    logbook: BTreeMap<&'a EntryId, &'a Entry>,
    /// The entries that count and record a flight, by id.
    flights: BTreeMap<EntryId, &'a Entry>,
    /// The flights an entry that counts was logged as co-pilot on.
    co_pilot: BTreeSet<SubjectId>,
    passes: Vec<Pass>,
}

impl<'a> AssessedFlights<'a> {
    /// Over `entries`, the entries [`Casa::counts`](crate::Casa) lets
    /// through, and every entry of the `logbook`, to read an assessment's
    /// information with its entry.
    pub(super) fn new(
        reference: Date,
        entries: impl IntoIterator<Item = &'a Entry>,
        logbook: &'a [Entry],
    ) -> Self {
        let mut flights = BTreeMap::new();
        let mut co_pilot = BTreeSet::new();
        for entry in entries {
            let subject = entry.subject().id();
            if !matches!(subject, SubjectId::Flight(_)) {
                continue;
            }
            if entry.capacity() == Capacity::CoPilot {
                co_pilot.insert(subject);
            }
            flights.insert(entry.id().clone(), entry);
        }
        Self {
            reference,
            logbook: logbook.iter().map(|entry| (entry.id(), entry)).collect(),
            flights,
            co_pilot,
            passes: Vec::new(),
        }
    }

    /// The passes, earliest first, the same day's in the order of their ids,
    /// so that nothing depends on the order assessments arrive in.
    fn sorted(&self) -> Vec<&Pass> {
        let mut passes: Vec<&Pass> = self.passes.iter().collect();
        passes.sort_by(|a, b| a.on.cmp(&b.on).then_with(|| a.id.cmp(&b.id)));
        passes
    }

    /// The passes `qualifies` accepts, earliest first.
    pub(super) fn passes(&self, qualifies: impl Fn(&Pass) -> bool) -> Vec<&Pass> {
        self.sorted()
            .into_iter()
            .filter(|pass| qualifies(pass))
            .collect()
    }

    /// "The holder's most recent instrument proficiency check": the latest
    /// passed instrument or operator proficiency check, wherever it was
    /// flown.
    pub(super) fn most_recent_check(&self) -> Option<&Pass> {
        self.sorted().into_iter().rev().find(|pass| {
            matches!(
                pass.passed,
                Passed::ProficiencyCheck | Passed::OperatorCheck
            )
        })
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

    fn flown(&self, entry: &EntryId) -> Option<Flown> {
        let entry = self.flights.get(entry)?;
        let approaches = entry.approaches();
        Some(Flown {
            circling: approaches.iter().any(|approach| approach.circled()),
            three_d: approaches.iter().any(is_3d),
            single_pilot: !self.co_pilot.contains(&entry.subject().id()),
        })
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
        let Some(information) = information else {
            return;
        };
        let passed = match information.check_type {
            Some(CheckType::FlightTest) => {
                // Each purpose has its own outcome.
                let passed = |purpose| {
                    information.check_purposes.contains(&AssessedPurpose {
                        purpose,
                        outcome: CheckOutcome::Passed,
                    })
                };
                if passed(CheckPurpose::InstrumentRating) {
                    Passed::RatingTest
                } else if passed(CheckPurpose::InstrumentEndorsement) {
                    Passed::EndorsementTest
                } else {
                    return;
                }
            }
            Some(CheckType::InstrumentProficiencyCheck)
                if information.outcome == Some(CheckOutcome::Passed) =>
            {
                Passed::ProficiencyCheck
            }
            Some(CheckType::OperatorProficiencyCheck)
                if information.outcome == Some(CheckOutcome::Passed) =>
            {
                Passed::OperatorCheck
            }
            _ => return,
        };
        let flown = self.flown(&assessment.entry);
        self.passes.push(Pass {
            id: assessment.id.clone(),
            on: assessment.date,
            passed,
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

/// For the checks' tests: a passed check of this type, or a flight test
/// passed for this purpose, in entry `entry`.
#[cfg(test)]
pub(super) fn passed(id: &str, on: Date, entry: &str, passed: Passed) -> Assessment {
    use crate::logbook::test_support::{assessed, flight_test};
    use crate::logbook::AssessmentInformation;
    let information = match passed {
        Passed::RatingTest => {
            flight_test(&[(CheckPurpose::InstrumentRating, CheckOutcome::Passed)])
        }
        Passed::EndorsementTest => {
            flight_test(&[(CheckPurpose::InstrumentEndorsement, CheckOutcome::Passed)])
        }
        Passed::ProficiencyCheck | Passed::OperatorCheck => AssessmentInformation {
            check_type: Some(if passed == Passed::ProficiencyCheck {
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
