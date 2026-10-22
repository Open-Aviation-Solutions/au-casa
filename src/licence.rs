//! The Part 61 term for each Annex 1 title of licence the kernel records: the
//! flight crew licence and category rating CASA issues for it, if it issues
//! one at all.
//!
//! The kernel's [`LicenceTitle`] is in ICAO's words. CASA's differ ("air
//! transport" for Annex 1's "airline transport", "powered-lift aircraft" for
//! "powered-lift"), and CASA does not issue every title (there is no free
//! balloon licence), so a CASA host names and offers titles through
//! [`part61_term`] rather than the kernel's own names.

use std::fmt;

use icao_shared_kernel::{AirlineTransportCategory, LicenceTitle, PrivateOrCommercialCategory};

use crate::aircraft::AircraftCategory;

/// A flight crew licence (reg 61.010) that a pilot holds.
///
/// These are the pilot licences that are Annex 1's (reg 61.010, "pilot
/// licence" (a) to (d)) and the glider pilot licence (reg 61.010, "flight crew
/// licence" (c); Subpart 61.Z). The recreational pilot licence, a pilot licence
/// that is not Annex 1's, and the flight engineer licence, which is not a
/// pilot's, are not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlightCrewLicence {
    /// Reg 61.010, "pilot licence" (a).
    AirTransport,
    /// Reg 61.010, "pilot licence" (b).
    Commercial,
    /// Reg 61.010, "pilot licence" (c).
    MultiCrew,
    /// Reg 61.010, "pilot licence" (d).
    Private,
    /// Reg 61.010, "flight crew licence" (c). Not a pilot licence under reg
    /// 61.010, so it carries no aircraft category rating.
    Glider,
}

impl fmt::Display for FlightCrewLicence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AirTransport => "air transport pilot licence",
            Self::Commercial => "commercial pilot licence",
            Self::MultiCrew => "multi-crew pilot licence",
            Self::Private => "private pilot licence",
            Self::Glider => "glider pilot licence",
        })
    }
}

/// The Part 61 term for a title of licence: the licence and, for a pilot
/// licence, the aircraft category rating it carries for the title's category,
/// with the regulations that say CASA issues that pair.
///
/// Division 61.L.2 does not list which category ratings each licence can
/// carry: reg 61.730 grants one on meeting the requirements for "the pilot
/// licence with the aircraft category rating", so the pairs are those each
/// licence's Subpart gives aeronautical experience requirements for, and those
/// requirements are cited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Part61Term {
    pub licence: FlightCrewLicence,
    /// `None` for the glider pilot licence alone: a glider is not a category
    /// of aircraft under reg 61.015.
    pub category_rating: Option<AircraftCategory>,
    pub citations: &'static [&'static str],
}

impl fmt::Display for Part61Term {
    /// "private pilot licence, aeroplane category rating", or "glider pilot
    /// licence".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.licence)?;
        if let Some(category) = self.category_rating {
            write!(f, ", {category} category rating")?;
        }
        Ok(())
    }
}

/// The Part 61 term for `title`, or `None` if CASA does not issue it.
///
/// `None` only for [`LicenceTitle::FreeBalloon`]: the categories of aircraft
/// in reg 61.015 do not include balloons, nor does the Part's application in
/// reg 61.007, and the Part does not mention them.
pub fn part61_term(title: LicenceTitle) -> Option<Part61Term> {
    use AircraftCategory as Category;
    use FlightCrewLicence::*;

    let term = |licence, category_rating, citations| {
        Some(Part61Term {
            licence,
            category_rating,
            citations,
        })
    };
    // Exhaustive, with no wildcard: a title the kernel adds is a compile error
    // here, and a decision.
    match title {
        LicenceTitle::Private(category) => match category {
            PrivateOrCommercialCategory::Aeroplane => term(
                Private,
                Some(Category::Aeroplane),
                &["reg 61.525", "reg 61.545"],
            ),
            PrivateOrCommercialCategory::Helicopter => term(
                Private,
                Some(Category::Helicopter),
                &["reg 61.530", "reg 61.550"],
            ),
            // Private powered-lift and airship, and commercial airship, have
            // requirements only for applicants who have not completed an
            // integrated training course (Divisions 61.H.3 and 61.I.3); CASA
            // still issues them.
            PrivateOrCommercialCategory::PoweredLift => {
                term(Private, Some(Category::PoweredLift), &["reg 61.555"])
            }
            PrivateOrCommercialCategory::Airship => {
                term(Private, Some(Category::Airship), &["reg 61.565"])
            }
        },
        LicenceTitle::Commercial(category) => match category {
            PrivateOrCommercialCategory::Aeroplane => term(
                Commercial,
                Some(Category::Aeroplane),
                &["reg 61.590", "reg 61.610"],
            ),
            PrivateOrCommercialCategory::Helicopter => term(
                Commercial,
                Some(Category::Helicopter),
                &["reg 61.595", "reg 61.615"],
            ),
            PrivateOrCommercialCategory::PoweredLift => term(
                Commercial,
                Some(Category::PoweredLift),
                &["reg 61.600", "reg 61.620"],
            ),
            PrivateOrCommercialCategory::Airship => {
                term(Commercial, Some(Category::Airship), &["reg 61.630"])
            }
        },
        LicenceTitle::MultiCrew => term(
            MultiCrew,
            Some(Category::Aeroplane),
            &["reg 61.635", "reg 61.660"],
        ),
        LicenceTitle::AirlineTransport(category) => match category {
            AirlineTransportCategory::Aeroplane => term(
                AirTransport,
                Some(Category::Aeroplane),
                &["reg 61.665", "reg 61.705"],
            ),
            AirlineTransportCategory::Helicopter => term(
                AirTransport,
                Some(Category::Helicopter),
                &["reg 61.665", "reg 61.710"],
            ),
            AirlineTransportCategory::PoweredLift => term(
                AirTransport,
                Some(Category::PoweredLift),
                &["reg 61.665", "reg 61.715"],
            ),
        },
        LicenceTitle::Glider => term(Glider, None, &["reg 61.010", "Subpart 61.Z"]),
        LicenceTitle::FreeBalloon => None,
    }
}
