//! The default display formatters for the size and modified columns.

/// `372.82 KB`-style size with 1000-based units up to GB (the Material Files convention).
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let value = bytes as f64;
    let mut unit = 0;
    let mut scaled = value;
    while scaled >= 1000.0 && unit < UNITS.len() - 1 {
        scaled /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.2} {}", scaled, UNITS[unit])
    }
}

/// Epoch seconds to `Apr 7, 2021, 17:45` (UTC; a localised browser should pass its own formatter).
pub fn format_epoch(epoch: i64) -> String {
    let (days, seconds_of_day) = (epoch.div_euclid(86_400), epoch.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    let (hour, minute) = (seconds_of_day / 3600 % 24, seconds_of_day % 3600 / 60);
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{month} {day}, {year}, {hour:0>2}:{minute:0>2}",
        month = MONTHS[(month - 1) as usize]
    )
}

/// Howard Hinnant's `civil_from_days` (public domain): days since 1970-01-01 to proleptic Gregorian (y, m, d).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::{format_bytes, format_epoch};

    #[test]
    fn epoch_formatting() {
        // 2021-04-07 17:45 UTC.
        assert_eq!(format_epoch(1_617_817_500), "Apr 7, 2021, 17:45");
        assert_eq!(format_bytes(372_820), "372.82 KB");
        assert_eq!(format_bytes(900), "900 B");
    }
}
