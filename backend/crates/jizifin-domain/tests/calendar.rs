use chrono::NaiveDate;
use jizifin_domain::{Month, Period};

#[test]
fn parse_valid_and_invalid_months() {
    assert!(Month::parse("2026-07").is_ok());
    assert!(Month::parse("2026-02").is_ok());
    assert!(Month::parse("2026-13").is_err());
    assert!(Month::parse("2026-7").is_err());
    assert!(Month::parse("not-a-month").is_err());
}

#[test]
fn month_navigation() {
    let jul = Month::parse("2026-07").unwrap();
    assert_eq!(jul.next(), Month::parse("2026-08").unwrap());
    assert_eq!(jul.previous(), Month::parse("2026-06").unwrap());
    let dec = Month::parse("2026-12").unwrap();
    assert_eq!(dec.next(), Month::parse("2027-01").unwrap());
    let jan = Month::parse("2026-01").unwrap();
    assert_eq!(jan.previous(), Month::parse("2025-12").unwrap());
}

#[test]
fn leap_year_february_bounds() {
    let feb_leap = Month::parse("2024-02").unwrap();
    assert_eq!(feb_leap.first_day(), NaiveDate::from_ymd_opt(2024, 2, 1).unwrap());
    assert_eq!(feb_leap.last_day(), NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());

    let feb_non_leap = Month::parse("2025-02").unwrap();
    assert_eq!(feb_non_leap.last_day(), NaiveDate::from_ymd_opt(2025, 2, 28).unwrap());
}

#[test]
fn clamp_day_limits() {
    let apr = Month::parse("2026-04").unwrap();
    assert_eq!(apr.clamp_day(31), 30);
    assert_eq!(apr.clamp_day(15), 15);
    assert_eq!(apr.clamp_day(0), 1);
}

#[test]
fn period_containment_and_inversion() {
    let d1 = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
    let d2 = NaiveDate::from_ymd_opt(2026, 6, 30).unwrap();
    let p = Period::new(d1, d2).unwrap();
    assert!(p.contains(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()));
    assert!(!p.contains(NaiveDate::from_ymd_opt(2026, 7, 1).unwrap()));
    assert!(Period::new(d2, d1).is_err());
}

#[test]
fn period_overlap_checks() {
    let p = Month::parse("2026-06").unwrap().period();
    let start_in = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
    let end_in = NaiveDate::from_ymd_opt(2026, 7, 15).unwrap();
    assert!(p.overlaps(start_in, Some(end_in)));
    let before = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
    let before_end = NaiveDate::from_ymd_opt(2026, 5, 31).unwrap();
    assert!(!p.overlaps(before, Some(before_end)));
    assert!(p.overlaps(start_in, None));
}
