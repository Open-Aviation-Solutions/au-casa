// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The titles of licence CASA issues, and the minimum ages it sets, for
//! logbook-shared-kernel's [`LicensingAuthority`](logbook_shared_kernel::LicensingAuthority).
//!
//! Annex 1's titles come from [`part61_term`], each under its Part 61 name;
//! the pairs Annex 1 has no title for are the [`NationalTitle`]s, each with
//! its licence's minimum age.
//!
//! Each title has a stable id, which a host keeps for the own titles (it
//! stores them with the licence), so once released one does not change: the
//! licence's and the category's stored names joined by `-`, as pilot-logbook
//! stores a title's pair (`recreational-aeroplane`, `private-gyroplane`), or
//! the licence's alone for the glider pilot licence, which has no category
//! rating (`glider`). A pair belongs to one licence and one category rating,
//! so no two titles share an id.

use icao_shared_kernel::{
    AirlineTransportCategory, LicenceTitle, MinimumAges, PrivateOrCommercialCategory,
};
use logbook_shared_kernel::{LicenceTitleTerm, Term, TermId};

use super::category_id;
use crate::licence::{part61_term, FlightCrewLicence, NationalTitle, Part61Term};

/// Every title of licence CASA issues: Annex 1's, each under its Part 61 name
/// and in Annex 1's order, and then the national titles.
pub(crate) fn licence_titles() -> Vec<LicenceTitleTerm> {
    let annex_1 = ANNEX_1_TITLES.into_iter().filter_map(|title| {
        part61_term(title).map(|part61| LicenceTitleTerm::Annex1 {
            title,
            term: term(part61),
        })
    });
    let national = NATIONAL_TITLES
        .into_iter()
        .map(|title| LicenceTitleTerm::National {
            term: term(title.part61_term()),
            minimum_age: title.minimum_age(),
        });
    annex_1.chain(national).collect()
}

/// CASA's minimum ages for Annex 1's titles: Annex 1's own. Private 17 (reg
/// 61.515(1)), commercial 18 (reg 61.580(1)), multi-crew 18 (reg 61.655(1)),
/// air transport 21 (reg 61.700(1)) and glider 16 (reg 61.1540(1)). The free
/// balloon's is Annex 1's 16, unused: CASA issues no free balloon licence.
pub(crate) fn minimum_ages() -> MinimumAges {
    MinimumAges::default()
}

/// Every title of the kernel, in Annex 1's order.
const ANNEX_1_TITLES: [LicenceTitle; 14] = {
    use LicenceTitle as T;
    type Pc = PrivateOrCommercialCategory;
    type At = AirlineTransportCategory;
    [
        T::Private(Pc::Aeroplane),
        T::Private(Pc::Airship),
        T::Private(Pc::Helicopter),
        T::Private(Pc::PoweredLift),
        T::Commercial(Pc::Aeroplane),
        T::Commercial(Pc::Airship),
        T::Commercial(Pc::Helicopter),
        T::Commercial(Pc::PoweredLift),
        T::MultiCrew,
        T::AirlineTransport(At::Aeroplane),
        T::AirlineTransport(At::Helicopter),
        T::AirlineTransport(At::PoweredLift),
        T::Glider,
        T::FreeBalloon,
    ]
};

/// Every national title, in the order `NationalTitle` declares them.
const NATIONAL_TITLES: [NationalTitle; 5] = [
    NationalTitle::CommercialGyroplane,
    NationalTitle::PrivateGyroplane,
    NationalTitle::RecreationalAeroplane,
    NationalTitle::RecreationalHelicopter,
    NationalTitle::RecreationalGyroplane,
];

/// A title's term: its id, and its Part 61 name.
fn term(part61: Part61Term) -> Term {
    Term {
        id: TermId::new(id(part61)),
        name: part61.to_string(),
    }
}

/// The licence's stored name and the category rating's, joined by `-`.
fn id(part61: Part61Term) -> String {
    let licence = licence_id(part61.licence);
    match part61.category_rating {
        Some(category) => format!("{licence}-{}", category_id(category)),
        None => licence.to_owned(),
    }
}

/// A licence's stored name, as pilot-logbook spells it (`air-transport` for
/// what the kernel calls airline transport, in Part 61's words).
fn licence_id(licence: FlightCrewLicence) -> &'static str {
    match licence {
        FlightCrewLicence::AirTransport => "air-transport",
        FlightCrewLicence::Commercial => "commercial",
        FlightCrewLicence::MultiCrew => "multi-crew",
        FlightCrewLicence::Private => "private",
        FlightCrewLicence::Recreational => "recreational",
        FlightCrewLicence::Glider => "glider",
    }
}
