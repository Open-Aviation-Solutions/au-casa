//! Recognition of a flight simulation training device under reg 61.010.

use au_casa::{counts_for_part61, FstdRecognition, RecognisedForeignState};
use rstest::rstest;

/// Every sub-type of the reg 61.010 definition — the regulation draws no
/// distinction between them, so all five count.
#[rstest]
#[case(FstdRecognition::QualifiedFlightSimulator)] // 61.010(a)
#[case(FstdRecognition::QualifiedFlightTrainingDevice)] // 61.010(b)
#[case(FstdRecognition::SyntheticTrainerCao45)] // 61.010(c)
#[case(FstdRecognition::PrescribedUnderReg61045)] // 61.010(d)
#[case(FstdRecognition::ForeignStateQualified(RecognisedForeignState::NewZealand))] // 61.010(e)
fn every_sub_type_counts_for_part61(#[case] recognition: FstdRecognition) {
    assert!(counts_for_part61(Some(&recognition)));
}

#[test]
fn unrecognised_device_does_not_count() {
    // A personal simulator: loggable, but no credit. This is the absence of a
    // recognition, not a sentinel variant within the enum.
    assert!(!counts_for_part61(None));
}

#[rstest]
#[case(RecognisedForeignState::Canada, "canada")]
#[case(RecognisedForeignState::HongKong, "hong-kong")]
#[case(
    RecognisedForeignState::UnitedStatesOfAmerica,
    "united-states-of-america"
)]
#[case(RecognisedForeignState::CzechRepublic, "czech-republic")]
fn foreign_states_serialise_as_kebab_case(
    #[case] state: RecognisedForeignState,
    #[case] expected: &str,
) {
    assert_eq!(
        serde_json::to_string(&state).unwrap(),
        format!("\"{expected}\"")
    );
}

#[test]
fn foreign_state_recognition_round_trips_through_json() {
    let recognition = FstdRecognition::ForeignStateQualified(RecognisedForeignState::UnitedKingdom);
    let json = serde_json::to_string(&recognition).unwrap();
    let parsed: FstdRecognition = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, recognition);
}

#[test]
fn an_absent_recognition_round_trips_as_null() {
    let recognition: Option<FstdRecognition> = None;
    let json = serde_json::to_string(&recognition).unwrap();
    assert_eq!(json, "null");
    let parsed: Option<FstdRecognition> = serde_json::from_str(&json).unwrap();
    assert!(!counts_for_part61(parsed.as_ref()));
}
