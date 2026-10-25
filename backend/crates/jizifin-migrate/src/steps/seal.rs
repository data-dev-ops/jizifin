//! Re-seals every month the legacy app had settled, using the new Auditor.

use crate::error::MigrateError;
use crate::report::Report;
use chrono::Utc;
use jizifin_auditor::{AuditScope, Auditor};
use jizifin_domain::Month;
use jizifin_store::repo::Seals;
use jizifin_store::{Database, DuckSnapshotLoader};
use std::sync::Arc;

/// Computes and stores a seal for each legacy settled month.
pub(super) fn reseal(db: &Database, months: &[String], report: &mut Report) -> Result<(), MigrateError> {
    let auditor = Auditor::new(Arc::new(DuckSnapshotLoader::new(db.clone())), db.version());
    for raw in months {
        seal_month(db, &auditor, Month::parse(raw)?)?;
    }
    report.count("months resealed", months.len());
    Ok(())
}

fn seal_month(db: &Database, auditor: &Auditor, month: Month) -> Result<(), MigrateError> {
    let audited = auditor.settlement(AuditScope::month(month, Utc::now().date_naive()))?;
    let json = serde_json::to_string(&audited.value).map_err(|e| MigrateError::Data(e.to_string()))?;
    db.write(|c| Seals::insert(c, month, &json, &audited.inputs_hash))?;
    Ok(())
}
