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
//! Each title has a stable id, which a host may keep, so once released one
//! does not change: the licence's name and the category rating's joined by
//! `-`, or the licence's alone for the glider pilot licence, which has no
//! category rating (`glider`). The five national titles' ids are
//! pilot-logbook's stored licence and category pair joined
//! (`recreational-aeroplane`, `private-gyroplane`). Annex 1's follow Part 61's
//! names (`air-transport-aeroplane`, where pilot-logbook stores
//! `airline-transport`) and are not stored by pilot-logbook, which stores the
//! kernel's `LicenceTitle`. A pair belongs to one licence and one category
//! rating, so no two titles share an id.

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

/// A licence's name in an id: Part 61's (`air-transport`, where the kernel
/// says airline transport). For the recreational, private and commercial
/// pilot licences it is also pilot-logbook's stored spelling.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A title's place in [`ANNEX_1_TITLES`]. Exhaustive, with no wildcard,
    /// so a title the kernel adds does not compile until it is listed.
    fn annex_1_position(title: LicenceTitle) -> usize {
        use LicenceTitle as T;
        type Pc = PrivateOrCommercialCategory;
        type At = AirlineTransportCategory;
        match title {
            T::Private(Pc::Aeroplane) => 0,
            T::Private(Pc::Airship) => 1,
            T::Private(Pc::Helicopter) => 2,
            T::Private(Pc::PoweredLift) => 3,
            T::Commercial(Pc::Aeroplane) => 4,
            T::Commercial(Pc::Airship) => 5,
            T::Commercial(Pc::Helicopter) => 6,
            T::Commercial(Pc::PoweredLift) => 7,
            T::MultiCrew => 8,
            T::AirlineTransport(At::Aeroplane) => 9,
            T::AirlineTransport(At::Helicopter) => 10,
            T::AirlineTransport(At::PoweredLift) => 11,
            T::Glider => 12,
            T::FreeBalloon => 13,
        }
    }

    /// A national title's place in [`NATIONAL_TITLES`], exhaustive as above.
    fn national_position(title: NationalTitle) -> usize {
        match title {
            NationalTitle::CommercialGyroplane => 0,
            NationalTitle::PrivateGyroplane => 1,
            NationalTitle::RecreationalAeroplane => 2,
            NationalTitle::RecreationalHelicopter => 3,
            NationalTitle::RecreationalGyroplane => 4,
        }
    }

    #[test]
    fn the_lists_hold_every_title_where_the_matches_say() {
        for (at, title) in ANNEX_1_TITLES.into_iter().enumerate() {
            assert_eq!(annex_1_position(title), at);
        }
        for (at, title) in NATIONAL_TITLES.into_iter().enumerate() {
            assert_eq!(national_position(title), at);
        }
    }
}
