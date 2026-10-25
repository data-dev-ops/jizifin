//! Human-readable migration summary.

use std::fmt;

/// Row counts and notes collected per step.
#[derive(Default)]
pub struct Report {
    lines: Vec<(String, i64)>,
    notes: Vec<String>,
}

impl Report {
    /// Records that `count` rows were written for `what`.
    pub fn count(&mut self, what: &str, count: usize) {
        self.lines.push((what.to_owned(), i64::try_from(count).unwrap_or(i64::MAX)));
    }

    /// Records a decision worth reviewing.
    pub fn note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "migration complete")?;
        for (what, count) in &self.lines {
            writeln!(f, "  {what:<32} {count:>8}")?;
        }
        self.notes.iter().try_for_each(|n| writeln!(f, "  note: {n}"))
    }
}
