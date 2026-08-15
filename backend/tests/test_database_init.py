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
