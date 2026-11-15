// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's own totals, each a kernel [`Total`] over a [`Measure`] of its
//! own. The totals the kernel's common part answers alone (the hours by
//! capacity, by day and night, instrument time) are not CASA's, and are
//! not here.

use icao_shared_kernel::FlightDuration;
use logbook_shared_kernel::{
    run, AuthorityTotal, DateRange, Entry, EntryAccumulator, Logbook, Measure, Subject, TermId,
    Total,
};

use crate::aircraft::{AircraftCategory, AircraftClassRating};
use crate::logbook::guards::{is_category, is_class_rating};
use crate::logbook::{category_id, class_id, Casa};

/// An entry's flight time, in minutes, when `counts`; otherwise none.
fn flight_time_if(entry: &Entry, counts: bool) -> u64 {
    if counts {
        entry.flight_time().total_minutes().unsigned_abs()
    } else {
        0
    }
}

/// The flight time of an entry known to be in an aeroplane (reg 61.015):
/// an unknown type is not counted, as the checks' guards have it.
struct Aeroplane;
impl Measure for Aeroplane {
    fn measure(entry: &Entry) -> u64 {
        flight_time_if(entry, is_category(entry, AircraftCategory::Aeroplane))
    }
}

/// The same, for a single-engine aeroplane (reg 61.020).
struct SingleEngineAeroplane;
impl Measure for SingleEngineAeroplane {
    fn measure(entry: &Entry) -> u64 {
        flight_time_if(
            entry,
            is_class_rating(entry, AircraftClassRating::SingleEngineAeroplane),
        )
    }
}

/// The same, for a multi-engine aeroplane (reg 61.020).
struct MultiEngineAeroplane;
impl Measure for MultiEngineAeroplane {
    fn measure(entry: &Entry) -> u64 {
        flight_time_if(
            entry,
            is_class_rating(entry, AircraftClassRating::MultiEngineAeroplane),
        )
    }
}

/// The same, for a registered sailplane (reg 61.007(2)).
struct RegisteredSailplane;
impl Measure for RegisteredSailplane {
    fn measure(entry: &Entry) -> u64 {
        flight_time_if(
            entry,
            is_category(entry, AircraftCategory::RegisteredSailplane),
        )
    }
}

/// A session's time: "time spent in a flight simulation training device
/// during which a pilot is performing the duties of a pilot" (reg 61.010,
/// *simulated flight time*). Only a session in an approved flight
/// simulation training device reaches a total, since `counts` refuses the
/// rest.
struct SimulatedFlightTime;
impl Measure for SimulatedFlightTime {
    fn measure(entry: &Entry) -> u64 {
        flight_time_if(entry, matches!(entry.subject(), Subject::Session { .. }))
    }
}

/// A flight's time when the pilot was "the sole occupant of the aircraft"
/// (reg 61.010, *solo*). Whatever the entry's capacity: no rule here makes
/// solo count as pilot in command, or the reverse.
struct SoloFlightTime;
impl Measure for SoloFlightTime {
    fn measure(entry: &Entry) -> u64 {
        let solo = matches!(
            entry.subject(),
            Subject::Flight {
                sole_occupant: true,
                ..
            }
        );
        flight_time_if(entry, solo)
    }
}

/// CASA's totals of the entries dated in `range`, through the kernel's
/// `run`, so over the entries `Casa::counts` lets through: aeroplane time,
/// then single-engine and multi-engine aeroplane time, registered
/// sailplane time, simulated flight time and solo flight time. A category's
/// or a class's total has its id, and each class is part of aeroplane time.
///
/// A session counts towards the category and class of the type its device
/// represented, as pilot-logbook counted it, as well as towards simulated
/// flight time.
pub(super) fn totals(casa: &Casa, logbook: &Logbook, range: DateRange) -> Vec<AuthorityTotal> {
    let mut aeroplane = Total::<Aeroplane>::new(range);
    let mut single_engine = Total::<SingleEngineAeroplane>::new(range);
    let mut multi_engine = Total::<MultiEngineAeroplane>::new(range);
    let mut sailplane = Total::<RegisteredSailplane>::new(range);
    let mut simulated = Total::<SimulatedFlightTime>::new(range);
    let mut solo = Total::<SoloFlightTime>::new(range);
    run(
        casa,
        &logbook.entries,
        &mut [
            &mut aeroplane as &mut dyn EntryAccumulator,
            &mut single_engine,
            &mut multi_engine,
            &mut sailplane,
            &mut simulated,
            &mut solo,
        ],
    );
    let aeroplane_id = category_id(AircraftCategory::Aeroplane);
    let class = |class| (class_id(class), Some(aeroplane_id));
    [
        ((aeroplane_id, None), "Aeroplane", aeroplane.total()),
        (
            class(AircraftClassRating::SingleEngineAeroplane),
            "Single-engine aeroplane",
            single_engine.total(),
        ),
        (
            class(AircraftClassRating::MultiEngineAeroplane),
            "Multi-engine aeroplane",
            multi_engine.total(),
        ),
        (
            (category_id(AircraftCategory::RegisteredSailplane), None),
            "Registered sailplane",
            sailplane.total(),
        ),
        (
            ("simulated-flight-time", None),
            "Simulated flight time",
            simulated.total(),
        ),
        (("solo-flight-time", None), "Solo flight time", solo.total()),
    ]
    .into_iter()
    .map(|((id, part_of), name, minutes)| AuthorityTotal {
        id: TermId::new(id),
        name: name.to_owned(),
        // A sum more than a duration can hold is held at the most it can.
        time: FlightDuration::new(i64::try_from(minutes).unwrap_or(i64::MAX))
            .expect("not negative"),
        part_of: part_of.map(TermId::new),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use logbook_shared_kernel::{AircraftCharacteristic, EntryParts};
    use time::macros::date;
    use time::Date;

    use super::*;
    use crate::logbook::test_support::{
        aeroplane, entry, flown, informed, sailplane, simulated, unclassified,
    };
    use crate::logbook::EntryInformation;

    const ON: Date = date!(2024 - 08 - 20);

    /// Each total's minutes, by name.
    fn named_totals(entries: Vec<Entry>, range: DateRange) -> Vec<(String, i64)> {
        let logbook = Logbook {
            entries,
            assessments: Vec::new(),
        };
        totals(&Casa::new(), &logbook, range)
            .into_iter()
            .map(|total| (total.name, total.time.total_minutes()))
            .collect()
    }

    /// The minutes of each total, in their order.
    fn minutes(entries: Vec<Entry>) -> Vec<i64> {
        named_totals(entries, DateRange::default())
            .into_iter()
            .map(|(_, minutes)| minutes)
            .collect()
    }

    fn approved(parts: EntryParts) -> EntryParts {
        informed(
            EntryInformation {
                approved_fstd: Some(true),
                ..EntryInformation::default()
            },
            parts,
        )
    }

    #[test]
    fn the_totals_are_named_in_order() {
        let names: Vec<String> = named_totals(Vec::new(), DateRange::default())
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(
            names,
            [
                "Aeroplane",
                "Single-engine aeroplane",
                "Multi-engine aeroplane",
                "Registered sailplane",
                "Simulated flight time",
                "Solo flight time",
            ]
        );
    }

    #[test]
    fn each_total_has_its_id_and_the_classes_are_part_of_aeroplane_time() {
        let layout: Vec<(String, Option<String>)> =
            totals(&Casa::new(), &Logbook::default(), DateRange::default())
                .into_iter()
                .map(|total| {
                    (
                        total.id.as_str().to_owned(),
                        total.part_of.map(|id| id.as_str().to_owned()),
                    )
                })
                .collect();
        let aeroplane = || Some("aeroplane".to_owned());
        assert_eq!(
            layout,
            [
                ("aeroplane".to_owned(), None),
                ("single-engine-aeroplane".to_owned(), aeroplane()),
                ("multi-engine-aeroplane".to_owned(), aeroplane()),
                ("registered-sailplane".to_owned(), None),
                ("simulated-flight-time".to_owned(), None),
                ("solo-flight-time".to_owned(), None),
            ]
        );
    }

    /// Reg 61.020(2)(a), from the characteristics the flight records.
    #[test]
    fn a_centre_line_thrust_flight_counts_as_single_engine() {
        let mut parts = flown("a", ON, "PA44");
        if let Subject::Flight {
            characteristics, ..
        } = &mut parts.subject
        {
            characteristics.insert(AircraftCharacteristic::MultiEngineCentreLineThrust);
        }
        assert_eq!(minutes(vec![entry(parts)]), [60, 60, 0, 0, 0, 0]);
    }

    #[test]
    fn each_type_counts_towards_its_category_and_class_only() {
        // (aeroplane, single-engine, multi-engine, sailplane, simulated, solo)
        assert_eq!(
            minutes(vec![entry(aeroplane("a", ON))]),
            [60, 60, 0, 0, 0, 0]
        );
        assert_eq!(
            minutes(vec![entry(flown("a", ON, "PA44"))]),
            [60, 0, 60, 0, 0, 0]
        );
        assert_eq!(
            minutes(vec![entry(sailplane("a", ON))]),
            [0, 0, 0, 60, 0, 0]
        );
        // A helicopter, and a type of no known category.
        assert_eq!(minutes(vec![entry(flown("a", ON, "R44"))]), [0; 6]);
        assert_eq!(minutes(vec![entry(unclassified("a", ON))]), [0; 6]);
    }

    #[test]
    fn a_session_counts_only_in_an_approved_device() {
        let session = || simulated("s", ON, "C172");
        assert_eq!(minutes(vec![entry(session())]), [0; 6]);
        assert_eq!(
            minutes(vec![entry(approved(session()))]),
            [60, 60, 0, 0, 60, 0]
        );
    }

    #[test]
    fn solo_flight_time_is_the_flights_with_the_pilot_the_sole_occupant() {
        let mut parts = aeroplane("a", ON);
        if let Subject::Flight { sole_occupant, .. } = &mut parts.subject {
            *sole_occupant = true;
        }
        assert_eq!(minutes(vec![entry(parts)]), [60, 60, 0, 0, 0, 60]);
    }

    #[test]
    fn only_the_entries_dated_in_the_range_are_totalled() {
        let range = DateRange {
            from: Some(ON),
            until: Some(ON),
        };
        let entries = vec![
            entry(aeroplane("a", ON.previous_day().unwrap())),
            entry(aeroplane("b", ON)),
            entry(aeroplane("c", ON.next_day().unwrap())),
        ];
        assert_eq!(
            named_totals(entries, range)[0],
            ("Aeroplane".to_owned(), 60)
        );
    }
}
