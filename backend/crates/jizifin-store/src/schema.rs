//! Table catalog shared by reset, backup and restore.

/// Ledger tables in dependency order (parents first) with their id sequence.
/// `sessions` and `schema_version` are deliberately absent: they are never backed up or restored.
pub(crate) const TABLES: &[(&str, Option<&str>)] = &[
    ("app_config", None),
    ("members", Some("seq_members")),
    ("categories", Some("seq_categories")),
    ("agreements", Some("seq_agreements")),
    ("agreement_shares", None),
    ("accounts", Some("seq_accounts")),
    ("account_members", None),
    ("account_categories", None),
    ("projects", Some("seq_projects")),
    ("project_members", None),
    ("tags", Some("seq_tags")),
    ("recurring_templates", Some("seq_recurring")),
    ("entries", Some("seq_entries")),
    ("entry_share_overrides", None),
    ("jobs", Some("seq_jobs")),
    ("income_overrides", None),
    ("budgets", Some("seq_budgets")),
    ("offset_rules", Some("seq_offset_rules")),
    ("month_seals", None),
];

/// Deletes every ledger row, children first.
pub(crate) fn clear(conn: &duckdb::Connection) -> duckdb::Result<()> {
    TABLES.iter().rev().try_for_each(|(table, _)| conn.execute(&format!("DELETE FROM {table}"), []).map(|_| ()))
}
