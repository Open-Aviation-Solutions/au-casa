// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's rules for a pilot's logbook, and CASA's information on an entry
//! and on an assessment.

use std::collections::BTreeMap;

use aircraft_type_descriptions::{AircraftDescription, AirframeKind, EngineType};
use au_casa::{
    AssessmentInformation, AssessmentInformationError, Casa, CheckOutcome, CheckPurpose, CheckType,
    EntryInformation, InformationError, SupervisionRoute,
};
use icao_shared_kernel::{
    Aircraft, AircraftRegistration, AircraftType, Capacity, FlightDuration, FlightRules,
    LocationIndicator, NationalityMark,
};
use logbook_shared_kernel::{
    Answer, Approach, ApproachType, Assessment, AssessmentId, AuthorityId,
    AuthoritySpecificInformation, DateRange, DayNight, Entry, EntryId, EntryParts, InstrumentTime,
    LicensingAuthority, Logbook, ProblemAt, ProblemKind, Question, Record, Subject,
};
use rstest::rstest;
use serde_json::{json, Value};
use time::macros::date;
use uuid::Uuid;

fn flight_parts() -> EntryParts {
    EntryParts {
        id: EntryId::new("e1"),
        date: date!(2026 - 11 - 06),
        subject: Subject::Flight {
            flight: Uuid::nil(),
            aircraft: Aircraft::new(
                AircraftType::parse("C172").unwrap(),
                AircraftRegistration::new(NationalityMark::parse("VH").unwrap(), "ABC").unwrap(),
            ),
            description: None,
            sole_occupant: false,
        },
        route: None,
        flight_rules: None,
        flight_time: FlightDuration::new(60).unwrap(),
        night: FlightDuration::new(0).unwrap(),
        capacity: Capacity::PilotInCommand,
        pilot_in_command: None,
        flight_crew: Vec::new(),
        instrument: InstrumentTime::default(),
        approaches: Vec::new(),
        take_offs: DayNight::default(),
        landings: DayNight::default(),
        authority_information: BTreeMap::new(),
    }
}

/// A session, with `information` as the information of `authority`.
fn session(authority: &str, information: Option<Value>) -> Entry {
    let mut parts = EntryParts {
        subject: Subject::Session {
            session: Uuid::nil(),
            device: Uuid::nil(),
            simulated: None,
        },
        ..flight_parts()
    };
    if let Some(information) = information {
        parts.authority_information.insert(
            AuthorityId::new("AU", authority).unwrap(),
            AuthoritySpecificInformation::new(information),
        );
    }
    Entry::new(parts).unwrap()
}

#[test]
fn casa_is_au_casa() {
    assert_eq!(Casa::new().id().to_string(), "AU-CASA");
}

#[test]
fn a_flight_counts() {
    assert!(Casa::new().counts(&Entry::new(flight_parts()).unwrap()));
}

/// Reg 61.010, "approved flight simulation training device".
#[rstest]
#[case::approved(Some(json!({"_version": 1, "approved_fstd": true})), true)]
#[case::not_approved(Some(json!({"_version": 1, "approved_fstd": false})), false)]
#[case::unanswered(Some(json!({"_version": 1})), false)]
#[case::no_information(None, false)]
#[case::unreadable(Some(json!({"_version": 1, "approved_fstd": "yes"})), false)]
#[case::unversioned(Some(json!({"approved_fstd": true})), false)]
fn a_session_counts_only_in_an_approved_device(
    #[case] information: Option<Value>,
    #[case] counts: bool,
) {
    assert_eq!(Casa::new().counts(&session("CASA", information)), counts);
}

/// An answer that cannot be read makes the information unreadable as
/// stored, even under a question not shown for the entry.
#[test]
fn a_badly_typed_hidden_answer_still_makes_the_information_unreadable() {
    let entry = session(
        "CASA",
        Some(json!({"_version": 1, "approved_fstd": true, "route": 42})),
    );
    assert!(!Casa::new().counts(&entry));
}

#[test]
fn another_authoritys_information_does_not_make_a_session_count() {
    let entry = session("OTHER", Some(json!({"_version": 1, "approved_fstd": true})));
    assert!(!Casa::new().counts(&entry));
}

/// Regs 61.015 and 61.020, and 61.007(2) for a registered sailplane.
#[rstest]
#[case(AirframeKind::LandPlane, 1, EngineType::Piston, &["aeroplane", "single-engine aeroplane"])]
#[case(AirframeKind::SeaPlane, 2, EngineType::TurboProp, &["aeroplane", "multi-engine aeroplane"])]
#[case(AirframeKind::Helicopter, 1, EngineType::TurboProp, &["helicopter", "single-engine helicopter"])]
#[case(AirframeKind::Helicopter, 2, EngineType::TurboProp, &["helicopter"])]
#[case(AirframeKind::Gyrocopter, 1, EngineType::Piston, &["gyroplane", "single-engine gyroplane"])]
#[case(AirframeKind::TiltRotor, 2, EngineType::TurboProp, &["powered-lift aircraft", "powered-lift aircraft"])]
#[case(AirframeKind::Airship, 2, EngineType::Piston, &["airship", "airship"])]
#[case(AirframeKind::Glider, 0, EngineType::None, &["registered sailplane"])]
fn a_type_counts_as_its_category_and_class(
    #[case] airframe: AirframeKind,
    #[case] engines: u8,
    #[case] engine_type: EngineType,
    #[case] counts_as: &[&str],
) {
    let classification = Casa::new()
        .classify(&AircraftDescription::new(airframe, engines, engine_type))
        .unwrap();
    assert_eq!(classification.counts_as, counts_as);
    assert!(classification.cautions.is_empty());
}

/// The keys of the questions about `record`, and of those shown for it
/// given `answers`.
fn asked_and_shown(record: Record, answers: Value) -> (Vec<&'static str>, Vec<&'static str>) {
    let questions = Casa::new().questions_for(record);
    let information = AuthoritySpecificInformation::new(answers);
    let keys = |questions: Vec<&Question>| questions.iter().map(|q| q.key).collect();
    (
        keys(questions.iter().collect()),
        keys(questions.shown_for(record, Some(&information))),
    )
}

/// Every key of the entry format, in order.
const ENTRY_KEYS: [&str; 6] = [
    "approved_fstd",
    "route",
    "receiving_flight_training",
    "azimuth_guidance_operation",
    "course_deviation_indicator_operation",
    "dme_or_gnss_arrivals",
];

#[test]
fn a_flight_is_shown_only_what_applies_to_it() {
    let entry = Entry::new(flight_parts()).unwrap();
    let questions = Casa::new().questions_for(Record::Entry(&entry));
    assert_eq!(questions.version(), EntryInformation::VERSION);
    let (asked, shown) = asked_and_shown(Record::Entry(&entry), json!({}));
    assert_eq!(asked, ENTRY_KEYS);
    assert_eq!(shown, ["dme_or_gnss_arrivals"]);
}

#[test]
fn a_session_is_shown_whether_its_device_was_approved() {
    let entry = session("CASA", None);
    let (asked, shown) = asked_and_shown(Record::Entry(&entry), json!({}));
    assert_eq!(asked, ENTRY_KEYS);
    assert_eq!(shown, ["approved_fstd", "dme_or_gnss_arrivals"]);
}

/// Reg 61.870(6) and (7), and reg 61.870's instrument approach operations
/// "conducted", whatever the flight rules.
#[rstest]
#[case::vfr(Some(FlightRules::Vfr))]
#[case::unknown(None)]
#[case::ifr(Some(FlightRules::Ifr))]
#[case::initially_ifr(Some(FlightRules::InitiallyIfr))]
#[case::initially_vfr(Some(FlightRules::InitiallyVfr))]
fn approaches_are_asked_about_and_arrivals_whatever_the_flight_rules(
    #[case] flight_rules: Option<FlightRules>,
) {
    let entry = Entry::new(EntryParts {
        flight_rules,
        approaches: vec![Approach::new(
            ApproachType::Vor,
            LocationIndicator::parse("YBAF").unwrap(),
            false,
        )],
        ..flight_parts()
    })
    .unwrap();
    let (_, shown) = asked_and_shown(Record::Entry(&entry), json!({}));
    assert_eq!(
        shown,
        [
            "azimuth_guidance_operation",
            "course_deviation_indicator_operation",
            "dme_or_gnss_arrivals",
        ]
    );
}

/// The host drops an answer to a question that no longer applies to the
/// entry, and keeps one to a question that still does.
#[test]
fn the_host_drops_only_the_answers_that_no_longer_apply() {
    let answers = json!({
        "_version": 1,
        "approved_fstd": true,
        "azimuth_guidance_operation": true,
        "course_deviation_indicator_operation": true,
        "dme_or_gnss_arrivals": 2,
    });
    let kept = |entry: &Entry| {
        let mut information = AuthoritySpecificInformation::new(answers.clone());
        Casa::new()
            .questions_for(Record::Entry(entry))
            .drop_hidden_answers_for(Record::Entry(entry), &mut information);
        information.value().clone()
    };
    let flight = Entry::new(flight_parts()).unwrap();
    assert_eq!(
        kept(&flight),
        json!({"_version": 1, "dme_or_gnss_arrivals": 2})
    );
    let session_with_an_approach = Entry::new(EntryParts {
        approaches: vec![Approach::new(
            ApproachType::Vor,
            LocationIndicator::parse("YBAF").unwrap(),
            false,
        )],
        subject: session("CASA", None).subject().clone(),
        ..flight_parts()
    })
    .unwrap();
    assert_eq!(kept(&session_with_an_approach), answers);
}

/// Reg 61.095(1)(c), (2) and (3).
#[rstest]
#[case::unanswered(json!({}), &["route", "dme_or_gnss_arrivals"])]
#[case::operator(json!({"route": "operator"}), &["route", "dme_or_gnss_arrivals"])]
#[case::instructor(
    json!({"route": "instructor"}),
    &["route", "receiving_flight_training", "dme_or_gnss_arrivals"]
)]
fn a_pilot_in_command_under_supervision_is_asked_the_route(
    #[case] answers: Value,
    #[case] expected: &[&str],
) {
    let entry = Entry::new(EntryParts {
        capacity: Capacity::PilotInCommandUnderSupervision,
        ..flight_parts()
    })
    .unwrap();
    let (_, shown) = asked_and_shown(Record::Entry(&entry), answers);
    assert_eq!(shown, expected);
    let questions = Casa::new().questions_for(Record::Entry(&entry));
    let unanswered: Vec<_> = questions
        .required_unanswered_for(
            Record::Entry(&entry),
            Some(&AuthoritySpecificInformation::new(
                json!({"route": "instructor"}),
            )),
        )
        .into_iter()
        .map(|q| q.key)
        .collect();
    assert_eq!(unanswered, ["receiving_flight_training"]);
}

/// An answer each question accepts, of its kind.
fn an_answer(question: &Question) -> Value {
    match &question.answer {
        Answer::Choice(choices) => choices[0].value.into(),
        Answer::YesNo => false.into(),
        Answer::Count => 2.into(),
        other => panic!("CASA asks no {other:?}"),
    }
}

#[test]
fn every_question_is_a_key_of_its_format() {
    let casa = Casa::new();
    let entry = Entry::new(EntryParts {
        subject: session("CASA", None).subject().clone(),
        flight_rules: Some(FlightRules::Ifr),
        approaches: vec![Approach::new(
            ApproachType::Ils,
            LocationIndicator::parse("YBAF").unwrap(),
            false,
        )],
        ..flight_parts()
    })
    .unwrap();
    let assessment = an_assessment();
    for record in [Record::Entry(&entry), Record::Assessment(&assessment)] {
        let questions = casa.questions_for(record);
        let mut answers = serde_json::Map::new();
        for question in &questions {
            answers.insert(question.key.into(), an_answer(question));
            if let Answer::Choice(choices) = &question.answer {
                for choice in choices {
                    let one = json!({ question.key: choice.value });
                    assert!(casa
                        .problems_for(record, &AuthoritySpecificInformation::new(one))
                        .is_empty());
                }
            }
        }
        assert_eq!(answers.len(), questions.iter().count());
        assert!(casa
            .problems_for(record, &AuthoritySpecificInformation::new(answers.into()))
            .is_empty());
    }
}

#[test]
fn an_assessment_is_asked_its_check_outcome_and_a_flight_tests_purpose() {
    let assessment = an_assessment();
    let record = Record::Assessment(&assessment);
    let questions = Casa::new().questions_for(record);
    assert_eq!(questions.version(), AssessmentInformation::VERSION);
    let (asked, shown) = asked_and_shown(record, json!({"check_type": "flight review"}));
    assert_eq!(asked, ["check_type", "outcome", "check_purpose"]);
    assert_eq!(shown, ["check_type", "outcome"]);
    let (_, shown) = asked_and_shown(record, json!({"check_type": "flight test"}));
    assert_eq!(shown, ["check_type", "outcome", "check_purpose"]);
    let required: Vec<_> = questions
        .required_unanswered_for(record, None)
        .into_iter()
        .map(|q| q.key)
        .collect();
    assert_eq!(required, ["check_type", "outcome"]);
}

/// Reg 61.095(3)(b): "the person is not receiving flight training".
#[test]
fn receiving_flight_training_on_the_instructors_route_is_refused() {
    let casa = Casa::new();
    let entry = Entry::new(EntryParts {
        capacity: Capacity::PilotInCommandUnderSupervision,
        ..flight_parts()
    })
    .unwrap();
    let problems = |receiving: bool| {
        casa.problems_for(
            Record::Entry(&entry),
            &AuthoritySpecificInformation::new(
                json!({"route": "instructor", "receiving_flight_training": receiving}),
            ),
        )
    };
    assert!(problems(false).is_empty());
    let refused = problems(true);
    assert_eq!(refused.len(), 1);
    assert_eq!(
        refused[0].at,
        ProblemAt::Question("receiving_flight_training")
    );
    assert_eq!(refused[0].kind, ProblemKind::Refusal);
    assert_eq!(refused[0].regulation, "61.095(3)(b)");
}

/// Information with no version is information not saved yet, whether the
/// host passes the record with it already inside or without it; stored
/// information that has lost its version is refused where it is read.
#[test]
fn information_with_no_version_is_information_not_saved_yet() {
    let casa = Casa::new();
    let answers = json!({"approved_fstd": true});
    let unversioned = AuthoritySpecificInformation::new(answers.clone());
    let with_it = session("CASA", Some(answers));
    let without_it = session("CASA", None);
    for entry in [&with_it, &without_it] {
        assert!(casa
            .problems_for(Record::Entry(entry), &unversioned)
            .is_empty());
    }
    assert!(!casa.counts(&with_it));
    assert_eq!(
        EntryInformation::read(&unversioned),
        Err(InformationError::UnsupportedVersion)
    );
    let refused = casa.problems_for(
        Record::Entry(&without_it),
        &AuthoritySpecificInformation::new(json!({"_version": 2, "approved_fstd": true})),
    );
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].at, ProblemAt::Information);
}

/// An answer left from when the entry was logged as pilot in command under
/// supervision is not the person's training on reg 61.095(3)'s route.
#[test]
fn receiving_flight_training_is_refused_only_where_it_is_asked() {
    let casa = Casa::new();
    let entry = Entry::new(flight_parts()).unwrap();
    let left = AuthoritySpecificInformation::new(
        json!({"route": "instructor", "receiving_flight_training": true}),
    );
    assert!(casa.problems_for(Record::Entry(&entry), &left).is_empty());
    let supervised = Entry::new(EntryParts {
        capacity: Capacity::PilotInCommandUnderSupervision,
        ..flight_parts()
    })
    .unwrap();
    let operator = AuthoritySpecificInformation::new(
        json!({"route": "operator", "receiving_flight_training": true}),
    );
    assert!(casa
        .problems_for(Record::Entry(&supervised), &operator)
        .is_empty());
}

#[test]
fn each_requirement_has_its_result_and_each_figure_its_time() {
    let casa = Casa::new();
    let logbook = Logbook {
        entries: vec![Entry::new(flight_parts()).unwrap()],
        assessments: vec![an_assessment()],
    };
    let requirements = casa.requirements();
    assert_eq!(requirements.len(), 17);
    assert_eq!(
        requirements[0].name,
        "Carrying a passenger by day in an aeroplane"
    );
    assert_eq!(requirements[0].regulation, "61.395(1)");
    let answered: Vec<_> = casa
        .currency(date!(2026 - 11 - 06), &logbook)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let ids: Vec<_> = requirements.into_iter().map(|r| r.id).collect();
    assert_eq!(answered, ids);
    // The hour in a C172.
    let figures: Vec<_> = casa
        .totals(&logbook, DateRange::default())
        .into_iter()
        .map(|figure| (figure.name, figure.time.total_minutes()))
        .collect();
    assert_eq!(figures[0], ("Aeroplane".to_owned(), 60));
    assert_eq!(figures.len(), 6);
}

/// Read as information about to be saved, which may have no version yet.
fn read(value: Value) -> Result<EntryInformation, InformationError> {
    EntryInformation::read_to_save(&AuthoritySpecificInformation::new(value))
}

#[test]
fn stored_information_must_carry_its_version() {
    let stored = |value| EntryInformation::read(&AuthoritySpecificInformation::new(value));
    assert_eq!(
        stored(json!({"approved_fstd": true})),
        Err(InformationError::UnsupportedVersion)
    );
    assert_eq!(
        read(json!({"approved_fstd": true})).unwrap().approved_fstd,
        Some(true)
    );
    assert_eq!(
        stored(json!({"_version": 1, "approved_fstd": true}))
            .unwrap()
            .approved_fstd,
        Some(true)
    );
}

#[test]
fn information_that_cannot_be_read_is_refused_before_it_is_saved() {
    let casa = Casa::new();
    let entry = Entry::new(flight_parts()).unwrap();
    let problems = |value| {
        casa.problems_for(
            Record::Entry(&entry),
            &AuthoritySpecificInformation::new(value),
        )
    };
    assert!(problems(json!({"route": "operator"})).is_empty());
    assert!(problems(json!({"_version": 1, "route": "operator"})).is_empty());
    let refused = problems(json!({"_version": 2}));
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].at, ProblemAt::Information);
    assert_eq!(refused[0].kind, ProblemKind::Refusal);
}

#[test]
fn every_answer_has_its_spelling() {
    let information = EntryInformation {
        approved_fstd: Some(true),
        route: Some(SupervisionRoute::Instructor),
        receiving_flight_training: Some(false),
        azimuth_guidance_operation: Some(true),
        course_deviation_indicator_operation: Some(false),
        dme_or_gnss_arrivals: Some(2),
    };
    let stored = json!({
        "approved_fstd": true,
        "route": "instructor",
        "receiving_flight_training": false,
        "azimuth_guidance_operation": true,
        "course_deviation_indicator_operation": false,
        "dme_or_gnss_arrivals": 2,
    });
    assert_eq!(information.to_information().into_value(), stored);
    assert_eq!(read(stored), Ok(information));
}

#[rstest]
#[case(SupervisionRoute::Operator, "operator")] // reg 61.095(2)
#[case(SupervisionRoute::Instructor, "instructor")] // reg 61.095(3)
fn every_route_has_its_spelling(#[case] route: SupervisionRoute, #[case] spelt: &str) {
    let information = EntryInformation {
        route: Some(route),
        ..EntryInformation::default()
    };
    assert_eq!(
        information.to_information().into_value(),
        json!({"route": spelt})
    );
    assert_eq!(read(json!({"route": spelt})), Ok(information));
}

#[test]
fn an_answer_not_given_has_no_key() {
    assert_eq!(
        EntryInformation::default().to_information().into_value(),
        json!({})
    );
    assert_eq!(read(json!({})), Ok(EntryInformation::default()));
}

#[test]
fn a_null_answer_on_an_entry_is_no_answer() {
    assert_eq!(
        read(json!({"azimuth_guidance_operation": null, "route": null})),
        Ok(EntryInformation::default())
    );
}

#[test]
fn a_count_is_any_whole_number_the_kernel_accepts() {
    let information = read(json!({"dme_or_gnss_arrivals": u64::MAX})).unwrap();
    assert_eq!(information.dme_or_gnss_arrivals, Some(u64::MAX));
}

#[test]
fn the_version_is_read_when_it_is_this_formats() {
    assert_eq!(EntryInformation::VERSION, 1);
    let information = read(json!({"_version": 1, "approved_fstd": true})).unwrap();
    assert_eq!(information.approved_fstd, Some(true));
}

#[rstest]
#[case::not_an_object(json!([]), InformationError::NotAnObject)]
#[case::another_version(json!({"_version": 2}), InformationError::UnsupportedVersion)]
#[case::version_as_text(json!({"_version": "1"}), InformationError::UnsupportedVersion)]
#[case::unknown_key(json!({"notes": "x"}), InformationError::UnknownKey("notes".into()))]
#[case::yes_no_as_text(json!({"approved_fstd": "yes"}), InformationError::InvalidAnswer("approved_fstd"))]
#[case::unknown_route(json!({"route": "pilot"}), InformationError::InvalidAnswer("route"))]
#[case::count_as_decimal(json!({"dme_or_gnss_arrivals": 2.0}), InformationError::InvalidAnswer("dme_or_gnss_arrivals"))]
#[case::negative_count(json!({"dme_or_gnss_arrivals": -1}), InformationError::InvalidAnswer("dme_or_gnss_arrivals"))]
fn what_is_not_this_format_is_refused(#[case] stored: Value, #[case] error: InformationError) {
    assert_eq!(read(stored), Err(error));
}

fn an_assessment() -> Assessment {
    Assessment {
        id: AssessmentId::new("a1"),
        date: date!(2026 - 11 - 06),
        flight: Uuid::nil(),
        authority_information: BTreeMap::new(),
    }
}

/// Read as information on an assessment about to be saved.
fn read_assessment(value: Value) -> Result<AssessmentInformation, AssessmentInformationError> {
    AssessmentInformation::read_to_save(&AuthoritySpecificInformation::new(value))
}

#[test]
fn information_on_an_assessment_that_cannot_be_read_is_refused_before_it_is_saved() {
    let casa = Casa::new();
    let assessment = an_assessment();
    let problems = |value| {
        casa.problems_for(
            Record::Assessment(&assessment),
            &AuthoritySpecificInformation::new(value),
        )
    };
    assert!(problems(json!({"outcome": "passed"})).is_empty());
    assert!(problems(json!({"_version": 1, "outcome": "passed"})).is_empty());
    // Information on an entry is not information on an assessment.
    let refused = problems(json!({"approved_fstd": true}));
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].at, ProblemAt::Information);
    assert_eq!(refused[0].kind, ProblemKind::Refusal);
}

#[test]
fn stored_information_on_an_assessment_must_carry_its_version() {
    let stored = |value| AssessmentInformation::read(&AuthoritySpecificInformation::new(value));
    assert_eq!(
        stored(json!({"outcome": "passed"})),
        Err(AssessmentInformationError::UnsupportedVersion)
    );
    assert_eq!(
        stored(json!({"_version": 1, "outcome": "passed"}))
            .unwrap()
            .outcome,
        Some(CheckOutcome::Passed)
    );
    assert_eq!(AssessmentInformation::VERSION, 1);
}

/// Regs 61.745(3) and (3A).
#[rstest]
#[case(CheckType::FlightTest, "flight test")]
#[case(CheckType::FlightReview, "flight review")]
#[case(CheckType::OperatorProficiencyCheck, "operator proficiency check")]
#[case(CheckType::InstrumentProficiencyCheck, "instrument proficiency check")]
#[case(
    CheckType::NightVisionImagingSystemProficiencyCheck,
    "night vision imaging system proficiency check"
)]
#[case(
    CheckType::AerialApplicationProficiencyCheck,
    "aerial application proficiency check"
)]
#[case(CheckType::InstructorProficiencyCheck, "instructor proficiency check")]
#[case(CheckType::ExaminerProficiencyCheck, "examiner proficiency check")]
fn every_check_type_has_its_spelling(#[case] check_type: CheckType, #[case] spelt: &str) {
    let information = AssessmentInformation {
        check_type: Some(check_type),
        ..AssessmentInformation::default()
    };
    assert_eq!(
        information.to_information().into_value(),
        json!({"check_type": spelt})
    );
    assert_eq!(
        read_assessment(json!({"check_type": spelt})),
        Ok(information)
    );
}

#[rstest]
#[case(CheckOutcome::Passed, "passed")]
#[case(CheckOutcome::Failed, "failed")]
fn every_outcome_has_its_spelling(#[case] outcome: CheckOutcome, #[case] spelt: &str) {
    let information = AssessmentInformation {
        outcome: Some(outcome),
        ..AssessmentInformation::default()
    };
    assert_eq!(
        information.to_information().into_value(),
        json!({"outcome": spelt})
    );
    assert_eq!(read_assessment(json!({"outcome": spelt})), Ok(information));
}

/// Reg 61.880(3)(a) and (b).
#[rstest]
#[case(CheckPurpose::InstrumentRating, "instrument rating")]
#[case(CheckPurpose::InstrumentEndorsement, "instrument endorsement")]
fn every_check_purpose_has_its_spelling(#[case] purpose: CheckPurpose, #[case] spelt: &str) {
    let information = AssessmentInformation {
        check_purpose: Some(purpose),
        ..AssessmentInformation::default()
    };
    assert_eq!(
        information.to_information().into_value(),
        json!({"check_purpose": spelt})
    );
    assert_eq!(
        read_assessment(json!({"check_purpose": spelt})),
        Ok(information)
    );
}

#[test]
fn an_assessment_answer_not_given_has_no_key() {
    assert_eq!(
        AssessmentInformation::default()
            .to_information()
            .into_value(),
        json!({})
    );
    assert_eq!(
        read_assessment(json!({})),
        Ok(AssessmentInformation::default())
    );
}

#[rstest]
#[case::not_an_object(json!("passed"), AssessmentInformationError::NotAnObject)]
#[case::another_version(json!({"_version": 2}), AssessmentInformationError::UnsupportedVersion)]
#[case::unknown_key(json!({"purpose": "instrument rating"}), AssessmentInformationError::UnknownKey("purpose".into()))]
#[case::unknown_check_type(json!({"check_type": "flight_review"}), AssessmentInformationError::InvalidAnswer("check_type"))]
#[case::outcome_as_yes_no(json!({"outcome": true}), AssessmentInformationError::InvalidAnswer("outcome"))]
fn what_is_not_the_assessment_format_is_refused(
    #[case] stored: Value,
    #[case] error: AssessmentInformationError,
) {
    assert_eq!(read_assessment(stored), Err(error));
}

/// pilot-logbook's stored assessment today (`flight_assessment_to_json`),
/// its three keys moved as they are and versioned: a flight review has a
/// `null` purpose.
#[test]
fn an_assessment_is_read_as_pilot_logbook_stores_it() {
    let stored = json!({
        "_version": 1,
        "check_type": "flight review",
        "outcome": "passed",
        "check_purpose": null,
    });
    assert_eq!(
        AssessmentInformation::read(&AuthoritySpecificInformation::new(stored)),
        Ok(AssessmentInformation {
            check_type: Some(CheckType::FlightReview),
            outcome: Some(CheckOutcome::Passed),
            check_purpose: None,
        })
    );
}
