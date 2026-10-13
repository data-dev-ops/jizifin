import datetime

import aiosqlite
import pytest
from httpx import AsyncClient, ASGITransport

from app.main import app, process_recurring_expenses
from app.database import init_db
from tests.conftest import derive_key, encrypt_text


@pytest.fixture(autouse=True)
async def setup_db(tmp_path, monkeypatch):
    test_db = tmp_path / "test.db"
    monkeypatch.setattr("app.database.DB_PATH", test_db)
    monkeypatch.setattr("app.main.DB_PATH", test_db)
    await init_db(test_db)
    return test_db


def _freeze_today(monkeypatch, y, m, d):
    class FrozenDate(datetime.date):
        @classmethod
        def today(cls):
            return cls(y, m, d)

    monkeypatch.setattr("app.main._date", FrozenDate)


async def _seed_template(db_path, day_of_month, start_date="2026-01-01"):
    key = derive_key("test-passphrase")
    who = encrypt_text("Alice", key)
    cat = encrypt_text("RENT", key)
    name = encrypt_text("Rent", key)

    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        await client.post("/users", json={"name": who, "color": "#ff0000"})
        res = await client.post("/splits", json={
            "category": cat,
            "allocations": [{"user_name": who, "pct": 100.0}],
        })
        assert res.status_code == 201

    async with aiosqlite.connect(db_path) as conn:
        await conn.execute("PRAGMA foreign_keys=ON;")
        await conn.execute(
            "INSERT INTO recurring_expenses (name, cost_cents, who_paid, category, frequency, day_of_month, start_date, is_active) "
            "VALUES (?, 10000, ?, ?, 'monthly', ?, ?, 1)",
            (name, who, cat, day_of_month, start_date),
        )
        await conn.commit()


async def _expense_dates(db_path):
    async with aiosqlite.connect(db_path) as conn:
        async with conn.execute("SELECT expense_date FROM expenses ORDER BY expense_date") as cur:
            return [r[0] for r in await cur.fetchall()]


@pytest.mark.asyncio
async def test_catch_up_logs_missed_occurrence_this_month(setup_db, monkeypatch):
    await _seed_template(setup_db, day_of_month=5)
    _freeze_today(monkeypatch, 2026, 10, 13)

    assert await process_recurring_expenses() == 1
    assert await _expense_dates(setup_db) == ["2026-10-05"]


@pytest.mark.asyncio
async def test_catch_up_is_idempotent(setup_db, monkeypatch):
    await _seed_template(setup_db, day_of_month=5)
    _freeze_today(monkeypatch, 2026, 10, 13)

    assert await process_recurring_expenses() == 1
    assert await process_recurring_expenses() == 0
    assert await _expense_dates(setup_db) == ["2026-10-05"]


@pytest.mark.asyncio
async def test_catch_up_skips_future_occurrences(setup_db, monkeypatch):
    await _seed_template(setup_db, day_of_month=20)
    _freeze_today(monkeypatch, 2026, 10, 13)

    assert await process_recurring_expenses() == 0
    assert await _expense_dates(setup_db) == []


@pytest.mark.asyncio
async def test_grace_period_recovers_previous_month_end(setup_db, monkeypatch):
    # Server down over the rollover: 31st (clamped to Sep 30) is within 7-day grace of Oct 3.
    await _seed_template(setup_db, day_of_month=31)
    _freeze_today(monkeypatch, 2026, 10, 3)

    assert await process_recurring_expenses() == 1
    assert await _expense_dates(setup_db) == ["2026-09-30"]


@pytest.mark.asyncio
async def test_outside_grace_period_previous_month_not_recovered(setup_db, monkeypatch):
    await _seed_template(setup_db, day_of_month=31)
    _freeze_today(monkeypatch, 2026, 10, 20)

    assert await process_recurring_expenses() == 0
    assert await _expense_dates(setup_db) == []
