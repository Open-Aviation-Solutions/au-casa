// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The titles of licence CASA issues, and its minimum ages, through
//! logbook-shared-kernel's `LicensingAuthority`.

use std::collections::BTreeSet;

use au_casa::logbook::Casa;
use au_casa::part61_term;
use icao_shared_kernel::{IssuedTitle, IssuingState, Licence, LicenceTitle, MinimumAges};
use logbook_shared_kernel::{
    HeldLicenceTitle, LicenceTitleId, LicenceTitleProblem, LicenceTitleTerm, LicensingAuthority,
    TermId,
};
use time::macros::date;

fn titles() -> Vec<LicenceTitleTerm> {
    Casa::new().licence_titles()
}

/// Each title's id and name, Annex 1's in its order and then the national
/// titles. The ids are kept by a host, so this list is what a release may not
/// change.
#[test]
fn lists_every_title_with_its_id_and_part61_name() {
    let listed: Vec<(String, String)> = titles()
        .iter()
        .map(|title| {
            (
                title.term().id.as_str().to_owned(),
                title.term().name.clone(),
            )
        })
        .collect();
    let expected = [
        (
            "private-aeroplane",
            "private pilot licence, aeroplane category rating",
        ),
        (
            "private-airship",
            "private pilot licence, airship category rating",
        ),
        (
            "private-helicopter",
            "private pilot licence, helicopter category rating",
        ),
        (
            "private-powered-lift",
            "private pilot licence, powered-lift aircraft category rating",
        ),
        (
            "commercial-aeroplane",
            "commercial pilot licence, aeroplane category rating",
        ),
        (
            "commercial-airship",
            "commercial pilot licence, airship category rating",
        ),
        (
            "commercial-helicopter",
            "commercial pilot licence, helicopter category rating",
        ),
        (
            "commercial-powered-lift",
            "commercial pilot licence, powered-lift aircraft category rating",
        ),
        (
            "multi-crew-aeroplane",
            "multi-crew pilot licence, aeroplane category rating",
        ),
        (
            "air-transport-aeroplane",
            "air transport pilot licence, aeroplane category rating",
        ),
        (
            "air-transport-helicopter",
            "air transport pilot licence, helicopter category rating",
        ),
        (
            "air-transport-powered-lift",
            "air transport pilot licence, powered-lift aircraft category rating",
        ),
        ("glider", "glider pilot licence"),
        (
            "commercial-gyroplane",
            "commercial pilot licence, gyroplane category rating",
        ),
        (
            "private-gyroplane",
            "private pilot licence, gyroplane category rating",
        ),
        (
            "recreational-aeroplane",
            "recreational pilot licence, aeroplane category rating",
        ),
        (
            "recreational-helicopter",
            "recreational pilot licence, helicopter category rating",
        ),
        (
            "recreational-gyroplane",
            "recreational pilot licence, gyroplane category rating",
        ),
    ]
    .map(|(id, name)| (id.to_owned(), name.to_owned()));
    assert_eq!(listed, expected);
}

#[test]
fn issues_every_annex_1_title_but_the_free_balloon() {
    let annex_1: Vec<LicenceTitle> = titles()
        .iter()
        .filter_map(|title| match title {
            LicenceTitleTerm::Annex1 { title, .. } => Some(*title),
            LicenceTitleTerm::National { .. } => None,
        })
        .collect();
    assert_eq!(annex_1.len(), 13);
    assert!(!annex_1.contains(&LicenceTitle::FreeBalloon));
    for title in &annex_1 {
        assert!(part61_term(*title).is_some());
    }
}

/// What a host relies on: no title is listed twice, and no two titles share
/// an id.
#[test]
fn no_two_titles_share_an_id() {
    let titles = titles();
    let ids: BTreeSet<TermId> = titles.iter().map(|title| title.term().id.clone()).collect();
    let titled: BTreeSet<LicenceTitleId> = titles.iter().map(LicenceTitleTerm::id).collect();
    assert_eq!(ids.len(), titles.len());
    assert_eq!(titled.len(), titles.len());
}

/// The national titles' minimum ages are their licences': regs 61.475(1),
/// 61.515(1) and 61.580(1).
#[test]
fn national_titles_carry_their_licences_minimum_ages() {
    let ages: Vec<(String, u8)> = titles()
        .iter()
        .filter_map(|title| match title {
            LicenceTitleTerm::National { term, minimum_age } => {
                Some((term.id.as_str().to_owned(), *minimum_age))
            }
            LicenceTitleTerm::Annex1 { .. } => None,
        })
        .collect();
    assert_eq!(
        ages,
        [
            ("commercial-gyroplane".to_owned(), 18),   // reg 61.580(1)
            ("private-gyroplane".to_owned(), 17),      // reg 61.515(1)
            ("recreational-aeroplane".to_owned(), 16), // reg 61.475(1)
            ("recreational-helicopter".to_owned(), 16),
            ("recreational-gyroplane".to_owned(), 16),
        ]
    );
}

/// Each age is Part 61's, held here, so a change to Annex 1's default cannot
/// move CASA's.
#[test]
fn minimum_ages_are_part61s() {
    assert_eq!(
        Casa::new().minimum_ages(),
        MinimumAges {
            private: 17,           // reg 61.515(1)
            commercial: 18,        // reg 61.580(1)
            multi_crew: 18,        // reg 61.655(1)
            airline_transport: 21, // reg 61.700(1) (air transport)
            glider: 16,            // reg 61.1540(1)
            // Annex 1's, unused: CASA issues no free balloon licence.
            free_balloon: 16,
        }
    );
}

/// The kernel's rules run over CASA's titles: an Annex 1 title CASA does not
/// issue, and an own title first issued before its minimum age.
#[test]
fn the_kernels_rules_run_over_casas_titles() {
    let licence = Licence::new(
        IssuingState::parse("AU").expect("a State"),
        "CASA",
        "1",
        vec![IssuedTitle::new(
            LicenceTitle::FreeBalloon,
            date!(2020 - 01 - 01),
        )],
    )
    .expect("a licence");
    let own = [
        HeldLicenceTitle::new(TermId::new("private-gyroplane"), date!(2020 - 05 - 01)),
        HeldLicenceTitle::new(TermId::new("recreational-gyroplane"), date!(2020 - 05 - 01)),
    ];
    // Born 2003-06-01: 16 on the first issue, so old enough for the
    // recreational title (16) and not the private one (17).
    let problems = Casa::new().licence_title_problems(&licence, &own, date!(2003 - 06 - 01));
    assert_eq!(
        problems,
        [
            LicenceTitleProblem::NotIssued(LicenceTitleId::Annex1(LicenceTitle::FreeBalloon)),
            LicenceTitleProblem::BeforeMinimumAge {
                title: TermId::new("private-gyroplane"),
                date_of_first_issue: date!(2020 - 05 - 01),
                minimum_age: 17,
            },
        ]
    );
}
