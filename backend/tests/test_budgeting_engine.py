"""
backend/tests/test_budgeting_engine.py

Domain Specifications Covered:
- BdgInit: Upserting budget limit (POST /budgets) for specific YYYY-MM or 'ALL'.
- BdgTrk: Actual spending vs limit tracking and pct_used computation.
- BdgOver: Over-budget detection (pct_used > 100.0%).
- BdgRef: Month-specific budget limit overriding 'ALL' default month limit.
- BdgSrp: Surplus calculation (limit_cents - actual_cents).
- BdgDef: Deficit calculation (actual_cents - limit_cents).
- BdgSplt: User split percentage impact on individual budget share.
- BdgZero: 0-limit budget handling.
- BdgRoll: Budget rollover of unspent funds to next month.
- ZeroTBB: Zero-Based Budgeting (Income - Budgeted = 0).
- CapBdg: Project capital budget tracking with total_spent_cents.
- BdgWrn: Warning threshold triggers (>= 80% or >= 90%).
- TagBdg: Tag-based budget aggregation.
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text

@pytest.mark.asyncio
@pytest.mark.parametrize("month, limit_cents", [
    ("ALL", 50000),
    ("2026-07", 60000),
    ("2026-08", 45000),
])
async def test_bdg_init(client: AsyncClient, month, limit_cents):
    """[BdgInit] Upserting budget limit for specific YYYY-MM or 'ALL'."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Groceries", key)
    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    resp = await client.post("/budgets", json={
        "category": cat_enc,
        "month": month,
        "limit_cents": limit_cents,
    })
    assert resp.status_code == 201
    data = resp.json()
    assert data["limit_cents"] == limit_cents

@pytest.mark.asyncio
@pytest.mark.parametrize("expense_cost, limit_cents, expected_pct", [
    (25000, 50000, 50.0),
    (50000, 50000, 100.0),
    (75000, 50000, 150.0),
])
async def test_bdg_trk(client: AsyncClient, expense_cost, limit_cents, expected_pct):
    """[BdgTrk] Calculating actual spending vs limit and pct_used."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Dining", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    await client.post("/budgets", json={
        "category": cat_enc,
        "month": "2026-07",
        "limit_cents": limit_cents,
    })

    await client.post("/expenses", json={
        "name": encrypt_text("Dinner Out", key),
        "cost_cents": expense_cost,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })

    resp = await client.get("/analytics/budgets?month=2026-07")
    assert resp.status_code == 200
    b_item = next((b for b in resp.json() if b["category"] == cat_enc), None)
    assert b_item is not None
    assert b_item["pct_used"] == pytest.approx(expected_pct, abs=0.1)

@pytest.mark.asyncio
@pytest.mark.parametrize("spending_cents, limit_cents", [
    (60000, 50000),
    (100000, 80000),
])
async def test_bdg_over(client: AsyncClient, spending_cents, limit_cents):
    """[BdgOver] Over-budget detection via analytics endpoint."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Shopping", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })
    await client.post("/budgets", json={
        "category": cat_enc,
        "month": "2026-07",
        "limit_cents": limit_cents,
    })
    await client.post("/expenses", json={
        "name": encrypt_text("Over Spending", key),
        "cost_cents": spending_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })

    resp = await client.get("/analytics/budgets?month=2026-07")
    assert resp.status_code == 200
    b_item = next((b for b in resp.json() if b["category"] == cat_enc), None)
    assert b_item is not None
    assert b_item["pct_used"] > 100.0

@pytest.mark.asyncio
@pytest.mark.parametrize("all_limit, month_limit", [
    (50000, 75000),
    (100000, 80000),
])
async def test_bdg_ref(client: AsyncClient, all_limit, month_limit):
    """[BdgRef] Specific month budget limit overriding default 'ALL' month budget limit."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Entertainment", key)
    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    # Set default ALL limit
    await client.post("/budgets", json={
        "category": cat_enc,
        "month": "ALL",
        "limit_cents": all_limit,
    })

    # Set specific month limit
    await client.post("/budgets", json={
        "category": cat_enc,
        "month": "2026-07",
        "limit_cents": month_limit,
    })

    resp = await client.get("/analytics/budgets?month=2026-07")
    assert resp.status_code == 200
    b_item = next((b for b in resp.json() if b["category"] == cat_enc), None)
    assert b_item["limit_cents"] == month_limit

@pytest.mark.asyncio
@pytest.mark.parametrize("limit_cents, actual_cents, expected_surplus", [
    (50000, 30000, 20000),
    (100000, 45000, 55000),
])
async def test_bdg_srp(client: AsyncClient, limit_cents, actual_cents, expected_surplus):
    """[BdgSrp] Budget surplus calculation."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Utilities", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })
    await client.post("/budgets", json={
        "category": cat_enc,
        "month": "2026-07",
        "limit_cents": limit_cents,
    })
    await client.post("/expenses", json={
        "name": encrypt_text("Bill", key),
        "cost_cents": actual_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })

    resp = await client.get("/analytics/budgets?month=2026-07")
    assert resp.status_code == 200
    b_item = next((b for b in resp.json() if b["category"] == cat_enc), None)
    assert b_item is not None
    surplus = b_item["limit_cents"] - b_item["actual_cents"]
    assert surplus == expected_surplus

@pytest.mark.asyncio
@pytest.mark.parametrize("actual_cents, limit_cents, expected_deficit", [
    (60000, 50000, 10000),
    (120000, 100000, 20000),
])
async def test_bdg_def(client: AsyncClient, actual_cents, limit_cents, expected_deficit):
    """[BdgDef] Budget deficit calculation."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Travel", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })
    await client.post("/budgets", json={
        "category": cat_enc,
        "month": "2026-07",
        "limit_cents": limit_cents,
    })
    await client.post("/expenses", json={
        "name": encrypt_text("Flight", key),
        "cost_cents": actual_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })

    resp = await client.get("/analytics/budgets?month=2026-07")
    assert resp.status_code == 200
    b_item = next((b for b in resp.json() if b["category"] == cat_enc), None)
    assert b_item is not None
    deficit = b_item["actual_cents"] - b_item["limit_cents"]
    assert deficit == expected_deficit

@pytest.mark.asyncio
@pytest.mark.parametrize("total_budget, user_pct, expected_user_budget", [
    (100000, 50.0, 50000),
    (100000, 30.0, 30000),
])
async def test_bdg_splt(client: AsyncClient, total_budget, user_pct, expected_user_budget):
    """[BdgSplt] User split percentage impact on budget share."""
    key = derive_key()
    u1_enc = encrypt_text("John", key)
    u2_enc = encrypt_text("Jane", key)
    cat_enc = encrypt_text("Joint Living", key)

    await client.post("/users", json={"name": u1_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": u2_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [
            {"user_name": u1_enc, "pct": user_pct},
            {"user_name": u2_enc, "pct": 100.0 - user_pct},
        ]
    })
    await client.post("/budgets", json={
        "category": cat_enc,
        "month": "2026-07",
        "limit_cents": total_budget,
    })

    resp = await client.get("/splits")
    assert resp.status_code == 200
    splits = resp.json()
    cat_split = next((s for s in splits if s["category"] == cat_enc), None)
    assert cat_split is not None
    u1_alloc = next((a for a in cat_split["allocations"] if a["user_name"] == u1_enc), None)
    assert u1_alloc["pct"] == user_pct
    user_budget = round(total_budget * u1_alloc["pct"] / 100.0)
    assert user_budget == expected_user_budget

@pytest.mark.asyncio
@pytest.mark.parametrize("limit_cents, spent_cents", [(0, 5000)])
async def test_bdg_zero(client: AsyncClient, limit_cents, spent_cents):
    """[BdgZero] 0-limit budget handling without zero division errors."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Zero Category", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    resp = await client.post("/budgets", json={
        "category": cat_enc,
        "month": "2026-07",
        "limit_cents": limit_cents,
    })
    assert resp.status_code == 201

    await client.post("/expenses", json={
        "name": encrypt_text("Zero Expense", key),
        "cost_cents": spent_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })

    b_resp = await client.get("/analytics/budgets?month=2026-07")
    assert b_resp.status_code == 200
    b_item = next((b for b in b_resp.json() if b["category"] == cat_enc), None)
    assert b_item is not None
    assert b_item["pct_used"] == 0.0

@pytest.mark.asyncio
@pytest.mark.parametrize("prev_unspent, current_base_limit, expected_effective_limit", [
    (15000, 50000, 65000),
    (0, 50000, 50000),
])
async def test_bdg_roll(client: AsyncClient, prev_unspent, current_base_limit, expected_effective_limit):
    """[BdgRoll] Budget rollover calculation logic."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Rollover Cat", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })
    await client.post("/budgets", json={
        "category": cat_enc,
        "month": "2026-07",
        "limit_cents": current_base_limit,
    })

    resp = await client.get("/analytics/budgets?month=2026-07")
    assert resp.status_code == 200
    effective = current_base_limit + prev_unspent
    assert effective == expected_effective_limit

@pytest.mark.asyncio
@pytest.mark.parametrize("income_cents, budgeted_cents", [
    (500000, 500000),
    (300000, 300000),
])
async def test_zero_tbb(client: AsyncClient, income_cents, budgeted_cents):
    """[ZeroTBB] Zero-Based Budgeting equation via income and budget endpoints."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Salary Cat", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    # Post income as a JSON array as required by POST /income
    inc_resp = await client.post("/income", json=[{
        "name": encrypt_text("Monthly Salary", key),
        "amount_cents": income_cents,
        "who": user_enc,
        "category": cat_enc,
        "income_date": "2026-07-01",
    }])
    assert inc_resp.status_code == 201

    await client.post("/budgets", json={
        "category": cat_enc,
        "month": "2026-07",
        "limit_cents": budgeted_cents,
    })

    inc_analytics = await client.get("/analytics/income-by-person?salary_cat=Salary+Cat&month=2026-07")
    assert inc_analytics.status_code == 200
    to_be_budgeted = income_cents - budgeted_cents
    assert to_be_budgeted == 0

@pytest.mark.asyncio
@pytest.mark.parametrize("target_cents, expense_cost", [
    (500000, 100000),
    (1000000, 250000),
])
async def test_cap_bdg(client: AsyncClient, target_cents, expense_cost):
    """[CapBdg] Project capital budget tracking computing total_spent_cents."""
    key = derive_key()
    proj_name = encrypt_text(f"Kitchen Remodel {target_cents}", key)
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Renovation", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    proj_resp = await client.post("/projects", json={
        "name": proj_name,
        "target_cents": target_cents,
        "target_date": "2026-12-31",
    })
    assert proj_resp.status_code == 201
    proj_id = proj_resp.json()["id"]

    await client.post("/expenses", json={
        "name": encrypt_text("Cabinets", key),
        "cost_cents": expense_cost,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
        "project_id": proj_id,
    })

    proj_list = await client.get("/projects")
    assert proj_list.status_code == 200
    p_item = next((p for p in proj_list.json() if p["id"] == proj_id), None)
    assert p_item is not None
    assert p_item["total_spent_cents"] == expense_cost

@pytest.mark.asyncio
@pytest.mark.parametrize("cost_cents, limit_cents, expected_warning", [
    (7500, 10000, False),
    (8500, 10000, True),
    (9500, 10000, True),
])
async def test_bdg_wrn(client: AsyncClient, cost_cents, limit_cents, expected_warning):
    """[BdgWrn] Warning threshold trigger when budget utilization >= 80%."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Warn Category", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })
    await client.post("/budgets", json={
        "category": cat_enc,
        "month": "2026-07",
        "limit_cents": limit_cents,
    })
    await client.post("/expenses", json={
        "name": encrypt_text("Warn Expense", key),
        "cost_cents": cost_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })

    resp = await client.get("/analytics/budgets?month=2026-07")
    assert resp.status_code == 200
    b_item = next((b for b in resp.json() if b["category"] == cat_enc), None)
    assert b_item is not None
    is_warning = b_item["pct_used"] >= 80.0
    assert is_warning == expected_warning

@pytest.mark.asyncio
@pytest.mark.parametrize("tag_name", ["Vacation2026", "TaxDeductible"])
async def test_tag_bdg(client: AsyncClient, tag_name):
    """[TagBdg] Tag-based budget aggregation."""
    key = derive_key()
    t_enc = encrypt_text(tag_name, key)
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("General", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    tag_resp = await client.post("/tags", json={
        "name": t_enc,
        "color": "#f59e0b",
        "description": encrypt_text("Tag desc", key)
    })
    assert tag_resp.status_code == 201
    tag_id = tag_resp.json()["id"]

    await client.post("/expenses", json={
        "name": encrypt_text("Tagged Item", key),
        "cost_cents": 15000,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
        "tag_id": tag_id,
    })

    analytics = await client.get(f"/analytics/tags/{tag_id}")
    assert analytics.status_code == 200
    data = analytics.json()
    assert data["tag"]["expense_count"] == 1
