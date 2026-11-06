// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The classification guards CASA's checks share.
//!
//! An entry's aircraft is classified from the description of its type that
//! the entry carries (the flight's aircraft, or the type a session's device
//! represented), by [`classify`]. Where a guard *requires* a category, an
//! entry with no description does not have it: a currency claim needs
//! positive evidence, and "the type could not be described" is not evidence
//! that it was an aeroplane.

use logbook_shared_kernel::{AircraftDescription, Entry, Subject};

use crate::aircraft::{classify, AircraftCategory};

/// The description of the type the entry was flown in, or that its session
/// represented; `None` when it carries none.
fn description(entry: &Entry) -> Option<AircraftDescription> {
    match entry.subject() {
        Subject::Flight { description, .. } => *description,
        Subject::Session { simulated, .. } => simulated.as_ref().and_then(|(_, d)| *d),
    }
}

/// Whether the type is *known* to be of this category (reg 61.015, or reg
/// 61.007(2) for a registered sailplane).
pub(crate) fn is_category(entry: &Entry, category: AircraftCategory) -> bool {
    description(entry).is_some_and(|description| classify(description).0 == category)
}
