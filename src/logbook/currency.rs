// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's currency checks, a module for each regulation or for regulations
//! whose checks differ only in their figures, written over the kernel's
//! [`EventWindow`](logbook_shared_kernel::EventWindow) (or, for a validity
//! to the end of a month, the kernel's date arithmetic), and the
//! requirements they answer. Each sees only the entries
//! [`Casa::counts`](crate::Casa) lets through, so none repeats that gate. A
//! check over assessments ([`AssessmentCurrencyCheck`]) that reads their
//! aircraft is given the entries it reads it from.

mod assessed_flights;
mod flight_review;
mod guidance_operations;
mod iap_3d_endorsement;
mod instrument_approaches;
mod instrument_approaches_2d;
mod instrument_approaches_3d;
mod instrument_proficiency_check;
mod instrument_rating_limits;
mod renewal;
mod single_pilot_ifr;
mod take_offs_and_landings;

use logbook_shared_kernel::{
    run, run_assessments, AssessmentAccumulator, AssessmentCurrencyCheck, CurrencyCheck,
    CurrencyResult, EntryAccumulator, EventsNoun, LicensingAuthority, Logbook, Requirement,
    RequirementId, RequirementKind, RequirementSet, TermId,
};
use serde::Serialize;
use std::collections::BTreeSet;
use time::Date;

use crate::aircraft::{AircraftCategory, AircraftClassRating};
use crate::logbook::licence_titles::holds_pilot_licence_in;
use crate::logbook::ratings_and_endorsements::{
    class_rating_id, InstrumentEndorsement, INSTRUMENT_RATING, NIGHT_VFR_RATING,
};
use crate::logbook::{class_name, Casa};

use assessed_flights::AssessedFlights;
use flight_review::FlightReview;
use guidance_operations::GuidanceOperations;
use iap_3d_endorsement::three_d_approach;
use instrument_approaches::InstrumentApproaches;
use instrument_approaches_2d::InstrumentApproaches2D;
use instrument_approaches_3d::InstrumentApproaches3D;
use instrument_proficiency_check::{InstrumentProficiencyCheck, ProficiencyCheckFor};
use instrument_rating_limits::{circling_approach, single_pilot_operation};
use single_pilot_ifr::SinglePilotIfr;
use take_offs_and_landings::{TakeOffsAndLandings, TakeOffsOrLandings, TAKE_OFFS_AND_LANDINGS};

/// A requirement CASA's rules check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CasaRequirement {
    /// Reg 61.395(1)'s take-offs (paragraph (a)) or landings ((b)).
    PassengerDay(TakeOffsOrLandings),
    /// Reg 61.395(2)'s take-offs (paragraph (a)) or landings ((b)).
    PassengerNight(TakeOffsOrLandings),
    /// Reg 61.965(a)'s night take-off (subparagraph (i)) or night landing
    /// ((ii)).
    NightVfr(TakeOffsOrLandings),
    InstrumentApproaches,
    /// Reg 61.870(3)'s instrument approach operation in this category.
    InstrumentApproachesInCategory(AircraftCategory),
    InstrumentApproaches2D,
    InstrumentApproaches3D,
    AzimuthGuidance,
    CourseDeviationIndicator,
    SinglePilotIfr,
    /// A flight review for this aircraft class rating.
    FlightReview(AircraftClassRating),
    /// Reg 61.880's instrument proficiency check, for a category ((1)) or
    /// for multi-engine aeroplanes or helicopters ((2)).
    InstrumentProficiencyCheck(ProficiencyCheckFor),
    /// Reg 61.860(2)'s flight test or instrument proficiency check in a
    /// single-pilot aircraft.
    SinglePilotOperation,
    /// Reg 61.860(3)'s flight test or proficiency check that included a
    /// circling approach.
    CirclingApproach,
    /// Reg 61.900(2)'s flight test or proficiency check that included a 3D
    /// instrument approach operation.
    ThreeDApproachAssessed,
}

/// Reg 61.015's categories of aircraft, in its order, each with a
/// requirement of reg 61.870(3). A registered sailplane is not one.
const CATEGORIES: [AircraftCategory; 5] = [
    AircraftCategory::Aeroplane,
    AircraftCategory::Helicopter,
    AircraftCategory::PoweredLift,
    AircraftCategory::Gyroplane,
    AircraftCategory::Airship,
];

/// Reg 61.880's instrument proficiency checks: one for each category of
/// reg 61.015, in its order ((1)), then for multi-engine aeroplanes and for
/// multi-engine helicopters ((2)(a) and (b)).
const PROFICIENCY_CHECKS: [ProficiencyCheckFor; 7] = [
    ProficiencyCheckFor::Category(CATEGORIES[0]),
    ProficiencyCheckFor::Category(CATEGORIES[1]),
    ProficiencyCheckFor::Category(CATEGORIES[2]),
    ProficiencyCheckFor::Category(CATEGORIES[3]),
    ProficiencyCheckFor::Category(CATEGORIES[4]),
    ProficiencyCheckFor::MultiEngineAeroplanes,
    ProficiencyCheckFor::MultiEngineHelicopters,
];

/// The requirements, in the order they are reported. Aeroplane recency
/// comes first, take-offs before landings for each regulation, then
/// instrument recency, then the flight reviews and the instrument
/// proficiency checks, as pilot-logbook listed them, then the limits that
/// turn on what a test or check included, in their regulations' order;
/// instrument recency is in reg 61.870's order, with a requirement of its
/// subregulation (3) for each category in reg 61.015's, and the instrument
/// proficiency checks in [`PROFICIENCY_CHECKS`]' order.
const REQUIREMENTS: [CasaRequirement; 29] = [
    CasaRequirement::PassengerDay(TakeOffsOrLandings::TakeOffs),
    CasaRequirement::PassengerDay(TakeOffsOrLandings::Landings),
    CasaRequirement::PassengerNight(TakeOffsOrLandings::TakeOffs),
    CasaRequirement::PassengerNight(TakeOffsOrLandings::Landings),
    CasaRequirement::NightVfr(TakeOffsOrLandings::TakeOffs),
    CasaRequirement::NightVfr(TakeOffsOrLandings::Landings),
    CasaRequirement::InstrumentApproaches,
    CasaRequirement::InstrumentApproachesInCategory(CATEGORIES[0]),
    CasaRequirement::InstrumentApproachesInCategory(CATEGORIES[1]),
    CasaRequirement::InstrumentApproachesInCategory(CATEGORIES[2]),
    CasaRequirement::InstrumentApproachesInCategory(CATEGORIES[3]),
    CasaRequirement::InstrumentApproachesInCategory(CATEGORIES[4]),
    CasaRequirement::InstrumentApproaches2D,
    CasaRequirement::InstrumentApproaches3D,
    CasaRequirement::AzimuthGuidance,
    CasaRequirement::CourseDeviationIndicator,
    CasaRequirement::SinglePilotIfr,
    CasaRequirement::FlightReview(AircraftClassRating::SingleEngineAeroplane),
    CasaRequirement::FlightReview(AircraftClassRating::MultiEngineAeroplane),
    CasaRequirement::InstrumentProficiencyCheck(PROFICIENCY_CHECKS[0]),
    CasaRequirement::InstrumentProficiencyCheck(PROFICIENCY_CHECKS[1]),
    CasaRequirement::InstrumentProficiencyCheck(PROFICIENCY_CHECKS[2]),
    CasaRequirement::InstrumentProficiencyCheck(PROFICIENCY_CHECKS[3]),
    CasaRequirement::InstrumentProficiencyCheck(PROFICIENCY_CHECKS[4]),
    CasaRequirement::InstrumentProficiencyCheck(PROFICIENCY_CHECKS[5]),
    CasaRequirement::InstrumentProficiencyCheck(PROFICIENCY_CHECKS[6]),
    CasaRequirement::SinglePilotOperation,
    CasaRequirement::CirclingApproach,
    CasaRequirement::ThreeDApproachAssessed,
];

impl CasaRequirement {
    /// The regulation that sets it, bare: `61.395(1)`, which sets
    /// `61.395(1)(a)` and `(b)`.
    fn regulation(self) -> &'static str {
        match self {
            Self::PassengerDay(_) => "61.395(1)",
            Self::PassengerNight(_) => "61.395(2)",
            Self::NightVfr(_) => "61.965",
            Self::InstrumentApproaches => "61.870(2)",
            Self::InstrumentApproachesInCategory(_) => "61.870(3)",
            Self::InstrumentApproaches2D => "61.870(4)",
            Self::InstrumentApproaches3D => "61.870(5)",
            Self::AzimuthGuidance => "61.870(6)",
            Self::CourseDeviationIndicator => "61.870(7)",
            Self::SinglePilotIfr => "61.875",
            Self::FlightReview(_) => "61.745",
            Self::InstrumentProficiencyCheck(_) => "61.880",
            Self::SinglePilotOperation => "61.860(2)",
            Self::CirclingApproach => "61.860(3)",
            Self::ThreeDApproachAssessed => "61.900(2)",
        }
    }

    /// Its id: the regulation and, for a flight review, `|` and the class
    /// rating's stored name (`61.745|single-engine-aeroplane`), each
    /// pilot-logbook's key for it (its `wording.ts`); for reg 61.870(3), `|`
    /// and the category's stored name (`61.870(3)|aeroplane`), which
    /// originate here, since pilot-logbook had no key for it; for the
    /// take-offs and landings of regs 61.395(1) and (2) and 61.965(a), the
    /// paragraph that requires them (`61.395(1)(a)`, `61.965(a)(ii)`), which
    /// originate here too (au-casa#67); for reg 61.880, `|` and the
    /// category's stored name (`61.880|aeroplane`), or
    /// `multi-engine-aeroplane` or `multi-engine-helicopter`, which
    /// originate here too (au-casa#58). Stable: a host may keep it.
    fn id(self) -> RequirementId {
        match self {
            Self::FlightReview(class_rating) => self.qualified(class_rating),
            Self::InstrumentApproachesInCategory(category) => self.qualified(category),
            Self::InstrumentProficiencyCheck(check_for) => match check_for {
                ProficiencyCheckFor::Category(category) => self.qualified(category),
                ProficiencyCheckFor::MultiEngineAeroplanes => {
                    self.qualified("multi-engine-aeroplane")
                }
                ProficiencyCheckFor::MultiEngineHelicopters => {
                    self.qualified("multi-engine-helicopter")
                }
            },
            Self::PassengerDay(kind) | Self::PassengerNight(kind) => self.paragraph(match kind {
                TakeOffsOrLandings::TakeOffs => "(a)",
                TakeOffsOrLandings::Landings => "(b)",
            }),
            Self::NightVfr(kind) => self.paragraph(match kind {
                TakeOffsOrLandings::TakeOffs => "(a)(i)",
                TakeOffsOrLandings::Landings => "(a)(ii)",
            }),
            _ => RequirementId::new(self.regulation()),
        }
    }

    /// The regulation, then `paragraph`.
    fn paragraph(self, paragraph: &str) -> RequirementId {
        RequirementId::new(format!("{}{paragraph}", self.regulation()))
    }

    /// The regulation, `|` and `value`'s stored name.
    fn qualified(self, value: impl Serialize) -> RequirementId {
        let name = serde_json::to_value(value).expect("a stored name serialises");
        let name = name.as_str().expect("a stored name is a string");
        RequirementId::new(format!("{}|{name}", self.regulation()))
    }

    /// Its name, in its regulation's words.
    fn name(self) -> String {
        match self {
            Self::PassengerDay(kind) => format!(
                "Carrying a passenger by day in an aeroplane: at least 3 {}, by day or night",
                kind.plural()
            ),
            Self::PassengerNight(kind) => format!(
                "Carrying a passenger at night in an aeroplane: at least 3 {} at night",
                kind.plural()
            ),
            Self::NightVfr(kind) => format!(
                "Recent experience for the night VFR rating in an aeroplane: at least one night {}",
                kind.one()
            ),
            Self::InstrumentApproaches => "At least 3 instrument approach operations".to_owned(),
            Self::InstrumentApproachesInCategory(category) => {
                format!(
                    "At least one instrument approach operation in an aircraft of the \
                     {category} category, or an approved flight simulation training device"
                )
            }
            Self::InstrumentApproaches2D => "2D instrument approach operation".to_owned(),
            Self::InstrumentApproaches3D => "3D instrument approach operation".to_owned(),
            Self::AzimuthGuidance => "Azimuth guidance operation".to_owned(),
            Self::CourseDeviationIndicator => "Course deviation indicator operation".to_owned(),
            Self::SinglePilotIfr => {
                "Flight or simulated flight under the IFR in a single-pilot operation".to_owned()
            }
            Self::FlightReview(class_rating) => format!(
                "Flight review for the {} class rating",
                class_name(class_rating)
            ),
            Self::InstrumentProficiencyCheck(check_for) => match check_for {
                ProficiencyCheckFor::Category(category) => {
                    format!("Instrument proficiency check for the {category} category")
                }
                ProficiencyCheckFor::MultiEngineAeroplanes => {
                    "Instrument proficiency check for multi-engine aeroplanes".to_owned()
                }
                ProficiencyCheckFor::MultiEngineHelicopters => {
                    "Instrument proficiency check for multi-engine helicopters".to_owned()
                }
            },
            Self::SinglePilotOperation => "Single-pilot operation under the IFR".to_owned(),
            Self::CirclingApproach => "Circling approach under the IFR".to_owned(),
            Self::ThreeDApproachAssessed => {
                "3D instrument approach operation: flight test or proficiency check".to_owned()
            }
        }
    }

    /// What meets it, in CASA's words: pilot-logbook's wording of it until
    /// au-casa#96 moved it here, for reg 61.870(3), the owner's on
    /// au-casa#96, and for regs 61.860(2) and (3) and 61.900(2), the owner's
    /// on au-casa#116.
    fn description(self) -> String {
        match self {
            Self::PassengerDay(kind) => format!("3 {} in the past 90 days", kind.plural()),
            Self::PassengerNight(kind) => format!("3 night {} in the past 90 days", kind.plural()),
            Self::NightVfr(kind) => format!("1 night {} in the past 6 months", kind.one()),
            Self::InstrumentApproaches => "At least 3 instrument approach operations within the \
                                           previous 90 days, in an aircraft or an approved flight \
                                           simulation training device"
                .to_owned(),
            Self::InstrumentApproachesInCategory(category) => format!(
                "At least 1 instrument approach operation within the previous 90 days, in {} or \
                 an approved flight simulation training device",
                with_article(&category.to_string())
            ),
            Self::InstrumentApproaches2D => "A 2D instrument approach operation (with lateral \
                                             navigation guidance only) within the previous 90 days"
                .to_owned(),
            Self::InstrumentApproaches3D => "A 3D instrument approach operation (with lateral and \
                                             vertical navigation guidance) within the previous 90 \
                                             days"
                .to_owned(),
            Self::AzimuthGuidance => {
                "An azimuth guidance operation within the previous 90 days".to_owned()
            }
            Self::CourseDeviationIndicator => {
                "A course deviation indicator operation within the previous 90 days".to_owned()
            }
            Self::SinglePilotIfr => "1 flight or simulated flight under the IFR in a single-pilot \
                                     operation, of at least 1 hour and with an approach, in the \
                                     past 6 months"
                .to_owned(),
            Self::FlightReview(class_rating) => format!(
                "A flight review for the rating, successfully completed in {} or an approved \
                 flight simulator, or what reg 61.745(3) counts as one; valid to the end of the \
                 24th month after the month it is completed",
                with_article(class_name(class_rating))
            ),
            Self::InstrumentProficiencyCheck(check_for) => {
                let (check, completed_in) = match check_for {
                    ProficiencyCheckFor::Category(category) => (
                        format!("the {category} category"),
                        format!("a single-engine or multi-engine {category}"),
                    ),
                    ProficiencyCheckFor::MultiEngineAeroplanes => (
                        "multi-engine aeroplanes".to_owned(),
                        "a multi-engine aeroplane".to_owned(),
                    ),
                    ProficiencyCheckFor::MultiEngineHelicopters => (
                        "multi-engine helicopters".to_owned(),
                        "a multi-engine helicopter".to_owned(),
                    ),
                };
                format!(
                    "An instrument proficiency check for {check}, successfully completed in \
                     {completed_in}, or a flight test or check reg 61.880(3) counts as one; valid \
                     to the end of the 12th month after the month it is completed"
                )
            }
            Self::SinglePilotOperation => "The flight test for the instrument rating passed, or \
                                           an instrument proficiency check completed, in a \
                                           single-pilot aircraft"
                .to_owned(),
            Self::CirclingApproach => "The flight test for the instrument rating passed within \
                                       the previous 12 months, or the most recent instrument \
                                       proficiency check, including a circling approach"
                .to_owned(),
            Self::ThreeDApproachAssessed => "The most recent instrument proficiency check \
                                             including a 3D instrument approach operation; \
                                             otherwise, competence in one demonstrated to an \
                                             examiner within the previous 15 months, in a flight \
                                             test or check, or in a demonstration not recorded \
                                             as an assessment"
                .to_owned(),
        }
    }

    /// The noun its events are counted in, for the take-offs, landings and
    /// operations counted by regs 61.395, 61.965 and 61.870; `None` for the
    /// others.
    fn events(self) -> Option<EventsNoun> {
        let noun = |one: &str, other: &str| {
            Some(EventsNoun {
                one: one.to_owned(),
                other: other.to_owned(),
            })
        };
        match self {
            Self::PassengerDay(kind) => noun(kind.one(), kind.plural()),
            Self::PassengerNight(kind) | Self::NightVfr(kind) => noun(
                &format!("night {}", kind.one()),
                &format!("night {}", kind.plural()),
            ),
            Self::InstrumentApproaches | Self::InstrumentApproachesInCategory(_) => noun(
                "instrument approach operation",
                "instrument approach operations",
            ),
            Self::InstrumentApproaches2D => noun(
                "2D instrument approach operation",
                "2D instrument approach operations",
            ),
            Self::InstrumentApproaches3D => noun(
                "3D instrument approach operation",
                "3D instrument approach operations",
            ),
            Self::AzimuthGuidance => {
                noun("azimuth guidance operation", "azimuth guidance operations")
            }
            Self::CourseDeviationIndicator => noun(
                "course deviation indicator operation",
                "course deviation indicator operations",
            ),
            Self::SinglePilotIfr
            | Self::FlightReview(_)
            | Self::InstrumentProficiencyCheck(_)
            | Self::SinglePilotOperation
            | Self::CirclingApproach
            | Self::ThreeDApproachAssessed => None,
        }
    }

    /// The set its regulation sets it with the others in, `None` for the
    /// others, each set on its own: for the take-offs and landings of regs
    /// 61.395(1) and (2) and of reg 61.965(a), the provision that sets them
    /// as its key (`61.395(1)`, `61.965(a)`), and a name and description in
    /// the members' words, which every member carries equal (au-casa#109).
    /// The members apply together, since each is chosen by its kind alone
    /// (`applies_to`).
    fn set(self) -> Option<RequirementSet> {
        let (key, name, description) = match self {
            Self::PassengerDay(_) => (
                "61.395(1)",
                "Carrying a passenger by day in an aeroplane",
                "At least 3 take-offs and 3 landings, by day or night, in the past 90 days",
            ),
            Self::PassengerNight(_) => (
                "61.395(2)",
                "Carrying a passenger at night in an aeroplane",
                "At least 3 night take-offs and 3 night landings in the past 90 days",
            ),
            Self::NightVfr(_) => (
                "61.965(a)",
                "Recent experience for the night VFR rating in an aeroplane",
                "At least 1 night take-off and 1 night landing in the past 6 months",
            ),
            _ => return None,
        };
        Some(RequirementSet::new(key, name, description))
    }

    /// Recent experience (regs 61.395, 61.870, 61.875 and 61.965), or a
    /// periodic assessment: a flight review (reg 61.745), an instrument
    /// proficiency check (reg 61.880), or what a flight test or check
    /// included (regs 61.860(2) and (3), 61.900(2)).
    fn kind(self) -> RequirementKind {
        match self {
            Self::FlightReview(_)
            | Self::InstrumentProficiencyCheck(_)
            | Self::SinglePilotOperation
            | Self::CirclingApproach
            | Self::ThreeDApproachAssessed => RequirementKind::PeriodicAssessment,
            Self::PassengerDay(_)
            | Self::PassengerNight(_)
            | Self::NightVfr(_)
            | Self::InstrumentApproaches
            | Self::InstrumentApproachesInCategory(_)
            | Self::InstrumentApproaches2D
            | Self::InstrumentApproaches3D
            | Self::AzimuthGuidance
            | Self::CourseDeviationIndicator
            | Self::SinglePilotIfr => RequirementKind::RecentExperience,
        }
    }
}

impl CasaRequirement {
    /// It, as the kernel describes a requirement.
    fn built(self) -> Requirement {
        let mut built = Requirement::new(
            self.id(),
            self.name(),
            self.description(),
            self.kind(),
            self.regulation(),
        );
        if let Some(events) = self.events() {
            built = built.with_events(events);
        }
        if let Some(set) = self.set() {
            built = built.with_set(set);
        }
        built
    }

    /// Whether it applies to a pilot who holds `held`, the ids of their
    /// titles of licence, ratings and endorsements. Each applies to the
    /// holder its regulation names:
    ///
    /// - reg 61.395(1), to the holder of a pilot licence, here in an
    ///   aeroplane, so of a title with the aeroplane category rating;
    /// - reg 61.395(2), to such a holder who may also fly an aeroplane at
    ///   night: one with the night VFR rating or the instrument rating with
    ///   an aeroplane instrument endorsement (table 61.375 item 3, reg
    ///   61.375(7); regs 61.855(b), 61.865(1)), or the
    ///   multi-crew or air transport pilot licence with the aeroplane
    ///   category rating (reg 61.375(8)(a)(ii), (b)(ii)); a pilot licence
    ///   alone does not authorise it;
    /// - reg 61.965(a), to the holder of a night VFR rating, in an aircraft
    ///   of a particular category, here an aeroplane, so of the night VFR
    ///   rating and a title with the aeroplane category rating;
    /// - regs 61.860(2) and (3), 61.870(2), (6) and (7) and 61.875, to the
    ///   holder of an instrument rating;
    /// - reg 61.870(3), for a category, and reg 61.880(1), to the holder of
    ///   an instrument endorsement in that category (Part 1 of table 61.890),
    ///   since without one the rating gives no privilege in it (reg
    ///   61.865(1)); reg 61.880(2), for multi-engine aeroplanes or
    ///   helicopters, to the holder of the multi-engine aeroplane or
    ///   helicopter instrument endorsement;
    /// - reg 61.870(4) and (5), to the holder of the IAP 2D or IAP 3D
    ///   instrument endorsement (Part 2 of table 61.890, reg 61.865(2)), and
    ///   reg 61.900(2) to the holder of the IAP 3D one;
    /// - reg 61.745, to the holder of its aircraft class rating.
    fn applies_to(self, held: &BTreeSet<TermId>) -> bool {
        use InstrumentEndorsement as E;
        let holds = |id: &str| held.contains(&TermId::new(id));
        let instrument_endorsement_in = |category| {
            E::ALL.into_iter().any(|endorsement| {
                endorsement.category() == Some(category) && holds(endorsement.id())
            })
        };
        let aeroplane_licence = || holds_pilot_licence_in(held, AircraftCategory::Aeroplane);
        // Table 61.375 item 3: an operation at night under the VFR needs the
        // night VFR rating or an instrument rating (reg 61.855(b) gives the
        // instrument rating's privilege, in an aircraft only with its
        // instrument endorsement, reg 61.865(1), so here the aeroplane
        // one); reg 61.375(8)(a)(ii) and (b)(ii)
        // need neither for the multi-crew and air transport pilot licences
        // with the aeroplane category rating.
        let authorised_at_night = || {
            holds(NIGHT_VFR_RATING)
                || instrument_endorsement_in(AircraftCategory::Aeroplane)
                || holds("multi-crew-aeroplane")
                || holds("air-transport-aeroplane")
        };
        match self {
            Self::PassengerDay(_) => aeroplane_licence(),
            Self::PassengerNight(_) => aeroplane_licence() && authorised_at_night(),
            Self::NightVfr(_) => holds(NIGHT_VFR_RATING) && aeroplane_licence(),
            Self::InstrumentApproaches
            | Self::AzimuthGuidance
            | Self::CourseDeviationIndicator
            | Self::SinglePilotIfr
            | Self::SinglePilotOperation
            | Self::CirclingApproach => holds(INSTRUMENT_RATING),
            Self::InstrumentApproachesInCategory(category)
            | Self::InstrumentProficiencyCheck(ProficiencyCheckFor::Category(category)) => {
                instrument_endorsement_in(category)
            }
            Self::InstrumentProficiencyCheck(ProficiencyCheckFor::MultiEngineAeroplanes) => {
                holds(E::MultiEngineAeroplane.id())
            }
            Self::InstrumentProficiencyCheck(ProficiencyCheckFor::MultiEngineHelicopters) => {
                holds(E::MultiEngineHelicopter.id())
            }
            Self::InstrumentApproaches2D => holds(E::Iap2D.id()),
            Self::InstrumentApproaches3D | Self::ThreeDApproachAssessed => holds(E::Iap3D.id()),
            Self::FlightReview(class) => holds(class_rating_id(class)),
        }
    }
}

/// `noun` after "a", or "an" before a vowel: "an aeroplane", "a
/// single-engine aeroplane".
fn with_article(noun: &str) -> String {
    let article = if noun.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {noun}")
}

/// The requirements, in [`REQUIREMENTS`]' order.
pub(super) fn requirements() -> Vec<Requirement> {
    REQUIREMENTS
        .into_iter()
        .map(CasaRequirement::built)
        .collect()
}

/// The requirements that apply to a pilot who holds `held`, in
/// [`REQUIREMENTS`]' order, with what it implies they hold too.
pub(super) fn requirements_for(held: &BTreeSet<TermId>) -> Vec<Requirement> {
    let held = with_implied(held);
    REQUIREMENTS
        .into_iter()
        .filter(|requirement| requirement.applies_to(&held))
        .map(CasaRequirement::built)
        .collect()
}

/// What the report leaves out of those that apply to `held` until
/// `endorsement` is recorded too: the requirements that apply with it and not
/// without, as "reg 61.870(3) and the instrument proficiency check", the
/// distinct regulations in [`REQUIREMENTS`]' order and then the reg 61.880
/// checks as one. `None` when there are none.
pub(super) fn left_out_until(
    held: &BTreeSet<TermId>,
    endorsement: InstrumentEndorsement,
) -> Option<String> {
    let without = with_implied(held);
    let mut with = held.clone();
    with.insert(TermId::new(endorsement.id()));
    let with = with_implied(&with);
    let mut regulations: Vec<&str> = Vec::new();
    let mut check = false;
    for requirement in REQUIREMENTS {
        if !requirement.applies_to(&with) || requirement.applies_to(&without) {
            continue;
        }
        if matches!(requirement, CasaRequirement::InstrumentProficiencyCheck(_)) {
            check = true;
        } else if !regulations.contains(&requirement.regulation()) {
            regulations.push(requirement.regulation());
        }
    }
    let mut parts: Vec<String> = Vec::new();
    if !regulations.is_empty() {
        let noun = if regulations.len() == 1 {
            "reg"
        } else {
            "regs"
        };
        parts.push(format!("{noun} {}", regulations.join(" and ")));
    }
    if check {
        parts.push("the instrument proficiency check".to_owned());
    }
    (!parts.is_empty()).then(|| parts.join(" and "))
}

/// `held`, with what holding it implies: an instrument endorsement of table
/// 61.890 the instrument rating, which its applicant must hold (reg
/// 61.905(1)(a)), and the IAP 3D instrument endorsement the IAP 2D one,
/// which its applicant must hold (item 9 of table 61.890, column 3, under
/// reg 61.905(1)(b)). So a pilot who records an endorsement and not what it
/// is granted on is not given only some of the rows it implies.
fn with_implied(held: &BTreeSet<TermId>) -> BTreeSet<TermId> {
    let holds = |endorsement: InstrumentEndorsement| held.contains(&TermId::new(endorsement.id()));
    let mut implied = held.clone();
    if InstrumentEndorsement::ALL.into_iter().any(holds) {
        implied.insert(TermId::new(INSTRUMENT_RATING));
    }
    if holds(InstrumentEndorsement::Iap3D) {
        implied.insert(TermId::new(InstrumentEndorsement::Iap2D.id()));
    }
    implied
}

/// Each requirement's result on `as_of`, each paired with its requirement
/// where its check is run, in [`REQUIREMENTS`]' order (a test holds the two
/// to the same order).
///
/// The checks over entries are run together, in one pass of the kernel's
/// `run`, which applies [`Casa::counts`]. The flight reviews read an
/// assessment's aircraft from the entry it names, which `run` does not
/// reach, so they are given the entries `counts` lets through, as are the
/// instrument proficiency checks and the limits on what a test or check
/// included, which are also given every entry, to read an assessment's
/// information with its entry whether or not it counts.
pub(super) fn currency(
    casa: &Casa,
    as_of: Date,
    logbook: &Logbook,
) -> Vec<(RequirementId, CurrencyResult)> {
    let mut passenger_day =
        TAKE_OFFS_AND_LANDINGS.map(|kind| TakeOffsAndLandings::passenger_day(as_of, kind));
    let mut passenger_night =
        TAKE_OFFS_AND_LANDINGS.map(|kind| TakeOffsAndLandings::passenger_night(as_of, kind));
    let mut night_vfr =
        TAKE_OFFS_AND_LANDINGS.map(|kind| TakeOffsAndLandings::night_vfr(as_of, kind));
    let mut approaches = InstrumentApproaches::new(as_of);
    let mut in_category =
        CATEGORIES.map(|category| InstrumentApproaches::in_category(as_of, category));
    let mut approaches_2d = InstrumentApproaches2D::new(as_of);
    let mut approaches_3d = InstrumentApproaches3D::new(as_of);
    let mut azimuth_guidance = GuidanceOperations::azimuth_guidance(as_of);
    let mut course_deviation_indicator = GuidanceOperations::course_deviation_indicator(as_of);
    let mut single_pilot_ifr = SinglePilotIfr::new(as_of);
    let [day_take_offs, day_landings] = &mut passenger_day;
    let [night_take_offs, night_landings] = &mut passenger_night;
    let [night_vfr_take_offs, night_vfr_landings] = &mut night_vfr;
    let mut accumulators: Vec<&mut dyn EntryAccumulator> = vec![
        day_take_offs,
        day_landings,
        night_take_offs,
        night_landings,
        night_vfr_take_offs,
        night_vfr_landings,
        &mut approaches,
        &mut approaches_2d,
        &mut approaches_3d,
        &mut azimuth_guidance,
        &mut course_deviation_indicator,
        &mut single_pilot_ifr,
    ];
    accumulators.extend(
        in_category
            .iter_mut()
            .map(|check| check as &mut dyn EntryAccumulator),
    );
    run(casa, &logbook.entries, &mut accumulators);

    let single_engine = AircraftClassRating::SingleEngineAeroplane;
    let multi_engine = AircraftClassRating::MultiEngineAeroplane;
    let counted = || logbook.entries.iter().filter(|entry| casa.counts(entry));
    let mut single_engine_review = FlightReview::new(as_of, single_engine, counted());
    let mut multi_engine_review = FlightReview::new(as_of, multi_engine, counted());
    let mut proficiency_checks = PROFICIENCY_CHECKS.map(|check_for| {
        InstrumentProficiencyCheck::new(as_of, check_for, counted(), &logbook.entries)
    });
    let mut assessed = AssessedFlights::new(as_of, counted(), &logbook.entries);
    let mut assessment_accumulators: Vec<&mut dyn AssessmentAccumulator> = vec![
        &mut single_engine_review,
        &mut multi_engine_review,
        &mut assessed,
    ];
    assessment_accumulators.extend(
        proficiency_checks
            .iter_mut()
            .map(|check| check as &mut dyn AssessmentAccumulator),
    );
    run_assessments(&logbook.assessments, &mut assessment_accumulators);

    let in_category = CATEGORIES
        .iter()
        .zip(&in_category)
        .map(|(&category, check)| {
            (
                CasaRequirement::InstrumentApproachesInCategory(category),
                check.result(),
            )
        });

    /// Each check's result, paired with its requirement by what the check
    /// counts.
    fn by_what_they_count(
        requirement: fn(TakeOffsOrLandings) -> CasaRequirement,
        checks: &[TakeOffsAndLandings],
    ) -> impl Iterator<Item = (CasaRequirement, CurrencyResult)> + '_ {
        checks
            .iter()
            .map(move |check| (requirement(check.kind()), check.result()))
    }

    by_what_they_count(CasaRequirement::PassengerDay, &passenger_day)
        .chain(by_what_they_count(
            CasaRequirement::PassengerNight,
            &passenger_night,
        ))
        .chain(by_what_they_count(CasaRequirement::NightVfr, &night_vfr))
        .chain([(CasaRequirement::InstrumentApproaches, approaches.result())])
        .chain(in_category)
        .chain([
            (
                CasaRequirement::InstrumentApproaches2D,
                approaches_2d.result(),
            ),
            (
                CasaRequirement::InstrumentApproaches3D,
                approaches_3d.result(),
            ),
            (CasaRequirement::AzimuthGuidance, azimuth_guidance.result()),
            (
                CasaRequirement::CourseDeviationIndicator,
                course_deviation_indicator.result(),
            ),
            (CasaRequirement::SinglePilotIfr, single_pilot_ifr.result()),
            (
                CasaRequirement::FlightReview(single_engine),
                single_engine_review.result(),
            ),
            (
                CasaRequirement::FlightReview(multi_engine),
                multi_engine_review.result(),
            ),
        ])
        .chain(
            PROFICIENCY_CHECKS
                .iter()
                .zip(&proficiency_checks)
                .map(|(&check_for, check)| {
                    (
                        CasaRequirement::InstrumentProficiencyCheck(check_for),
                        check.result(),
                    )
                }),
        )
        .chain([
            (
                CasaRequirement::SinglePilotOperation,
                single_pilot_operation(&assessed),
            ),
            (
                CasaRequirement::CirclingApproach,
                circling_approach(&assessed),
            ),
            (
                CasaRequirement::ThreeDApproachAssessed,
                three_d_approach(&assessed),
            ),
        ])
        .map(|(requirement, result)| (requirement.id(), result))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use logbook_shared_kernel::{ApproachType, Assessment, DayNight, EntryParts};
    use time::macros::date;

    use super::*;
    use crate::logbook::test_support::{
        aeroplane, approach, assessed, day, entry, informed, night, simulated,
    };
    use crate::logbook::{AssessmentInformation, CheckOutcome, CheckType, EntryInformation};

    const REFERENCE: Date = date!(2024 - 09 - 01);
    const ON: Date = date!(2024 - 08 - 20);

    /// A passed check of this type in entry `e`.
    fn passed(id: &str, check_type: CheckType) -> Assessment {
        assessed(
            id,
            ON,
            "e",
            AssessmentInformation {
                check_type: Some(check_type),
                outcome: Some(CheckOutcome::Passed),
                check_purposes: Vec::new(),
            },
        )
    }

    /// `currency` pairs each result with its requirement where it runs the
    /// check, apart from `REQUIREMENTS`, so this holds the two lists to the
    /// same requirements in the same order.
    #[test]
    fn every_requirement_is_answered_once_in_the_order_listed() {
        let ids: Vec<RequirementId> = requirements().into_iter().map(|r| r.id).collect();
        let answered: Vec<RequirementId> = currency(&Casa::new(), REFERENCE, &Logbook::default())
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(answered, ids);
        assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len());
    }

    /// Reg 61.880's ids are stable, and a host words each by its id.
    #[test]
    fn each_instrument_proficiency_check_has_its_id_and_name() {
        let checks: Vec<(String, String)> = requirements()
            .into_iter()
            .filter(|requirement| requirement.regulation == "61.880")
            .map(|requirement| (requirement.id.as_str().to_owned(), requirement.name))
            .collect();
        let expected = [
            ("61.880|aeroplane", "the aeroplane category"),
            ("61.880|helicopter", "the helicopter category"),
            ("61.880|powered-lift", "the powered-lift aircraft category"),
            ("61.880|gyroplane", "the gyroplane category"),
            ("61.880|airship", "the airship category"),
            ("61.880|multi-engine-aeroplane", "multi-engine aeroplanes"),
            ("61.880|multi-engine-helicopter", "multi-engine helicopters"),
        ]
        .map(|(id, check_for)| {
            (
                id.to_owned(),
                format!("Instrument proficiency check for {check_for}"),
            )
        });
        assert_eq!(checks, expected);
    }

    /// The ids of the requirements that apply to a pilot who holds `held`.
    fn applying_to(held: &[&str]) -> Vec<String> {
        let held: BTreeSet<TermId> = held.iter().map(|&id| TermId::new(id)).collect();
        requirements_for(&held)
            .into_iter()
            .map(|requirement| requirement.id.as_str().to_owned())
            .collect()
    }

    const PASSENGER_BY_DAY: [&str; 2] = ["61.395(1)(a)", "61.395(1)(b)"];

    const PASSENGER: [&str; 4] = [
        "61.395(1)(a)",
        "61.395(1)(b)",
        "61.395(2)(a)",
        "61.395(2)(b)",
    ];

    /// A set's members apply together, all or none, whatever is held.
    #[test]
    fn a_sets_members_apply_together() {
        let everything = requirements();
        let members_of = |key: &str, among: &[Requirement]| {
            among
                .iter()
                .filter(|requirement| requirement.set.as_ref().is_some_and(|set| set.key == key))
                .count()
        };
        let keys: BTreeSet<String> = everything
            .iter()
            .filter_map(|requirement| requirement.set.as_ref().map(|set| set.key.clone()))
            .collect();
        let held_by = |held: &[&str]| -> BTreeSet<TermId> {
            held.iter().map(|&id| TermId::new(id)).collect()
        };
        for held in [
            vec![],
            vec!["private-aeroplane"],
            vec!["night-vfr-rating"],
            vec!["private-aeroplane", "night-vfr-rating"],
            vec!["private-helicopter", "night-vfr-rating"],
        ] {
            let applying = requirements_for(&held_by(&held));
            if held == ["private-aeroplane", "night-vfr-rating"] {
                // Not vacuous: this holder is given every member of every set.
                for key in &keys {
                    assert_eq!(
                        members_of(key, &applying),
                        members_of(key, &everything),
                        "{key}"
                    );
                    assert_eq!(members_of(key, &applying), 2, "{key}");
                }
            }
            for key in &keys {
                let count = members_of(key, &applying);
                assert!(
                    count == 0 || count == members_of(key, &everything),
                    "{key} with {held:?}"
                );
            }
        }
    }

    /// Nothing held, nothing applies; nor for an id CASA does not issue.
    #[test]
    fn nothing_applies_for_nothing_held() {
        assert!(applying_to(&[]).is_empty());
        assert!(applying_to(&["airline-transport"]).is_empty());
    }

    /// Reg 61.395(1) for a title with the aeroplane category rating,
    /// whatever the licence, and reg 61.395(2) too where the title alone
    /// authorises flight at night (reg 61.375(8)(a)(ii), (b)(ii)); not for
    /// another category, nor for the glider pilot licence, which is not a
    /// pilot licence.
    #[test]
    fn carrying_a_passenger_applies_to_an_aeroplane_licence() {
        for title in [
            "recreational-aeroplane",
            "private-aeroplane",
            "commercial-aeroplane",
        ] {
            assert_eq!(applying_to(&[title]), PASSENGER_BY_DAY, "{title}");
        }
        for title in ["multi-crew-aeroplane", "air-transport-aeroplane"] {
            assert_eq!(applying_to(&[title]), PASSENGER, "{title}");
        }
        for title in ["private-helicopter", "recreational-gyroplane", "glider"] {
            assert!(applying_to(&[title]).is_empty(), "{title}");
        }
    }

    /// Reg 61.395(2) for a pilot licence with the night VFR rating, or the
    /// instrument rating with an aeroplane instrument endorsement (table
    /// 61.375 item 3, regs 61.855(b), 61.865(1)); not without one, nor for
    /// the bare instrument rating or one with only a helicopter endorsement,
    /// nor for the night VFR rating without a pilot licence in an aeroplane.
    #[test]
    fn carrying_a_passenger_at_night_applies_to_a_pilot_authorised_at_night() {
        let night = |held: &[&str]| {
            applying_to(held)
                .iter()
                .any(|id| id.starts_with("61.395(2)"))
        };
        assert!(night(&["private-aeroplane", "night-vfr-rating"]));
        assert!(night(&[
            "private-aeroplane",
            "single-engine-aeroplane-instrument-endorsement"
        ]));
        assert!(!night(&["private-aeroplane"]));
        assert!(!night(&["private-aeroplane", "instrument-rating"]));
        assert!(!night(&[
            "private-aeroplane",
            "private-helicopter",
            "single-engine-helicopter-instrument-endorsement"
        ]));
        assert!(applying_to(&["private-aeroplane"]).contains(&"61.395(1)(a)".to_owned()));
        assert!(applying_to(&["private-helicopter", "night-vfr-rating"]).is_empty());
    }

    /// Reg 61.965(a) for the night VFR rating with an aeroplane licence; not
    /// for the rating alone, nor with another category's licence.
    #[test]
    fn night_vfr_recency_applies_to_the_rating_in_an_aeroplane() {
        let night_vfr = ["61.965(a)(i)", "61.965(a)(ii)"];
        assert_eq!(
            applying_to(&["private-aeroplane", "night-vfr-rating"]),
            [PASSENGER.as_slice(), &night_vfr].concat()
        );
        assert!(applying_to(&["night-vfr-rating"]).is_empty());
        assert!(applying_to(&["private-helicopter", "night-vfr-rating"]).is_empty());
    }

    /// The instrument rating with only the single-engine aeroplane
    /// instrument endorsement: instrument recency, reg 61.870(3) and the
    /// instrument proficiency check for aeroplanes only, and neither 2D nor
    /// 3D without their endorsements.
    #[test]
    fn the_instrument_rating_applies_in_its_endorsements_categories() {
        assert_eq!(
            applying_to(&[
                "instrument-rating",
                "single-engine-aeroplane-instrument-endorsement"
            ]),
            [
                "61.870(2)",
                "61.870(3)|aeroplane",
                "61.870(6)",
                "61.870(7)",
                "61.875",
                "61.880|aeroplane",
                "61.860(2)",
                "61.860(3)",
            ]
        );
        assert_eq!(applying_to(&["instrument-rating"]), INSTRUMENT_RATING_ROWS);
    }

    /// The instrument rating's own rows, which any instrument endorsement
    /// implies.
    const INSTRUMENT_RATING_ROWS: [&str; 6] = [
        "61.870(2)",
        "61.870(6)",
        "61.870(7)",
        "61.875",
        "61.860(2)",
        "61.860(3)",
    ];

    /// Each aircraft category or class instrument endorsement makes reg
    /// 61.870(3) and the instrument proficiency check apply for its category,
    /// and a multi-engine one the check for its multi-engine aircraft too,
    /// besides the instrument rating's rows.
    #[test]
    fn each_instrument_endorsement_applies_in_its_category() {
        let cases = [
            (
                "single-engine-aeroplane-instrument-endorsement",
                vec!["61.870(3)|aeroplane", "61.880|aeroplane"],
            ),
            (
                "multi-engine-aeroplane-instrument-endorsement",
                vec![
                    "61.870(3)|aeroplane",
                    "61.880|aeroplane",
                    "61.880|multi-engine-aeroplane",
                ],
            ),
            (
                "single-engine-helicopter-instrument-endorsement",
                vec!["61.870(3)|helicopter", "61.880|helicopter"],
            ),
            (
                "multi-engine-helicopter-instrument-endorsement",
                vec![
                    "61.870(3)|helicopter",
                    "61.880|helicopter",
                    "61.880|multi-engine-helicopter",
                ],
            ),
            (
                "powered-lift-aircraft-instrument-endorsement",
                vec!["61.870(3)|powered-lift", "61.880|powered-lift"],
            ),
            (
                "gyroplane-instrument-endorsement",
                vec!["61.870(3)|gyroplane", "61.880|gyroplane"],
            ),
            (
                "airship-instrument-endorsement",
                vec!["61.870(3)|airship", "61.880|airship"],
            ),
            ("iap-2d-instrument-endorsement", vec!["61.870(4)"]),
            (
                "iap-3d-instrument-endorsement",
                vec!["61.870(4)", "61.870(5)", "61.900(2)"],
            ),
        ];
        for (endorsement, expected) in cases {
            let applying = applying_to(&[endorsement]);
            let (rating, own): (Vec<_>, Vec<_>) = applying
                .iter()
                .partition(|id| INSTRUMENT_RATING_ROWS.contains(&id.as_str()));
            assert_eq!(rating, INSTRUMENT_RATING_ROWS, "{endorsement}");
            assert_eq!(own, expected, "{endorsement}");
        }
    }

    /// An endorsement recorded without the instrument rating implies it, and
    /// the IAP 3D instrument endorsement without the IAP 2D one implies that.
    #[test]
    fn an_endorsement_implies_what_it_is_granted_on() {
        assert_eq!(
            applying_to(&[
                "single-engine-aeroplane-instrument-endorsement",
                "iap-2d-instrument-endorsement"
            ]),
            [
                "61.870(2)",
                "61.870(3)|aeroplane",
                "61.870(4)",
                "61.870(6)",
                "61.870(7)",
                "61.875",
                "61.880|aeroplane",
                "61.860(2)",
                "61.860(3)",
            ]
        );
        assert_eq!(
            applying_to(&["instrument-rating", "iap-3d-instrument-endorsement"]),
            [
                "61.870(2)",
                "61.870(4)",
                "61.870(5)",
                "61.870(6)",
                "61.870(7)",
                "61.875",
                "61.860(2)",
                "61.860(3)",
                "61.900(2)",
            ]
        );
    }

    /// A flight review for each aeroplane class rating held, and none for a
    /// class rating CASA checks no flight review for.
    #[test]
    fn a_flight_review_applies_to_its_class_rating() {
        assert_eq!(
            applying_to(&["single-engine-aeroplane-class-rating"]),
            ["61.745|single-engine-aeroplane"]
        );
        assert_eq!(
            applying_to(&["multi-engine-aeroplane-class-rating"]),
            ["61.745|multi-engine-aeroplane"]
        );
        assert!(applying_to(&["single-engine-helicopter-class-rating"]).is_empty());
    }

    /// Every requirement applies to someone, in the order listed.
    #[test]
    fn every_requirement_applies_to_its_holder_in_the_order_listed() {
        let everything: Vec<String> = Casa::new()
            .ratings_and_endorsements()
            .into_iter()
            .map(|term| term.id.as_str().to_owned())
            .chain(["private-aeroplane".to_owned()])
            .collect();
        let everything: Vec<&str> = everything.iter().map(String::as_str).collect();
        let all: Vec<String> = requirements()
            .into_iter()
            .map(|requirement| requirement.id.as_str().to_owned())
            .collect();
        assert_eq!(applying_to(&everything), all);
    }

    /// Each requirement's kind and set, by its regulation, and the wording
    /// that varies with what it is for.
    #[test]
    fn each_requirement_has_its_kind_set_and_words() {
        for requirement in requirements() {
            let periodic = ["61.745", "61.880", "61.860(2)", "61.860(3)", "61.900(2)"]
                .contains(&requirement.regulation);
            let kind = if periodic {
                RequirementKind::PeriodicAssessment
            } else {
                RequirementKind::RecentExperience
            };
            assert_eq!(requirement.kind, kind, "{}", requirement.id.as_str());
            let set = [
                (
                    "61.395(1)",
                    "Carrying a passenger by day in an aeroplane",
                    "At least 3 take-offs and 3 landings, by day or night, in the past 90 days",
                ),
                (
                    "61.395(2)",
                    "Carrying a passenger at night in an aeroplane",
                    "At least 3 night take-offs and 3 night landings in the past 90 days",
                ),
                (
                    "61.965(a)",
                    "Recent experience for the night VFR rating in an aeroplane",
                    "At least 1 night take-off and 1 night landing in the past 6 months",
                ),
            ]
            .into_iter()
            .find(|(key, ..)| requirement.id.as_str().starts_with(&format!("{key}(")))
            .map(|(key, name, description)| RequirementSet::new(key, name, description));
            assert_eq!(requirement.set, set, "{}", requirement.id.as_str());
            // Each member's own name begins with its set's name.
            if let Some(set) = &requirement.set {
                assert!(
                    requirement.name.starts_with(&format!("{}: ", set.name)),
                    "{}",
                    requirement.id.as_str()
                );
            }
        }
        let by_id = |id: &str| {
            requirements()
                .into_iter()
                .find(|requirement| requirement.id.as_str() == id)
                .unwrap()
        };
        assert_eq!(
            by_id("61.870(3)|airship").description,
            "At least 1 instrument approach operation within the previous 90 days, in an \
             airship or an approved flight simulation training device"
        );
        assert_eq!(
            by_id("61.395(2)(b)").events,
            Some(EventsNoun {
                one: "night landing".into(),
                other: "night landings".into()
            })
        );
        assert_eq!(by_id("61.875").events, None);
    }

    #[test]
    fn each_requirement_is_answered_by_its_own_check_over_the_entries_that_count() {
        // Entry `e`, a single-engine aeroplane: 3 take-offs and landings by
        // day, and an ILS flown as an azimuth guidance operation.
        let flight = informed(
            EntryInformation {
                azimuth_guidance_operation: Some(true),
                ..EntryInformation::default()
            },
            EntryParts {
                take_offs: day(3),
                landings: day(3),
                approaches: vec![approach(ApproachType::Ils)],
                ..aeroplane("e", ON)
            },
        );
        // A session that does not count, in no approved device, with what
        // would make the night checks current.
        let session = EntryParts {
            take_offs: night(3),
            landings: night(3),
            ..simulated("s", ON, "C172")
        };
        let logbook = Logbook {
            entries: vec![entry(flight), entry(session)],
            assessments: vec![
                passed("review", CheckType::FlightReview),
                passed("ipc", CheckType::InstrumentProficiencyCheck),
            ],
        };
        let current: Vec<String> = currency(&Casa::new(), REFERENCE, &logbook)
            .into_iter()
            .filter(|(_, result)| result.is_current())
            .map(|(id, _)| id.as_str().to_owned())
            .collect();
        assert_eq!(
            current,
            [
                "61.395(1)(a)",
                "61.395(1)(b)",
                "61.870(3)|aeroplane",
                "61.870(5)",
                "61.870(6)",
                "61.745|single-engine-aeroplane",
                "61.880|aeroplane",
                // The proficiency check's flight was a single-pilot
                // operation with a 3D approach.
                "61.860(2)",
                "61.900(2)",
            ]
        );
    }

    /// Each requirement of regs 61.395(1), (2) and 61.965(a) is answered by
    /// the check that counts its own take-offs or landings.
    #[test]
    fn take_offs_and_landings_answer_their_own_requirements() {
        let flight = EntryParts {
            take_offs: DayNight { day: 4, night: 1 },
            landings: DayNight { day: 2, night: 0 },
            ..aeroplane("e", ON)
        };
        let logbook = Logbook {
            entries: vec![entry(flight)],
            assessments: Vec::new(),
        };
        let found: Vec<(String, usize)> = currency(&Casa::new(), REFERENCE, &logbook)
            .into_iter()
            .take(6)
            .map(|(id, result)| (id.as_str().to_owned(), result.found))
            .collect();
        let expected = [
            ("61.395(1)(a)", 5),
            ("61.395(1)(b)", 2),
            ("61.395(2)(a)", 1),
            ("61.395(2)(b)", 0),
            ("61.965(a)(i)", 1),
            ("61.965(a)(ii)", 0),
        ]
        .map(|(id, found)| (id.to_owned(), found));
        assert_eq!(found, expected);
    }

    /// Reg 61.395(2) allows an approved flight simulator, and reg 61.965(a)
    /// names none: an approved session's night take-off and landing count
    /// for the first and not for the second.
    #[test]
    fn an_approved_session_counts_for_night_passengers_and_not_for_night_vfr() {
        let session = informed(
            EntryInformation {
                approved_fstd: Some(true),
                ..EntryInformation::default()
            },
            EntryParts {
                take_offs: night(3),
                landings: night(3),
                ..simulated("s", ON, "C172")
            },
        );
        let logbook = Logbook {
            entries: vec![entry(session)],
            assessments: Vec::new(),
        };
        let found: Vec<(String, usize)> = currency(&Casa::new(), REFERENCE, &logbook)
            .into_iter()
            .take(6)
            .map(|(id, result)| (id.as_str().to_owned(), result.found))
            .collect();
        let expected = [
            ("61.395(1)(a)", 3),
            ("61.395(1)(b)", 3),
            ("61.395(2)(a)", 3),
            ("61.395(2)(b)", 3),
            ("61.965(a)(i)", 0),
            ("61.965(a)(ii)", 0),
        ]
        .map(|(id, found)| (id.to_owned(), found));
        assert_eq!(found, expected);
    }

    /// An approved session counts towards reg 61.870(3) for the category
    /// of the type its device represented.
    #[test]
    fn an_approved_session_representing_a_helicopter_answers_the_helicopter_category() {
        let session = informed(
            EntryInformation {
                approved_fstd: Some(true),
                ..EntryInformation::default()
            },
            EntryParts {
                approaches: vec![approach(ApproachType::Vor)],
                ..simulated("s", ON, "R44")
            },
        );
        let logbook = Logbook {
            entries: vec![entry(session)],
            assessments: Vec::new(),
        };
        let current: Vec<String> = currency(&Casa::new(), REFERENCE, &logbook)
            .into_iter()
            .filter(|(id, result)| id.as_str().starts_with("61.870(3)") && result.is_current())
            .map(|(id, _)| id.as_str().to_owned())
            .collect();
        assert_eq!(current, ["61.870(3)|helicopter"]);
    }
}
