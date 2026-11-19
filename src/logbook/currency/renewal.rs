// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A validity that runs "to the end of the Nth month after the month in
//! which" a check is passed, and that a check passed shortly before it
//! expires renews from that expiry: the flight review (reg 61.745(1A)) and
//! the instrument proficiency check (reg 61.880(3)(f)).

use logbook_shared_kernel::authority::window::{add_months_clamped, end_of_nth_month_after};
use time::Date;

/// What [`validity`] found.
pub(super) struct Validity {
    /// The last day each pass is valid to, in the passes' order.
    pub(super) each: Vec<Date>,
    /// The longest validity and the pass that gives it (the latest, of
    /// passes that tie), if there are any passes.
    pub(super) longest: Option<(Date, usize)>,
}

/// The validity of `passes`, each a date and whether it may renew early,
/// ordered by date, earliest first.
///
/// A pass is valid to the end of the `valid_months`th month after its own
/// month, unless it may renew and is "within" `renewal_months` "before the
/// validity of the existing check expires": then it runs from that expiry,
/// as though it were passed in the month the existing one ends. The
/// existing one is the one valid longest of those passed before it (a
/// reading, kept to the longest alone: see the pull request that added it).
/// The validity ends with its last day, so the renewal window opens
/// `renewal_months` back from the day after, and includes that last day.
pub(super) fn validity(
    passes: &[(Date, bool)],
    valid_months: u32,
    renewal_months: i32,
) -> Validity {
    debug_assert!(
        passes.windows(2).all(|pair| pair[0].0 <= pair[1].0),
        "passes are ordered by date"
    );
    let mut longest: Option<(Date, usize)> = None;
    let each = passes
        .iter()
        .enumerate()
        .map(|(index, &(on, may_renew))| {
            let renewed = longest.map(|(last, _)| last).filter(|last| {
                let window_opens = last
                    .next_day()
                    .and_then(|expires| add_months_clamped(expires, -renewal_months))
                    .unwrap_or(Date::MIN);
                may_renew && window_opens <= on && on <= *last
            });
            let to =
                end_of_nth_month_after(renewed.unwrap_or(on), valid_months).unwrap_or(Date::MAX);
            if longest.is_none_or(|(last, _)| to >= last) {
                longest = Some((to, index));
            }
            to
        })
        .collect();
    Validity { each, longest }
}

#[cfg(test)]
mod tests {
    use time::macros::date;

    use super::*;

    fn each(passes: &[(Date, bool)]) -> Vec<Date> {
        validity(passes, 24, 3).each
    }

    #[test]
    fn a_pass_is_valid_to_the_end_of_its_nth_month() {
        let v = validity(&[(date!(2024 - 03 - 10), true)], 24, 3);
        assert_eq!(v.each, [date!(2026 - 03 - 31)]);
        assert_eq!(v.longest, Some((date!(2026 - 03 - 31), 0)));
        assert!(validity(&[], 24, 3).longest.is_none());
    }

    /// The window opens on the 1st of the month `renewal_months` before the
    /// month after the last valid day's, and ends with that last day.
    #[test]
    fn a_pass_in_the_window_runs_from_the_expiry() {
        let first = (date!(2024 - 03 - 10), true);
        for (on, to) in [
            (date!(2025 - 12 - 31), date!(2027 - 12 - 31)),
            (date!(2026 - 01 - 01), date!(2028 - 03 - 31)),
            (date!(2026 - 03 - 31), date!(2028 - 03 - 31)),
            (date!(2026 - 04 - 01), date!(2028 - 04 - 30)),
        ] {
            assert_eq!(each(&[first, (on, true)])[1], to, "{on}");
        }
    }

    /// A pass that may not renew is valid from its own month, in the window
    /// or not, and does not shorten the existing validity.
    #[test]
    fn a_pass_that_may_not_renew_runs_from_its_own_month() {
        let v = validity(
            &[
                (date!(2024 - 03 - 10), true),
                (date!(2026 - 01 - 15), false),
            ],
            24,
            3,
        );
        assert_eq!(v.each, [date!(2026 - 03 - 31), date!(2028 - 01 - 31)]);
        assert_eq!(v.longest, Some((date!(2028 - 01 - 31), 1)));
    }

    /// A pass is renewed from the longest validity so far, not the latest
    /// pass's: the third is valid for less than the second it follows, and
    /// the fourth is in its window but not the second's.
    #[test]
    fn a_pass_renews_from_the_longest_not_the_latest() {
        let v = validity(
            &[
                (date!(2024 - 03 - 10), true),
                (date!(2026 - 01 - 05), true),
                (date!(2026 - 02 - 05), true),
                (date!(2027 - 12 - 15), true),
            ],
            24,
            3,
        );
        assert_eq!(
            v.each,
            [
                date!(2026 - 03 - 31),
                date!(2028 - 03 - 31),
                date!(2028 - 02 - 29),
                date!(2029 - 12 - 31),
            ]
        );
        assert_eq!(v.longest, Some((date!(2029 - 12 - 31), 3)));
    }
}
