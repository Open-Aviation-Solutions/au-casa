// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A validity that runs "to the end of the Nth month after the month in
//! which" a check is passed, and that a check passed shortly before it
//! expires renews from that expiry: the flight review (reg 61.745(1A)) and
//! the instrument proficiency check (reg 61.880(3)(f)).

use logbook_shared_kernel::authority::window::{add_months_clamped, end_of_nth_month_after};
use time::Date;

/// The last day each of `passes` is valid to, in their order, which is by
/// date, earliest first.
///
/// A pass is valid to the end of the `valid_months`th month after its own
/// month, unless it is "within" `renewal_months` "before the validity of
/// the existing check expires": then it runs from that expiry, as though it
/// were passed in the month the existing one ends. The existing check is
/// the one valid longest of those passed before it. The validity ends with
/// its last day, so the renewal window opens `renewal_months` back from the
/// day after, and includes that last day.
pub(super) fn valid_to(passes: &[Date], valid_months: u32, renewal_months: i32) -> Vec<Date> {
    let mut existing: Option<Date> = None;
    passes
        .iter()
        .map(|&on| {
            let renewed = existing.filter(|last| {
                let window_opens = last
                    .next_day()
                    .and_then(|expires| add_months_clamped(expires, -renewal_months))
                    .unwrap_or(Date::MIN);
                window_opens <= on && on <= *last
            });
            let to =
                end_of_nth_month_after(renewed.unwrap_or(on), valid_months).unwrap_or(Date::MAX);
            existing = Some(existing.map_or(to, |last| last.max(to)));
            to
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use time::macros::date;

    use super::*;

    #[test]
    fn a_pass_is_valid_to_the_end_of_its_nth_month() {
        assert_eq!(
            valid_to(&[date!(2024 - 03 - 10)], 24, 3),
            [date!(2026 - 03 - 31)]
        );
    }

    /// The window opens on the 1st of the month `renewal_months` before the
    /// month after the last valid day's, and ends with that last day.
    #[test]
    fn a_pass_in_the_window_runs_from_the_expiry() {
        let first = date!(2024 - 03 - 10);
        for (on, to) in [
            (date!(2025 - 12 - 31), date!(2027 - 12 - 31)),
            (date!(2026 - 01 - 01), date!(2028 - 03 - 31)),
            (date!(2026 - 03 - 31), date!(2028 - 03 - 31)),
            (date!(2026 - 04 - 01), date!(2028 - 04 - 30)),
        ] {
            assert_eq!(valid_to(&[first, on], 24, 3)[1], to, "{on}");
        }
    }

    /// A pass outside the window is valid from its own month, and the
    /// existing validity, the longest so far, is not shortened by it.
    #[test]
    fn a_pass_after_a_renewal_renews_from_the_longest() {
        let to = valid_to(
            &[
                date!(2024 - 03 - 10),
                date!(2026 - 01 - 05),
                date!(2026 - 02 - 05),
            ],
            24,
            3,
        );
        assert_eq!(
            to,
            [
                date!(2026 - 03 - 31),
                date!(2028 - 03 - 31),
                date!(2028 - 02 - 29)
            ]
        );
    }
}
