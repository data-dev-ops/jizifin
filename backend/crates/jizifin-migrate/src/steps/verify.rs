//! Auth carry-over and pre-commit checksum verification.

use super::Steps;
use crate::error::MigrateError;
use jizifin_domain::entities::{EntryKind, EntryKind::*};
use jizifin_store::repo::{Auth, Entries, EntryFilter};
use sha2::{Digest, Sha256};

impl Steps<'_> {
    /// The legacy magic word is exactly the client's login proof; store only its hash.
    pub(super) fn auth(&mut self) -> Result<(), MigrateError> {
        match self.legacy.magic_word()? {
            Some(proof) => Auth::initialize(self.conn, &hex::encode(Sha256::digest(proof.as_bytes())))?,
            None => self.report.note("legacy instance was never initialized; first boot will ask for a passphrase"),
        }
        Ok(())
    }

    /// Fails (rolling everything back) unless per-kind cent totals match the legacy source.
    pub(super) fn verify(&mut self) -> Result<(), MigrateError> {
        let expected = [
            (Expense, self.legacy.expenses()?.iter().map(|e| e.cost).sum::<i64>()),
            (Income, self.legacy.income()?.iter().filter(|i| i.category != self.markers.salary).map(|i| i.amount).sum()),
            (Deposit, self.legacy.paid_deposits()?.iter().map(|d| d.amount).sum()),
        ];
        expected.into_iter().try_for_each(|(kind, total)| self.check_total(kind, total))
    }

    fn check_total(&mut self, kind: EntryKind, expected: i64) -> Result<(), MigrateError> {
        let actual = self.total(kind)?;
        self.report.note(format!("{} cents: legacy {expected}, migrated {actual}", kind.label()));
        match actual == expected {
            true => Ok(()),
            false => Err(MigrateError::Data(format!("{} total mismatch: legacy {expected}, migrated {actual}", kind.label()))),
        }
    }

    fn total(&self, kind: EntryKind) -> Result<i64, MigrateError> {
        let (mut sum, mut cursor) = (0, None);
        loop {
            let page = Entries::list(self.conn, &EntryFilter { kind: Some(kind), cursor, ..EntryFilter::default() })?;
            sum += page.entries.iter().map(|e| e.entry.amount.value()).sum::<i64>();
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => return Ok(sum),
            }
        }
    }
}
