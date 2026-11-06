// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's questions, for its information on an entry and on an assessment:
//! one question for each key of the two formats.

use icao_shared_kernel::{Capacity, FlightRules};
use logbook_shared_kernel::{Answer, Choice, Condition, Entry, Question, Questions, Subject};

use super::assessment_information::{CHECK_PURPOSE, CHECK_TYPE, OUTCOME};
use super::entry_information::{
    APPROVED_FSTD, AZIMUTH_GUIDANCE_OPERATION, COURSE_DEVIATION_INDICATOR_OPERATION,
    DME_OR_GNSS_ARRIVALS, RECEIVING_FLIGHT_TRAINING, ROUTE,
};
use super::{
    AssessmentInformation, CheckOutcome, CheckPurpose, CheckType, EntryInformation,
    SupervisionRoute,
};

/// The questions about `entry`. A question that depends on the entry's
/// subject, approaches or flight rules is asked only of an entry it applies
/// to; the route, which depends on the capacity, is shown on the kernel's
/// [`Condition::Capacity`], and its follow-up on the route's answer.
pub(super) fn for_entry(entry: &Entry) -> Questions {
    let session = matches!(entry.subject(), Subject::Session { .. });
    let approaches = !entry.approaches().is_empty();
    // Some part of the flight under the IFR.
    let ifr = matches!(
        entry.flight_rules(),
        Some(FlightRules::Ifr | FlightRules::InitiallyIfr | FlightRules::InitiallyVfr)
    );
    let mut questions = Vec::new();
    if session {
        questions.push(Question {
            key: APPROVED_FSTD,
            label: "Was the device an approved flight simulation training device for the purpose?"
                .into(),
            regulation: "61.010",
            answer: Answer::YesNo,
            shown_when: None,
            required: true,
        });
    }
    questions.push(Question {
        key: ROUTE,
        label: "Which applies to the flight?".into(),
        regulation: "61.095(1)(c)",
        answer: Answer::Choice(vec![
            route(
                SupervisionRoute::Operator,
                "The flight is conducted by an operator that has training and checking \
                 responsibilities, and the pilot in command of the flight is authorised by the \
                 operator or the operator's Part 142 operator to conduct the supervision of the \
                 person",
                "61.095(2)",
            ),
            route(
                SupervisionRoute::Instructor,
                "The person is supervised by a flight instructor or flight examiner",
                "61.095(3)",
            ),
        ]),
        shown_when: Some(Condition::Capacity(
            Capacity::PilotInCommandUnderSupervision,
        )),
        required: true,
    });
    questions.push(Question {
        key: RECEIVING_FLIGHT_TRAINING,
        label: "Was the person receiving flight training?".into(),
        regulation: "61.095(3)(b)",
        answer: Answer::YesNo,
        shown_when: Some(Condition::Answer {
            key: ROUTE,
            is: SupervisionRoute::Instructor.as_str().into(),
        }),
        required: true,
    });
    if approaches {
        questions.push(Question {
            key: AZIMUTH_GUIDANCE_OPERATION,
            label: "Was an approach an azimuth guidance operation (an instrument approach \
                    operation using azimuth bearings for lateral navigation guidance)?"
                .into(),
            regulation: "61.870(6)",
            answer: Answer::YesNo,
            shown_when: None,
            required: false,
        });
        questions.push(Question {
            key: COURSE_DEVIATION_INDICATOR_OPERATION,
            label: "Was an approach a course deviation indicator operation (an instrument \
                    approach operation using a course deviation indicator for lateral navigation \
                    guidance)?"
                .into(),
            regulation: "61.870(7)",
            answer: Answer::YesNo,
            shown_when: None,
            required: false,
        });
    }
    if ifr {
        questions.push(Question {
            key: DME_OR_GNSS_ARRIVALS,
            label: "How many instrument approach operations were conducted under the IFR using \
                    distance measuring equipment or a global navigation satellite system (DME or \
                    GNSS arrival procedure)?"
                .into(),
            regulation: "Table 61.935, item 20",
            answer: Answer::Count,
            shown_when: None,
            required: false,
        });
    }
    Questions::new(EntryInformation::VERSION, questions)
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
            shown_when: None,
            required: true,
        },
        Question {
            key: OUTCOME,
            label: "Did the pilot pass it?".into(),
            regulation: "61.745(3)",
            answer: Answer::Choice(
                [CheckOutcome::Passed, CheckOutcome::Failed]
                    .map(|outcome| choice(outcome.as_str(), "61.745(3)"))
                    .into(),
            ),
            shown_when: None,
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
            shown_when: Some(Condition::Answer {
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
