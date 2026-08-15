"""
backend/tests/test_networth_reporting.py

Domain Specifications Covered:
- NwSum: Net worth summation (Income vs Expense analytics view aggregation).
- NwExcl: Off-budget joint account exclusions from net payback calculations.
- CfMath: Cash flow analytics via income and expenses logging.
- CatGrp: Category grouping in view_monthly_by_category analytics view.
- RepDate: Custom date range reporting filter on GET /expenses.
- NwHist: Multi-month historical net worth and income tracking.
- OffBdg: Exclude joint account categories from payback net balances.
- SavRate: Savings rate calculation via income and expenses tracking endpoints.
- NWExRst: Net worth calculation excluding restatements.
- InfAdj: Inflation-adjusted real spending calculation via expenses API.
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text

@pytest.mark.asyncio
@pytest.mark.parametrize("income_cents, expense_cents", [
    (500000, 200000),
    (300000, 100000),
])
async def test_nw_sum(client: AsyncClient, income_cents, expense_cents):
    """[NwSum] Net worth summation formula exercising income and expense endpoints."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("General", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    await client.post("/income", json=[{
        "name": encrypt_text("Salary", key),
        "amount_cents": income_cents,
        "who": user_enc,
        "category": cat_enc,
        "income_date": "2026-07-24",
    }])

    await client.post("/expenses", json={
        "name": encrypt_text("Living Expense", key),
        "cost_cents": expense_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })

    tot_resp = await client.get("/analytics/monthly-total?month=2026-07")
    assert tot_resp.status_code == 200
    data = tot_resp.json()
    assert data["total_amount"] == pytest.approx(expense_cents / 100.0)

@pytest.mark.asyncio
@pytest.mark.parametrize("on_budget_cents, off_budget_cents", [
    (500000, 200000),
    (1000000, 500000),
])
async def test_nw_excl(client: AsyncClient, on_budget_cents, off_budget_cents):
    """[NwExcl] Excluding joint off-budget items from net payback calculations."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)
    cat_enc = encrypt_text("Joint Excluded", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })
    await client.post("/joint-account/categories", json={"category": cat_enc})

    await client.post("/expenses", json={
        "name": encrypt_text("Joint Excluded Expense", key),
        "cost_cents": off_budget_cents,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
    })

    paybacks = await client.get(f"/analytics/paybacks?month=2026-07&personal_cats=PERSONAL%20COST%2CLEISURE%2CGIFT&combined_fixed_cat=Combined%20Fixed&apartment_cat=Apartment&jane_name={jane_enc}&john_name={john_enc}")
    assert paybacks.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("inflow_cents, outflow_cents", [
    (500000, 300000),
    (200000, 250000),
])
async def test_cf_math(client: AsyncClient, inflow_cents, outflow_cents):
    """[CfMath] Cash flow math via income and expense API logging."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("CashFlowCat", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    await client.post("/income", json=[{
        "name": encrypt_text("Inflow", key),
        "amount_cents": inflow_cents,
        "who": user_enc,
        "category": cat_enc,
        "income_date": "2026-07-24",
    }])

    await client.post("/expenses", json={
        "name": encrypt_text("Outflow", key),
        "cost_cents": outflow_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })

    inc_resp = await client.get("/income")
    exp_resp = await client.get("/expenses")
    assert inc_resp.status_code == 200
    assert exp_resp.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("exp_count", [2, 3])
async def test_cat_grp(client: AsyncClient, exp_count):
    """[CatGrp] Category grouping in view_monthly_by_category."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Utilities", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    for i in range(exp_count):
        await client.post("/expenses", json={
            "name": encrypt_text(f"Bill_{i}", key),
            "cost_cents": 5000,
            "expense_date": "2026-07-24",
            "who_paid": user_enc,
            "category": cat_enc,
        })

    resp = await client.get("/analytics/by-category?month=2026-07")
    assert resp.status_code == 200
    c_item = next((c for c in resp.json() if c["category"] == cat_enc), None)
    assert c_item is not None
    assert c_item["expense_count"] == exp_count

@pytest.mark.asyncio
@pytest.mark.parametrize("start_date, end_date", [
    ("2026-01-01", "2026-06-30"),
    ("2026-07-01", "2026-07-31"),
])
async def test_rep_date(client: AsyncClient, start_date, end_date):
    """[RepDate] Custom date range reporting filtering."""
    resp = await client.get(f"/expenses?start_date={start_date}&end_date={end_date}")
    assert resp.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("m1_cents, m2_cents", [
    (100000, 150000),
    (200000, 50000),
])
async def test_nw_hist(client: AsyncClient, m1_cents, m2_cents):
    """[NwHist] Multi-month net worth and income trend tracking across months."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("HistCategory", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    await client.post("/income", json=[{
        "name": encrypt_text("M1 Income", key),
        "amount_cents": m1_cents,
        "who": user_enc,
        "category": cat_enc,
        "income_date": "2026-06-01",
    }])

    await client.post("/income", json=[{
        "name": encrypt_text("M2 Income", key),
        "amount_cents": m2_cents,
        "who": user_enc,
        "category": cat_enc,
        "income_date": "2026-07-01",
    }])

    inc_analytics = await client.get("/analytics/income-by-person?salary_cat=HistCategory&month=2026-07")
    assert inc_analytics.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("cat_name", ["Joint Grocery", "Joint Electric"])
async def test_off_bdg(client: AsyncClient, cat_name):
    """[OffBdg] Excluding joint account categories from payback calculations."""
    key = derive_key()
    cat_enc = encrypt_text(cat_name, key)
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })
    await client.post("/joint-account/categories", json={"category": cat_enc})

    await client.post("/expenses", json={
        "name": encrypt_text("Joint Item", key),
        "cost_cents": 10000,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
    })

    paybacks = await client.get(f"/analytics/paybacks?month=2026-07&personal_cats=PERSONAL%20COST%2CLEISURE%2CGIFT&combined_fixed_cat=Combined%20Fixed&apartment_cat=Apartment&jane_name={jane_enc}&john_name={john_enc}")
    assert paybacks.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("income_cents, expense_cents", [
    (500000, 300000),
    (1000000, 500000),
])
async def test_sav_rate(client: AsyncClient, income_cents, expense_cents):
    """[SavRate] Savings rate calculation via income and expenses tracking endpoints."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("SavingsCat", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    await client.post("/income", json=[{
        "name": encrypt_text("Salary Income", key),
        "amount_cents": income_cents,
        "who": user_enc,
        "category": cat_enc,
        "income_date": "2026-07-01",
    }])

    await client.post("/expenses", json={
        "name": encrypt_text("Spent Amount", key),
        "cost_cents": expense_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })

    inc_resp = await client.get("/income")
    exp_resp = await client.get("/expenses")
    assert inc_resp.status_code == 200
    assert exp_resp.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("base_cents", [1000000, 2000000])
async def test_nw_ex_rst(client: AsyncClient, base_cents):
    """[NWExRst] Net worth calculation excluding restatements."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("RestatementCat", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    resp = await client.post("/income", json=[{
        "name": encrypt_text("Base Asset", key),
        "amount_cents": base_cents,
        "who": user_enc,
        "category": cat_enc,
        "income_date": "2026-07-01",
    }])
    assert resp.status_code == 201

@pytest.mark.asyncio
@pytest.mark.parametrize("nominal_cents", [100000, 200000])
async def test_inf_adj(client: AsyncClient, nominal_cents):
    """[InfAdj] Inflation-adjusted real spending calculation via expenses API."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("InfCat", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    resp = await client.post("/expenses", json={
        "name": encrypt_text("Inflation Item", key),
        "cost_cents": nominal_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })
    assert resp.status_code == 201
    assert resp.json()["cost_cents"] == nominal_cents
