"""
backend/tests/test_database_init.py

Tests for backend/app/database.py schema initialization and ensure_column utility.
"""

import pytest
import aiosqlite
import tempfile
from pathlib import Path
from unittest.mock import patch
from app.database import init_db, ensure_column, get_db

@pytest.mark.asyncio
async def test_init_db_creates_all_tables_including_expense_overrides():
    with tempfile.TemporaryDirectory() as tmpdir:
        temp_db_path = Path(tmpdir) / "test_init.db"
        with patch("app.database.DB_PATH", temp_db_path):
            await init_db()

            async with aiosqlite.connect(temp_db_path) as conn:
                async with conn.execute(
                    "SELECT name FROM sqlite_master WHERE type='table'"
                ) as cur:
                    tables = {row[0] async for row in cur}

                expected_tables = {
                    "app_config",
                    "users",
                    "splits",
                    "income_categories",
                    "projects",
                    "tags",
                    "expenses",
                    "expense_overrides",
                    "income",
                    "recurring_expenses",
                    "budgets",
                    "settlements",
                    "split_allocations",
                    "joint_account",
                    "joint_account_categories",
                    "joint_account_deposits",
                    "joint_account_monthly_deposits",
                    "joint_account_expected_costs",
                    "joint_account_corrections",
                }
                assert expected_tables.issubset(tables), f"Missing tables: {expected_tables - tables}"

                # Verify views exist
                async with conn.execute(
                    "SELECT name FROM sqlite_master WHERE type='view'"
                ) as cur:
                    views = {row[0] async for row in cur}

                expected_views = {
                    "view_monthly_total",
                    "view_monthly_by_category",
                    "view_expenses_by_month_category",
                    "view_monthly_by_payer",
                    "view_project_summary",
                    "view_tag_totals",
                    "view_joint_account_monthly",
                }
                assert expected_views.issubset(views), f"Missing views: {expected_views - views}"


@pytest.mark.asyncio
async def test_ensure_column():
    with tempfile.TemporaryDirectory() as tmpdir:
        temp_db_path = Path(tmpdir) / "test_col.db"
        async with aiosqlite.connect(temp_db_path) as conn:
            await conn.execute("CREATE TABLE sample (id INTEGER PRIMARY KEY)")
            await ensure_column(conn, "sample", "notes", "TEXT")
            await ensure_column(conn, "sample", "notes", "TEXT")  # Idempotent

            async with conn.execute("PRAGMA table_info(sample)") as cur:
                cols = [row[1] async for row in cur]
            assert "notes" in cols


@pytest.mark.asyncio
async def test_get_db_generator():
    with tempfile.TemporaryDirectory() as tmpdir:
        temp_db_path = Path(tmpdir) / "test_gen.db"
        with patch("app.database.DB_PATH", temp_db_path):
            gen = get_db()
            conn = await anext(gen)
            assert conn is not None
            # trigger clean close
            try:
                await anext(gen)
            except StopAsyncIteration:
                pass


@pytest.mark.asyncio
async def test_legacy_schema_migration_and_defaults():
    """Verify that an older database missing tables and columns is migrated cleanly with defaults populated."""
    with tempfile.TemporaryDirectory() as tmpdir:
        temp_db_path = Path(tmpdir) / "legacy.db"
        # Create an old schema with missing columns and old rows
        async with aiosqlite.connect(temp_db_path) as conn:
            await conn.execute("""
                CREATE TABLE recurring_expenses (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    cost_cents INTEGER NOT NULL,
                    who_paid TEXT NOT NULL,
                    category TEXT NOT NULL,
                    day_of_month INTEGER NOT NULL,
                    is_joint INTEGER NOT NULL DEFAULT 0
                )
            """)
            await conn.execute("""
                INSERT INTO recurring_expenses (name, cost_cents, who_paid, category, day_of_month, is_joint)
                VALUES ('Old Rent', 100000, 'John', 'RENT', 1, 0)
            """)
            await conn.execute("""
                CREATE TABLE users (
                    name TEXT PRIMARY KEY
                )
            """)
            await conn.execute("INSERT INTO users (name) VALUES ('John')")
            await conn.commit()

        # Run init_db on the legacy DB
        await init_db(db_path=temp_db_path)

        async with aiosqlite.connect(temp_db_path) as conn:
            conn.row_factory = aiosqlite.Row
            # Check recurring_expenses
            async with conn.execute("SELECT * FROM recurring_expenses WHERE name = 'Old Rent'") as cur:
                row = await cur.fetchone()
                assert row is not None
                assert row["frequency"] == "monthly"
                assert row["start_date"] == "2026-01-01"
                assert row["is_active"] == 1
                assert row["day_of_month"] == 1

            # Check users defaults
            async with conn.execute("SELECT * FROM users WHERE name = 'John'") as cur:
                row = await cur.fetchone()
                assert row is not None
                assert row["color"] == "#6366f1"
                assert row["is_active"] == 1
                assert row["created_at"] is not None

            # Check missing tables like jobs were created
            async with conn.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='jobs'") as cur:
                assert await cur.fetchone() is not None

