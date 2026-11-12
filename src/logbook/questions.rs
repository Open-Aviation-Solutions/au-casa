// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's questions, for its information on an entry and on an assessment:
//! one question for each key of the two formats.

use std::sync::LazyLock;

use icao_shared_kernel::Capacity;
use logbook_shared_kernel::{
    Answer, Choice, ChoiceGroup, ChoiceKind, ChoicesLabels, Condition, Question, Questions,
};

use super::assessment_information::{CHECK_PURPOSES, CHECK_TYPE, OUTCOME};
use super::entry_information::{
    APPROVED_FSTD, AZIMUTH_GUIDANCE_OPERATION, COURSE_DEVIATION_INDICATOR_OPERATION,
    DME_OR_GNSS_ARRIVALS, RECEIVING_FLIGHT_TRAINING, ROUTE,
};
use super::{
    AssessmentInformation, CheckOutcome, CheckPurpose, CheckType, EntryInformation, PurposeKind,
    SupervisionRoute,
};

/// The questions about an entry, built once.
pub(super) static ENTRY: LazyLock<Questions> = LazyLock::new(for_entry);

/// The questions about an assessment, built once.
pub(super) static ASSESSMENT: LazyLock<Questions> = LazyLock::new(for_assessment);

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

/// The questions about an assessment: the check, then its outcome or, for a
/// flight test, its purposes, each with its own outcome, since a flight
/// test has no outcome of its own. Each is required where it applies, as
/// pilot-logbook requires the check and its outcome, and a flight test must
/// have its purposes.
fn for_assessment() -> Questions {
    use CheckType::*;
    let regulation = |check: CheckType| match check {
        FlightTest => "61.745(3)(a), (b)",
        FlightReview => "61.745(1)",
        OperatorProficiencyCheck => "61.745(3)(e)(i)",
        InstrumentProficiencyCheck => "61.745(3A)(a)",
        NightVisionImagingSystemProficiencyCheck => "61.745(3A)(b)",
        AerialApplicationProficiencyCheck => "61.745(3A)(c)",
        InstructorProficiencyCheck => "61.745(3A)(d)",
        ExaminerProficiencyCheck => "61.745(3A)(e)",
    };
    let is = |check: CheckType| Condition::Answer {
        key: CHECK_TYPE,
        is: check.as_str().into(),
    };
    let outcomes = || {
        vec![
            choice(CheckOutcome::Passed.as_str(), "passed", "61.745(3)"),
            // Reg 61.745(3) speaks only of passing; no one regulation
            // speaks of failing every kind of check.
            Choice {
                adverse: true,
                ..choice(CheckOutcome::Failed.as_str(), "failed", "")
            },
        ]
    };
    let questions = vec![
        Question {
            key: CHECK_TYPE,
            label: "Which check was it?".into(),
            regulation: "61.745(3), (3A)",
            answer: Answer::Choice(
                CheckType::ALL
                    .map(|check| choice(check.as_str(), check.label(), regulation(check)))
                    .into(),
            ),
            applies_when: None,
            required: true,
        },
        Question {
            key: OUTCOME,
            label: "What was its outcome?".into(),
            regulation: "61.745(3)",
            answer: Answer::Choice(outcomes()),
            // Every check but a flight test, whose purposes have outcomes.
            applies_when: Some(Condition::Any(
                CheckType::ALL
                    .into_iter()
                    .filter(|check| *check != FlightTest)
                    .map(is)
                    .collect(),
            )),
            required: true,
        },
        Question {
            key: CHECK_PURPOSES,
            label: "What was the flight test for?".into(),
            regulation: "61.010",
            answer: Answer::Choices {
                groups: PurposeKind::ALL
                    .into_iter()
                    .map(|kind| ChoiceGroup {
                        kind: ChoiceKind {
                            value: kind.as_str(),
                            label: capitalised(kind.label()),
                        },
                        choices: CheckPurpose::ALL
                            .into_iter()
                            .filter(|purpose| purpose.kind() == kind)
                            .map(|purpose| {
                                choice(purpose.as_str(), purpose.label(), purpose.regulation())
                            })
                            .collect(),
                    })
                    .collect(),
                each: outcomes(),
                // The item's words are reg 61.010's definition of a flight
                // test ("for the licence, rating or endorsement"); "passes"
                // is reg 61.250's.
                labels: ChoicesLabels {
                    item: "licence, rating or endorsement".into(),
                    answer: "Passed or failed".into(),
                },
            },
            applies_when: Some(is(FlightTest)),
            required: true,
        },
    ];
    Questions::new(AssessmentInformation::VERSION, questions)
}

fn route(route: SupervisionRoute, label: &str, regulation: &'static str) -> Choice {
    Choice {
        value: route.as_str(),
        label: label.into(),
        regulation,
        adverse: false,
    }
}

/// A choice, labelled in its regulation's words, capitalised.
fn choice(value: &'static str, label: &str, regulation: &'static str) -> Choice {
    Choice {
        value,
        label: capitalised(label),
        regulation,
        adverse: false,
    }
}

fn capitalised(words: &str) -> String {
    let mut capitalised = words.to_owned();
    if let Some(first) = capitalised.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    capitalised
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
