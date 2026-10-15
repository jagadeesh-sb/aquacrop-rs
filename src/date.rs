//! Civil dates as day numbers, so the model needs no date library.
//!
//! A [`Date`] is the number of days since 1970-01-01 in the proleptic Gregorian
//! calendar (Howard Hinnant's `days_from_civil`).

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(pub i64);

impl Date {
    pub fn from_ymd(y: i64, m: u32, d: u32) -> Option<Date> {
        if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
            return None;
        }
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = m as i64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Some(Date(era * 146_097 + doe - 719_468))
    }

    pub fn ymd(self) -> (i64, u32, u32) {
        let z = self.0 + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        (if m <= 2 { y + 1 } else { y }, m, d)
    }

    pub fn year(self) -> i64 {
        self.ymd().0
    }

    pub fn month(self) -> u32 {
        self.ymd().1
    }

    pub fn day(self) -> u32 {
        self.ymd().2
    }

    pub fn add_days(self, n: i64) -> Date {
        Date(self.0 + n)
    }

    /// Parse `YYYY/MM/DD` or `YYYY-MM-DD`.
    pub fn parse(s: &str) -> Option<Date> {
        let parts: Vec<&str> = s.trim().split(['/', '-']).collect();
        if parts.len() != 3 {
            return None;
        }
        let y = parts[0].parse().ok()?;
        let m = parts[1].parse().ok()?;
        let d = parts[2].parse().ok()?;
        Date::from_ymd(y, m, d)
    }

    /// A month and day (`MM/DD`) in a given year, as pandas reads `"YYYY/MM/DD"`.
    pub fn from_month_day(year: i64, md: MonthDay) -> Option<Date> {
        Date::from_ymd(year, md.month, md.day)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (y, m, d) = self.ymd();
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

impl Serialize for Date {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Date::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid date {s:?}, expected YYYY-MM-DD")))
    }
}

pub fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        2 => 28,
        _ => 0,
    }
}

/// A recurring month and day, written `MM/DD` as AquaCrop-OSPy writes planting dates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonthDay {
    pub month: u32,
    pub day: u32,
}

impl MonthDay {
    pub fn parse(s: &str) -> Option<MonthDay> {
        let (m, d) = s.trim().split_once('/')?;
        let month: u32 = m.parse().ok()?;
        let day: u32 = d.parse().ok()?;
        // 29 February is valid in a leap year only; accept it here and fail where a year is fixed.
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(2000, month) {
            return None;
        }
        Some(MonthDay { month, day })
    }
}

impl fmt::Display for MonthDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}/{:02}", self.month, self.day)
    }
}

impl Serialize for MonthDay {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for MonthDay {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        MonthDay::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid month/day {s:?}, expected MM/DD")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for (y, m, d) in [(1970, 1, 1), (1979, 10, 1), (2000, 2, 29), (2026, 10, 15), (1900, 3, 1)] {
            let date = Date::from_ymd(y, m, d).unwrap();
            assert_eq!(date.ymd(), (y, m, d));
        }
        assert_eq!(Date::from_ymd(1970, 1, 1).unwrap().0, 0);
        assert_eq!(Date::parse("1979/10/01").unwrap().add_days(1).to_string(), "1979-10-02");
        assert!(Date::from_ymd(2001, 2, 29).is_none());
    }
}
