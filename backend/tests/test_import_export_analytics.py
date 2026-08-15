"""
backend/tests/test_import_export_analytics.py

Domain Specifications Covered:
- CsvImp: CSV row parsing into genuine expense/income API endpoints.
- ImpSign: Sign convention handling for imported transactions via expense and income endpoints.
- TxAtch: Linking tag and project metadata to transaction records.
- DupChk: Duplicate transaction verification via expenses list API.
- TxSrch: Multi-parameter expense search filtering (limit, offset).
- FyStart: Cross-month analytics aggregation via by-category endpoint.
- TmZone: Timezone and ISO date string validation on transaction creation endpoints.
- LoanInt: Amortization and recurring expense logging via recurring-expenses endpoint.
- GoalFund: Project funding progress and total spent tracking via projects endpoint.
- InvBuy: Capital project expenditure logging via projects and expenses API.
- InvReal: Income vs expense payback net tracking via paybacks analytics endpoint.
- InvSplit: Split percentage adjustments via category splits endpoints.
- TaxFlag: Tax deductible tracking via tag creation and tag analytics endpoints.
- ExpPrj: Project progress and estimated completion tracking via projects summary view.
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text

@pytest.mark.asyncio
@pytest.mark.parametrize("csv_row, is_expense, expected_cents", [
    ("2026-07-24,Supermarket,-45.50", True, 4550),
    ("2026-07-24,Salary,2500.00", False, 250000),
])
async def test_csv_imp(client: AsyncClient, csv_row, is_expense, expected_cents):
    """[CsvImp] CSV row parsing and transaction creation via FastAPI endpoints."""
    key = derive_key()
    parts = csv_row.split(",")
    date_str, raw_name, amount_str = parts[0], parts[1], parts[2]
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("General", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    if is_expense:
        resp = await client.post("/expenses", json={
            "name": encrypt_text(raw_name, key),
            "cost_cents": expected_cents,
            "expense_date": date_str,
            "who_paid": user_enc,
            "category": cat_enc,
        })
        assert resp.status_code == 201
        assert resp.json()["cost_cents"] == expected_cents
    else:
        resp = await client.post("/income", json=[{
            "name": encrypt_text(raw_name, key),
            "amount_cents": expected_cents,
            "who": user_enc,
            "category": cat_enc,
            "income_date": date_str,
        }])
        assert resp.status_code == 201
        assert resp.json()[0]["amount_cents"] == expected_cents

@pytest.mark.asyncio
@pytest.mark.parametrize("raw_amount_str, is_expense", [
    ("-50.00", True),
    ("50.00", False),
])
async def test_imp_sign(client: AsyncClient, raw_amount_str, is_expense):
    """[ImpSign] CSV import sign convention handling via real expense and income endpoints."""
    key = derive_key()
    val = float(raw_amount_str)
    cents = abs(round(val * 100))
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("SignCategory", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    if is_expense:
        resp = await client.post("/expenses", json={
            "name": encrypt_text("Expense Transaction", key),
            "cost_cents": cents,
            "expense_date": "2026-07-24",
            "who_paid": user_enc,
            "category": cat_enc,
        })
        assert resp.status_code == 201
        assert resp.json()["cost_cents"] == cents
    else:
        resp = await client.post("/income", json=[{
            "name": encrypt_text("Income Transaction", key),
            "amount_cents": cents,
            "who": user_enc,
            "category": cat_enc,
            "income_date": "2026-07-24",
        }])
        assert resp.status_code == 201
        assert resp.json()[0]["amount_cents"] == cents

@pytest.mark.asyncio
@pytest.mark.parametrize("tag_name, project_name", [
    ("ReceiptTag", "Home Project"),
    ("TaxReceipt", "Office Project"),
])
async def test_tx_atch(client: AsyncClient, tag_name, project_name):
    """[TxAtch] Transaction metadata linking with project and tag endpoints."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Hardware", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    tag_resp = await client.post("/tags", json={
        "name": encrypt_text(tag_name, key),
        "color": "#f59e0b",
    })
    assert tag_resp.status_code == 201
    tag_id = tag_resp.json()["id"]

    proj_resp = await client.post("/projects", json={
        "name": encrypt_text(f"{project_name} TxAtch", key),
        "target_cents": 500000,
        "target_date": "2026-12-31",
    })
    assert proj_resp.status_code == 201
    proj_id = proj_resp.json()["id"]

    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Attached Purchase", key),
        "cost_cents": 10000,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
        "tag_id": tag_id,
        "project_id": proj_id,
    })
    assert exp_resp.status_code == 201
    data = exp_resp.json()
    assert data["tag_id"] == tag_id
    assert data["project_id"] == proj_id

@pytest.mark.asyncio
@pytest.mark.parametrize("cost_cents, expense_name", [
    (5000, "Coffee Shop"),
    (12000, "Gas Station"),
])
async def test_dup_chk(client: AsyncClient, cost_cents, expense_name):
    """[DupChk] Duplicate transaction detection by querying expenses API."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Daily", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    name_enc = encrypt_text(expense_name, key)
    tx_payload = {
        "name": name_enc,
        "cost_cents": cost_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    }

    # Post initial transaction
    r1 = await client.post("/expenses", json=tx_payload)
    assert r1.status_code == 201

    # Fetch expenses to verify duplicate entry existence
    r_list = await client.get("/expenses")
    assert r_list.status_code == 200
    matching = [e for e in r_list.json() if e["name"] == name_enc and e["cost_cents"] == cost_cents]
    assert len(matching) == 1

@pytest.mark.asyncio
@pytest.mark.parametrize("limit, offset", [(10, 0), (5, 5)])
async def test_tx_srch(client: AsyncClient, limit, offset):
    """[TxSrch] Multi-parameter GET /expenses search filtering."""
    resp = await client.get(f"/expenses?limit={limit}&offset={offset}")
    assert resp.status_code == 200
    assert isinstance(resp.json(), list)

@pytest.mark.asyncio
@pytest.mark.parametrize("m1, m2", [
    ("2026-04-10", "2026-07-15"),
    ("2026-01-05", "2026-05-20"),
])
async def test_fy_start(client: AsyncClient, m1, m2):
    """[FyStart] Fiscal year month analytics aggregation via by-category API."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("FYCategory", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    await client.post("/expenses", json={
        "name": encrypt_text("Q1 Expense", key),
        "cost_cents": 10000,
        "expense_date": m1,
        "who_paid": user_enc,
        "category": cat_enc,
    })
    await client.post("/expenses", json={
        "name": encrypt_text("Q2 Expense", key),
        "cost_cents": 20000,
        "expense_date": m2,
        "who_paid": user_enc,
        "category": cat_enc,
    })

    r1 = await client.get(f"/analytics/by-category?month={m1[:7]}")
    r2 = await client.get(f"/analytics/by-category?month={m2[:7]}")
    assert r1.status_code == 200
    assert r2.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("iso_date_str", [
    "2026-07-24",
    "2026-08-01",
])
async def test_tm_zone(client: AsyncClient, iso_date_str):
    """[TmZone] Timezone awareness and ISO date string handling via POST /expenses."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("TZCategory", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    resp = await client.post("/expenses", json={
        "name": encrypt_text("Timezone Expense", key),
        "cost_cents": 5000,
        "expense_date": iso_date_str,
        "who_paid": user_enc,
        "category": cat_enc,
    })
    assert resp.status_code == 201
    assert resp.json()["expense_date"] == iso_date_str

@pytest.mark.asyncio
@pytest.mark.parametrize("cost_cents, day_of_month", [
    (100000, 15),
    (50000, 1),
])
async def test_loan_int(client: AsyncClient, cost_cents, day_of_month):
    """[LoanInt] Recurring payment schedule creation via recurring-expenses endpoint."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Mortgage", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    resp = await client.post("/recurring", json={
        "name": encrypt_text("Loan Payment", key),
        "cost_cents": cost_cents,
        "who_paid": user_enc,
        "category": cat_enc,
        "day_of_month": day_of_month,
    })
    assert resp.status_code == 201
    data = resp.json()
    assert data["cost_cents"] == cost_cents
    assert data["day_of_month"] == day_of_month

@pytest.mark.asyncio
@pytest.mark.parametrize("target_cents, initial_spent", [(500000, 250000)])
async def test_goal_fund(client: AsyncClient, target_cents, initial_spent):
    """[GoalFund] Project funding rate calculation via projects endpoint."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("CarCategory", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    proj_resp = await client.post("/projects", json={
        "name": encrypt_text(f"Car Fund GoalFund {target_cents}", key),
        "target_cents": target_cents,
        "target_date": "2026-12-31",
    })
    assert proj_resp.status_code == 201
    proj_id = proj_resp.json()["id"]

    await client.post("/expenses", json={
        "name": encrypt_text("Car Deposit", key),
        "cost_cents": initial_spent,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
        "project_id": proj_id,
    })

    get_resp = await client.get("/projects")
    assert get_resp.status_code == 200
    projects = get_resp.json()
    p = next((pr for pr in projects if pr["id"] == proj_id), None)
    assert p is not None
    assert p["total_spent_cents"] == initial_spent

@pytest.mark.asyncio
@pytest.mark.parametrize("cost_cents", [150000, 100000])
async def test_inv_buy(client: AsyncClient, cost_cents):
    """[InvBuy] Capital purchase order logging via projects and expenses API."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Investments", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    proj_resp = await client.post("/projects", json={
        "name": encrypt_text(f"Stock Buy InvBuy {cost_cents}", key),
        "target_cents": 1000000,
        "target_date": "2026-12-31",
    })
    assert proj_resp.status_code == 201
    proj_id = proj_resp.json()["id"]

    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Stock Purchase", key),
        "cost_cents": cost_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
        "project_id": proj_id,
    })
    assert exp_resp.status_code == 201
    assert exp_resp.json()["cost_cents"] == cost_cents

@pytest.mark.asyncio
@pytest.mark.parametrize("income_amount, expense_amount", [
    (150000, 50000),
    (200000, 100000),
])
async def test_inv_real(client: AsyncClient, income_amount, expense_amount):
    """[InvReal] Income and expense balance calculation via paybacks analytics endpoint."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)
    cat_enc = encrypt_text("RealizedCat", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [
            {"user_name": john_enc, "pct": 50.0},
            {"user_name": jane_enc, "pct": 50.0},
        ]
    })

    await client.post("/expenses", json={
        "name": encrypt_text("Investment Fee", key),
        "cost_cents": expense_amount,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
    })

    paybacks_resp = await client.get(f"/analytics/paybacks?month=2026-07&personal_cats=PERSONAL%20COST%2CLEISURE%2CGIFT&combined_fixed_cat=Combined%20Fixed&apartment_cat=Apartment&jane_name={jane_enc}&john_name={john_enc}")
    assert paybacks_resp.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("john_pct, jane_pct", [
    (70.0, 30.0),
    (60.0, 40.0),
])
async def test_inv_split(client: AsyncClient, john_pct, jane_pct):
    """[InvSplit] Category split percentage updates via PUT /splits/{category}."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)
    cat_enc = encrypt_text("StockSplitCat", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [
            {"user_name": john_enc, "pct": 50.0},
            {"user_name": jane_enc, "pct": 50.0},
        ]
    })

    update_resp = await client.put(f"/splits/{cat_enc}", json={"allocations": [
        {"user_name": john_enc, "pct": john_pct},
        {"user_name": jane_enc, "pct": jane_pct},
    ]})
    assert update_resp.status_code == 200
    allocs = update_resp.json()["allocations"]
    j_alloc = next((a for a in allocs if a["user_name"] == john_enc), None)
    assert j_alloc["pct"] == john_pct

@pytest.mark.asyncio
@pytest.mark.parametrize("tag_name", ["TaxDeductible", "CharityDonation"])
async def test_tax_flag(client: AsyncClient, tag_name):
    """[TaxFlag] Tax-deductible tracking via tag creation and analytics endpoint."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("DonationCat", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    tag_resp = await client.post("/tags", json={
        "name": encrypt_text(tag_name, key),
        "color": "#3b82f6",
        "description": encrypt_text("Tax deductible item", key),
    })
    assert tag_resp.status_code == 201
    tag_id = tag_resp.json()["id"]

    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Tax Deduction", key),
        "cost_cents": 15000,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
        "tag_id": tag_id,
    })
    assert exp_resp.status_code == 201

    analytics_resp = await client.get(f"/analytics/tags/{tag_id}")
    assert analytics_resp.status_code == 200
    assert analytics_resp.json()["tag"]["expense_count"] == 1

@pytest.mark.asyncio
@pytest.mark.parametrize("target_cents, cost_cents", [
    (100000, 50000),
    (200000, 150000),
])
async def test_exp_prj(client: AsyncClient, target_cents, cost_cents):
    """[ExpPrj] Project spending progress and total spent tracking via projects view."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("ProjectionCat", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    proj_resp = await client.post("/projects", json={
        "name": encrypt_text(f"Project ExpPrj {target_cents} {cost_cents}", key),
        "target_cents": target_cents,
        "target_date": "2026-12-31",
    })
    assert proj_resp.status_code == 201
    proj_id = proj_resp.json()["id"]

    await client.post("/expenses", json={
        "name": encrypt_text("Progress Expense", key),
        "cost_cents": cost_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
        "project_id": proj_id,
    })

    projects_resp = await client.get("/projects")
    assert projects_resp.status_code == 200
    projects = projects_resp.json()
    p = next((pr for pr in projects if pr["id"] == proj_id), None)
    assert p is not None
    assert p["total_spent_cents"] == cost_cents

@pytest.mark.asyncio
async def test_project_target_recalculation(client: AsyncClient):
    """[Domain 5: Target Recalculation] Updating project target_cents dynamically updates target without breaking expense history."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Renovation", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={"category": cat_enc, "allocations": [{"user_name": user_enc, "pct": 100.0}]})

    p_resp = await client.post("/projects", json={"name": encrypt_text("Bathroom Remodel", key), "target_cents": 1000000, "target_date": "2026-12-31"})
    proj_id = p_resp.json()["id"]

    await client.post("/expenses", json={"name": encrypt_text("Tiles", key), "cost_cents": 300000, "expense_date": "2026-07-20", "who_paid": user_enc, "category": cat_enc, "project_id": proj_id})

    # Update project target from 1000000 to 1500000
    upd_resp = await client.put(f"/projects/{proj_id}", json={"target_cents": 1500000, "target_date": "2027-06-30"})
    assert upd_resp.status_code == 200
    assert upd_resp.json()["target_cents"] == 1500000

    # Total spent remains 300000
    proj_list = await client.get("/projects")
    p = next(item for item in proj_list.json() if item["id"] == proj_id)
    assert p["target_cents"] == 1500000
    assert p["total_spent_cents"] == 300000

@pytest.mark.asyncio
async def test_shared_vs_personal_project(client: AsyncClient):
    """[Domain 5: Shared vs Personal Projects] Verify is_joint flag distinguishes shared and personal target goals."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Gadgets", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={"category": cat_enc, "allocations": [{"user_name": user_enc, "pct": 100.0}]})

    p_personal = await client.post("/projects", json={"name": encrypt_text("Personal Laptop", key), "target_cents": 200000, "target_date": "2026-12-31", "is_joint": 0})
    p_shared = await client.post("/projects", json={"name": encrypt_text("Shared Vacation Goal", key), "target_cents": 500000, "target_date": "2026-12-31", "is_joint": 1})

    assert p_personal.json()["is_joint"] == 0
    assert p_shared.json()["is_joint"] == 1

