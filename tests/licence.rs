// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The Part 61 term for each of the kernel's Annex 1 titles of licence.

use au_casa::{part61_term, AircraftCategory, FlightCrewLicence, Part61Term};
use icao_shared_kernel::{AirlineTransportCategory, LicenceTitle, PrivateOrCommercialCategory};
use rstest::rstest;

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
