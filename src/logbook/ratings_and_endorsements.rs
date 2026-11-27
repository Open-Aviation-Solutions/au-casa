// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The ratings and endorsements CASA issues, for logbook-shared-kernel's
//! [`LicensingAuthority`](logbook_shared_kernel::LicensingAuthority).
//!
//! Each has a stable id, which a host may keep, so once released one does
//! not change. A rating or endorsement that `Casa::classify` requires has the
//! same term there, so a host matches a held one against what an aircraft
//! requires by equal ids. An aircraft class rating has an id of its own,
//! not its class's (which `counts_as` and the totals give the class), so
//! that no id names two things: its class's id followed by `-class-rating`
//! (`single-engine-aeroplane-class-rating`), but for the airship class
//! rating, `airship-class-rating`, as the airship class's id is already
//! `airship-class`. A design feature endorsement has its design feature's id
//! (`tailwheel-undercarriage`). An operational rating's id, and an
//! instrument endorsement's, is its Part 61 name in lower case joined by `-`
//! (`instrument-rating`, `night-vfr-rating`,
//! `single-engine-aeroplane-instrument-endorsement`,
//! `iap-2d-instrument-endorsement`).

use logbook_shared_kernel::{
    LicenceProblem, LicenceProblemAt, LicenceTitleId, ProblemKind, Term, TermId,
};

use super::licence_titles::pilot_licences_with_category;
use super::{class_name, term};
use crate::aircraft::{AircraftCategory, AircraftClassRating, DesignFeature};

/// The ratings and endorsements CASA issues, in Part 61's order: the
/// aircraft class ratings (Division 61.L.3, one for each class of reg
/// 61.020(1)), the design feature endorsements (reg 61.755, one for each
/// design feature), the instrument rating (Subpart 61.M) and its instrument
/// endorsements (table 61.890), the private instrument rating (Subpart 61.N)
/// and the night VFR rating (Subpart 61.O).
pub(crate) fn ratings_and_endorsements() -> Vec<Term> {
    // TODO(#101): type ratings are not listed yet.
    let class_ratings = CLASSES.into_iter().map(class_rating);
    let design_feature_endorsements = DesignFeature::ALL
        .into_iter()
        .map(design_feature_endorsement);
    let [instrument_rating, operational_ratings @ ..] =
        OPERATIONAL_RATINGS.map(|(id, name)| term(id, name));
    let instrument_endorsements = InstrumentEndorsement::ALL
        .into_iter()
        .map(|endorsement| term(endorsement.id(), endorsement.name()));
    class_ratings
        .chain(design_feature_endorsements)
        .chain([instrument_rating])
        .chain(instrument_endorsements)
        .chain(operational_ratings)
        .collect()
}

/// The aircraft class rating for a class: its id, and its name as Part 61
/// names one ("single-engine aeroplane class rating", reg 61.375(4)).
pub(crate) fn class_rating(class: AircraftClassRating) -> Term {
    term(
        class_rating_id(class),
        format!("{} class rating", class_name(class)),
    )
}

/// A class rating's stable id, which a host may keep: its class's id
/// followed by `-class-rating`, but for the airship class's, whose class id
/// already ends in `-class`.
pub(crate) fn class_rating_id(class: AircraftClassRating) -> &'static str {
    match class {
        AircraftClassRating::SingleEngineAeroplane => "single-engine-aeroplane-class-rating",
        AircraftClassRating::MultiEngineAeroplane => "multi-engine-aeroplane-class-rating",
        AircraftClassRating::SingleEngineHelicopter => "single-engine-helicopter-class-rating",
        AircraftClassRating::PoweredLiftAircraft => "powered-lift-aircraft-class-rating",
        AircraftClassRating::SingleEngineGyroplane => "single-engine-gyroplane-class-rating",
        AircraftClassRating::Airship => "airship-class-rating",
    }
}

/// The design feature endorsement for a design feature: its feature's id and
/// its name, as reg 61.755 words it.
pub(crate) fn design_feature_endorsement(feature: DesignFeature) -> Term {
    term(feature.id(), feature.name())
}

/// Every class of reg 61.020(1), in its order.
const CLASSES: [AircraftClassRating; 6] = [
    AircraftClassRating::SingleEngineAeroplane,
    AircraftClassRating::MultiEngineAeroplane,
    AircraftClassRating::SingleEngineHelicopter,
    AircraftClassRating::PoweredLiftAircraft,
    AircraftClassRating::SingleEngineGyroplane,
    AircraftClassRating::Airship,
];

/// The operational ratings listed (reg 61.010), each `(id, name)`: those
/// pilot-logbook reports on, the instrument rating (reg 61.855), the private
/// instrument rating (reg 61.910) and the night VFR rating (reg 61.955).
const OPERATIONAL_RATINGS: [(&str, &str); 3] = [
    (INSTRUMENT_RATING, "instrument rating"),
    ("private-instrument-rating", "private instrument rating"),
    (NIGHT_VFR_RATING, "night VFR rating"),
];

/// The instrument rating's id.
pub(crate) const INSTRUMENT_RATING: &str = "instrument-rating";

/// The night VFR rating's id.
pub(crate) const NIGHT_VFR_RATING: &str = "night-vfr-rating";

/// The kinds of instrument endorsement (reg 61.890), on an instrument
/// rating: Part 1 of table 61.890, the aircraft category and class
/// endorsements, and Part 2, the instrument approach endorsements. The
/// private instrument endorsements (table 61.935) are on the private
/// instrument rating, and are not listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstrumentEndorsement {
    SingleEngineAeroplane,
    MultiEngineAeroplane,
    SingleEngineHelicopter,
    MultiEngineHelicopter,
    PoweredLiftAircraft,
    Gyroplane,
    Airship,
    Iap2D,
    Iap3D,
}

impl InstrumentEndorsement {
    /// Each, in table 61.890's order.
    pub(crate) const ALL: [Self; 9] = [
        Self::SingleEngineAeroplane,
        Self::MultiEngineAeroplane,
        Self::SingleEngineHelicopter,
        Self::MultiEngineHelicopter,
        Self::PoweredLiftAircraft,
        Self::Gyroplane,
        Self::Airship,
        Self::Iap2D,
        Self::Iap3D,
    ];

    /// Its stable id, which a host may keep: its name in lower case, joined
    /// by `-`.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::SingleEngineAeroplane => "single-engine-aeroplane-instrument-endorsement",
            Self::MultiEngineAeroplane => "multi-engine-aeroplane-instrument-endorsement",
            Self::SingleEngineHelicopter => "single-engine-helicopter-instrument-endorsement",
            Self::MultiEngineHelicopter => "multi-engine-helicopter-instrument-endorsement",
            Self::PoweredLiftAircraft => "powered-lift-aircraft-instrument-endorsement",
            Self::Gyroplane => "gyroplane-instrument-endorsement",
            Self::Airship => "airship-instrument-endorsement",
            Self::Iap2D => "iap-2d-instrument-endorsement",
            Self::Iap3D => "iap-3d-instrument-endorsement",
        }
    }

    /// Its name, as column 1 of table 61.890 gives it.
    fn name(self) -> &'static str {
        match self {
            Self::SingleEngineAeroplane => "single-engine aeroplane instrument endorsement",
            Self::MultiEngineAeroplane => "multi-engine aeroplane instrument endorsement",
            Self::SingleEngineHelicopter => "single-engine helicopter instrument endorsement",
            Self::MultiEngineHelicopter => "multi-engine helicopter instrument endorsement",
            Self::PoweredLiftAircraft => "powered-lift aircraft instrument endorsement",
            Self::Gyroplane => "gyroplane instrument endorsement",
            Self::Airship => "airship instrument endorsement",
            Self::Iap2D => "IAP 2D instrument endorsement",
            Self::Iap3D => "IAP 3D instrument endorsement",
        }
    }

    /// The category of the aircraft it authorises its holder to pilot under
    /// the IFR (column 2 of Part 1 of table 61.890); `None` for an
    /// instrument approach endorsement (Part 2).
    pub(crate) fn category(self) -> Option<AircraftCategory> {
        match self {
            Self::SingleEngineAeroplane | Self::MultiEngineAeroplane => {
                Some(AircraftCategory::Aeroplane)
            }
            Self::SingleEngineHelicopter | Self::MultiEngineHelicopter => {
                Some(AircraftCategory::Helicopter)
            }
            Self::PoweredLiftAircraft => Some(AircraftCategory::PoweredLift),
            Self::Gyroplane => Some(AircraftCategory::Gyroplane),
            Self::Airship => Some(AircraftCategory::Airship),
            Self::Iap2D | Self::Iap3D => None,
        }
    }
}

/// The problems with what is held under one licence: its `titles` and the
/// ratings and endorsements (`held`). A warning at the first title of each
/// category with class ratings (reg 61.020(1)) when none of the category's
/// class ratings is held (reg 61.375(3); see [`class_rating_warnings`]); and
/// a refusal at the instrument rating for each part of table 61.890
/// none of whose instrument endorsements is held with it, as an applicant
/// for the rating must meet the requirements for the grant of at least one
/// from each (reg 61.885(1)(b)(i) and (ii)). A missing endorsement is
/// reported at the rating it is required for, not at the licence as the
/// kernel's contract has something missing reported (au-casa#108 decided
/// so). The private instrument rating is not checked, as its endorsements
/// (table 61.935) are not listed (#113).
pub(crate) fn licence_problems(titles: &[LicenceTitleId], held: &[TermId]) -> Vec<LicenceProblem> {
    let mut problems = instrument_rating_problems(held);
    problems.extend(class_rating_warnings(titles, held));
    problems
}

/// A warning for each category with class ratings, in order of the first
/// title given in it, when none of the category's class ratings is held. A
/// class rating is held under the licence, not with a title, so one warning
/// serves every title in the category (a private and a commercial pilot
/// licence, aeroplane category rating, need one aeroplane class rating
/// between them), and it is reported at the first. Reg 61.375(3) and (5) only
/// limit where the licence's privileges are exercised, and a type rating
/// serves in a type-rated aircraft, so it is a warning (au-casa#112); type
/// ratings are not listed yet (#101), so one does not count.
fn class_rating_warnings(titles: &[LicenceTitleId], held: &[TermId]) -> Vec<LicenceProblem> {
    let mut warned = Vec::new();
    let mut warnings = Vec::new();
    for (title, part61) in pilot_licences_with_category(titles) {
        let Some(category) = part61.category_rating else {
            continue;
        };
        let class_ratings = || {
            CLASSES
                .into_iter()
                .filter(|class| class_category(*class) == category)
        };
        if warned.contains(&category)
            || class_ratings().next().is_none()
            || class_ratings().any(|class| held.contains(&TermId::new(class_rating_id(class))))
        {
            continue;
        }
        warned.push(category);
        warnings.push(LicenceProblem::new(
            LicenceProblemAt::Title(title),
            ProblemKind::Warning,
            format!(
                "Other than in an aircraft mentioned in reg 61.375(5), the holder of a pilot \
                 licence is authorised to exercise its privileges in an aircraft only if the \
                 holder also holds \"an appropriate aircraft class rating for the aircraft\". \
                 No {category} class rating is recorded with the {part61}. Record the class \
                 rating or pilot type rating held."
            ),
            "61.375(3)",
        ));
    }
    warnings
}

/// The category a class belongs to (reg 61.020(1)).
fn class_category(class: AircraftClassRating) -> AircraftCategory {
    match class {
        AircraftClassRating::SingleEngineAeroplane | AircraftClassRating::MultiEngineAeroplane => {
            AircraftCategory::Aeroplane
        }
        AircraftClassRating::SingleEngineHelicopter => AircraftCategory::Helicopter,
        AircraftClassRating::PoweredLiftAircraft => AircraftCategory::PoweredLift,
        AircraftClassRating::SingleEngineGyroplane => AircraftCategory::Gyroplane,
        AircraftClassRating::Airship => AircraftCategory::Airship,
    }
}

fn instrument_rating_problems(held: &[TermId]) -> Vec<LicenceProblem> {
    let instrument_rating = TermId::new(INSTRUMENT_RATING);
    if !held.contains(&instrument_rating) {
        return Vec::new();
    }
    let holds_one = |part_1: bool| {
        InstrumentEndorsement::ALL
            .into_iter()
            .filter(|endorsement| endorsement.category().is_some() == part_1)
            .any(|endorsement| held.contains(&TermId::new(endorsement.id())))
    };
    [
        (true, PART_1_MISSING, PART_1_LEAVES_OUT, "61.885(1)(b)(i)"),
        (false, PART_2_MISSING, PART_2_LEAVES_OUT, "61.885(1)(b)(ii)"),
    ]
    .into_iter()
    .filter(|(part_1, _, _, _)| !holds_one(*part_1))
    .map(|(_, text, leaves_out, regulation)| {
        LicenceProblem::new(
            LicenceProblemAt::RatingOrEndorsement(instrument_rating.clone()),
            ProblemKind::Refusal,
            text,
            regulation,
        )
        .with_report_leaves_out(leaves_out)
    })
    .collect()
}

/// What the report leaves out while no Part 1 endorsement is held: reg
/// 61.870(3) and reg 61.880's checks apply to the holder of one (see
/// `CasaRequirement::applies_to`).
const PART_1_LEAVES_OUT: &str = "The report leaves out reg 61.870(3) and the instrument \
     proficiency check until the endorsements are recorded.";

/// The same for Part 2: regs 61.870(4) and (5) and 61.900(2) apply to the
/// holder of an IAP 2D or IAP 3D instrument endorsement.
const PART_2_LEAVES_OUT: &str = "The report leaves out regs 61.870(4) and (5) and 61.900(2) \
     until the endorsement is recorded.";

/// The refusal of an instrument rating held with no instrument endorsement
/// from Part 1 of table 61.890, quoting reg 61.885(1)(b)(i).
const PART_1_MISSING: &str = "An instrument rating is granted only to a pilot who meets \"the \
     requirements for the grant of: (i) at least one instrument endorsement mentioned in column 1 \
     of an item in Part 1 of table 61.890\". Record the instrument endorsement held.";

/// The same for Part 2, quoting reg 61.885(1)(b)(ii).
const PART_2_MISSING: &str = "An instrument rating is granted only to a pilot who meets \"the \
     requirements for the grant of: (ii) at least one instrument endorsement mentioned in column \
     1 of an item in Part 2 of table 61.890\". Record the instrument endorsement held.";

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use aircraft_type_descriptions::{AircraftDescription, AirframeKind, EngineType};
    use logbook_shared_kernel::{AircraftCharacteristic, LicensingAuthority};

    use super::*;
    use crate::Casa;

    /// Each term as `(id, name)`.
    fn ids_and_names(terms: Vec<Term>) -> Vec<(String, String)> {
        terms
            .into_iter()
            .map(|term| (term.id.as_str().to_owned(), term.name))
            .collect()
    }

    /// Every one, in Part 61's order, under its id and its Part 61 name.
    #[test]
    fn casa_lists_its_ratings_and_endorsements() {
        let listed = ids_and_names(Casa::new().ratings_and_endorsements());
        let expected: Vec<_> = [
            (
                "single-engine-aeroplane-class-rating",
                "single-engine aeroplane class rating",
            ),
            (
                "multi-engine-aeroplane-class-rating",
                "multi-engine aeroplane class rating",
            ),
            (
                "single-engine-helicopter-class-rating",
                "single-engine helicopter class rating",
            ),
            (
                "powered-lift-aircraft-class-rating",
                "powered-lift aircraft class rating",
            ),
            (
                "single-engine-gyroplane-class-rating",
                "single-engine gyroplane class rating",
            ),
            ("airship-class-rating", "airship class rating"),
            ("tailwheel-undercarriage", "tailwheel undercarriage"),
            ("retractable-undercarriage", "retractable undercarriage"),
            (
                "manual-propeller-pitch-control",
                "manual propeller pitch control (piston engine)",
            ),
            ("gas-turbine-engine", "gas turbine engine"),
            (
                "multi-engine-centre-line-thrust",
                "multi-engine centre-line thrust",
            ),
            ("pressurisation-system", "pressurisation system"),
            ("floatplane", "floatplane"),
            ("floating-hull", "floating hull"),
            ("ski-landing-gear", "ski landing gear"),
            ("float-alighting-gear", "float alighting gear"),
            ("instrument-rating", "instrument rating"),
            (
                "single-engine-aeroplane-instrument-endorsement",
                "single-engine aeroplane instrument endorsement",
            ),
            (
                "multi-engine-aeroplane-instrument-endorsement",
                "multi-engine aeroplane instrument endorsement",
            ),
            (
                "single-engine-helicopter-instrument-endorsement",
                "single-engine helicopter instrument endorsement",
            ),
            (
                "multi-engine-helicopter-instrument-endorsement",
                "multi-engine helicopter instrument endorsement",
            ),
            (
                "powered-lift-aircraft-instrument-endorsement",
                "powered-lift aircraft instrument endorsement",
            ),
            (
                "gyroplane-instrument-endorsement",
                "gyroplane instrument endorsement",
            ),
            (
                "airship-instrument-endorsement",
                "airship instrument endorsement",
            ),
            (
                "iap-2d-instrument-endorsement",
                "IAP 2D instrument endorsement",
            ),
            (
                "iap-3d-instrument-endorsement",
                "IAP 3D instrument endorsement",
            ),
            ("private-instrument-rating", "private instrument rating"),
            ("night-vfr-rating", "night VFR rating"),
        ]
        .into_iter()
        .map(|(id, name)| (id.to_owned(), name.to_owned()))
        .collect();
        assert_eq!(listed, expected);
    }

    /// A class's place in [`CLASSES`], a design feature's in
    /// [`DesignFeature::ALL`] and an instrument endorsement's in
    /// [`InstrumentEndorsement::ALL`]. Exhaustive, with no wildcard, so a variant
    /// added does not compile until it is given a place here, which is
    /// where it goes in its array; the test holds each array to them.
    fn class_position(class: AircraftClassRating) -> usize {
        use AircraftClassRating as C;
        match class {
            C::SingleEngineAeroplane => 0,
            C::MultiEngineAeroplane => 1,
            C::SingleEngineHelicopter => 2,
            C::PoweredLiftAircraft => 3,
            C::SingleEngineGyroplane => 4,
            C::Airship => 5,
        }
    }

    fn feature_position(feature: DesignFeature) -> usize {
        use DesignFeature as F;
        match feature {
            F::TailwheelUndercarriage => 0,
            F::RetractableUndercarriage => 1,
            F::ManualPropellerPitchControl => 2,
            F::GasTurbineEngine => 3,
            F::MultiEngineCentreLineThrust => 4,
            F::PressurisationSystem => 5,
            F::Floatplane => 6,
            F::FloatingHull => 7,
            F::SkiLandingGear => 8,
            F::FloatAlightingGear => 9,
        }
    }

    fn instrument_endorsement_position(endorsement: InstrumentEndorsement) -> usize {
        use InstrumentEndorsement as E;
        match endorsement {
            E::SingleEngineAeroplane => 0,
            E::MultiEngineAeroplane => 1,
            E::SingleEngineHelicopter => 2,
            E::MultiEngineHelicopter => 3,
            E::PoweredLiftAircraft => 4,
            E::Gyroplane => 5,
            E::Airship => 6,
            E::Iap2D => 7,
            E::Iap3D => 8,
        }
    }

    #[test]
    fn every_class_design_feature_and_instrument_endorsement_is_in_its_place() {
        for (at, class) in CLASSES.into_iter().enumerate() {
            assert_eq!(class_position(class), at);
        }
        for (at, feature) in DesignFeature::ALL.into_iter().enumerate() {
            assert_eq!(feature_position(feature), at);
        }
        for (at, endorsement) in InstrumentEndorsement::ALL.into_iter().enumerate() {
            assert_eq!(instrument_endorsement_position(endorsement), at);
        }
    }

    /// A class rating's id is its own: no listed id is a class's, nor a
    /// category's, which `counts_as` and the totals give.
    #[test]
    fn no_listed_id_is_a_class_or_a_category() {
        use crate::aircraft::AircraftCategory as Category;
        use crate::logbook::{category_id, class_id};
        let listed: Vec<_> = Casa::new()
            .ratings_and_endorsements()
            .into_iter()
            .map(|term| term.id)
            .collect();
        let categories = [
            Category::Aeroplane,
            Category::Helicopter,
            Category::PoweredLift,
            Category::Gyroplane,
            Category::Airship,
            Category::RegisteredSailplane,
        ]
        .map(category_id);
        for id in CLASSES.map(class_id).into_iter().chain(categories) {
            assert!(!listed.iter().any(|listed| listed.as_str() == id), "{id}");
        }
    }

    /// What the kernel says a host may rely on: no id is listed twice, none
    /// is also a title of licence's, and every term `classify` requires,
    /// for any description and characteristics, is listed as it is.
    #[test]
    fn the_list_keeps_the_kernels_promises() {
        let casa = Casa::new();
        let listed = casa.ratings_and_endorsements();
        let ids: HashSet<_> = listed.iter().map(|term| term.id.clone()).collect();
        assert_eq!(ids.len(), listed.len());
        for title in casa.licence_titles() {
            assert!(!ids.contains(&title.term().id), "{:?}", title.term().id);
        }

        let airframes = [
            AirframeKind::LandPlane,
            AirframeKind::SeaPlane,
            AirframeKind::Amphibian,
            AirframeKind::Helicopter,
            AirframeKind::Gyrocopter,
            AirframeKind::TiltRotor,
            AirframeKind::Airship,
            AirframeKind::Glider,
        ];
        let every_characteristic: BTreeSet<_> = [
            AircraftCharacteristic::TailwheelUndercarriage,
            AircraftCharacteristic::RetractableUndercarriage,
            AircraftCharacteristic::ManualPropellerPitchControl,
            AircraftCharacteristic::MultiEngineCentreLineThrust,
            AircraftCharacteristic::PressurisationSystem,
            AircraftCharacteristic::FloatAlightingGear,
            AircraftCharacteristic::FloatingHull,
            AircraftCharacteristic::SkiLandingGear,
        ]
        .into();
        for airframe in airframes {
            for engines in [1, 2] {
                for engine_type in [EngineType::Piston, EngineType::TurboProp] {
                    for characteristics in [BTreeSet::new(), every_characteristic.clone()] {
                        let description = AircraftDescription::new(airframe, engines, engine_type);
                        let classification = casa.classify(&description, &characteristics).unwrap();
                        for required in classification.requires {
                            assert!(listed.contains(&required), "{required:?}");
                        }
                    }
                }
            }
        }
    }

    fn held(ids: &[&str]) -> Vec<TermId> {
        ids.iter().copied().map(TermId::new).collect()
    }

    /// Each part of table 61.890 with none of its instrument endorsements
    /// held is refused at the instrument rating, in the table's order.
    #[test]
    fn an_instrument_rating_without_an_endorsement_from_each_part_is_refused() {
        let refused = |text: &str, regulation| {
            LicenceProblem::new(
                LicenceProblemAt::RatingOrEndorsement(TermId::new(INSTRUMENT_RATING)),
                ProblemKind::Refusal,
                text,
                regulation,
            )
            .with_report_leaves_out(if regulation == "61.885(1)(b)(i)" {
                "The report leaves out reg 61.870(3) and the instrument proficiency check \
                 until the endorsements are recorded."
            } else {
                "The report leaves out regs 61.870(4) and (5) and 61.900(2) until the \
                 endorsement is recorded."
            })
        };
        let part_1 = refused(
            "An instrument rating is granted only to a pilot who meets \"the requirements for \
             the grant of: (i) at least one instrument endorsement mentioned in column 1 of an \
             item in Part 1 of table 61.890\". Record the instrument endorsement held.",
            "61.885(1)(b)(i)",
        );
        let part_2 = refused(
            "An instrument rating is granted only to a pilot who meets \"the requirements for \
             the grant of: (ii) at least one instrument endorsement mentioned in column 1 of an \
             item in Part 2 of table 61.890\". Record the instrument endorsement held.",
            "61.885(1)(b)(ii)",
        );
        let problems = |ids: &[&str]| Casa::new().licence_problems(&[], &held(ids));
        assert_eq!(
            problems(&[INSTRUMENT_RATING]),
            [part_1.clone(), part_2.clone()]
        );
        assert_eq!(
            problems(&[INSTRUMENT_RATING, "iap-3d-instrument-endorsement"]),
            [part_1]
        );
        assert_eq!(
            problems(&[INSTRUMENT_RATING, "gyroplane-instrument-endorsement"]),
            [part_2]
        );
    }

    /// Each refusal's note names requirements the report really leaves out
    /// of those that apply to the holder, and the others it does not.
    #[test]
    fn each_refusals_note_names_what_the_report_leaves_out() {
        let casa = Casa::new();
        let applying = |ids: &[&str]| -> Vec<String> {
            casa.requirements_for(&held(ids).into_iter().collect())
                .into_iter()
                .map(|requirement| requirement.id.as_str().to_owned())
                .collect()
        };
        let any_of = |ids: &[String], prefix: &str| ids.iter().any(|id| id.starts_with(prefix));
        let notes = |ids: &[&str]| -> Vec<Option<String>> {
            casa.licence_problems(&[], &held(ids))
                .into_iter()
                .map(|problem| problem.report_leaves_out)
                .collect()
        };
        let part_1 = Some(PART_1_LEAVES_OUT.to_owned());
        let part_2 = Some(PART_2_LEAVES_OUT.to_owned());

        // Neither part: all of them are left out.
        let bare = applying(&[INSTRUMENT_RATING]);
        for prefix in ["61.870(3)", "61.880", "61.870(4)", "61.870(5)", "61.900(2)"] {
            assert!(!any_of(&bare, prefix), "{prefix}");
        }
        assert_eq!(
            notes(&[INSTRUMENT_RATING]),
            [part_1.clone(), part_2.clone()]
        );

        // Part 2 held, Part 1 missing: the Part 1 note's requirements are left out.
        let ids = ["iap-2d-instrument-endorsement"];
        let with_part_2 = applying(&[INSTRUMENT_RATING, ids[0]]);
        assert!(!any_of(&with_part_2, "61.870(3)"));
        assert!(!any_of(&with_part_2, "61.880"));
        assert!(any_of(&with_part_2, "61.870(4)"));
        assert_eq!(notes(&[INSTRUMENT_RATING, ids[0]]), [part_1]);

        // Part 1 held, Part 2 missing: the Part 2 note's are left out, and
        // the Part 1 ones are not.
        let ids = ["single-engine-aeroplane-instrument-endorsement"];
        let with_part_1 = applying(&[INSTRUMENT_RATING, ids[0]]);
        assert!(any_of(&with_part_1, "61.870(3)"));
        assert!(any_of(&with_part_1, "61.880"));
        for prefix in ["61.870(4)", "61.870(5)", "61.900(2)"] {
            assert!(!any_of(&with_part_1, prefix), "{prefix}");
        }
        assert_eq!(notes(&[INSTRUMENT_RATING, ids[0]]), [part_2]);
    }

    /// One endorsement from each part is enough, whichever they are; and
    /// with no instrument rating there is nothing to refuse.
    #[test]
    fn one_endorsement_from_each_part_or_no_instrument_rating_has_no_problem() {
        let casa = Casa::new();
        for endorsement in InstrumentEndorsement::ALL {
            let other = if endorsement.category().is_some() {
                InstrumentEndorsement::Iap2D
            } else {
                InstrumentEndorsement::Airship
            };
            let ids = [INSTRUMENT_RATING, endorsement.id(), other.id()];
            assert_eq!(
                casa.licence_problems(&[], &held(&ids)),
                [],
                "{endorsement:?}"
            );
        }
        assert_eq!(casa.licence_problems(&[], &held(&[])), []);
        assert_eq!(
            casa.licence_problems(&[], &held(&["private-instrument-rating", NIGHT_VFR_RATING])),
            []
        );
    }

    /// One warning for a category with none of its class ratings held, at
    /// its first title and quoting reg 61.375(3); none once one is held,
    /// whichever class, or for a title without class ratings.
    #[test]
    fn a_pilot_licence_without_a_class_rating_in_its_category_is_warned_once() {
        use icao_shared_kernel::{LicenceTitle, PrivateOrCommercialCategory as Pc};

        let annex_1 = |title| LicenceTitleId::Annex1(title);
        let private = annex_1(LicenceTitle::Private(Pc::Aeroplane));
        let commercial = annex_1(LicenceTitle::Commercial(Pc::Aeroplane));
        let helicopter = annex_1(LicenceTitle::Private(Pc::Helicopter));
        let casa = Casa::new();

        let warned = casa.licence_problems(&[commercial.clone(), private.clone()], &[]);
        assert_eq!(
            warned,
            [LicenceProblem::new(
                LicenceProblemAt::Title(commercial.clone()),
                ProblemKind::Warning,
                "Other than in an aircraft mentioned in reg 61.375(5), the holder of a pilot \
                 licence is authorised to exercise its privileges in an aircraft only if the \
                 holder also holds \"an appropriate aircraft class rating for the aircraft\". \
                 No aeroplane class rating is recorded with the commercial pilot licence, \
                 aeroplane category rating. Record the class rating or pilot type rating held.",
                "61.375(3)",
            )]
        );

        let both = casa.licence_problems(&[private.clone(), helicopter.clone()], &[]);
        assert_eq!(both.len(), 2);
        let multi = held(&["multi-engine-aeroplane-class-rating"]);
        let rest = casa.licence_problems(&[private.clone(), helicopter.clone()], &multi);
        assert_eq!(rest, [both[1].clone()]);

        // The multi-crew pilot licence is not warned; the air transport pilot
        // licence is, even with it.
        let multi_crew = annex_1(LicenceTitle::MultiCrew);
        assert_eq!(
            casa.licence_problems(std::slice::from_ref(&multi_crew), &[]),
            []
        );
        let air_transport = annex_1(LicenceTitle::AirlineTransport(
            icao_shared_kernel::AirlineTransportCategory::Aeroplane,
        ));
        let with_mpl = casa.licence_problems(&[multi_crew, air_transport.clone()], &[]);
        assert_eq!(with_mpl.len(), 1);
        assert_eq!(with_mpl[0].at, LicenceProblemAt::Title(air_transport));

        let glider = annex_1(LicenceTitle::Glider);
        assert_eq!(casa.licence_problems(&[glider], &[]), []);
    }
}
