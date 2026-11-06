// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Entry builders for the checks' own tests.
//!
//! Each builds the parts of an entry, so that a test sets what it is about
//! (`EntryParts { take_offs: night(3), ..aeroplane("a", on) }`) and passes
//! them to [`entry`]. A type is described by the real table in
//! aircraft-type-descriptions, as a host would describe it.

use std::collections::BTreeMap;

use icao_shared_kernel::{
    Aircraft, AircraftRegistration, AircraftType, Capacity, FlightDuration, LocationIndicator,
    NationalityMark,
};
use logbook_shared_kernel::{
    Approach, ApproachType, Assessment, AssessmentId, DayNight, Entry, EntryId, EntryParts,
    InstrumentTime, Questions, Subject,
};
use time::Date;
use uuid::Uuid;

use crate::logbook::guards::tabled;
use crate::logbook::{casa, AssessmentInformation, EntryInformation};

/// The entry these parts make.
pub(crate) fn entry(parts: EntryParts) -> Entry {
    Entry::new(parts).expect("the test's parts make an entry")
}

/// `count` by day.
pub(crate) fn day(count: u32) -> DayNight<u32> {
    DayNight {
        day: count,
        night: 0,
    }
}

/// `count` at night.
pub(crate) fn night(count: u32) -> DayNight<u32> {
    DayNight {
        day: 0,
        night: count,
    }
}

/// An hour's flight as pilot in command, in an aircraft of `subject`, with
/// no take-offs or landings.
fn parts(id: &str, on: Date, subject: Subject) -> EntryParts {
    EntryParts {
        id: EntryId::new(id),
        date: on,
        subject,
        route: None,
        flight_rules: None,
        flight_time: FlightDuration::new(60).unwrap(),
        night: FlightDuration::new(0).unwrap(),
        capacity: Capacity::PilotInCommand,
        pilot_in_command: None,
        flight_crew: Vec::new(),
        instrument: InstrumentTime::default(),
        approaches: Vec::new(),
        take_offs: DayNight::default(),
        landings: DayNight::default(),
        authority_information: BTreeMap::new(),
    }
}

/// A flight in a type, described by the table (no description if it does
/// not catalogue the designator).
pub(crate) fn flown(id: &str, on: Date, designator: &str) -> EntryParts {
    parts(
        id,
        on,
        Subject::Flight {
            flight: Uuid::nil(),
            aircraft: Aircraft::new(
                AircraftType::parse(designator).unwrap(),
                AircraftRegistration::new(NationalityMark::parse("VH").unwrap(), "ABC").unwrap(),
            ),
            description: tabled(designator),
            sole_occupant: false,
        },
    )
}

/// A flight in a single-engine aeroplane (a C172).
pub(crate) fn aeroplane(id: &str, on: Date) -> EntryParts {
    flown(id, on, "C172")
}

/// A flight in a registered sailplane (an ASK 21).
pub(crate) fn sailplane(id: &str, on: Date) -> EntryParts {
    flown(id, on, "AS21")
}

/// A flight in a type with no description, which the table does not
/// catalogue either (`ZZZZ`, no type designator assigned): no known
/// category, so a check that requires one must not count it.
pub(crate) fn unclassified(id: &str, on: Date) -> EntryParts {
    flown(id, on, "ZZZZ")
}

/// A session in a device that represented a type, described by the table.
/// Whether the device was approved is CASA's information, which
/// `Casa::counts` reads before any check sees the entry, so
/// it is not set here.
pub(crate) fn simulated(id: &str, on: Date, designator: &str) -> EntryParts {
    parts(
        id,
        on,
        Subject::Session {
            session: Uuid::nil(),
            device: Uuid::nil(),
            simulated: Some((AircraftType::parse(designator).unwrap(), tabled(designator))),
        },
    )
}

/// An approach of this procedure, flown straight in (not circled).
pub(crate) fn approach(procedure: ApproachType) -> Approach {
    Approach::new(procedure, LocationIndicator::parse("YBAF").unwrap(), false)
}

/// An approach of this procedure that ended in a circling manoeuvre.
pub(crate) fn circled(procedure: ApproachType) -> Approach {
    Approach::new(procedure, LocationIndicator::parse("YBAF").unwrap(), true)
}

/// The parts, with CASA's information recording `count` DME or GNSS
/// arrivals.
pub(crate) fn arrivals(count: u64, parts: EntryParts) -> EntryParts {
    informed(
        EntryInformation {
            dme_or_gnss_arrivals: Some(count),
            ..EntryInformation::default()
        },
        parts,
    )
}

/// The parts, with this as CASA's information, stamped with its version as
/// the host stores it.
pub(crate) fn informed(information: EntryInformation, mut parts: EntryParts) -> EntryParts {
    let mut information = information.to_information();
    Questions::new(EntryInformation::VERSION, Vec::new()).stamp(&mut information);
    parts.authority_information.insert(casa(), information);
    parts
}

/// The parts of a flight, as flight `flight`.
pub(crate) fn on_flight(flight: u128, mut parts: EntryParts) -> EntryParts {
    if let Subject::Flight { flight: id, .. } = &mut parts.subject {
        *id = Uuid::from_u128(flight);
    }
    parts
}

/// An assessment on flight `flight`, with this as CASA's information,
/// stamped with its version as the host stores it.
pub(crate) fn assessed(
    id: &str,
    on: Date,
    flight: u128,
    information: AssessmentInformation,
) -> Assessment {
    let mut information = information.to_information();
    Questions::new(AssessmentInformation::VERSION, Vec::new()).stamp(&mut information);
    Assessment {
        id: AssessmentId::new(id),
        date: on,
        flight: Uuid::from_u128(flight),
        authority_information: BTreeMap::from([(casa(), information)]),
    }
}
