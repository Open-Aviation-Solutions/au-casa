// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The ratings and endorsements CASA issues, for logbook-shared-kernel's
//! [`LicensingAuthority`](logbook_shared_kernel::LicensingAuthority).
//!
//! Each has a stable id, which a host may keep, so once released one does
//! not change. A rating or endorsement that `Casa::classify` requires has the
//! same term there, so a host matches a held one against what an aircraft
//! requires by equal ids: an aircraft class rating has its class's id
//! (`single-engine-aeroplane`, `airship-class`), as a design feature
//! endorsement has its design feature's (`tailwheel-undercarriage`). An
//! operational rating's id is its Part 61 name joined by `-`
//! (`instrument-rating`, `night-vfr-rating`).

use logbook_shared_kernel::Term;

use super::{class_id, class_name, term};
use crate::aircraft::{AircraftClassRating, DesignFeature};

/// The ratings and endorsements CASA issues, in Part 61's order: the
/// aircraft class ratings (Division 61.L.3, one for each class of reg
/// 61.020(1)), the design feature endorsements (reg 61.755, one for each
/// design feature), the instrument rating (Subpart 61.M), the private
/// instrument rating (Subpart 61.N) and the night VFR rating (Subpart 61.O).
pub(crate) fn ratings_and_endorsements() -> Vec<Term> {
    // TODO(#101): type ratings are not listed yet.
    let class_ratings = CLASSES.into_iter().map(class_rating);
    let design_feature_endorsements = DesignFeature::ALL
        .into_iter()
        .map(design_feature_endorsement);
    let operational_ratings = OPERATIONAL_RATINGS
        .into_iter()
        .map(|(id, name)| term(id, name));
    class_ratings
        .chain(design_feature_endorsements)
        .chain(operational_ratings)
        .collect()
}

/// The aircraft class rating for a class: its class's id, and its name as
/// Part 61 names one ("single-engine aeroplane class rating", reg 61.375(4)).
pub(crate) fn class_rating(class: AircraftClassRating) -> Term {
    term(
        class_id(class),
        format!("{} class rating", class_name(class)),
    )
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
    ("instrument-rating", "instrument rating"),
    ("private-instrument-rating", "private instrument rating"),
    ("night-vfr-rating", "night VFR rating"),
];

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
                "single-engine-aeroplane",
                "single-engine aeroplane class rating",
            ),
            (
                "multi-engine-aeroplane",
                "multi-engine aeroplane class rating",
            ),
            (
                "single-engine-helicopter",
                "single-engine helicopter class rating",
            ),
            (
                "powered-lift-aircraft",
                "powered-lift aircraft class rating",
            ),
            (
                "single-engine-gyroplane",
                "single-engine gyroplane class rating",
            ),
            ("airship-class", "airship class rating"),
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
            ("private-instrument-rating", "private instrument rating"),
            ("night-vfr-rating", "night VFR rating"),
        ]
        .into_iter()
        .map(|(id, name)| (id.to_owned(), name.to_owned()))
        .collect();
        assert_eq!(listed, expected);
    }

    /// A class's place in [`CLASSES`] and a design feature's in
    /// [`DesignFeature::ALL`]. Exhaustive, with no wildcard, so a variant
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

    #[test]
    fn every_class_and_design_feature_is_in_its_place() {
        for (at, class) in CLASSES.into_iter().enumerate() {
            assert_eq!(class_position(class), at);
        }
        for (at, feature) in DesignFeature::ALL.into_iter().enumerate() {
            assert_eq!(feature_position(feature), at);
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
}
