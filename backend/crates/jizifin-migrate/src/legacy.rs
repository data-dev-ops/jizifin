//! Read-only access to the legacy FastAPI SQLite database.

use crate::error::MigrateError;
use rusqlite::{Connection, OpenFlags, Row};
use std::path::Path;

/// `users`
pub struct LegacyUser { pub name: String, pub color: String, pub is_active: bool }
/// `split_agreements` (active only)
pub struct LegacyAgreement { pub id: i64, pub category: String, pub start: String, pub end: Option<String>, pub note: Option<String> }
/// `split_allocations`
pub struct LegacyAllocation { pub agreement_id: Option<i64>, pub category: String, pub user: String, pub pct: f64 }
/// `joint_accounts`
pub struct LegacyAccount { pub id: i64, pub name: String, pub balance: i64, pub margin: f64, pub mode: String, pub expected_total: Option<i64> }
/// `joint_account_deposits`
pub struct LegacyDepositPlan { pub account_id: i64, pub user: String, pub amount: i64, pub day: u32 }
/// `joint_account_expected_costs` ∪ `joint_account_categories`
pub struct LegacyAccountCategory { pub account_id: i64, pub category: String, pub expected: i64 }
/// `projects`
pub struct LegacyProject { pub id: i64, pub name: String, pub target: i64, pub target_date: Option<String> }
/// `tags`
pub struct LegacyTag { pub id: i64, pub name: String, pub description: Option<String>, pub color: String, pub start: Option<String>, pub end: Option<String> }
/// `expenses`
pub struct LegacyExpense {
    pub id: i64, pub name: String, pub cost: i64, pub date: String, pub payer: String, pub category: String,
    pub project_id: Option<i64>, pub tag_id: Option<i64>, pub is_joint: bool, pub account_id: Option<i64>,
}
/// `income`
pub struct LegacyIncome { pub name: String, pub amount: i64, pub who: String, pub category: String, pub date: String, pub is_joint: bool }
/// `joint_account_monthly_deposits` (paid only)
pub struct LegacyDeposit { pub account_id: i64, pub user: String, pub month: String, pub amount: i64, pub paid_date: Option<String> }
/// `joint_account_corrections`
pub struct LegacyCorrection { pub account_id: i64, pub amount: i64, pub date: String, pub note: Option<String> }
/// `jobs`
pub struct LegacyJob { pub name: String, pub who: String, pub amount: i64, pub frequency: String, pub start: String, pub end: Option<String>, pub notes: Option<String>, pub is_active: bool }
/// `salary_overrides`
pub struct LegacySalaryOverride { pub user: String, pub month: String, pub amount: i64, pub note: Option<String> }
/// `recurring_expenses`
pub struct LegacyRecurring {
    pub name: String, pub cost: i64, pub payer: String, pub category: String, pub frequency: String, pub day: Option<u32>,
    pub start: String, pub end: Option<String>, pub is_active: bool, pub is_joint: bool, pub account_id: Option<i64>,
}
/// `budgets`
pub struct LegacyBudget { pub category: String, pub month: String, pub limit: i64 }

/// Read-only legacy connection.
pub struct Legacy {
    conn: Connection,
}

impl Legacy {
    /// Opens `path` read-only; the file is never modified.
    pub fn open(path: &Path) -> Result<Self, MigrateError> {
        Ok(Self { conn: Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)? })
    }

    fn rows<T>(&self, sql: &str, map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>) -> Result<Vec<T>, MigrateError> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], map)?;
        Ok(rows.collect::<rusqlite::Result<Vec<T>>>()?)
    }

    /// Encrypted magic word (the login proof), if initialized.
    pub fn magic_word(&self) -> Result<Option<String>, MigrateError> {
        Ok(self.rows("SELECT value FROM app_config WHERE key = 'magic_word'", |r| r.get(0))?.into_iter().next())
    }

    pub fn users(&self) -> Result<Vec<LegacyUser>, MigrateError> {
        self.rows("SELECT name, color, is_active FROM users ORDER BY rowid", |r| {
            Ok(LegacyUser { name: r.get(0)?, color: r.get(1)?, is_active: r.get::<_, i64>(2)? == 1 })
        })
    }

    pub fn expense_categories(&self) -> Result<Vec<String>, MigrateError> {
        self.rows("SELECT category FROM splits ORDER BY rowid", |r| r.get(0))
    }

    pub fn income_categories(&self) -> Result<Vec<String>, MigrateError> {
        self.rows("SELECT category FROM income_categories UNION SELECT DISTINCT category FROM income ORDER BY 1", |r| r.get(0))
    }

    pub fn agreements(&self) -> Result<Vec<LegacyAgreement>, MigrateError> {
        self.rows("SELECT id, category, start_date, end_date, note FROM split_agreements WHERE is_active = 1 ORDER BY id", |r| {
            Ok(LegacyAgreement { id: r.get(0)?, category: r.get(1)?, start: r.get(2)?, end: r.get(3)?, note: r.get(4)? })
        })
    }

    pub fn allocations(&self) -> Result<Vec<LegacyAllocation>, MigrateError> {
        self.rows("SELECT agreement_id, category, user_name, pct FROM split_allocations ORDER BY id", |r| {
            Ok(LegacyAllocation { agreement_id: r.get(0)?, category: r.get(1)?, user: r.get(2)?, pct: r.get(3)? })
        })
    }

    pub fn accounts(&self) -> Result<Vec<LegacyAccount>, MigrateError> {
        let sql = "SELECT id, name, balance_cents, safety_margin_pct, deposit_split_mode, expected_total_cents FROM joint_accounts ORDER BY id";
        self.rows(sql, |r| {
            Ok(LegacyAccount { id: r.get(0)?, name: r.get(1)?, balance: r.get(2)?, margin: r.get(3)?, mode: r.get(4)?, expected_total: r.get(5)? })
        })
    }

    pub fn account_members(&self) -> Result<Vec<(i64, String)>, MigrateError> {
        self.rows("SELECT account_id, user_name FROM joint_account_members ORDER BY 1, 2", |r| Ok((r.get(0)?, r.get(1)?)))
    }

    pub fn deposit_plans(&self) -> Result<Vec<LegacyDepositPlan>, MigrateError> {
        self.rows("SELECT account_id, user_name, amount_cents, day_of_month FROM joint_account_deposits", |r| {
            Ok(LegacyDepositPlan { account_id: r.get(0)?, user: r.get(1)?, amount: r.get(2)?, day: r.get(3)? })
        })
    }

    pub fn account_categories(&self) -> Result<Vec<LegacyAccountCategory>, MigrateError> {
        let sql = "SELECT c.account_id, c.category, COALESCE(e.expected_cents, 0) FROM joint_account_categories c \
                   LEFT JOIN joint_account_expected_costs e ON e.category = c.category AND e.account_id = c.account_id \
                   UNION SELECT account_id, category, expected_cents FROM joint_account_expected_costs";
        self.rows(sql, |r| Ok(LegacyAccountCategory { account_id: r.get(0)?, category: r.get(1)?, expected: r.get(2)? }))
    }

    pub fn projects(&self) -> Result<Vec<LegacyProject>, MigrateError> {
        self.rows("SELECT id, name, target_cents, target_date FROM projects ORDER BY id", |r| {
            Ok(LegacyProject { id: r.get(0)?, name: r.get(1)?, target: r.get(2)?, target_date: r.get(3)? })
        })
    }

    pub fn project_users(&self) -> Result<Vec<(i64, String)>, MigrateError> {
        self.rows("SELECT project_id, user_name FROM project_users ORDER BY 1, 2", |r| Ok((r.get(0)?, r.get(1)?)))
    }

    pub fn tags(&self) -> Result<Vec<LegacyTag>, MigrateError> {
        self.rows("SELECT id, name, description, color, start_date, end_date FROM tags ORDER BY id", |r| {
            Ok(LegacyTag { id: r.get(0)?, name: r.get(1)?, description: r.get(2)?, color: r.get(3)?, start: r.get(4)?, end: r.get(5)? })
        })
    }

    pub fn expenses(&self) -> Result<Vec<LegacyExpense>, MigrateError> {
        let sql = "SELECT id, name, cost_cents, expense_date, who_paid, category, project_id, tag_id, is_joint, joint_account_id \
                   FROM expenses ORDER BY expense_date, id";
        self.rows(sql, |r| {
            Ok(LegacyExpense {
                id: r.get(0)?, name: r.get(1)?, cost: r.get(2)?, date: r.get(3)?, payer: r.get(4)?, category: r.get(5)?,
                project_id: r.get(6)?, tag_id: r.get(7)?, is_joint: r.get::<_, i64>(8)? == 1, account_id: r.get(9)?,
            })
        })
    }

    pub fn expense_overrides(&self) -> Result<Vec<(i64, String, f64)>, MigrateError> {
        self.rows("SELECT expense_id, user_name, pct FROM expense_overrides ORDER BY 1, 2", |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
    }

    pub fn income(&self) -> Result<Vec<LegacyIncome>, MigrateError> {
        self.rows("SELECT name, amount_cents, who, category, income_date, is_joint FROM income ORDER BY income_date, id", |r| {
            Ok(LegacyIncome { name: r.get(0)?, amount: r.get(1)?, who: r.get(2)?, category: r.get(3)?, date: r.get(4)?, is_joint: r.get::<_, i64>(5)? == 1 })
        })
    }

    pub fn paid_deposits(&self) -> Result<Vec<LegacyDeposit>, MigrateError> {
        let sql = "SELECT account_id, user_name, month, CASE WHEN actual_cents > 0 THEN actual_cents ELSE scheduled_cents END, paid_date \
                   FROM joint_account_monthly_deposits WHERE is_paid = 1 ORDER BY month, account_id, user_name";
        self.rows(sql, |r| Ok(LegacyDeposit { account_id: r.get(0)?, user: r.get(1)?, month: r.get(2)?, amount: r.get(3)?, paid_date: r.get(4)? }))
    }

    pub fn corrections(&self) -> Result<Vec<LegacyCorrection>, MigrateError> {
        self.rows("SELECT account_id, amount_cents, correction_date, note FROM joint_account_corrections ORDER BY correction_date, id", |r| {
            Ok(LegacyCorrection { account_id: r.get(0)?, amount: r.get(1)?, date: r.get(2)?, note: r.get(3)? })
        })
    }

    pub fn jobs(&self) -> Result<Vec<LegacyJob>, MigrateError> {
        self.rows("SELECT name, who, amount_cents, frequency, start_date, end_date, notes, is_active FROM jobs ORDER BY id", |r| {
            Ok(LegacyJob {
                name: r.get(0)?, who: r.get(1)?, amount: r.get(2)?, frequency: r.get(3)?, start: r.get(4)?, end: r.get(5)?,
                notes: r.get(6)?, is_active: r.get::<_, i64>(7)? == 1,
            })
        })
    }

    pub fn salary_overrides(&self) -> Result<Vec<LegacySalaryOverride>, MigrateError> {
        self.rows("SELECT user_name, month, amount_cents, note FROM salary_overrides ORDER BY month", |r| {
            Ok(LegacySalaryOverride { user: r.get(0)?, month: r.get(1)?, amount: r.get(2)?, note: r.get(3)? })
        })
    }

    pub fn recurring(&self) -> Result<Vec<LegacyRecurring>, MigrateError> {
        let sql = "SELECT name, cost_cents, who_paid, category, frequency, day_of_month, start_date, end_date, is_active, is_joint, joint_account_id \
                   FROM recurring_expenses ORDER BY id";
        self.rows(sql, |r| {
            Ok(LegacyRecurring {
                name: r.get(0)?, cost: r.get(1)?, payer: r.get(2)?, category: r.get(3)?, frequency: r.get(4)?, day: r.get(5)?,
                start: r.get(6)?, end: r.get(7)?, is_active: r.get::<_, i64>(8)? == 1, is_joint: r.get::<_, i64>(9)? == 1,
                account_id: r.get(10)?,
            })
        })
    }

    pub fn budgets(&self) -> Result<Vec<LegacyBudget>, MigrateError> {
        self.rows("SELECT category, month, limit_cents FROM budgets ORDER BY category, month", |r| {
            Ok(LegacyBudget { category: r.get(0)?, month: r.get(1)?, limit: r.get(2)? })
        })
    }

    pub fn settled_months(&self) -> Result<Vec<String>, MigrateError> {
        self.rows("SELECT month FROM settlements ORDER BY month", |r| r.get(0))
    }
}
