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
    Aircraft, AircraftRegistration, AircraftType, Capacity, FlightDuration, NationalityMark,
};
use logbook_shared_kernel::{
    Assessment, AssessmentId, AuthorityId, AuthoritySpecificInformation, DateRange, DayNight,
    Entry, EntryId, EntryParts, InstrumentTime, LicensingAuthority, Logbook, ProblemAt,
    ProblemKind, Record, Subject,
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

#[test]
fn nothing_is_asked_or_found_yet() {
    let casa = Casa::new();
    let entry = Entry::new(flight_parts()).unwrap();
    let assessment = an_assessment();
    let information = AuthoritySpecificInformation::new(json!({}));
    for record in [Record::Entry(&entry), Record::Assessment(&assessment)] {
        let questions = casa.questions_for(record);
        assert!(questions.is_empty());
        assert_eq!(questions.version(), 1);
        assert!(casa.problems_for(record, &information).is_empty());
    }
}

#[test]
fn each_requirement_has_its_result_and_each_figure_its_time() {
    let casa = Casa::new();
    let logbook = Logbook {
        entries: vec![Entry::new(flight_parts()).unwrap()],
        assessments: vec![an_assessment()],
    };
    let requirements = casa.requirements();
    assert_eq!(requirements.len(), 12);
    assert_eq!(
        requirements[0].name,
        "Carrying a passenger by day in an aeroplane"
    );
    assert_eq!(requirements[0].regulation, "reg 61.395(1)");
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
