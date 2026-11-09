// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's questions, for its information on an entry and on an assessment:
//! one question for each key of the two formats.

use std::sync::LazyLock;

use icao_shared_kernel::Capacity;
use logbook_shared_kernel::{Answer, Choice, Condition, Question, Questions};

use super::assessment_information::{CHECK_PURPOSE, CHECK_TYPE, OUTCOME};
use super::entry_information::{
    APPROVED_FSTD, AZIMUTH_GUIDANCE_OPERATION, COURSE_DEVIATION_INDICATOR_OPERATION,
    DME_OR_GNSS_ARRIVALS, RECEIVING_FLIGHT_TRAINING, ROUTE,
};
use super::{
    AssessmentInformation, CheckOutcome, CheckPurpose, CheckType, EntryInformation,
    SupervisionRoute,
};

/// The questions about an entry, built once.
pub(super) static ENTRY: LazyLock<Questions> = LazyLock::new(for_entry);

/// The questions about an entry. Each is in the list whatever the entry,
/// and one that applies only to some entries has a kernel [`Condition`], so
/// that the host removes its answer when the entry no longer meets it:
/// whether the device was approved for a session, the guidance operations
/// for an entry with an approach or a DME or GNSS arrival
/// ([`guidance_applies`]), the route for a pilot in command under
/// supervision, and its follow-up on the route's answer. The DME or GNSS
/// arrivals come before the guidance operations, as a condition reads only
/// the answer to an earlier question.
fn for_entry() -> Questions {
    let questions = vec![
        Question {
            key: APPROVED_FSTD,
            label: "Was the device an approved flight simulation training device for the purpose?"
                .into(),
            regulation: "61.010",
            answer: Answer::YesNo,
            applies_when: Some(Condition::Session),
            required: true,
        },
        Question {
            key: ROUTE,
            label: "Which applies to the flight?".into(),
            regulation: "61.095(1)(c)",
            answer: Answer::Choice(vec![
                route(
                    SupervisionRoute::Operator,
                    "The flight is conducted by an operator that has training and checking \
                     responsibilities, and the pilot in command of the flight is authorised by \
                     the operator or the operator's Part 142 operator to conduct the supervision \
                     of the person",
                    "61.095(2)",
                ),
                route(
                    SupervisionRoute::Instructor,
                    "The person is supervised by a flight instructor or flight examiner",
                    "61.095(3)",
                ),
            ]),
            applies_when: Some(Condition::Capacity(
                Capacity::PilotInCommandUnderSupervision,
            )),
            required: true,
        },
        Question {
            key: RECEIVING_FLIGHT_TRAINING,
            label: "Was the person receiving flight training?".into(),
            regulation: "61.095(3)(b)",
            answer: Answer::YesNo,
            applies_when: Some(Condition::Answer {
                key: ROUTE,
                is: SupervisionRoute::Instructor.as_str().into(),
            }),
            required: true,
        },
        Question {
            key: DME_OR_GNSS_ARRIVALS,
            label: "How many instrument approach operations were conducted using distance \
                    measuring equipment or a global navigation satellite system (DME or GNSS \
                    arrival procedure)?"
                .into(),
            regulation: "Table 61.935, item 20",
            answer: Answer::Count,
            applies_when: None,
            required: false,
        },
        Question {
            key: AZIMUTH_GUIDANCE_OPERATION,
            label: "Was an instrument approach operation (an approach or a DME or GNSS \
                    arrival) an azimuth guidance operation (an instrument approach operation \
                    using azimuth bearings for lateral navigation guidance)?"
                .into(),
            regulation: "61.870(6)",
            answer: Answer::YesNo,
            applies_when: Some(guidance_applies()),
            required: false,
        },
        Question {
            key: COURSE_DEVIATION_INDICATOR_OPERATION,
            label: "Was an instrument approach operation (an approach or a DME or GNSS \
                    arrival) a course deviation indicator operation (an instrument approach \
                    operation using a course deviation indicator for lateral navigation \
                    guidance)?"
                .into(),
            regulation: "61.870(7)",
            answer: Answer::YesNo,
            applies_when: Some(guidance_applies()),
            required: false,
        },
    ];
    Questions::new(EntryInformation::VERSION, questions)
}

/// When the guidance operations of regs 61.870(6) and (7) apply: to an
/// entry that records an instrument approach, or a DME or GNSS arrival (a
/// count of at least one), which is also "an instrument approach operation"
/// (Table 61.935, item 20), and so may be either operation (reg 61.010).
fn guidance_applies() -> Condition {
    Condition::Any(vec![
        Condition::AnyApproach,
        Condition::CountAtLeast {
            key: DME_OR_GNSS_ARRIVALS,
            n: 1,
        },
    ])
}

/// The questions about an assessment: the check, its outcome and, for a
/// flight test, its purpose where reg 61.880(3) depends on it. The check and
/// its outcome are required, as pilot-logbook requires them.
pub(super) fn for_assessment() -> Questions {
    use CheckType::*;
    let check_types = [
        (FlightTest, "61.745(3)(a), (b)"),
        (FlightReview, "61.745(1)"),
        (OperatorProficiencyCheck, "61.745(3)(e)(i)"),
        (InstrumentProficiencyCheck, "61.745(3A)(a)"),
        (NightVisionImagingSystemProficiencyCheck, "61.745(3A)(b)"),
        (AerialApplicationProficiencyCheck, "61.745(3A)(c)"),
        (InstructorProficiencyCheck, "61.745(3A)(d)"),
        (ExaminerProficiencyCheck, "61.745(3A)(e)"),
    ];
    let questions = vec![
        Question {
            key: CHECK_TYPE,
            label: "Which check was it?".into(),
            regulation: "61.745(3), (3A)",
            answer: Answer::Choice(
                check_types
                    .map(|(check, regulation)| choice(check.as_str(), regulation))
                    .into(),
            ),
            applies_when: None,
            required: true,
        },
        Question {
            key: OUTCOME,
            label: "What was its outcome?".into(),
            regulation: "61.745(3)",
            answer: Answer::Choice(vec![
                choice(CheckOutcome::Passed.as_str(), "61.745(3)"),
                // Reg 61.745(3) speaks only of passing; no one regulation
                // speaks of failing every kind of check.
                choice(CheckOutcome::Failed.as_str(), ""),
            ]),
            applies_when: None,
            required: true,
        },
        Question {
            key: CHECK_PURPOSE,
            label: "Was the flight test for the instrument rating, or for an instrument \
                    endorsement?"
                .into(),
            regulation: "61.880(3)",
            answer: Answer::Choice(vec![
                choice(CheckPurpose::InstrumentRating.as_str(), "61.880(3)(a)"),
                choice(CheckPurpose::InstrumentEndorsement.as_str(), "61.880(3)(b)"),
            ]),
            applies_when: Some(Condition::Answer {
                key: CHECK_TYPE,
                is: FlightTest.as_str().into(),
            }),
            // A flight test for another rating has neither purpose.
            required: false,
        },
    ];
    Questions::new(AssessmentInformation::VERSION, questions)
}

fn route(route: SupervisionRoute, label: &str, regulation: &'static str) -> Choice {
    Choice {
        value: route.as_str(),
        label: label.into(),
        regulation,
    }
}

/// A choice labelled with its stored spelling, which is in its regulation's
/// words, capitalised.
fn choice(value: &'static str, regulation: &'static str) -> Choice {
    let mut label = value.to_owned();
    if let Some(first) = label.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    Choice {
        value,
        label,
        regulation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keys of the answers `condition` reads. A condition the kernel
    /// adds later reads none here until it is matched.
    fn keys_read(condition: &Condition) -> Vec<&'static str> {
        match condition {
            Condition::Answer { key, .. } | Condition::CountAtLeast { key, .. } => vec![key],
            Condition::All(conditions) | Condition::Any(conditions) => {
                conditions.iter().flat_map(keys_read).collect()
            }
            _ => Vec::new(),
        }
    }

    /// A condition reads only the answer to an earlier question, so each
    /// key a condition reads is an earlier question's: the route before
    /// whether the person was receiving flight training, and the DME or
    /// GNSS arrivals before the guidance operations.
    #[test]
    fn every_condition_reads_an_earlier_questions_answer() {
        for questions in [ENTRY.clone(), for_assessment()] {
            let keys: Vec<_> = questions.iter().map(|question| question.key).collect();
            for (at, question) in questions.iter().enumerate() {
                for key in question.applies_when.iter().flat_map(keys_read) {
                    assert!(keys[..at].contains(&key), "{} reads {key}", question.key);
                }
            }
        }
    }
}
