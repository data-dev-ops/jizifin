//! Statement dates `DD/MM/YYYY` or `DD/MM/YY` to ISO `YYYY-MM-DD`.

/// Converts a European date; anything else is returned unchanged.
pub fn iso_date(raw: &str) -> String {
    let parts: Vec<&str> = raw.trim().split('/').collect();
    match parts.as_slice() {
        [day, month, year] => format!("{}-{month:0>2}-{day:0>2}", full_year(year)),
        _ => raw.to_owned(),
    }
}

fn full_year(year: &str) -> String {
    match year.len() {
        2 => format!("20{year}"),
        _ => year.to_owned(),
    }
}
