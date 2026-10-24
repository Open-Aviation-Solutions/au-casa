// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The Part 61 term for each of the kernel's Annex 1 titles of licence, and the
//! national titles.

use au_casa::{
    part61_term, AircraftCategory, FlightCrewLicence, IssuedNationalTitle, NationalTitle,
    Part61Term,
};
use icao_shared_kernel::{AirlineTransportCategory, LicenceTitle, PrivateOrCommercialCategory};
use rstest::rstest;
use time::macros::date;

use AirlineTransportCategory as Atpl;
use PrivateOrCommercialCategory as PrivOrComm;

/// Every title CASA issues, with its licence, category rating, citations and
/// the term a host shows.
#[rstest]
#[case(LicenceTitle::Private(PrivOrComm::Aeroplane), FlightCrewLicence::Private, Some(AircraftCategory::Aeroplane), &["reg 61.525", "reg 61.545"], "private pilot licence, aeroplane category rating")]
#[case(LicenceTitle::Private(PrivOrComm::Helicopter), FlightCrewLicence::Private, Some(AircraftCategory::Helicopter), &["reg 61.530", "reg 61.550"], "private pilot licence, helicopter category rating")]
#[case(LicenceTitle::Private(PrivOrComm::PoweredLift), FlightCrewLicence::Private, Some(AircraftCategory::PoweredLift), &["reg 61.555"], "private pilot licence, powered-lift aircraft category rating")]
#[case(LicenceTitle::Private(PrivOrComm::Airship), FlightCrewLicence::Private, Some(AircraftCategory::Airship), &["reg 61.565"], "private pilot licence, airship category rating")]
#[case(LicenceTitle::Commercial(PrivOrComm::Aeroplane), FlightCrewLicence::Commercial, Some(AircraftCategory::Aeroplane), &["reg 61.590", "reg 61.610"], "commercial pilot licence, aeroplane category rating")]
#[case(LicenceTitle::Commercial(PrivOrComm::Helicopter), FlightCrewLicence::Commercial, Some(AircraftCategory::Helicopter), &["reg 61.595", "reg 61.615"], "commercial pilot licence, helicopter category rating")]
#[case(LicenceTitle::Commercial(PrivOrComm::PoweredLift), FlightCrewLicence::Commercial, Some(AircraftCategory::PoweredLift), &["reg 61.600", "reg 61.620"], "commercial pilot licence, powered-lift aircraft category rating")]
#[case(LicenceTitle::Commercial(PrivOrComm::Airship), FlightCrewLicence::Commercial, Some(AircraftCategory::Airship), &["reg 61.630"], "commercial pilot licence, airship category rating")]
#[case(LicenceTitle::MultiCrew, FlightCrewLicence::MultiCrew, Some(AircraftCategory::Aeroplane), &["reg 61.635", "reg 61.660"], "multi-crew pilot licence, aeroplane category rating")]
#[case(LicenceTitle::AirlineTransport(Atpl::Aeroplane), FlightCrewLicence::AirTransport, Some(AircraftCategory::Aeroplane), &["reg 61.665", "reg 61.705"], "air transport pilot licence, aeroplane category rating")]
#[case(LicenceTitle::AirlineTransport(Atpl::Helicopter), FlightCrewLicence::AirTransport, Some(AircraftCategory::Helicopter), &["reg 61.665", "reg 61.710"], "air transport pilot licence, helicopter category rating")]
#[case(LicenceTitle::AirlineTransport(Atpl::PoweredLift), FlightCrewLicence::AirTransport, Some(AircraftCategory::PoweredLift), &["reg 61.665", "reg 61.715"], "air transport pilot licence, powered-lift aircraft category rating")]
#[case(LicenceTitle::Glider, FlightCrewLicence::Glider, None, &["reg 61.010", "Subpart 61.Z"], "glider pilot licence")]
fn casa_issues_title(
    #[case] title: LicenceTitle,
    #[case] licence: FlightCrewLicence,
    #[case] category_rating: Option<AircraftCategory>,
    #[case] citations: &'static [&'static str],
    #[case] shown: &str,
) {
    let term = part61_term(title).expect("CASA issues this title");
    assert_eq!(
        term,
        Part61Term {
            licence,
            category_rating,
            citations,
        }
    );
    assert_eq!(term.to_string(), shown);
}

#[test]
fn casa_does_not_issue_a_free_balloon_licence() {
    assert_eq!(part61_term(LicenceTitle::FreeBalloon), None);
}

/// Every national title, with its licence, category rating, citations and the
/// term a host shows.
#[rstest]
#[case(NationalTitle::RecreationalAeroplane, FlightCrewLicence::Recreational, AircraftCategory::Aeroplane, &["reg 61.475", "MOS Schedule 1, Appendix G.1"], "recreational pilot licence, aeroplane category rating")]
#[case(NationalTitle::RecreationalHelicopter, FlightCrewLicence::Recreational, AircraftCategory::Helicopter, &["reg 61.475", "MOS Schedule 1, Appendix G.2"], "recreational pilot licence, helicopter category rating")]
#[case(NationalTitle::RecreationalGyroplane, FlightCrewLicence::Recreational, AircraftCategory::Gyroplane, &["reg 61.475", "MOS Schedule 1, Appendix G.3"], "recreational pilot licence, gyroplane category rating")]
#[case(NationalTitle::PrivateGyroplane, FlightCrewLicence::Private, AircraftCategory::Gyroplane, &["reg 61.535", "reg 61.560"], "private pilot licence, gyroplane category rating")]
#[case(NationalTitle::CommercialGyroplane, FlightCrewLicence::Commercial, AircraftCategory::Gyroplane, &["reg 61.625"], "commercial pilot licence, gyroplane category rating")]
fn a_national_title_is_its_part61_pair(
    #[case] title: NationalTitle,
    #[case] licence: FlightCrewLicence,
    #[case] category: AircraftCategory,
    #[case] citations: &'static [&'static str],
    #[case] shown: &str,
) {
    assert_eq!(
        title.part61_term(),
        Part61Term {
            licence,
            category_rating: Some(category),
            citations,
        }
    );
    assert_eq!(title.to_string(), shown);
}

#[test]
fn every_licence_but_the_glider_pilot_licence_is_a_pilot_licence() {
    use FlightCrewLicence::*;
    for licence in [AirTransport, Commercial, MultiCrew, Private, Recreational] {
        assert!(licence.is_pilot_licence(), "{licence}");
    }
    assert!(!Glider.is_pilot_licence());
}

/// The minimum age is the licence's (regs 61.475(1), 61.515(1), 61.580(1)),
/// reached on the birthday.
#[rstest]
#[case(NationalTitle::RecreationalAeroplane, 16)]
#[case(NationalTitle::RecreationalHelicopter, 16)]
#[case(NationalTitle::RecreationalGyroplane, 16)]
#[case(NationalTitle::PrivateGyroplane, 17)]
#[case(NationalTitle::CommercialGyroplane, 18)]
fn a_national_title_is_held_from_its_licences_minimum_age(
    #[case] title: NationalTitle,
    #[case] age: i32,
) {
    let born = date!(2000 - 06 - 15);
    let first_issued = |on| IssuedNationalTitle::new(title, on);
    let birthday = born.replace_year(2000 + age).unwrap();
    assert!(first_issued(birthday).holder_of_age(born));
    assert!(!first_issued(birthday.previous_day().unwrap()).holder_of_age(born));
}
