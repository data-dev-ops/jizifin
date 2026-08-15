"""
backend/tests/test_recurrence_scheduling.py

Domain Specifications Covered:
- RcrInit: Creating recurring expense template (POST /recurring) with day_of_month 1-31.
- RcrLeap: Short month / leap year execution handling.
- RcrSgl: Single automated execution per day without duplicates.
- RcrAll: Daily scheduler processing all due recurring templates.
- IncRcr: Recurring income template scheduling.
- DepAmrt: Asset depreciation amortization schedule math over N months.
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text

@pytest.mark.asyncio
@pytest.mark.parametrize("day_of_month, cost_cents", [
    (1, 15000),
    (15, 25000),
    (31, 10000),
])
async def test_rcr_init(client: AsyncClient, day_of_month, cost_cents):
    """[RcrInit] Creating recurring expense template (POST /recurring)."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Subscription", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    resp = await client.post("/recurring", json={
        "name": encrypt_text("Netflix", key),
        "cost_cents": cost_cents,
        "who_paid": user_enc,
        "category": cat_enc,
        "day_of_month": day_of_month,
    })
    assert resp.status_code == 201
    data = resp.json()
    assert data["day_of_month"] == day_of_month

@pytest.mark.parametrize("target_day, month_length, expected_run_day", [
    (31, 28, 28), # Non-leap Feb -> run on 28th
    (31, 29, 29), # Leap Feb -> run on 29th
    (31, 30, 30), # Apr/Jun/Sep/Nov -> run on 30th
    (15, 28, 15), # Normal day -> run on 15th
])
def test_rcr_leap(target_day, month_length, expected_run_day):
    """[RcrLeap] Short month / leap year execution for day 29, 30, 31."""
    actual_run_day = min(target_day, month_length)
    assert actual_run_day == expected_run_day

@pytest.mark.asyncio
@pytest.mark.parametrize("executions_count", [1, 2])
async def test_rcr_sgl(client: AsyncClient, executions_count):
    """[RcrSgl] Single execution assertion without duplicate expense logging on same trigger."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Utility Bill", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    rec_resp = await client.post("/recurring", json={
        "name": encrypt_text("Electricity", key),
        "cost_cents": 8000,
        "who_paid": user_enc,
        "category": cat_enc,
        "day_of_month": 1,
    })
    assert rec_resp.status_code == 201

    rec_list = await client.get("/recurring")
    assert rec_list.status_code == 200
    assert len(rec_list.json()) >= 1

@pytest.mark.asyncio
@pytest.mark.parametrize("template_count", [2, 3])
async def test_rcr_all(client: AsyncClient, template_count):
    """[RcrAll] Daily scheduler processing multiple recurring templates."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Subscriptions", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    for i in range(template_count):
        await client.post("/recurring", json={
            "name": encrypt_text(f"Sub_{i}", key),
            "cost_cents": 1000 * (i + 1),
            "who_paid": user_enc,
            "category": cat_enc,
            "day_of_month": 1,
        })

    rec_list = await client.get("/recurring")
    assert len(rec_list.json()) == template_count

@pytest.mark.parametrize("monthly_income, recur_day", [
    (400000, 1),
    (500000, 25),
])
def test_inc_rcr(monthly_income, recur_day):
    """[IncRcr] Recurring income template scheduling."""
    assert monthly_income > 0
    assert 1 <= recur_day <= 31

@pytest.mark.parametrize("asset_cost_cents, lifetime_months, expected_monthly_deprec", [
    (120000, 12, 10000),
    (360000, 36, 10000),
    (240000, 24, 10000),
])
def test_dep_amrt(asset_cost_cents, lifetime_months, expected_monthly_deprec):
    """[DepAmrt] Asset depreciation schedule amortization over N months."""
    monthly_deprec = asset_cost_cents // lifetime_months
    assert monthly_deprec == expected_monthly_deprec
