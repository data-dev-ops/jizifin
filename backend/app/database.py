"""
database.py — aiosqlite connection management, schema definition, and migration.

Creates all tables, indexes, and views on first boot and applies backwards-compatible
migrations and default value backfills on startup and database import.
"""

from __future__ import annotations

import aiosqlite
from pathlib import Path
from typing import AsyncGenerator

DB_PATH: Path = Path(__file__).resolve().parent.parent / "finance.db"


# ---------------------------------------------------------------------------
# Connection dependency
# ---------------------------------------------------------------------------

async def get_db() -> AsyncGenerator[aiosqlite.Connection, None]:
    """
    Yield an open, WAL-enabled aiosqlite connection for a single request.
    Rolls back automatically on unhandled exceptions.
    """
    async with aiosqlite.connect(DB_PATH) as conn:
        await conn.execute("PRAGMA journal_mode=WAL;")
        await conn.execute("PRAGMA foreign_keys=ON;")
        conn.row_factory = aiosqlite.Row
        try:
            yield conn
        except Exception:
            await conn.rollback()
            raise


async def ensure_column(
    conn: aiosqlite.Connection,
    table: str,
    column: str,
    col_definition: str,
    default_value_sql: str | None = None,
) -> None:
    """
    Ensure a column exists on a table, adding it via ALTER TABLE if missing.
    Only attempts ALTER TABLE if the table already exists.
    If default_value_sql is provided, updates any existing NULL entries with the default.
    """
    async with conn.execute(f"PRAGMA table_info({table})") as cur:
        cols = {row[1] async for row in cur}
    if cols and column not in cols:
        await conn.execute(f"ALTER TABLE {table} ADD COLUMN {column} {col_definition}")
    if cols and default_value_sql is not None:
        await conn.execute(f"UPDATE {table} SET {column} = {default_value_sql} WHERE {column} IS NULL")


# ---------------------------------------------------------------------------
# Schema initialisation & Backwards-Compatible Migration
# ---------------------------------------------------------------------------

async def _init_db_schema(conn: aiosqlite.Connection) -> None:
    """Execute all table creations, column migrations, index creations, and view drops/recreates."""
    await conn.execute("PRAGMA journal_mode=WAL;")
    await conn.execute("PRAGMA foreign_keys=ON;")
    conn.row_factory = aiosqlite.Row

    # ── app_config ─────────────────────────────────────────────────────────
    # Key-value store for app-wide settings.
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS app_config (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        """
    )

    # ── users ──────────────────────────────────────────────────────────
    # Household members. Manage via the Settings tab in the UI.
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS users (
            name       TEXT    PRIMARY KEY CHECK(length(name) <= 256),
            color      TEXT    NOT NULL DEFAULT '#6366f1',
            is_active  INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1)),
            created_at TEXT    NOT NULL DEFAULT (datetime('now'))
        )
        """
    )

    # ── splits ─────────────────────────────────────────────────────────
    # Category registry. Each category's per-user percentages live in
    # split_allocations, not here.
    await conn.execute(
        "CREATE TABLE IF NOT EXISTS splits (category TEXT PRIMARY KEY CHECK(length(category) <= 256))"
    )

    # ── income_categories ──────────────────────────────────────────────
    # Registry of user-defined income category labels (encrypted).
    # No FK from income.category — historical entries survive category
    # deletion intentionally.
    await conn.execute(
        "CREATE TABLE IF NOT EXISTS income_categories (category TEXT PRIMARY KEY CHECK(length(category) <= 256))"
    )

    # ── projects ───────────────────────────────────────────────────────
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS projects (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            name                TEXT    NOT NULL UNIQUE CHECK(length(name) <= 256),
            target_cents        INTEGER NOT NULL CHECK(target_cents > 0),
            target_date         TEXT    NOT NULL
                                        CHECK(target_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            is_joint            INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1)),
            allow_subcategories INTEGER NOT NULL DEFAULT 1 CHECK(allow_subcategories IN (0, 1))
        )
        """
    )

    # ── tags ────────────────────────────────────────────────────────────
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS tags (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT    NOT NULL UNIQUE CHECK(length(name) <= 256),
            color       TEXT    NOT NULL DEFAULT '#f59e0b',
            description TEXT             CHECK(length(description) <= 512),
            created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
            is_joint    INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1)),
            is_active   INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1))
        )
        """  
    )

    # ── expenses ────────────────────────────────────────────────────────
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS expenses (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            name         TEXT    NOT NULL CHECK(length(name) <= 256),
            cost_cents   INTEGER NOT NULL CHECK(cost_cents > 0),
            expense_date TEXT    NOT NULL
                                 CHECK(expense_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            who_paid     TEXT    NOT NULL REFERENCES users(name)           ON UPDATE CASCADE,
            category     TEXT    NOT NULL REFERENCES splits(category)      ON UPDATE CASCADE,
            project_id   INTEGER          REFERENCES projects(id)          ON DELETE SET NULL,
            tag_id       INTEGER          REFERENCES tags(id)              ON DELETE SET NULL,
            is_joint     INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1))
        )
        """
    )

    # ── expense_overrides ───────────────────────────────────────────────
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS expense_overrides (
            expense_id INTEGER NOT NULL REFERENCES expenses(id) ON DELETE CASCADE,
            user_name  TEXT    NOT NULL REFERENCES users(name)    ON UPDATE CASCADE ON DELETE CASCADE,
            pct        REAL    NOT NULL CHECK(pct >= 0.0 AND pct <= 100.0),
            PRIMARY KEY (expense_id, user_name)
        )
        """
    )

    # ── income ──────────────────────────────────────────────────────────
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS income (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            name         TEXT    NOT NULL CHECK(length(name) <= 256),
            amount_cents INTEGER NOT NULL CHECK(amount_cents > 0),
            who          TEXT    NOT NULL REFERENCES users(name) ON UPDATE CASCADE,
            category     TEXT    NOT NULL CHECK(length(category) <= 256),
            income_date  TEXT    NOT NULL
                                 CHECK(income_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            is_joint     INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1))
        )
        """
    )

    # ── jobs ────────────────────────────────────────────────────────────
    # Employment & recurring income streams with timeline support.
    # name, who, notes are encrypted.
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS jobs (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            name         TEXT    NOT NULL CHECK(length(name) <= 256),
            who          TEXT    NOT NULL REFERENCES users(name) ON UPDATE CASCADE,
            amount_cents INTEGER NOT NULL CHECK(amount_cents > 0),
            frequency    TEXT    NOT NULL DEFAULT 'monthly'
                                 CHECK(frequency IN ('monthly', 'weekly', 'biweekly', 'annual')),
            start_date   TEXT    NOT NULL
                                 CHECK(start_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            end_date     TEXT    CHECK(end_date IS NULL OR end_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            notes        TEXT    CHECK(notes IS NULL OR length(notes) <= 512),
            is_active    INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1))
        )
        """
    )

    # ── recurring_expenses ──────────────────────────────────────────────
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS recurring_expenses (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            name         TEXT    NOT NULL CHECK(length(name) <= 256),
            cost_cents   INTEGER NOT NULL CHECK(cost_cents > 0),
            who_paid     TEXT    NOT NULL REFERENCES users(name)      ON UPDATE CASCADE,
            category     TEXT    NOT NULL REFERENCES splits(category) ON UPDATE CASCADE,
            frequency    TEXT    NOT NULL DEFAULT 'monthly'
                                 CHECK(frequency IN ('monthly', 'weekly', 'biweekly', '4-weekly', 'quarterly', 'annual')),
            day_of_month INTEGER CHECK(day_of_month IS NULL OR (day_of_month >= 1 AND day_of_month <= 31)),
            start_date   TEXT    NOT NULL DEFAULT '2026-01-01'
                                 CHECK(start_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            end_date     TEXT    CHECK(end_date IS NULL OR end_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            is_active    INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1)),
            is_joint     INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1))
        )
        """
    )

    # ── budgets ─────────────────────────────────────────────────────────
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS budgets (
            category    TEXT    NOT NULL
                                REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE,
            month       TEXT    NOT NULL
                                CHECK(month = 'ALL' OR month GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]'),
            limit_cents INTEGER NOT NULL CHECK(limit_cents >= 0),
            PRIMARY KEY (category, month)
        )
        """
    )

    # ── settlements ─────────────────────────────────────────────────────
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS settlements (
            month                         TEXT PRIMARY KEY
                                          CHECK(month GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]'),
            settled_at                    TEXT NOT NULL,
            net_balance_transferred_cents INTEGER NOT NULL
        )
        """
    )

    # ── split_allocations ───────────────────────────────────────────────
    # Per-user percentage share per category. Allocations for a category
    # must sum to 100.0 — enforced at the API layer.
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS split_allocations (
            category  TEXT NOT NULL REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE,
            user_name TEXT NOT NULL REFERENCES users(name)       ON UPDATE CASCADE ON DELETE CASCADE,
            pct       REAL NOT NULL CHECK(pct >= 0.0 AND pct <= 100.0),
            PRIMARY KEY (category, user_name)
        )
        """
    )

    # ── joint_account ───────────────────────────────────────────────────
    # Singleton config row (id always 1). name is encrypted.
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS joint_account (
            id                   INTEGER PRIMARY KEY CHECK(id = 1),
            name                 TEXT    NOT NULL CHECK(length(name) <= 256),
            balance_cents        INTEGER NOT NULL DEFAULT 0,
            safety_margin_pct    INTEGER NOT NULL DEFAULT 10
                                 CHECK(safety_margin_pct >= 0 AND safety_margin_pct <= 100),
            deposit_split_mode   TEXT    NOT NULL DEFAULT 'even'
                                 CHECK(deposit_split_mode IN ('salary', 'even', 'manual')),
            expected_total_cents INTEGER
        )
        """
    )

    # ── joint_account_categories ────────────────────────────────────────
    # Which expense categories are paid from the joint account.
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS joint_account_categories (
            category TEXT PRIMARY KEY
                     REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE
        )
        """
    )

    # ── joint_account_deposits ──────────────────────────────────────────
    # Per-user monthly deposit configuration. user_name is encrypted.
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS joint_account_deposits (
            user_name    TEXT    PRIMARY KEY
                         REFERENCES users(name) ON UPDATE CASCADE ON DELETE CASCADE,
            amount_cents INTEGER NOT NULL DEFAULT 0 CHECK(amount_cents >= 0),
            day_of_month INTEGER NOT NULL DEFAULT 1 CHECK(day_of_month >= 1 AND day_of_month <= 31)
        )
        """
    )

    # ── joint_account_monthly_deposits ──────────────────────────────────
    # Per-user monthly deposit execution log (is_paid, actual_cents, paid_date).
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS joint_account_monthly_deposits (
            month           TEXT    NOT NULL CHECK(month GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]'),
            user_name       TEXT    NOT NULL REFERENCES users(name) ON UPDATE CASCADE ON DELETE CASCADE,
            scheduled_cents INTEGER NOT NULL DEFAULT 0 CHECK(scheduled_cents >= 0),
            actual_cents    INTEGER NOT NULL DEFAULT 0 CHECK(actual_cents >= 0),
            is_paid         INTEGER NOT NULL DEFAULT 0 CHECK(is_paid IN (0, 1)),
            paid_date       TEXT,
            PRIMARY KEY (month, user_name)
        )
        """
    )

    # ── joint_account_expected_costs ────────────────────────────────────
    # Per-category expected monthly costs for joint-account categories.
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS joint_account_expected_costs (
            category       TEXT    PRIMARY KEY
                           REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE,
            expected_cents INTEGER NOT NULL CHECK(expected_cents >= 0)
        )
        """
    )

    # ── joint_account_corrections ───────────────────────────────────────
    # Manual balance corrections (deposits, withdrawals, corrections).
    # note is encrypted. amount_cents is signed.
    await conn.execute(
        """
        CREATE TABLE IF NOT EXISTS joint_account_corrections (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            amount_cents    INTEGER NOT NULL,
            correction_date TEXT    NOT NULL
                            CHECK(correction_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            note            TEXT    CHECK(length(note) <= 512)
        )
        """
    )

    # ── Migrate existing tables: add columns if missing and backfill defaults ───
    is_joint_def = "INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1))"

    # users
    await ensure_column(conn, "users", "color", "TEXT NOT NULL DEFAULT '#6366f1'", "'#6366f1'")
    await ensure_column(conn, "users", "is_active", "INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1))", "1")
    await ensure_column(conn, "users", "created_at", "TEXT", "datetime('now')")

    # projects
    await ensure_column(conn, "projects", "is_joint", is_joint_def, "0")
    await ensure_column(conn, "projects", "allow_subcategories", "INTEGER NOT NULL DEFAULT 1 CHECK(allow_subcategories IN (0, 1))", "1")

    # tags
    await ensure_column(conn, "tags", "color", "TEXT NOT NULL DEFAULT '#f59e0b'", "'#f59e0b'")
    await ensure_column(conn, "tags", "description", "TEXT")
    await ensure_column(conn, "tags", "created_at", "TEXT", "datetime('now')")
    await ensure_column(conn, "tags", "is_joint", is_joint_def, "0")
    await ensure_column(conn, "tags", "is_active", "INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1))", "1")

    # expenses
    await ensure_column(conn, "expenses", "project_id", "INTEGER REFERENCES projects(id) ON DELETE SET NULL")
    await ensure_column(conn, "expenses", "tag_id", "INTEGER REFERENCES tags(id) ON DELETE SET NULL")
    await ensure_column(conn, "expenses", "is_joint", is_joint_def, "0")

    # income
    await ensure_column(conn, "income", "is_joint", is_joint_def, "0")

    # jobs
    await ensure_column(conn, "jobs", "frequency", "TEXT NOT NULL DEFAULT 'monthly'", "'monthly'")
    await ensure_column(conn, "jobs", "start_date", "TEXT NOT NULL DEFAULT '2026-01-01'", "'2026-01-01'")
    await ensure_column(conn, "jobs", "end_date", "TEXT")
    await ensure_column(conn, "jobs", "notes", "TEXT")
    await ensure_column(conn, "jobs", "is_active", "INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1))", "1")

    # recurring_expenses
    await ensure_column(conn, "recurring_expenses", "frequency", "TEXT NOT NULL DEFAULT 'monthly'", "'monthly'")
    await ensure_column(conn, "recurring_expenses", "day_of_month", "INTEGER")
    await ensure_column(conn, "recurring_expenses", "start_date", "TEXT NOT NULL DEFAULT '2026-01-01'", "'2026-01-01'")
    await ensure_column(conn, "recurring_expenses", "end_date", "TEXT")
    await ensure_column(conn, "recurring_expenses", "is_active", "INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1))", "1")
    await ensure_column(conn, "recurring_expenses", "is_joint", is_joint_def, "0")

    # joint_account
    await ensure_column(conn, "joint_account", "balance_cents", "INTEGER NOT NULL DEFAULT 0", "0")
    await ensure_column(conn, "joint_account", "safety_margin_pct", "INTEGER NOT NULL DEFAULT 10", "10")
    await ensure_column(conn, "joint_account", "deposit_split_mode", "TEXT NOT NULL DEFAULT 'even'", "'even'")
    await ensure_column(conn, "joint_account", "expected_total_cents", "INTEGER")

    # joint_account_deposits
    await ensure_column(conn, "joint_account_deposits", "amount_cents", "INTEGER NOT NULL DEFAULT 0", "0")
    await ensure_column(conn, "joint_account_deposits", "day_of_month", "INTEGER NOT NULL DEFAULT 1", "1")

    # joint_account_monthly_deposits
    await ensure_column(conn, "joint_account_monthly_deposits", "scheduled_cents", "INTEGER NOT NULL DEFAULT 0", "0")
    await ensure_column(conn, "joint_account_monthly_deposits", "actual_cents", "INTEGER NOT NULL DEFAULT 0", "0")
    await ensure_column(conn, "joint_account_monthly_deposits", "is_paid", "INTEGER NOT NULL DEFAULT 0", "0")
    await ensure_column(conn, "joint_account_monthly_deposits", "paid_date", "TEXT")

    # joint_account_expected_costs
    await ensure_column(conn, "joint_account_expected_costs", "expected_cents", "INTEGER NOT NULL DEFAULT 0", "0")

    # joint_account_corrections
    await ensure_column(conn, "joint_account_corrections", "note", "TEXT")

    # ── Indexes ─────────────────────────────────────────────────────────
    await conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_income_who_date ON income (who, income_date DESC)"
    )
    await conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_jobs_who_dates ON jobs (who, start_date, end_date)"
    )

    # ── Analytics views ─────────────────────────────────────────────────
    # Dropped and recreated on every boot so schema changes take effect
    # without a manual migration step.
    for view in (
        "view_monthly_total",
        "view_monthly_by_category",
        "view_monthly_by_payer",
        "view_expenses_by_month_category",
        "view_project_summary",
        "view_tag_totals",
        "view_joint_account_monthly",
    ):
        await conn.execute(f"DROP VIEW IF EXISTS {view}")

    await conn.execute(
        """
        CREATE VIEW view_monthly_total AS
        SELECT
            COALESCE(ROUND(SUM(cost_cents) / 100.0, 2), 0.0) AS total_amount,
            COUNT(*)                                           AS expense_count,
            strftime('%Y-%m', 'now')                          AS month
        FROM expenses
        WHERE strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now')
        """
    )
    await conn.execute(
        """
        CREATE VIEW view_monthly_by_category AS
        SELECT
            category,
            ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount,
            COUNT(*)                           AS expense_count
        FROM   expenses
        WHERE  strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now')
        GROUP  BY category
        """
    )
    await conn.execute(
        """
        CREATE VIEW view_expenses_by_month_category AS
        SELECT
            strftime('%Y-%m', expense_date)   AS month,
            category,
            ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount,
            COUNT(*)                           AS expense_count
        FROM   expenses
        GROUP  BY strftime('%Y-%m', expense_date), category
        """
    )
    await conn.execute(
        """
        CREATE VIEW view_monthly_by_payer AS
        SELECT
            who_paid,
            ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount,
            COUNT(*)                           AS expense_count
        FROM   expenses
        WHERE  strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now')
        GROUP  BY who_paid
        """
    )

    # ── view_project_summary ─────────────────────────────────────────────
    await conn.execute(
        """
        CREATE VIEW view_project_summary AS
        SELECT
            p.id,
            p.name,
            p.target_cents,
            p.target_date,
            p.is_joint,
            COALESCE(SUM(e.cost_cents), 0) AS total_spent_cents,
            COUNT(e.id)                     AS expense_count
        FROM projects p
        LEFT JOIN expenses e ON e.project_id = p.id
        GROUP BY p.id, p.name, p.target_cents, p.target_date, p.is_joint
        """
    )

    # ── view_tag_totals ─────────────────────────────────────────────────
    await conn.execute(
        """
        CREATE VIEW view_tag_totals AS
        SELECT
            t.id,
            t.name,
            t.color,
            t.description,
            t.is_joint,
            t.is_active,
            COALESCE(ROUND(SUM(e.cost_cents) / 100.0, 2), 0.0) AS total_amount,
            COUNT(e.id)                                          AS expense_count,
            MIN(e.expense_date)                                  AS first_date,
            MAX(e.expense_date)                                  AS last_date
        FROM tags t
        LEFT JOIN expenses e ON e.tag_id = t.id
        GROUP BY t.id, t.name, t.color, t.description, t.is_joint, t.is_active
        """
    )

    # ── view_joint_account_monthly ───────────────────────────────────────
    # Monthly spending per category for joint-account-assigned categories.
    await conn.execute(
        """
        CREATE VIEW view_joint_account_monthly AS
        SELECT
            strftime('%Y-%m', e.expense_date)  AS month,
            e.category,
            ROUND(SUM(e.cost_cents) / 100.0, 2) AS total_amount,
            COUNT(*)                             AS expense_count
        FROM expenses e
        INNER JOIN joint_account_categories jac ON jac.category = e.category
        GROUP BY strftime('%Y-%m', e.expense_date), e.category
        """
    )

    await conn.commit()


async def init_db(db_path: Path | None = None, conn: aiosqlite.Connection | None = None) -> None:
    """
    Create all tables/views if they do not yet exist, and run backwards-compatible migrations.
    Can be invoked on DB_PATH, a custom db_path, or an existing connection.
    Called on application startup and during database imports.
    """
    if conn is not None:
        await _init_db_schema(conn)
    else:
        target_path = db_path or DB_PATH
        async with aiosqlite.connect(target_path) as c:
            await _init_db_schema(c)
