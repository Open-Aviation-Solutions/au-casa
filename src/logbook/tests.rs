// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's rules for a pilot's logbook, and CASA's information on an entry
//! and on an assessment.

use std::collections::{BTreeMap, BTreeSet};

use crate::logbook::assessment_information::AssessmentInformationError;
use crate::logbook::entry_information::InformationError;
use crate::logbook::test_support;
use crate::logbook::{migrate_entry_information_from_version_1, EntryMigrationError};
use crate::logbook::{
    AssessedPurpose, AssessmentInformation, CheckOutcome, CheckPurpose, CheckType,
    EntryInformation, SupervisionRoute,
};
use crate::Casa;
use aircraft_type_descriptions::{AircraftDescription, AirframeKind, EngineType};
use icao_shared_kernel::{Capacity, FlightRules, LocationIndicator};
use logbook_shared_kernel::{
    AircraftCharacteristic, Answer, Approach, ApproachType, Assessment, AssessmentId, AuthorityId,
    AuthoritySpecificInformation, Choice, DateRange, Entry, EntryId, EntryParts,
    LicensingAuthority, Logbook, MigrationRefused, ProblemAt, ProblemKind, Question, Record,
    Subject,
};
use rstest::rstest;
use serde_json::{json, Value};
use time::macros::date;
use uuid::Uuid;

/// An hour's flight in a single-engine aeroplane on 6 November 2026.
fn flight_parts() -> EntryParts {
    test_support::aeroplane("e1", date!(2026 - 11 - 06))
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
#[case::approved(Some(json!({"_version": 2, "approved_fstd": true})), true)]
#[case::not_approved(Some(json!({"_version": 2, "approved_fstd": false})), false)]
#[case::unanswered(Some(json!({"_version": 2})), false)]
#[case::no_information(None, false)]
#[case::unreadable(Some(json!({"_version": 2, "approved_fstd": "yes"})), false)]
#[case::unversioned(Some(json!({"approved_fstd": true})), false)]
fn a_session_counts_only_in_an_approved_device(
    #[case] information: Option<Value>,
    #[case] counts: bool,
) {
    assert_eq!(Casa::new().counts(&session("CASA", information)), counts);
}

/// An answer that cannot be read makes the information unreadable as
/// stored, even to a question that does not apply to the entry.
#[test]
fn a_badly_typed_answer_to_a_question_that_does_not_apply_still_makes_the_information_unreadable() {
    let answers = json!({"_version": 2, "approved_fstd": true, "route": 42});
    let entry = session("CASA", Some(answers.clone()));
    let (_, applicable) = asked_and_applicable(Record::Entry(&entry), answers);
    assert!(!applicable.contains(&"route"));
    assert!(!Casa::new().counts(&entry));
}

#[test]
fn another_authoritys_information_does_not_make_a_session_count() {
    let entry = session("OTHER", Some(json!({"_version": 2, "approved_fstd": true})));
    assert!(!Casa::new().counts(&entry));
}

/// Terms, each `(id, name)`.
type Terms = Vec<(String, String)>;

/// What an aircraft of this description and these characteristics counts as
/// and requires, each `(id, name)`, and its cautions.
fn classified(
    airframe: AirframeKind,
    engines: u8,
    engine_type: EngineType,
    characteristics: &[AircraftCharacteristic],
) -> (Terms, Terms, Vec<String>) {
    let classification = Casa::new()
        .classify(
            &AircraftDescription::new(airframe, engines, engine_type),
            &characteristics.iter().copied().collect::<BTreeSet<_>>(),
        )
        .unwrap();
    let terms = |terms: Vec<logbook_shared_kernel::Term>| {
        terms
            .into_iter()
            .map(|term| (term.id.as_str().to_owned(), term.name))
            .collect()
    };
    (
        terms(classification.counts_as),
        terms(classification.requires),
        classification.cautions,
    )
}

/// Regs 61.015 and 61.020, and 61.007(2) for a registered sailplane.
#[rstest]
#[case(AirframeKind::LandPlane, 1, &[("aeroplane", "aeroplane"), ("single-engine-aeroplane", "single-engine aeroplane")])]
#[case(AirframeKind::LandPlane, 2, &[("aeroplane", "aeroplane"), ("multi-engine-aeroplane", "multi-engine aeroplane")])]
#[case(AirframeKind::Helicopter, 1, &[("helicopter", "helicopter"), ("single-engine-helicopter", "single-engine helicopter")])]
#[case(AirframeKind::Helicopter, 2, &[("helicopter", "helicopter")])]
#[case(AirframeKind::Gyrocopter, 1, &[("gyroplane", "gyroplane"), ("single-engine-gyroplane", "single-engine gyroplane")])]
#[case(AirframeKind::TiltRotor, 2, &[("powered-lift", "powered-lift aircraft"), ("powered-lift-aircraft", "powered-lift aircraft")])]
#[case(AirframeKind::Airship, 2, &[("airship", "airship"), ("airship-class", "airship")])]
#[case(AirframeKind::Glider, 0, &[("registered-sailplane", "registered sailplane")])]
fn a_type_counts_as_its_category_and_class(
    #[case] airframe: AirframeKind,
    #[case] engines: u8,
    #[case] counts_as: &[(&str, &str)],
) {
    let (terms, _, cautions) = classified(airframe, engines, EngineType::Piston, &[]);
    let expected: Vec<_> = counts_as
        .iter()
        .map(|&(id, name)| (id.to_owned(), name.to_owned()))
        .collect();
    assert_eq!(terms, expected);
    assert!(cautions.is_empty());
}

/// The class's aircraft class rating (reg 61.735), under an id of its own,
/// is required; an aircraft with no class requires none. Type ratings are not
/// required yet (#101).
#[rstest]
#[case(AirframeKind::LandPlane, 1, Some(("single-engine-aeroplane-class-rating", "single-engine aeroplane class rating")))]
#[case(AirframeKind::LandPlane, 2, Some(("multi-engine-aeroplane-class-rating", "multi-engine aeroplane class rating")))]
#[case(AirframeKind::Helicopter, 1, Some(("single-engine-helicopter-class-rating", "single-engine helicopter class rating")))]
#[case(AirframeKind::Helicopter, 2, None)]
#[case(AirframeKind::Gyrocopter, 1, Some(("single-engine-gyroplane-class-rating", "single-engine gyroplane class rating")))]
#[case(AirframeKind::Gyrocopter, 2, None)]
#[case(AirframeKind::TiltRotor, 2, Some(("powered-lift-aircraft-class-rating", "powered-lift aircraft class rating")))]
#[case(AirframeKind::Airship, 2, Some(("airship-class-rating", "airship class rating")))]
#[case(AirframeKind::Glider, 0, None)]
fn a_type_requires_its_class_rating(
    #[case] airframe: AirframeKind,
    #[case] engines: u8,
    #[case] class_rating: Option<(&str, &str)>,
) {
    let (_, requires, _) = classified(airframe, engines, EngineType::Piston, &[]);
    let expected: Terms = class_rating
        .into_iter()
        .map(|(id, name)| (id.to_owned(), name.to_owned()))
        .collect();
    assert_eq!(requires, expected);
}

/// The ids of the design feature endorsements among what is required: all
/// but the class rating.
fn design_feature_ids(required: &Terms) -> Vec<&str> {
    required
        .iter()
        .map(|(id, _)| id.as_str())
        .filter(|id| !id.ends_with("-class-rating"))
        .collect()
}

/// Reg 61.755's design features for each category, each from the
/// characteristic recorded for it; one its category does not list requires
/// nothing. A piston engine, so that gas turbine engine is not required.
#[rstest]
#[case(
    AirframeKind::LandPlane,
    AircraftCharacteristic::TailwheelUndercarriage,
    Some("tailwheel-undercarriage")
)]
#[case(
    AirframeKind::LandPlane,
    AircraftCharacteristic::RetractableUndercarriage,
    Some("retractable-undercarriage")
)]
#[case(
    AirframeKind::LandPlane,
    AircraftCharacteristic::ManualPropellerPitchControl,
    Some("manual-propeller-pitch-control")
)]
#[case(
    AirframeKind::LandPlane,
    AircraftCharacteristic::MultiEngineCentreLineThrust,
    Some("multi-engine-centre-line-thrust")
)]
#[case(
    AirframeKind::LandPlane,
    AircraftCharacteristic::PressurisationSystem,
    Some("pressurisation-system")
)]
#[case(
    AirframeKind::SeaPlane,
    AircraftCharacteristic::FloatAlightingGear,
    Some("floatplane")
)]
#[case(
    AirframeKind::Amphibian,
    AircraftCharacteristic::FloatingHull,
    Some("floating-hull")
)]
#[case(
    AirframeKind::LandPlane,
    AircraftCharacteristic::SkiLandingGear,
    Some("ski-landing-gear")
)]
#[case(
    AirframeKind::Helicopter,
    AircraftCharacteristic::FloatAlightingGear,
    Some("float-alighting-gear")
)]
#[case(
    AirframeKind::Helicopter,
    AircraftCharacteristic::RetractableUndercarriage,
    Some("retractable-undercarriage")
)]
#[case(AirframeKind::Helicopter, AircraftCharacteristic::SkiLandingGear, None)]
#[case(
    AirframeKind::Helicopter,
    AircraftCharacteristic::PressurisationSystem,
    None
)]
#[case(
    AirframeKind::TiltRotor,
    AircraftCharacteristic::RetractableUndercarriage,
    Some("retractable-undercarriage")
)]
#[case(
    AirframeKind::TiltRotor,
    AircraftCharacteristic::PressurisationSystem,
    Some("pressurisation-system")
)]
#[case(
    AirframeKind::Gyrocopter,
    AircraftCharacteristic::RetractableUndercarriage,
    Some("retractable-undercarriage")
)]
#[case(
    AirframeKind::Gyrocopter,
    AircraftCharacteristic::PressurisationSystem,
    Some("pressurisation-system")
)]
#[case(
    AirframeKind::Gyrocopter,
    AircraftCharacteristic::TailwheelUndercarriage,
    None
)]
#[case(
    AirframeKind::Airship,
    AircraftCharacteristic::PressurisationSystem,
    Some("pressurisation-system")
)]
#[case(
    AirframeKind::Airship,
    AircraftCharacteristic::RetractableUndercarriage,
    None
)]
#[case(
    AirframeKind::Glider,
    AircraftCharacteristic::RetractableUndercarriage,
    None
)]
fn a_recorded_characteristic_requires_its_categorys_design_feature(
    #[case] airframe: AirframeKind,
    #[case] characteristic: AircraftCharacteristic,
    #[case] requires: Option<&str>,
) {
    let (_, required, _) = classified(airframe, 1, EngineType::Piston, &[characteristic]);
    let ids = design_feature_ids(&required);
    assert_eq!(ids, Vec::from_iter(requires));
}

/// Gas turbine engine is read from the description, for each of reg
/// 61.015's categories, and the features come in reg 61.755's order.
#[rstest]
#[case(AirframeKind::LandPlane, &["tailwheel-undercarriage", "gas-turbine-engine", "ski-landing-gear"])]
#[case(AirframeKind::Helicopter, &["gas-turbine-engine"])]
#[case(AirframeKind::TiltRotor, &["gas-turbine-engine"])]
#[case(AirframeKind::Gyrocopter, &["gas-turbine-engine"])]
#[case(AirframeKind::Airship, &["gas-turbine-engine"])]
#[case(AirframeKind::Glider, &[])]
fn a_gas_turbine_engine_is_read_from_the_description(
    #[case] airframe: AirframeKind,
    #[case] requires: &[&str],
) {
    let recorded = [
        AircraftCharacteristic::SkiLandingGear,
        AircraftCharacteristic::TailwheelUndercarriage,
    ];
    let (_, required, _) = classified(airframe, 1, EngineType::TurboProp, &recorded);
    let ids = design_feature_ids(&required);
    assert_eq!(ids, requires);
}

/// Each design feature is named in reg 61.755's words.
#[test]
fn a_design_feature_is_named_as_the_regulation_words_it() {
    let (_, required, _) = classified(
        AirframeKind::LandPlane,
        1,
        EngineType::Piston,
        &[AircraftCharacteristic::ManualPropellerPitchControl],
    );
    assert_eq!(
        required[1].1,
        "manual propeller pitch control (piston engine)"
    );
}

/// Reg 61.755(2)(c) is manual propeller pitch control "(piston engine)":
/// with any other engine, recording it requires nothing.
#[rstest]
#[case(EngineType::Piston, &["manual-propeller-pitch-control"])]
#[case(EngineType::TurboProp, &["gas-turbine-engine"])]
#[case(EngineType::None, &[])]
fn manual_propeller_pitch_control_is_required_only_with_a_piston_engine(
    #[case] engine_type: EngineType,
    #[case] requires: &[&str],
) {
    let (_, required, _) = classified(
        AirframeKind::LandPlane,
        1,
        engine_type,
        &[AircraftCharacteristic::ManualPropellerPitchControl],
    );
    let ids = design_feature_ids(&required);
    assert_eq!(ids, requires);
}

/// A category's and a class's id is its stored name, as the requirements'
/// ids have it, but for the airship class's.
#[test]
fn the_ids_are_the_stored_names() {
    use crate::aircraft::{AircraftCategory as Category, AircraftClassRating as Class};
    use crate::logbook::{category_id, class_id};
    let stored = |value: Value| value.as_str().unwrap().to_owned();
    for category in [
        Category::Aeroplane,
        Category::Helicopter,
        Category::PoweredLift,
        Category::Gyroplane,
        Category::Airship,
        Category::RegisteredSailplane,
    ] {
        assert_eq!(category_id(category), stored(json!(category)));
    }
    for class in [
        Class::SingleEngineAeroplane,
        Class::MultiEngineAeroplane,
        Class::SingleEngineHelicopter,
        Class::PoweredLiftAircraft,
        Class::SingleEngineGyroplane,
    ] {
        assert_eq!(class_id(class), stored(json!(class)));
    }
    assert_eq!(class_id(Class::Airship), "airship-class");
}

/// Reg 61.020(2)(a), au-casa#13.
#[test]
fn a_centre_line_thrust_aeroplane_counts_as_single_engine() {
    let (counts_as, required, _) = classified(
        AirframeKind::LandPlane,
        2,
        EngineType::Piston,
        &[AircraftCharacteristic::MultiEngineCentreLineThrust],
    );
    assert_eq!(counts_as[1].0, "single-engine-aeroplane");
    let ids: Vec<_> = required.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "single-engine-aeroplane-class-rating",
            "multi-engine-centre-line-thrust"
        ]
    );
}

/// A seaplane or amphibian's description cannot say whether it has float
/// alighting gear or a floating hull, so it is cautioned until one is
/// recorded, and requires neither.
#[rstest]
#[case(AirframeKind::SeaPlane, &[], &[], true)]
#[case(AirframeKind::Amphibian, &[AircraftCharacteristic::RetractableUndercarriage], &["retractable-undercarriage"], true)]
#[case(AirframeKind::SeaPlane, &[AircraftCharacteristic::FloatAlightingGear], &["floatplane"], false)]
#[case(AirframeKind::Amphibian, &[AircraftCharacteristic::FloatingHull], &["floating-hull"], false)]
#[case(AirframeKind::LandPlane, &[], &[], false)]
fn a_type_on_water_is_cautioned_until_its_gear_is_recorded(
    #[case] airframe: AirframeKind,
    #[case] recorded: &[AircraftCharacteristic],
    #[case] requires: &[&str],
    #[case] cautioned: bool,
) {
    let (_, required, cautions) = classified(airframe, 1, EngineType::Piston, recorded);
    let ids = design_feature_ids(&required);
    assert_eq!(ids, requires);
    assert_eq!(cautions.len(), usize::from(cautioned));
}

/// The keys of the questions about `record`, and of those that apply to it
/// given `answers`.
fn asked_and_applicable(record: Record, answers: Value) -> (Vec<&'static str>, Vec<&'static str>) {
    let questions = Casa::new().questions_for(record);
    let information = AuthoritySpecificInformation::new(answers);
    let keys = |questions: Vec<&Question>| questions.iter().map(|q| q.key).collect();
    (
        keys(questions.iter().collect()),
        keys(questions.applicable_for(record, Some(&information))),
    )
}

/// Every key of the entry format, in the order asked.
const ENTRY_KEYS: [&str; 7] = [
    "approved_fstd",
    "route",
    "receiving_flight_training",
    "cross_country_flight",
    "dme_or_gnss_arrivals",
    "azimuth_guidance_operation",
    "course_deviation_indicator_operation",
];

#[test]
fn only_whether_it_was_cross_country_and_the_arrivals_apply_to_a_flight() {
    let entry = Entry::new(flight_parts()).unwrap();
    let questions = Casa::new().questions_for(Record::Entry(&entry));
    assert_eq!(questions.version(), EntryInformation::VERSION);
    let (asked, applicable) = asked_and_applicable(Record::Entry(&entry), json!({}));
    assert_eq!(asked, ENTRY_KEYS);
    assert_eq!(applicable, FLIGHT_KEYS);
}

/// The questions that apply to every flight.
const FLIGHT_KEYS: [&str; 2] = ["cross_country_flight", "dme_or_gnss_arrivals"];

#[test]
fn whether_its_device_was_approved_applies_to_a_session() {
    let entry = session("CASA", None);
    let (asked, applicable) = asked_and_applicable(Record::Entry(&entry), json!({}));
    assert_eq!(asked, ENTRY_KEYS);
    assert_eq!(applicable, ["approved_fstd", "dme_or_gnss_arrivals"]);
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
    let (_, applicable) = asked_and_applicable(Record::Entry(&entry), json!({}));
    assert_eq!(applicable, GUIDANCE_KEYS);
}

/// The questions that apply to a flight with an approach or an arrival:
/// whether it was cross-country, the arrivals, and the guidance operations
/// of reg 61.870(6) and (7) after them.
const GUIDANCE_KEYS: [&str; 4] = [
    "cross_country_flight",
    "dme_or_gnss_arrivals",
    "azimuth_guidance_operation",
    "course_deviation_indicator_operation",
];

/// A DME or GNSS arrival is an instrument approach operation (Table 61.935,
/// item 20), so the guidance operations apply to an entry that records one
/// and no approach.
#[rstest]
#[case::one(json!({"dme_or_gnss_arrivals": 1}), &GUIDANCE_KEYS)]
#[case::two(json!({"dme_or_gnss_arrivals": 2}), &GUIDANCE_KEYS)]
#[case::none(json!({"dme_or_gnss_arrivals": 0}), &FLIGHT_KEYS)]
#[case::unanswered(json!({}), &FLIGHT_KEYS)]
fn arrivals_are_asked_about_as_approaches_are(#[case] answers: Value, #[case] expected: &[&str]) {
    let entry = Entry::new(flight_parts()).unwrap();
    let (_, applicable) = asked_and_applicable(Record::Entry(&entry), answers);
    assert_eq!(applicable, expected);
}

/// The host drops an answer to a question that no longer applies to the
/// entry, and keeps one to a question that still does.
#[test]
fn the_host_drops_only_the_answers_that_no_longer_apply() {
    let answers = json!({
        "_version": 2,
        "approved_fstd": true,
        "azimuth_guidance_operation": true,
        "course_deviation_indicator_operation": true,
        "dme_or_gnss_arrivals": 2,
    });
    let kept = |entry: &Entry, answers: &Value| {
        let mut information = AuthoritySpecificInformation::new(answers.clone());
        Casa::new()
            .questions_for(Record::Entry(entry))
            .retain_applicable_answers_for(Record::Entry(entry), &mut information);
        information.value().clone()
    };
    let flight = Entry::new(flight_parts()).unwrap();
    assert_eq!(
        kept(&flight, &answers),
        json!({
            "_version": 2,
            "azimuth_guidance_operation": true,
            "course_deviation_indicator_operation": true,
            "dme_or_gnss_arrivals": 2,
        })
    );
    let mut no_arrivals = answers.clone();
    no_arrivals
        .as_object_mut()
        .unwrap()
        .remove("dme_or_gnss_arrivals");
    assert_eq!(kept(&flight, &no_arrivals), json!({"_version": 2}));
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
    assert_eq!(kept(&session_with_an_approach, &answers), answers);
}

/// Reg 61.095(1)(c), (2) and (3).
#[rstest]
#[case::unanswered(json!({}), &["route", "cross_country_flight", "dme_or_gnss_arrivals"])]
#[case::operator(
    json!({"route": "operator"}),
    &["route", "cross_country_flight", "dme_or_gnss_arrivals"]
)]
#[case::instructor(
    json!({"route": "instructor"}),
    &["route", "receiving_flight_training", "cross_country_flight", "dme_or_gnss_arrivals"]
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
    let (_, applicable) = asked_and_applicable(Record::Entry(&entry), answers);
    assert_eq!(applicable, expected);
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
        Answer::Choices { groups, each, .. } => json!([{
            "kind": groups[0].kind.value,
            "value": groups[0].choices[0].value,
            "answer": each[0].value,
        }]),
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
    for record in [
        Record::Entry(&entry),
        Record::Assessment {
            assessment: &assessment,
            entry: &entry,
        },
    ] {
        let questions = casa.questions_for(record);
        let mut answers = serde_json::Map::new();
        for question in &questions {
            answers.insert(question.key.into(), an_answer(question));
            let alone = json!({ question.key: an_answer(question) });
            assert!(casa
                .problems_for(record, &AuthoritySpecificInformation::new(alone))
                .is_empty());
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
        // Together, once the host has removed the answers to questions that
        // do not apply (an outcome of a flight test's own, for one).
        let mut information = AuthoritySpecificInformation::new(answers.into());
        questions.retain_applicable_answers_for(record, &mut information);
        assert!(casa.problems_for(record, &information).is_empty());
    }
}

/// Pins the arrivals question's wording: an operation recorded as an approach
/// and as an arrival would count twice towards reg 61.870(2), and a count
/// cannot be de-duplicated, so the question excludes the entry's approaches.
#[test]
fn the_dme_or_gnss_arrivals_question_excludes_the_entrys_approaches() {
    let entry = session("CASA", None);
    let questions = Casa::new().questions_for(Record::Entry(&entry));
    let arrivals = questions
        .iter()
        .find(|question| question.key == "dme_or_gnss_arrivals")
        .expect("a question for the arrivals");
    assert_eq!(
        arrivals.label,
        "How many instrument approach operations using distance measuring equipment or a \
         global navigation satellite system (DME or GNSS arrival procedure), not also \
         recorded in this entry's approaches, were conducted?"
    );
}

#[test]
fn a_flight_tests_purposes_are_asked_in_part_61s_words_and_only_failing_is_adverse() {
    let assessment = an_assessment();
    let entry = an_entry();
    let questions = Casa::new().questions_for(Record::Assessment {
        assessment: &assessment,
        entry: &entry,
    });
    let purposes = questions
        .iter()
        .find(|question| matches!(question.answer, Answer::Choices { .. }))
        .expect("a question for the purposes");
    assert_eq!(purposes.label, "What was the flight test for?");
    let Answer::Choices {
        groups,
        each,
        labels,
    } = &purposes.answer
    else {
        panic!("purposes are several choices");
    };
    assert_eq!(labels.item, "licence, rating or endorsement");
    assert_eq!(labels.answer, "Passed or failed");
    let adverse = |choices: &[Choice]| -> Vec<&str> {
        choices
            .iter()
            .filter(|choice| choice.adverse)
            .map(|choice| choice.value)
            .collect()
    };
    assert_eq!(adverse(each), ["failed"]);
    for question in &questions {
        match &question.answer {
            Answer::Choice(choices) if choices.iter().any(|c| c.value == "failed") => {
                assert_eq!(adverse(choices), ["failed"]);
            }
            Answer::Choice(choices) => assert!(adverse(choices).is_empty()),
            _ => {}
        }
    }
    let entry = Entry::new(flight_parts()).unwrap();
    for question in &Casa::new().questions_for(Record::Entry(&entry)) {
        match &question.answer {
            Answer::Choice(choices) => assert!(adverse(choices).is_empty()),
            Answer::Choices { groups, each, .. } => {
                assert!(adverse(each).is_empty());
                assert!(groups.iter().all(|g| adverse(&g.choices).is_empty()));
            }
            _ => {}
        }
    }
    for group in groups {
        assert!(adverse(&group.choices).is_empty());
    }
}

#[test]
fn an_assessment_is_asked_its_check_and_its_outcome_or_a_flight_tests_purposes() {
    let assessment = an_assessment();
    let entry = an_entry();
    let record = Record::Assessment {
        assessment: &assessment,
        entry: &entry,
    };
    let questions = Casa::new().questions_for(record);
    assert_eq!(questions.version(), AssessmentInformation::VERSION);
    let (asked, applicable) = asked_and_applicable(record, json!({"check_type": "flight_review"}));
    assert_eq!(asked, ["check_type", "outcome", "check_purposes"]);
    assert_eq!(applicable, ["check_type", "outcome"]);
    let (_, applicable) = asked_and_applicable(record, json!({"check_type": "flight_test"}));
    assert_eq!(applicable, ["check_type", "check_purposes"]);
    let required = |answers| -> Vec<_> {
        questions
            .required_unanswered_for(record, Some(&AuthoritySpecificInformation::new(answers)))
            .into_iter()
            .map(|q| q.key)
            .collect()
    };
    assert_eq!(required(json!({})), ["check_type"]);
    assert_eq!(
        required(json!({"check_type": "flight_review"})),
        ["outcome"]
    );
    assert_eq!(
        required(json!({"check_type": "flight_test"})),
        ["check_purposes"]
    );
}

/// The stored order is part of format v2: the licences in reg 61.010's
/// order, an air transport pilot licence before a private pilot licence.
#[test]
fn the_purposes_are_stored_in_their_regulations_order() {
    let atpl =
        json!({"kind": "licence", "value": "air_transport_pilot_licence", "answer": "passed"});
    let ppl = json!({"kind": "licence", "value": "private_pilot_licence", "answer": "passed"});
    assert!(read_assessment(json!({"check_purposes": [atpl, ppl]})).is_ok());
    assert_eq!(
        read_assessment(json!({"check_purposes": [ppl, atpl]})),
        Err(AssessmentInformationError::InvalidAnswer("check_purposes"))
    );
}

/// An answer to a question that does not apply (a flight test's own
/// outcome, another check's purposes) is read as stored, not refused; the
/// checks read the information without it, as the host saves it.
#[test]
fn an_answer_its_check_does_not_take_is_read_as_stored() {
    let purposes =
        json!([{"kind": "licence", "value": "private_pilot_licence", "answer": "passed"}]);
    let test = read_assessment(json!({
        "check_type": "flight_test",
        "outcome": "failed",
        "check_purposes": purposes,
    }))
    .unwrap();
    assert_eq!(test.outcome, Some(CheckOutcome::Failed));
    let review = read_assessment(json!({
        "check_type": "flight_review",
        "outcome": "failed",
        "check_purposes": purposes,
    }))
    .unwrap();
    assert_eq!(review.check_purposes.len(), 1);
}

/// What the questions accept as a flight test's purposes is what the format
/// reads, so the host saves nothing CASA cannot read: each purpose alone,
/// and the purposes in their order, but not out of it.
#[test]
fn the_purposes_question_accepts_what_the_format_reads() {
    let assessment = an_assessment();
    let entry = an_entry();
    let questions = Casa::new().questions_for(Record::Assessment {
        assessment: &assessment,
        entry: &entry,
    });
    let question = questions
        .iter()
        .find(|question| question.key == "check_purposes")
        .unwrap();
    let mut all = Vec::new();
    for purpose in CheckPurpose::ALL {
        let information = AssessmentInformation {
            check_purposes: vec![AssessedPurpose {
                purpose,
                outcome: CheckOutcome::Passed,
            }],
            ..AssessmentInformation::default()
        };
        let stored = information.to_information().into_value()["check_purposes"].clone();
        assert!(question.answer.accepts(&stored), "{purpose:?}");
        all.push(stored[0].clone());
    }
    assert!(question.answer.accepts(&Value::Array(all.clone())));
    all.reverse();
    assert!(!question.answer.accepts(&Value::Array(all)));
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
        &AuthoritySpecificInformation::new(json!({"_version": 3, "approved_fstd": true})),
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
fn each_requirement_has_its_result_and_each_total_its_time() {
    let casa = Casa::new();
    let logbook = Logbook {
        entries: vec![Entry::new(flight_parts()).unwrap()],
        assessments: vec![an_assessment()],
    };
    let requirements = casa.requirements();
    assert_eq!(requirements.len(), 26);
    assert_eq!(
        requirements[0].name,
        "Carrying a passenger by day in an aeroplane: at least 3 take-offs, by day or night"
    );
    assert_eq!(requirements[0].regulation, "61.395(1)");
    assert_eq!(requirements[0].id.as_str(), "61.395(1)(a)");
    assert_eq!(requirements[1].id.as_str(), "61.395(1)(b)");
    assert_eq!(requirements[1].regulation, "61.395(1)");
    let answered: Vec<_> = casa
        .currency(date!(2026 - 11 - 06), &logbook)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let ids: Vec<_> = requirements.into_iter().map(|r| r.id).collect();
    assert_eq!(answered, ids);
    // The hour in a C172.
    let totals: Vec<_> = casa
        .totals(&logbook, DateRange::default())
        .into_iter()
        .map(|total| (total.id.as_str().to_owned(), total.time.total_minutes()))
        .collect();
    assert_eq!(totals[0], ("aeroplane".to_owned(), 60));
    assert_eq!(totals.len(), 7);
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
        stored(json!({"_version": 2, "approved_fstd": true}))
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
    assert!(problems(json!({"_version": 2, "route": "operator"})).is_empty());
    let refused = problems(json!({"_version": 3}));
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
        cross_country_flight: Some(true),
    };
    let stored = json!({
        "approved_fstd": true,
        "route": "instructor",
        "receiving_flight_training": false,
        "azimuth_guidance_operation": true,
        "course_deviation_indicator_operation": false,
        "dme_or_gnss_arrivals": 2,
        "cross_country_flight": true,
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
    assert_eq!(EntryInformation::VERSION, 2);
    let information = read(json!({"_version": 2, "approved_fstd": true})).unwrap();
    assert_eq!(information.approved_fstd, Some(true));
}

#[rstest]
#[case::not_an_object(json!([]), InformationError::NotAnObject)]
#[case::another_version(json!({"_version": 3}), InformationError::UnsupportedVersion)]
#[case::version_as_text(json!({"_version": "2"}), InformationError::UnsupportedVersion)]
#[case::unknown_key(json!({"notes": "x"}), InformationError::UnknownKey("notes".into()))]
#[case::yes_no_as_text(json!({"approved_fstd": "yes"}), InformationError::InvalidAnswer("approved_fstd"))]
#[case::unknown_route(json!({"route": "pilot"}), InformationError::InvalidAnswer("route"))]
#[case::count_as_decimal(json!({"dme_or_gnss_arrivals": 2.0}), InformationError::InvalidAnswer("dme_or_gnss_arrivals"))]
#[case::negative_count(json!({"dme_or_gnss_arrivals": -1}), InformationError::InvalidAnswer("dme_or_gnss_arrivals"))]
fn what_is_not_this_format_is_refused(#[case] stored: Value, #[case] error: InformationError) {
    assert_eq!(read(stored), Err(error));
}

/// Migrated from version 1, as its value.
fn migrated(value: Value) -> Result<Value, EntryMigrationError> {
    migrate_entry_information_from_version_1(&AuthoritySpecificInformation::new(value))
        .map(AuthoritySpecificInformation::into_value)
}

#[test]
fn version_1_is_restamped_with_every_answer_kept() {
    let answers = json!({
        "approved_fstd": true,
        "route": "instructor",
        "receiving_flight_training": false,
        "azimuth_guidance_operation": true,
        "course_deviation_indicator_operation": null,
        "dme_or_gnss_arrivals": 2,
    });
    let mut version_1 = answers.clone();
    version_1["_version"] = 1.into();
    let mut version_2 = answers;
    version_2["_version"] = 2.into();
    assert_eq!(migrated(version_1), Ok(version_2.clone()));
    assert!(EntryInformation::read(&AuthoritySpecificInformation::new(version_2)).is_ok());
    assert_eq!(migrated(json!({"_version": 1})), Ok(json!({"_version": 2})));
}

#[test]
fn version_2_is_given_back_as_it_is() {
    let version_2 = json!({"_version": 2, "approved_fstd": true, "cross_country_flight": true});
    assert_eq!(migrated(version_2.clone()), Ok(version_2));
}

#[rstest]
#[case::not_an_object(json!([]), EntryMigrationError::NotAnObject)]
#[case::no_version(json!({"approved_fstd": true}), EntryMigrationError::UnsupportedVersion)]
#[case::another_version(json!({"_version": 3}), EntryMigrationError::UnsupportedVersion)]
#[case::a_key_version_1_has_not(
    json!({"_version": 1, "cross_country_flight": true}),
    EntryMigrationError::UnknownKey("cross_country_flight".into())
)]
#[case::version_1_badly_typed(
    json!({"_version": 1, "approved_fstd": "yes"}),
    EntryMigrationError::InvalidAnswer("approved_fstd")
)]
#[case::version_2_badly_typed(
    json!({"_version": 2, "cross_country_flight": "yes"}),
    EntryMigrationError::InvalidAnswer("cross_country_flight")
)]
fn what_reads_as_neither_version_is_refused(
    #[case] stored: Value,
    #[case] error: EntryMigrationError,
) {
    assert_eq!(migrated(stored), Err(error));
}

/// An assessment in [`an_entry`].
fn an_assessment() -> Assessment {
    Assessment {
        id: AssessmentId::new("a1"),
        date: date!(2026 - 11 - 06),
        entry: EntryId::new("e1"),
        authority_information: BTreeMap::new(),
    }
}

/// The entry `e1`, a flight.
fn an_entry() -> Entry {
    Entry::new(flight_parts()).unwrap()
}

/// Read as information on an assessment about to be saved.
fn read_assessment(value: Value) -> Result<AssessmentInformation, AssessmentInformationError> {
    AssessmentInformation::read_to_save(&AuthoritySpecificInformation::new(value))
}

#[test]
fn information_on_an_assessment_that_cannot_be_read_is_refused_before_it_is_saved() {
    let casa = Casa::new();
    let assessment = an_assessment();
    let entry = an_entry();
    let problems = |value| {
        casa.problems_for(
            Record::Assessment {
                assessment: &assessment,
                entry: &entry,
            },
            &AuthoritySpecificInformation::new(value),
        )
    };
    assert!(problems(json!({"outcome": "passed"})).is_empty());
    assert!(problems(json!({"_version": 2, "outcome": "passed"})).is_empty());
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
        stored(json!({"_version": 2, "outcome": "passed"}))
            .unwrap()
            .outcome,
        Some(CheckOutcome::Passed)
    );
    assert_eq!(AssessmentInformation::VERSION, 2);
}

/// Regs 61.745(3) and (3A).
#[rstest]
#[case(CheckType::FlightTest, "flight_test")]
#[case(CheckType::FlightReview, "flight_review")]
#[case(CheckType::OperatorProficiencyCheck, "operator_proficiency_check")]
#[case(CheckType::InstrumentProficiencyCheck, "instrument_proficiency_check")]
#[case(
    CheckType::NightVisionImagingSystemProficiencyCheck,
    "night_vision_imaging_system_proficiency_check"
)]
#[case(
    CheckType::AerialApplicationProficiencyCheck,
    "aerial_application_proficiency_check"
)]
#[case(CheckType::InstructorProficiencyCheck, "instructor_proficiency_check")]
#[case(CheckType::ExaminerProficiencyCheck, "examiner_proficiency_check")]
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

/// A flight test's purposes, each stored with its own outcome.
#[rstest]
#[case(
    CheckPurpose::RecreationalPilotLicence,
    "licence",
    "recreational_pilot_licence"
)]
#[case(CheckPurpose::PrivatePilotLicence, "licence", "private_pilot_licence")]
#[case(
    CheckPurpose::CommercialPilotLicence,
    "licence",
    "commercial_pilot_licence"
)]
#[case(
    CheckPurpose::MultiCrewPilotLicence,
    "licence",
    "multi_crew_pilot_licence"
)]
#[case(
    CheckPurpose::AirTransportPilotLicence,
    "licence",
    "air_transport_pilot_licence"
)]
#[case(
    CheckPurpose::AeroplaneCategoryRating,
    "category_rating",
    "aeroplane_category_rating"
)]
#[case(
    CheckPurpose::HelicopterCategoryRating,
    "category_rating",
    "helicopter_category_rating"
)]
#[case(
    CheckPurpose::PoweredLiftAircraftCategoryRating,
    "category_rating",
    "powered_lift_aircraft_category_rating"
)]
#[case(
    CheckPurpose::GyroplaneCategoryRating,
    "category_rating",
    "gyroplane_category_rating"
)]
#[case(
    CheckPurpose::AirshipCategoryRating,
    "category_rating",
    "airship_category_rating"
)]
#[case(
    CheckPurpose::SingleEngineAeroplaneClassRating,
    "class_rating",
    "single_engine_aeroplane_class_rating"
)]
#[case(
    CheckPurpose::MultiEngineAeroplaneClassRating,
    "class_rating",
    "multi_engine_aeroplane_class_rating"
)]
#[case(
    CheckPurpose::SingleEngineHelicopterClassRating,
    "class_rating",
    "single_engine_helicopter_class_rating"
)]
#[case(
    CheckPurpose::PoweredLiftAircraftClassRating,
    "class_rating",
    "powered_lift_aircraft_class_rating"
)]
#[case(
    CheckPurpose::SingleEngineGyroplaneClassRating,
    "class_rating",
    "single_engine_gyroplane_class_rating"
)]
#[case(
    CheckPurpose::AirshipClassRating,
    "class_rating",
    "airship_class_rating"
)]
#[case(
    CheckPurpose::AerialApplicationRating,
    "operational_rating",
    "aerial_application_rating"
)]
#[case(CheckPurpose::ExaminerRating, "operational_rating", "examiner_rating")]
#[case(
    CheckPurpose::InstructorRating,
    "operational_rating",
    "instructor_rating"
)]
#[case(
    CheckPurpose::InstrumentRating,
    "operational_rating",
    "instrument_rating"
)]
#[case(CheckPurpose::LowLevelRating, "operational_rating", "low_level_rating")]
#[case(CheckPurpose::NightVfrRating, "operational_rating", "night_vfr_rating")]
#[case(
    CheckPurpose::NightVisionImagingSystemRating,
    "operational_rating",
    "night_vision_imaging_system_rating"
)]
#[case(
    CheckPurpose::PrivateInstrumentRating,
    "operational_rating",
    "private_instrument_rating"
)]
#[case(
    CheckPurpose::InstrumentEndorsement,
    "operational_endorsement",
    "instrument_endorsement"
)]
fn every_check_purpose_has_its_kind_and_spelling(
    #[case] purpose: CheckPurpose,
    #[case] kind: &str,
    #[case] spelt: &str,
) {
    let information = test_support::flight_test(&[(purpose, CheckOutcome::Failed)]);
    let stored = json!({
        "check_type": "flight_test",
        "check_purposes": [{"kind": kind, "value": spelt, "answer": "failed"}],
    });
    assert_eq!(information.to_information().into_value(), stored);
    assert_eq!(read_assessment(stored), Ok(information));
}

#[test]
fn every_check_purpose_is_one_of_the_list_once() {
    assert_eq!(CheckPurpose::ALL.len(), 25);
    for (at, purpose) in CheckPurpose::ALL.iter().enumerate() {
        assert!(!CheckPurpose::ALL[..at].contains(purpose), "{purpose:?}");
    }
}

/// The owner's multi-engine instrument rating test: passed for the class
/// rating and failed for the instrument rating, which come in the order of
/// the purposes, whatever order they are held in.
#[test]
fn each_purpose_is_stored_with_its_own_outcome_in_the_purposes_order() {
    let information = test_support::flight_test(&[
        (CheckPurpose::InstrumentRating, CheckOutcome::Failed),
        (
            CheckPurpose::MultiEngineAeroplaneClassRating,
            CheckOutcome::Passed,
        ),
    ]);
    let stored = json!({
        "check_type": "flight_test",
        "check_purposes": [
            {"kind": "class_rating", "value": "multi_engine_aeroplane_class_rating", "answer": "passed"},
            {"kind": "operational_rating", "value": "instrument_rating", "answer": "failed"},
        ],
    });
    assert_eq!(information.to_information().into_value(), stored);
    let read = read_assessment(stored).unwrap();
    assert_eq!(read.check_purposes.len(), 2);
    assert_eq!(read.to_information(), information.to_information());
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

const PURPOSES: AssessmentInformationError =
    AssessmentInformationError::InvalidAnswer("check_purposes");

#[rstest]
#[case::not_an_object(json!("passed"), AssessmentInformationError::NotAnObject)]
#[case::another_version(json!({"_version": 1}), AssessmentInformationError::UnsupportedVersion)]
#[case::version_1s_purpose(json!({"check_purpose": "instrument rating"}), AssessmentInformationError::UnknownKey("check_purpose".into()))]
#[case::version_1s_check_type(json!({"check_type": "flight review"}), AssessmentInformationError::InvalidAnswer("check_type"))]
#[case::outcome_as_yes_no(json!({"outcome": true}), AssessmentInformationError::InvalidAnswer("outcome"))]
#[case::no_purposes(json!({"check_purposes": []}), PURPOSES)]
#[case::not_a_list(json!({"check_purposes": {"kind": "operational_endorsement", "value": "instrument_endorsement", "answer": "passed"}}), PURPOSES)]
#[case::another_kind(json!({"check_purposes": [{"kind": "operational_rating", "value": "instrument_endorsement", "answer": "passed"}]}), PURPOSES)]
#[case::no_answer(json!({"check_purposes": [{"kind": "operational_endorsement", "value": "instrument_endorsement"}]}), PURPOSES)]
#[case::another_answer(json!({"check_purposes": [{"kind": "operational_endorsement", "value": "instrument_endorsement", "answer": "yes"}]}), PURPOSES)]
#[case::another_key(json!({"check_purposes": [{"kind": "operational_endorsement", "value": "instrument_endorsement", "answer": "passed", "on": "2024-01-01"}]}), PURPOSES)]
#[case::out_of_order(json!({"check_purposes": [
    {"kind": "operational_endorsement", "value": "instrument_endorsement", "answer": "passed"},
    {"kind": "licence", "value": "private_pilot_licence", "answer": "passed"},
]}), PURPOSES)]
#[case::twice(json!({"check_purposes": [
    {"kind": "licence", "value": "private_pilot_licence", "answer": "passed"},
    {"kind": "licence", "value": "private_pilot_licence", "answer": "failed"},
]}), PURPOSES)]
fn what_is_not_the_assessment_format_is_refused(
    #[case] stored: Value,
    #[case] error: AssessmentInformationError,
) {
    assert_eq!(read_assessment(stored), Err(error));
}

/// What `Casa` migrates `value` to, as an entry's or an assessment's
/// information: its value, or `None` when there is nothing to rewrite.
fn migrated_as(record: Record, value: Value) -> Result<Option<Value>, MigrationRefused> {
    Casa::new()
        .migrate_information(record, &AuthoritySpecificInformation::new(value))
        .map(|migrated| migrated.map(AuthoritySpecificInformation::into_value))
}

#[test]
fn an_entry_at_version_1_is_migrated_once() {
    let entry = session("CASA", None);
    let record = Record::Entry(&entry);
    let version_1 = json!({"_version": 1, "approved_fstd": true, "dme_or_gnss_arrivals": 2});
    let version_2 = json!({"_version": 2, "approved_fstd": true, "dme_or_gnss_arrivals": 2});
    let migrated = migrated_as(record, version_1).unwrap();
    assert_eq!(migrated, Some(version_2.clone()));
    assert_eq!(migrated_as(record, version_2), Ok(None));
}

#[test]
fn an_entry_at_version_2_is_current_without_its_answers_read() {
    let entry = session("CASA", None);
    assert_eq!(
        migrated_as(
            Record::Entry(&entry),
            json!({"_version": 2, "approved_fstd": "yes"})
        ),
        Ok(None)
    );
}

#[test]
fn an_assessment_at_version_2_is_current() {
    let assessment = an_assessment();
    let entry = an_entry();
    assert_eq!(
        migrated_as(
            Record::Assessment {
                assessment: &assessment,
                entry: &entry
            },
            json!({"_version": 2})
        ),
        Ok(None)
    );
}

#[rstest]
#[case::newer(json!({"_version": 3}), "newer")]
#[case::no_version(json!({"approved_fstd": true}), "unreadable")]
#[case::not_an_object(json!([]), "unreadable")]
#[case::version_0(json!({"_version": 0}), "too_old")]
fn what_cannot_be_migrated_is_refused_by_kind(#[case] stored: Value, #[case] kind: &str) {
    let entry = session("CASA", None);
    let assessment = an_assessment();
    for record in [
        Record::Entry(&entry),
        Record::Assessment {
            assessment: &assessment,
            entry: &entry,
        },
    ] {
        let refused = migrated_as(record, stored.clone()).unwrap_err();
        let expected = match kind {
            "newer" => matches!(refused, MigrationRefused::Newer(_)),
            "too_old" => matches!(refused, MigrationRefused::TooOld(_)),
            "unreadable" => matches!(refused, MigrationRefused::Unreadable(_)),
            other => unreachable!("no kind {other}"),
        };
        assert!(expected, "{refused:?} for {stored}");
    }
}

#[test]
fn an_entry_at_version_1_that_does_not_read_is_unreadable() {
    let entry = session("CASA", None);
    assert!(matches!(
        migrated_as(
            Record::Entry(&entry),
            json!({"_version": 1, "cross_country_flight": true})
        ),
        Err(MigrationRefused::Unreadable(_))
    ));
}

#[test]
fn an_assessment_at_version_1_is_too_old_as_none_is_kept() {
    let assessment = an_assessment();
    let entry = an_entry();
    assert!(matches!(
        migrated_as(
            Record::Assessment {
                assessment: &assessment,
                entry: &entry
            },
            json!({"_version": 1})
        ),
        Err(MigrationRefused::TooOld(_))
    ));
}
