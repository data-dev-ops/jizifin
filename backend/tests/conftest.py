"""
backend/tests/conftest.py — Pytest test harness configuration for Finance Tracker.

Provides:
  - Event loop & in-memory/isolated SQLite database connection for full state isolation.
  - FastAPI app dependency overrides for get_db.
  - httpx.AsyncClient fixture bound to app via ASGITransport.
  - Static IV AES-GCM crypto helpers matching backend/app/crypto_utils.py.
  - Per-test automatic cleanup to ensure zero state pollution between test runs.
"""

import base64
import pytest
import pytest_asyncio
import aiosqlite
from pathlib import Path
from typing import AsyncGenerator
from httpx import AsyncClient, ASGITransport

from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.ciphers.aead import AESGCM

from app.main import app
from app.database import get_db

STATIC_IV = b"jizifin-cryp"
SALT = b"jizifin-salt-pbkdf2"

# ---------------------------------------------------------------------------
# Crypto test helpers
# ---------------------------------------------------------------------------

def derive_key(password: str = "test-master-passphrase") -> bytes:
    kdf = PBKDF2HMAC(
        algorithm=hashes.SHA256(),
        length=32,
        salt=SALT,
        iterations=100000,
    )
    return kdf.derive(password.encode("utf-8"))

def encrypt_text(plaintext: str, key: bytes) -> str:
    if not plaintext:
        return plaintext
    aesgcm = AESGCM(key)
    ciphertext = aesgcm.encrypt(STATIC_IV, str(plaintext).encode("utf-8"), None)
    return base64.urlsafe_b64encode(ciphertext).decode("ascii").rstrip("=")

def decrypt_text(ciphertext: str, key: bytes) -> str:
    if not ciphertext:
        return ciphertext
    try:
        aesgcm = AESGCM(key)
        padded = ciphertext + "=" * ((4 - len(ciphertext) % 4) % 4)
        raw = base64.urlsafe_b64decode(padded)
        return aesgcm.decrypt(STATIC_IV, raw, None).decode("utf-8")
    except Exception:
        return ciphertext

# ---------------------------------------------------------------------------
# Isolated Database & Async Client Fixtures
# ---------------------------------------------------------------------------

import uuid

@pytest_asyncio.fixture
async def test_db(tmp_path: Path) -> AsyncGenerator[aiosqlite.Connection, None]:
    """Provision a fresh, isolated SQLite database file per test."""
    db_file = tmp_path / f"test_finance_{uuid.uuid4().hex}.db"
    async with aiosqlite.connect(db_file) as conn:
        await conn.execute("PRAGMA journal_mode=WAL;")
        await conn.execute("PRAGMA foreign_keys=ON;")
        conn.row_factory = aiosqlite.Row

        # Execute table DDL statements
        await conn.execute("CREATE TABLE IF NOT EXISTS app_config (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS users (
                name       TEXT    PRIMARY KEY CHECK(length(name) <= 256),
                color      TEXT    NOT NULL DEFAULT '#6366f1',
                is_active  INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1)),
                created_at TEXT    NOT NULL DEFAULT (datetime('now'))
            )
        """)
        await conn.execute("CREATE TABLE IF NOT EXISTS splits (category TEXT PRIMARY KEY CHECK(length(category) <= 256))")
        await conn.execute("CREATE TABLE IF NOT EXISTS income_categories (category TEXT PRIMARY KEY CHECK(length(category) <= 256))")
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS projects (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                name         TEXT    NOT NULL UNIQUE CHECK(length(name) <= 256),
                target_cents INTEGER NOT NULL CHECK(target_cents > 0),
                target_date  TEXT    NOT NULL CHECK(target_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
                is_joint     INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1)),
                allow_subcategories INTEGER NOT NULL DEFAULT 1 CHECK(allow_subcategories IN (0, 1))
            )
        """)
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS tags (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT    NOT NULL UNIQUE CHECK(length(name) <= 256),
                color       TEXT    NOT NULL DEFAULT '#f59e0b',
                description TEXT    CHECK(length(description) <= 512),
                created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
                is_joint    INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1))
            )
        """)
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS expenses (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                name         TEXT    NOT NULL CHECK(length(name) <= 256),
                cost_cents   INTEGER NOT NULL CHECK(cost_cents > 0),
                expense_date TEXT    NOT NULL CHECK(expense_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
                who_paid     TEXT    NOT NULL REFERENCES users(name) ON UPDATE CASCADE,
                category     TEXT    NOT NULL REFERENCES splits(category) ON UPDATE CASCADE,
                project_id   INTEGER REFERENCES projects(id) ON DELETE SET NULL,
                tag_id       INTEGER REFERENCES tags(id) ON DELETE SET NULL,
                is_joint     INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1))
            )
        """)
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS expense_overrides (
                expense_id INTEGER NOT NULL REFERENCES expenses(id) ON DELETE CASCADE,
                user_name  TEXT    NOT NULL REFERENCES users(name) ON UPDATE CASCADE ON DELETE CASCADE,
                pct        REAL    NOT NULL CHECK(pct >= 0.0 AND pct <= 100.0),
                PRIMARY KEY (expense_id, user_name)
            )
        """)
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS income (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                name         TEXT    NOT NULL CHECK(length(name) <= 256),
                amount_cents INTEGER NOT NULL CHECK(amount_cents > 0),
                who          TEXT    NOT NULL REFERENCES users(name) ON UPDATE CASCADE,
                category     TEXT    NOT NULL CHECK(length(category) <= 256),
                income_date  TEXT    NOT NULL CHECK(income_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
                is_joint     INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1))
            )
        """)
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS jobs (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                name         TEXT    NOT NULL CHECK(length(name) <= 256),
                who          TEXT    NOT NULL REFERENCES users(name) ON UPDATE CASCADE,
                amount_cents INTEGER NOT NULL CHECK(amount_cents > 0),
                frequency    TEXT    NOT NULL DEFAULT 'monthly' CHECK(frequency IN ('monthly', 'weekly', 'biweekly', 'annual')),
                start_date   TEXT    NOT NULL CHECK(start_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
                end_date     TEXT    CHECK(end_date IS NULL OR end_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
                notes        TEXT    CHECK(notes IS NULL OR length(notes) <= 512),
                is_active    INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1))
            )
        """)
        await conn.execute("CREATE INDEX IF NOT EXISTS idx_jobs_who_dates ON jobs (who, start_date, end_date)")
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS recurring_expenses (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                name         TEXT    NOT NULL CHECK(length(name) <= 256),
                cost_cents   INTEGER NOT NULL CHECK(cost_cents > 0),
                who_paid     TEXT    NOT NULL REFERENCES users(name) ON UPDATE CASCADE,
                category     TEXT    NOT NULL REFERENCES splits(category) ON UPDATE CASCADE,
                day_of_month INTEGER NOT NULL CHECK(day_of_month >= 1 AND day_of_month <= 31),
                is_joint     INTEGER NOT NULL DEFAULT 0 CHECK(is_joint IN (0, 1))
            )
        """)
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS budgets (
                category    TEXT    NOT NULL REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE,
                month       TEXT    NOT NULL CHECK(month = 'ALL' OR month GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]'),
                limit_cents INTEGER NOT NULL CHECK(limit_cents >= 0),
                PRIMARY KEY (category, month)
            )
        """)
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS settlements (
                month                          TEXT PRIMARY KEY CHECK(month GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]'),
                settled_at                     TEXT NOT NULL,
                net_balance_transferred_cents INTEGER NOT NULL
            )
        """)
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS split_allocations (
                category  TEXT NOT NULL REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE,
                user_name TEXT NOT NULL REFERENCES users(name) ON UPDATE CASCADE ON DELETE CASCADE,
                pct       REAL NOT NULL CHECK(pct >= 0.0 AND pct <= 100.0),
                PRIMARY KEY (category, user_name)
            )
        """)
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS joint_account (
                id                  INTEGER PRIMARY KEY CHECK(id = 1),
                name                TEXT    NOT NULL CHECK(length(name) <= 256),
                balance_cents       INTEGER NOT NULL DEFAULT 0,
                safety_margin_pct   INTEGER NOT NULL DEFAULT 10 CHECK(safety_margin_pct >= 0 AND safety_margin_pct <= 100),
                deposit_split_mode TEXT    NOT NULL DEFAULT 'even' CHECK(deposit_split_mode IN ('salary', 'even', 'manual')),
                expected_total_cents INTEGER
            )
        """)
        await conn.execute("CREATE TABLE IF NOT EXISTS joint_account_categories (category TEXT PRIMARY KEY REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE)")
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS joint_account_deposits (
                user_name    TEXT    PRIMARY KEY REFERENCES users(name) ON UPDATE CASCADE ON DELETE CASCADE,
                amount_cents INTEGER NOT NULL DEFAULT 0 CHECK(amount_cents >= 0),
                day_of_month INTEGER NOT NULL DEFAULT 1 CHECK(day_of_month >= 1 AND day_of_month <= 31)
            )
        """)
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS joint_account_monthly_deposits (
                month           TEXT    NOT NULL CHECK(month GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]'),
                user_name       TEXT    NOT NULL REFERENCES users(name) ON UPDATE CASCADE ON DELETE CASCADE,
                scheduled_cents INTEGER NOT NULL DEFAULT 0 CHECK(scheduled_cents >= 0),
                actual_cents    INTEGER NOT NULL DEFAULT 0 CHECK(actual_cents >= 0),
                is_paid         INTEGER NOT NULL DEFAULT 0 CHECK(is_paid IN (0, 1)),
                paid_date       TEXT,
                PRIMARY KEY (month, user_name)
            )
        """)
        await conn.execute("CREATE TABLE IF NOT EXISTS joint_account_expected_costs (category TEXT PRIMARY KEY REFERENCES splits(category) ON UPDATE CASCADE ON DELETE CASCADE)")
        await conn.execute("""
            CREATE TABLE IF NOT EXISTS joint_account_corrections (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                amount_cents   INTEGER NOT NULL,
                correction_date TEXT   NOT NULL CHECK(correction_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
                note           TEXT    CHECK(length(note) <= 512)
            )
        """)

        # Views
        await conn.execute("CREATE VIEW IF NOT EXISTS view_monthly_total AS SELECT COALESCE(ROUND(SUM(cost_cents) / 100.0, 2), 0.0) AS total_amount, COUNT(*) AS expense_count, strftime('%Y-%m', 'now') AS month FROM expenses WHERE strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now')")
        await conn.execute("CREATE VIEW IF NOT EXISTS view_monthly_by_category AS SELECT category, ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount, COUNT(*) AS expense_count FROM expenses WHERE strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now') GROUP BY category")
        await conn.execute("CREATE VIEW IF NOT EXISTS view_expenses_by_month_category AS SELECT strftime('%Y-%m', expense_date) AS month, category, ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount, COUNT(*) AS expense_count FROM expenses GROUP BY strftime('%Y-%m', expense_date), category")
        await conn.execute("CREATE VIEW IF NOT EXISTS view_monthly_by_payer AS SELECT who_paid, ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount, COUNT(*) AS expense_count FROM expenses WHERE strftime('%Y-%m', expense_date) = strftime('%Y-%m', 'now') GROUP BY who_paid")
        await conn.execute("CREATE VIEW IF NOT EXISTS view_project_summary AS SELECT p.id, p.name, p.target_cents, p.target_date, COALESCE(SUM(e.cost_cents), 0) AS total_spent_cents, COUNT(e.id) AS expense_count FROM projects p LEFT JOIN expenses e ON e.project_id = p.id GROUP BY p.id, p.name, p.target_cents, p.target_date")
        await conn.execute("CREATE VIEW IF NOT EXISTS view_tag_totals AS SELECT t.id, t.name, t.color, t.description, t.is_joint, COALESCE(ROUND(SUM(e.cost_cents) / 100.0, 2), 0.0) AS total_amount, COUNT(e.id) AS expense_count, MIN(e.expense_date) AS first_date, MAX(e.expense_date) AS last_date FROM tags t LEFT JOIN expenses e ON e.tag_id = t.id GROUP BY t.id, t.name, t.color, t.description, t.is_joint")
        await conn.execute("CREATE VIEW IF NOT EXISTS view_joint_account_monthly AS SELECT strftime('%Y-%m', e.expense_date) AS month, e.category, ROUND(SUM(e.cost_cents) / 100.0, 2) AS total_amount, COUNT(*) AS expense_count FROM expenses e INNER JOIN joint_account_categories jac ON jac.category = e.category GROUP BY strftime('%Y-%m', e.expense_date), e.category")

        await conn.commit()
        yield conn

@pytest_asyncio.fixture
async def client(test_db: aiosqlite.Connection) -> AsyncGenerator[AsyncClient, None]:
    """FastAPI AsyncClient dependency-overridden with test_db."""
    async def override_get_db():
        yield test_db

    app.dependency_overrides[get_db] = override_get_db
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as ac:
        yield ac
    app.dependency_overrides.clear()
