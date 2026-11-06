// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! CASA's currency checks, one module for each regulation, written over the
//! kernel's [`EventWindow`](logbook_shared_kernel::EventWindow). Each sees
//! only the entries [`Casa::counts`](crate::Casa) lets through, so none
//! repeats that gate.

mod instrument_approaches;
mod instrument_approaches_2d;
mod instrument_approaches_3d;
mod night_vfr;
mod passenger_day;
mod passenger_night;
mod single_pilot_ifr;
