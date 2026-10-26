//! Counterparty extraction from ING Belgium transaction descriptions.

use regex_lite::Regex;
use std::sync::LazyLock;

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("static pattern")
}

static DIRECT_DEBIT: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)Domicili.*?ring in euro \(SEPA\)\s+(.*?)(?:\s+Bericht als bijlage|$|\s+Identificatienr:)"));
static TO: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)Naar:\s*(.*?)(?:\s*-\s*[A-Z0-9]{10,34}|$|\s*Mededeling:)"));
static FROM: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)Van:\s*(.*?)(?:\s*-\s*[A-Z0-9]{10,34}|$|\s*Mededeling:)"));
static CARD: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^(?:Betaling|Geldopneming|Terugbetaling)"));
static CARD_PREFIX: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^(?:Betaling|Geldopneming|Terugbetaling)[^-]*-?\s*"));
static TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\d{1,2}[.:]\d{2}\s*uur\s*-\s*(.*)"));
static FEES: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^Kostenafrekening"));
/// Card, reference, fuel volume, location and currency-conversion suffixes, stripped in order.
static SUFFIXES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?i)\s+Kaartnummer.*",
        r"(?i)\s+Referentie.*",
        r"(?i)\s+\d+[,.]\d+\s+liter.*",
        r"(?i)\s+[0-9]{2,5}(?:\s+[0-9]{2,5})?\s*-\s*[^-]+-[A-Z\s]+$",
        r"(?i)\s+[A-Z0-9]{2,8}(?:\s+[A-Z0-9]{2,5})?\s*-\s*[^-]+-[A-Z\s]+$",
        r"(?i)\s+[0-9]{4}\s+[A-Z]{2}\s*-\s*[^-]+-[A-Z\s]+.*$",
        r"(?i)\s+[0-9]{4}\s*-\s*[^-]+-[A-Z\s]+$",
        r"(?i)\s+\d+[,.]\d+\s+[A-Z]{3}\s+aan\s+.*$",
    ]
    .into_iter()
    .map(re)
    .collect()
});

/// Clean merchant or counterparty name for `description`.
pub fn ing_beneficiary(description: &str) -> String {
    let desc = description.trim();
    if desc.is_empty() {
        return "Unknown".to_owned();
    }
    transfer_party(desc)
        .or_else(|| card_merchant(desc))
        .or_else(|| FEES.is_match(desc).then(|| "ING Bankkosten".to_owned()))
        .unwrap_or_else(|| desc.to_owned())
}

/// Direct debit creditor, or transfer beneficiary/sender.
fn transfer_party(desc: &str) -> Option<String> {
    let capture = |pattern: &Regex| pattern.captures(desc).map(|c| c[1].trim().to_owned()).filter(|s| !s.is_empty());
    capture(&DIRECT_DEBIT)
        .or_else(|| desc.contains("Naar:").then(|| capture(&TO)).flatten())
        .or_else(|| desc.contains("Van:").then(|| capture(&FROM)).flatten())
}

fn card_merchant(desc: &str) -> Option<String> {
    if !CARD.is_match(desc) {
        return None;
    }
    let named = [("ING : VISA", "ING : VISA"), ("Financiering meubilering", "ING Financiering")];
    match named.iter().find(|(needle, _)| desc.contains(needle)) {
        Some((_, label)) => Some((*label).to_owned()),
        None => Some(strip_card_noise(desc)),
    }
}

fn strip_card_noise(desc: &str) -> String {
    let start = match TIMESTAMP.captures(desc) {
        Some(c) => c[1].to_owned(),
        None => CARD_PREFIX.replace(desc, "").into_owned(),
    };
    let merchant = SUFFIXES.iter().fold(start, |m, suffix| suffix.replace(&m, "").into_owned());
    match merchant.trim() {
        "" => desc.to_owned(),
        clean => clean.to_owned(),
    }
}
