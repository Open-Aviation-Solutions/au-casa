// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The classification guards CASA's checks share.
//!
//! An entry's aircraft is classified by [`classify`], as pilot-logbook
//! classified it: from the table's description of its type designator (the
//! flight's aircraft, or the type a session's device represented) or, for a
//! designator the table does not catalogue, from the description the entry
//! carries. Where a guard *requires* a category, an entry with neither does
//! not have it: a currency claim needs positive evidence, and "the type
//! could not be described" is not evidence that it was an aeroplane.

use aircraft_type_descriptions::DesignatorTable;
use logbook_shared_kernel::{AircraftDescription, Entry, Subject};

use crate::aircraft::{classify, AircraftCategory};

/// The description of the type the entry was flown in, or that its session
/// represented: the table's, else the entry's; `None` when neither has one,
/// or a session's type was not recorded.
fn description(entry: &Entry) -> Option<AircraftDescription> {
    let (designator, description) = match entry.subject() {
        Subject::Flight {
            aircraft,
            description,
            ..
        } => (&aircraft.aircraft_type, *description),
        Subject::Session { simulated, .. } => {
            let (designator, description) = simulated.as_ref()?;
            (designator, *description)
        }
    };
    tabled(designator.as_str()).or(description)
}

/// The table's description of a type designator.
pub(crate) fn tabled(designator: &str) -> Option<AircraftDescription> {
    DesignatorTable::packaged()
        .lookup(designator)
        .map(|facts| facts.description)
}

/// Whether the type is *known* to be of this category (reg 61.015, or reg
/// 61.007(2) for a registered sailplane).
pub(crate) fn is_category(entry: &Entry, category: AircraftCategory) -> bool {
    description(entry).is_some_and(|description| classify(description).0 == category)
}

#[cfg(test)]
mod tests {
    use aircraft_type_descriptions::{AirframeKind, EngineType};
    use logbook_shared_kernel::Subject;
    use time::macros::date;

    use super::*;
    use crate::logbook::test_support::{aeroplane, entry, unclassified};

    const ON: time::Date = date!(2024 - 08 - 20);

    /// The parts, with `description` as the flight's description.
    fn describing(
        mut parts: logbook_shared_kernel::EntryParts,
        with: Option<AircraftDescription>,
    ) -> Entry {
        if let Subject::Flight { description, .. } = &mut parts.subject {
            *description = with;
        }
        entry(parts)
    }

    fn helicopter() -> Option<AircraftDescription> {
        Some(AircraftDescription::new(
            AirframeKind::Helicopter,
            1,
            EngineType::Piston,
        ))
    }

    #[test]
    fn with_no_description_the_tables_description_of_the_designator_is_used() {
        let entry = describing(aeroplane("a", ON), None);
        assert!(is_category(&entry, AircraftCategory::Aeroplane));
    }

    #[test]
    fn the_tables_description_is_used_over_the_entrys() {
        // A C172, which the table catalogues, described as a helicopter.
        let entry = describing(aeroplane("a", ON), helicopter());
        assert!(is_category(&entry, AircraftCategory::Aeroplane));
        assert!(!is_category(&entry, AircraftCategory::Helicopter));
    }

    #[test]
    fn for_an_uncatalogued_designator_the_entrys_description_is_used() {
        let entry = describing(unclassified("a", ON), helicopter());
        assert!(is_category(&entry, AircraftCategory::Helicopter));
        assert!(!is_category(&entry, AircraftCategory::Aeroplane));
    }
}
