//! Pure date-math helpers for `ScheduledGrant`. Lives in `billing::` so
//! tests can run on the host target without depending on `js_sys::Date`.
//!
//! Inputs and outputs are ISO-8601 strings; only the date portion
//! (`YYYY-MM-DD`) is used for cadence math, so timestamps can carry any
//! time-of-day or fractional seconds without tripping the parser.

use crate::types::GrantCadence;

/// Compute the next run timestamp strictly after `after_iso` for `cadence`.
/// Returns an ISO-8601 timestamp at 00:00 UTC on the resulting date.
///
/// "Strictly after" matters: we never want a cron tick to re-fire the same
/// scheduled grant in the same window if `next_run_at` happens to equal
/// the current instant.
pub fn next_run_after(after_iso: &str, cadence: GrantCadence) -> String {
    let (mut y, mut m, mut d) = parse_ymd(after_iso).unwrap_or((1970, 1, 1));
    // Always step at least one day forward.
    let (ny, nm, nd) = add_one_day(y, m, d);
    y = ny;
    m = nm;
    d = nd;

    // For daily cadence, one step is enough.
    // For weekly / monthly_first, keep stepping until the target date matches.
    let max_steps = 366; // safety: should never need more than a year
    for _ in 0..max_steps {
        if matches_cadence(y, m, d, cadence) {
            return format!("{y:04}-{m:02}-{d:02}T00:00:00Z");
        }
        let (ny, nm, nd) = add_one_day(y, m, d);
        y = ny;
        m = nm;
        d = nd;
    }
    // Pathological fallback — a year of forward stepping should always
    // hit Daily/Weekly/MonthlyFirst, so this is unreachable in practice.
    format!("{y:04}-{m:02}-{d:02}T00:00:00Z")
}

fn matches_cadence(y: i32, m: u32, d: u32, cadence: GrantCadence) -> bool {
    match cadence {
        GrantCadence::Daily => true,
        GrantCadence::MonthlyFirst => d == 1,
        GrantCadence::Weekly(target_dow) => day_of_week(y, m, d) == target_dow,
    }
}

/// Sakamoto's algorithm. Returns 0..6 with 0 = Sunday.
fn day_of_week(y: i32, m: u32, d: u32) -> u8 {
    let t: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if m < 3 { y - 1 } else { y };
    let mi = (m - 1) as usize;
    let dow = (y + y / 4 - y / 100 + y / 400 + t[mi] + d as i32) % 7;
    ((dow + 7) % 7) as u8
}

fn parse_ymd(iso: &str) -> Option<(i32, u32, u32)> {
    // Accept anything that starts with "YYYY-MM-DD".
    let bytes = iso.as_bytes();
    if bytes.len() < 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let y: i32 = iso.get(0..4)?.parse().ok()?;
    let m: u32 = iso.get(5..7)?.parse().ok()?;
    let d: u32 = iso.get(8..10)?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

fn add_one_day(y: i32, m: u32, d: u32) -> (i32, u32, u32) {
    let dim = days_in_month(y, m);
    if d < dim {
        (y, m, d + 1)
    } else if m < 12 {
        (y, m + 1, 1)
    } else {
        (y + 1, 1, 1)
    }
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dow_known_dates() {
        // 2026-05-02 is a Saturday.
        assert_eq!(day_of_week(2026, 5, 2), 6);
        // 2026-01-01 is a Thursday.
        assert_eq!(day_of_week(2026, 1, 1), 4);
        // 2024-02-29 (leap day) is a Thursday.
        assert_eq!(day_of_week(2024, 2, 29), 4);
    }

    #[test]
    fn add_one_day_rolls_month() {
        assert_eq!(add_one_day(2026, 1, 31), (2026, 2, 1));
        assert_eq!(add_one_day(2026, 2, 28), (2026, 3, 1));
        assert_eq!(add_one_day(2024, 2, 28), (2024, 2, 29)); // leap
        assert_eq!(add_one_day(2024, 2, 29), (2024, 3, 1));
        assert_eq!(add_one_day(2026, 12, 31), (2027, 1, 1));
    }

    #[test]
    fn daily_advances_one_day() {
        assert_eq!(
            next_run_after("2026-05-02T13:00:00Z", GrantCadence::Daily),
            "2026-05-03T00:00:00Z"
        );
    }

    #[test]
    fn monthly_first_advances_to_next_month() {
        // Mid-month → next 1st.
        assert_eq!(
            next_run_after("2026-05-02T00:00:00Z", GrantCadence::MonthlyFirst),
            "2026-06-01T00:00:00Z"
        );
        // Already on the 1st → next month's 1st (strictly-after rule).
        assert_eq!(
            next_run_after("2026-05-01T00:00:00Z", GrantCadence::MonthlyFirst),
            "2026-06-01T00:00:00Z"
        );
        // Year rollover.
        assert_eq!(
            next_run_after("2026-12-15T00:00:00Z", GrantCadence::MonthlyFirst),
            "2027-01-01T00:00:00Z"
        );
    }

    #[test]
    fn weekly_advances_to_target_dow() {
        // 2026-05-02 is Saturday (6). Next Monday (1) = 2026-05-04.
        assert_eq!(
            next_run_after("2026-05-02T00:00:00Z", GrantCadence::Weekly(1)),
            "2026-05-04T00:00:00Z"
        );
        // From a Monday to the next Monday — should advance 7 days.
        // 2026-05-04 is a Monday.
        assert_eq!(
            next_run_after("2026-05-04T00:00:00Z", GrantCadence::Weekly(1)),
            "2026-05-11T00:00:00Z"
        );
    }

    #[test]
    fn malformed_input_falls_back_to_epoch() {
        // Non-iso string — should not panic; we accept the epoch fallback
        // and just return *some* future date.
        let s = next_run_after("not-a-date", GrantCadence::Daily);
        assert_eq!(s, "1970-01-02T00:00:00Z");
    }
}
