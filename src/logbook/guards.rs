// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The classification guards CASA's checks share.
//!
//! An entry's aircraft is classified by [`classify`], from the description
//! of its type that the entry carries (the flight's aircraft, or the type a
//! session's device represented), or, when it carries none, from the table's
//! description of its type designator. Where a guard *requires* a category,
//! an entry with neither does not have it: a currency claim needs positive
//! evidence, and "the type could not be described" is not evidence that it
//! was an aeroplane.

use aircraft_type_descriptions::DesignatorTable;
use icao_shared_kernel::AircraftType;
use logbook_shared_kernel::{AircraftDescription, Entry, Subject};

use crate::aircraft::{classify, AircraftCategory};

/// The description of the type the entry was flown in, or that its session
/// represented; `None` when it carries none and the table has none, or a
/// session's type was not recorded.
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
    description.or_else(|| tabled(designator))
}

/// The table's description of a type designator.
fn tabled(designator: &AircraftType) -> Option<AircraftDescription> {
    DesignatorTable::packaged()
        .lookup(designator.as_str())
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

    #[test]
    fn with_no_description_the_tables_description_of_the_designator_is_used() {
        let mut parts = aeroplane("a", ON);
        if let Subject::Flight { description, .. } = &mut parts.subject {
            *description = None;
        }
        assert!(is_category(&entry(parts), AircraftCategory::Aeroplane));
    }

    #[test]
    fn the_entrys_description_is_used_over_the_tables() {
        let mut parts = unclassified("a", ON);
        if let Subject::Flight { description, .. } = &mut parts.subject {
            *description = Some(AircraftDescription::new(
                AirframeKind::Helicopter,
                1,
                EngineType::Piston,
            ));
        }
        let entry = entry(parts);
        assert!(is_category(&entry, AircraftCategory::Helicopter));
        assert!(!is_category(&entry, AircraftCategory::Aeroplane));
    }
}
