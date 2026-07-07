"""
database.py — aiosqlite connection management and schema definition.

Creates all tables, indexes, and views on first boot.
No migration logic — this is the canonical v4 multi-user schema.
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


# ---------------------------------------------------------------------------
# Schema initialisation
# ---------------------------------------------------------------------------

async def init_db() -> None:
    """
    Create all tables/views if they do not yet exist.
    Called once at application startup via the FastAPI lifespan hook.
    Tables are created in dependency order so FK enforcement can stay ON.
    """
    async with aiosqlite.connect(DB_PATH) as conn:
        await conn.execute("PRAGMA journal_mode=WAL;")
        await conn.execute("PRAGMA foreign_keys=ON;")
        conn.row_factory = aiosqlite.Row

        # ── users ──────────────────────────────────────────────────────────
        # Household members.  Manage via the Settings tab in the UI.
        await conn.execute(
            """
            CREATE TABLE IF NOT EXISTS users (
                name       TEXT    PRIMARY KEY CHECK(length(name) <= 64),
                color      TEXT    NOT NULL DEFAULT '#6366f1',
                is_active  INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1)),
                created_at TEXT    NOT NULL DEFAULT (datetime('now'))
            )
            """
        )

        # ── splits ─────────────────────────────────────────────────────────
        # Category registry.  Each category's per-user percentages live in
        # split_allocations, not here.
        await conn.execute(
            "CREATE TABLE IF NOT EXISTS splits (category TEXT PRIMARY KEY CHECK(length(category) <= 32))"
        )

        # ── projects ───────────────────────────────────────────────────────
        await conn.execute(
            """
            CREATE TABLE IF NOT EXISTS projects (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                name         TEXT    NOT NULL UNIQUE CHECK(length(name) <= 96),
                target_cents INTEGER NOT NULL CHECK(target_cents > 0),
                target_date  TEXT    NOT NULL
                                     CHECK(target_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]')
            )
            """
        )

        # ── expenses ────────────────────────────────────────────────────────
        await conn.execute(
            """
            CREATE TABLE IF NOT EXISTS expenses (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                name         TEXT    NOT NULL CHECK(length(name) <= 96),
                cost_cents   INTEGER NOT NULL CHECK(cost_cents > 0),
                expense_date TEXT    NOT NULL
                                     CHECK(expense_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
                who_paid     TEXT    NOT NULL REFERENCES users(name)           ON UPDATE CASCADE,
                category     TEXT    NOT NULL REFERENCES splits(category)      ON UPDATE CASCADE,
                project_id   INTEGER          REFERENCES projects(id)          ON DELETE SET NULL
            )
            """
        )

        # ── expense_overrides ──────────────────────────────────────────────
        # Per-transaction split overrides.  Present only when a custom split
        # deviates from the category default.
        await conn.execute(
            """
            CREATE TABLE IF NOT EXISTS expense_overrides (
                expense_id INTEGER NOT NULL REFERENCES expenses(id) ON DELETE CASCADE,
                user_name  TEXT    NOT NULL REFERENCES users(name)  ON UPDATE CASCADE ON DELETE CASCADE,
                pct        REAL    NOT NULL CHECK(pct >= 0.0 AND pct <= 100.0),
                PRIMARY KEY (expense_id, user_name)
            )
            """
        )

        # ── income ──────────────────────────────────────────────────────────
        # Append-only ledger.  category = 'SALARY' entries drive the salary
        # carry-forward logic in /analytics/income-by-person.
        await conn.execute(
            """
            CREATE TABLE IF NOT EXISTS income (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                name         TEXT    NOT NULL CHECK(length(name) <= 96),
                amount_cents INTEGER NOT NULL CHECK(amount_cents > 0),
                who          TEXT    NOT NULL REFERENCES users(name) ON UPDATE CASCADE,
                category     TEXT    NOT NULL CHECK(length(category) <= 32),
                income_date  TEXT    NOT NULL
                                     CHECK(income_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]')
            )
            """
        )

        await conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_income_who_date ON income (who, income_date DESC)"
        )

        # ── recurring_expenses ──────────────────────────────────────────────
        await conn.execute(
            """
            CREATE TABLE IF NOT EXISTS recurring_expenses (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                name         TEXT    NOT NULL CHECK(length(name) <= 96),
                cost_cents   INTEGER NOT NULL CHECK(cost_cents > 0),
                who_paid     TEXT    NOT NULL REFERENCES users(name)      ON UPDATE CASCADE,
                category     TEXT    NOT NULL REFERENCES splits(category) ON UPDATE CASCADE,
                day_of_month INTEGER NOT NULL CHECK(day_of_month >= 1 AND day_of_month <= 31)
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
        # Per-user percentage share per category.  Allocations for a category
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

        # ── Seed default split categories ───────────────────────────────────
        # INSERT OR IGNORE is idempotent — safe on every boot.
        # Allocations are left empty until users are added via the Settings tab.
        default_categories = [
            "GROCERIES", "UTILITIES", "RENT", "OTHER", "FIXED COSTS",
            "DATING", "LEISURE", "GIFT", "PET", "PERSONAL COST",
        ]
        for cat in default_categories:
            await conn.execute("INSERT OR IGNORE INTO splits (category) VALUES (?)", (cat,))

        # ── Analytics views ─────────────────────────────────────────────────
        # Dropped and recreated on every boot so schema changes take effect
        # without a manual migration step.
        for view in ("view_monthly_total", "view_monthly_by_category", "view_monthly_by_payer"):
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

        await conn.commit()
