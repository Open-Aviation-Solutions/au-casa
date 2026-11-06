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

use aircraft_type_descriptions::DesignatorTable;
use icao_shared_kernel::{
    Aircraft, AircraftRegistration, AircraftType, Capacity, FlightDuration, NationalityMark,
};
use logbook_shared_kernel::{
    AircraftDescription, DayNight, Entry, EntryId, EntryParts, InstrumentTime, Subject,
};
use time::Date;
use uuid::Uuid;

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

/// The description the table has for a designator, if it has one.
fn described(designator: &str) -> Option<AircraftDescription> {
    DesignatorTable::packaged()
        .lookup(designator)
        .map(|facts| facts.description)
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
            description: described(designator),
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
            simulated: Some((
                AircraftType::parse(designator).unwrap(),
                described(designator),
            )),
        },
    )
}
