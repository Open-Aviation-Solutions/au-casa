//! The Part 61 term for each Annex 1 title of licence the kernel records: the
//! flight crew licence and category rating CASA issues for it, if it issues
//! one at all.
//!
//! The kernel's [`LicenceTitle`] is in ICAO's words. CASA's differ ("air
//! transport" for Annex 1's "airline transport", "powered-lift aircraft" for
//! "powered-lift"), and CASA does not issue every title (there is no free
//! balloon licence), so a CASA host names and offers titles through
//! [`part61_term`] rather than the kernel's own names.
//!
//! Part 61 also grants licence and category rating pairs that Annex 1 has no
//! title for: the recreational pilot licence, and the gyroplane category rating
//! (Annex 1 2.1.1.1 lists no gyroplane). Those are [`NationalTitle`]s, held from
//! a date of first issue as [`IssuedNationalTitle`]s, beside the kernel's
//! titles rather than in place of them: the kernel stays the source for what
//! Annex 1 can say.

use std::fmt;

use icao_shared_kernel::{AirlineTransportCategory, LicenceTitle, PrivateOrCommercialCategory};
use time::Date;

use crate::aircraft::AircraftCategory;

/// A flight crew licence (reg 61.010) that a pilot holds.
///
/// These are the pilot licences (reg 61.010, "pilot licence" (a) to (e)) and
/// the glider pilot licence (reg 61.010, "flight crew licence" (c); Subpart
/// 61.Z). The flight engineer licence, which is not a pilot's, is not here.
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
    /// Reg 61.010, "pilot licence" (e). Not Annex 1's: see [`NationalTitle`].
    Recreational,
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
            Self::Recreational => "recreational pilot licence",
            Self::Glider => "glider pilot licence",
        })
    }
}

impl FlightCrewLicence {
    /// Whether this is a pilot licence (reg 61.010, "pilot licence"): every
    /// licence here but the glider pilot licence, which is a flight crew
    /// licence and not a pilot licence.
    pub fn is_pilot_licence(self) -> bool {
        // Exhaustive, so a licence added here is a decision.
        match self {
            Self::AirTransport
            | Self::Commercial
            | Self::MultiCrew
            | Self::Private
            | Self::Recreational => true,
            Self::Glider => false,
        }
    }
}

/// The Part 61 term for a title of licence (the kernel's, or a
/// [`NationalTitle`]): the licence and, for a pilot licence, the aircraft
/// category rating it carries for the title's category,
/// with the regulations that say CASA issues that pair.
///
/// Division 61.L.2 does not list which category ratings each licence can
/// carry: reg 61.730 grants one on meeting the requirements for "the pilot
/// licence with the aircraft category rating", so the pairs are those each
/// licence's Subpart gives aeronautical experience requirements for. Those
/// requirements are cited, and for the multi-crew and air transport pilot
/// licences also the privileges regulation that names the category (regs
/// 61.635 and 61.665).
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
        // Regs 61.635 and 61.665 are the privileges of the multi-crew and air
        // transport pilot licences, which name the categories; the other
        // citations are aeronautical experience requirements.
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

/// A Part 61 pilot licence and aircraft category rating pair that Annex 1 has
/// no title for, so the kernel's [`LicenceTitle`] cannot hold it.
///
/// A pair, not a category rating alone: a category rating "has effect only for
/// the pilot licence with which it is associated" (reg 61.010, "associated";
/// reg 61.725), so a gyroplane category rating on a private pilot licence and
/// one on a commercial pilot licence are different grants.
///
/// The pairs are listed by [`NationalTitle::part61_term`]'s exhaustive
/// `match`, as [`part61_term`] lists Annex 1's. The flight engineer licence is
/// not a pilot licence and is left out; CASA issues no free balloon licence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NationalTitle {
    /// Recreational pilot licence, aeroplane category rating.
    RecreationalAeroplane,
    /// Recreational pilot licence, helicopter category rating.
    RecreationalHelicopter,
    /// Recreational pilot licence, gyroplane category rating.
    RecreationalGyroplane,
    /// Private pilot licence, gyroplane category rating.
    PrivateGyroplane,
    /// Commercial pilot licence, gyroplane category rating.
    CommercialGyroplane,
}

impl NationalTitle {
    /// The licence and category rating, with the regulations that say CASA
    /// issues the pair.
    ///
    /// Part 61 does not list the recreational pilot licence's category
    /// ratings: reg 61.475 grants it with "the associated aircraft category
    /// rating" of any category. The Part 61 Manual of Standards has standards
    /// for three (Schedule 1, Appendices G.1 to G.3); its airship category
    /// rating (Appendix G.4) is reserved, and it has none for a powered-lift
    /// aircraft, so neither is a pair here (decided on issue #15). The
    /// gyroplane category rating on a private or commercial pilot licence
    /// cites its aeronautical experience requirements, as [`part61_term`]'s
    /// pairs do; no multi-crew or air transport pilot licence carries one
    /// (regs 61.635 and 61.665).
    pub fn part61_term(self) -> Part61Term {
        use AircraftCategory as Category;
        use FlightCrewLicence::*;

        let term = |licence, category, citations| Part61Term {
            licence,
            category_rating: Some(category),
            citations,
        };
        // Exhaustive, with no wildcard: a pair added is a compile error here,
        // and a decision.
        match self {
            Self::RecreationalAeroplane => term(
                Recreational,
                Category::Aeroplane,
                &["reg 61.475", "MOS Schedule 1, Appendix G.1"],
            ),
            Self::RecreationalHelicopter => term(
                Recreational,
                Category::Helicopter,
                &["reg 61.475", "MOS Schedule 1, Appendix G.2"],
            ),
            Self::RecreationalGyroplane => term(
                Recreational,
                Category::Gyroplane,
                &["reg 61.475", "MOS Schedule 1, Appendix G.3"],
            ),
            Self::PrivateGyroplane => {
                term(Private, Category::Gyroplane, &["reg 61.535", "reg 61.560"])
            }
            Self::CommercialGyroplane => term(Commercial, Category::Gyroplane, &["reg 61.625"]),
        }
    }

    /// The age, in whole years, an applicant for the licence must have reached
    /// (regs 61.475(1), 61.515(1) and 61.580(1)): the licence's, whatever its
    /// category, as Annex 1's ages are in the kernel.
    pub fn minimum_age(self) -> u8 {
        match self {
            Self::RecreationalAeroplane
            | Self::RecreationalHelicopter
            | Self::RecreationalGyroplane => 16,
            Self::PrivateGyroplane => 17,
            Self::CommercialGyroplane => 18,
        }
    }
}

impl fmt::Display for NationalTitle {
    /// The Part 61 term: "recreational pilot licence, gyroplane category
    /// rating".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.part61_term().fmt(f)
    }
}

/// A national title held, from the date the holder was first issued it: the
/// counterpart of the kernel's `IssuedTitle`, kept beside a licence's titles
/// by a consumer, keyed by the licence's identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IssuedNationalTitle {
    pub title: NationalTitle,
    pub date_of_first_issue: Date,
}

impl IssuedNationalTitle {
    pub fn new(title: NationalTitle, date_of_first_issue: Date) -> Self {
        Self {
            title,
            date_of_first_issue,
        }
    }

    /// Whether the holder, born on `date_of_birth`, had reached the title's
    /// [`NationalTitle::minimum_age`] on its date of first issue.
    ///
    /// Checked against the date of first issue, as the kernel checks Annex 1's
    /// ages, so it can miss an applicant who was too young when they applied,
    /// though not one who was too young when the title was issued. A year is
    /// complete on the birthday, compared by month and day, as the kernel
    /// counts it: someone born on 29 February completes a year on 1 March in a
    /// year that is not a leap year.
    pub fn holder_of_age(&self, date_of_birth: Date) -> bool {
        let on = self.date_of_first_issue;
        let mut age = on.year() - date_of_birth.year();
        if (u8::from(on.month()), on.day()) < (u8::from(date_of_birth.month()), date_of_birth.day())
        {
            age -= 1;
        }
        age >= i32::from(self.title.minimum_age())
    }
}
