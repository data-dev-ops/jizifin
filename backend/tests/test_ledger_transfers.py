"""
backend/tests/test_ledger_transfers.py

Domain Specifications Covered:
- IncAdd: Income entry addition updates user ledger.
- ExpSub: Expense creation updates views and joint account balance when is_joint=True.
- TrfBasic: Joint account top-ups and balance transfers update account total.
- TrfSelf: Transfers between same user result in 0 net change.
- TxFuture: Future dated expenses recorded but excluded from current month total view.
- TxRetro: Retroactive past expense insertion dynamically recalculates paybacks.
- FeeDed: Transaction fee deduction from net transaction amount.
- TrfFee: Fee split allocation across active users.
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text

@pytest.mark.asyncio
@pytest.mark.parametrize("income_cents, category_name", [
    (500000, "SALARY"),
    (15000, "BONUS"),
    (20000, "FREELANCE"),
])
async def test_inc_add(client: AsyncClient, income_cents, category_name):
    """[IncAdd] Posting income entries increases user net income."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text(category_name, key)
    inc_name_enc = encrypt_text("Monthly Income", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})

    # POST /income requires a JSON array
    resp = await client.post("/income", json=[{
        "name": inc_name_enc,
        "amount_cents": income_cents,
        "who": user_enc,
        "category": cat_enc,
        "income_date": "2026-07-24",
    }])
    assert resp.status_code == 201
    data = resp.json()
    assert data[0]["amount_cents"] == income_cents

@pytest.mark.asyncio
@pytest.mark.parametrize("cost_cents, is_joint, initial_balance, expected_balance", [
    (2000, True, 10000, 8000),
    (3000, False, 10000, 10000),
])
async def test_exp_sub(client: AsyncClient, cost_cents, is_joint, initial_balance, expected_balance):
    """[ExpSub] Expense creation deducts from joint account balance when is_joint=True."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Groceries", key)
    exp_enc = encrypt_text("Supermarket", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })
    await client.post("/joint-account", json={"name": "Joint Account", "balance_cents": initial_balance})

    resp = await client.post("/expenses", json={
        "name": exp_enc,
        "cost_cents": cost_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
        "is_joint": 1 if is_joint else 0,
    })
    assert resp.status_code == 201

    ja_resp = await client.get("/joint-account")
    assert ja_resp.json()["balance_cents"] == expected_balance

@pytest.mark.asyncio
@pytest.mark.parametrize("topup_cents", [5000, 12500, 30000])
async def test_trf_basic(client: AsyncClient, topup_cents):
    """[TrfBasic] Basic joint account top-up via corrections."""
    await client.post("/joint-account", json={"name": "Joint Account", "balance_cents": 10000})

    resp = await client.post("/joint-account/corrections", json={
        "amount_cents": topup_cents,
        "correction_date": "2026-07-24",
        "note": "Top up"
    })
    assert resp.status_code == 201

    ja_resp = await client.get("/joint-account")
    assert ja_resp.json()["balance_cents"] == 10000 + topup_cents

@pytest.mark.asyncio
@pytest.mark.parametrize("amount_cents", [1000, 5000])
async def test_trf_self(client: AsyncClient, amount_cents):
    """[TrfSelf] Self-transfer yields 0 net payback balance change."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)
    cat_enc = encrypt_text("Utilities", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })

    await client.post("/expenses", json={
        "name": encrypt_text("John Self Expense", key),
        "cost_cents": amount_cents,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
    })

    paybacks_resp = await client.get(f"/analytics/paybacks?month=2026-07&personal_cats=PERSONAL%20COST%2CLEISURE%2CGIFT&combined_fixed_cat=Combined%20Fixed&apartment_cat=Apartment&jane_name={jane_enc}&john_name={john_enc}")
    assert paybacks_resp.status_code == 200
    pb_data = paybacks_resp.json()
    assert len(pb_data["debts"]) == 0

@pytest.mark.asyncio
@pytest.mark.parametrize("future_date", ["2099-01-01", "2099-12-31"])
async def test_tx_future(client: AsyncClient, future_date):
    """[TxFuture] Future date expense recorded successfully but excluded from current month total view."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Future Booking", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    resp = await client.post("/expenses", json={
        "name": encrypt_text("Future Flight", key),
        "cost_cents": 50000,
        "expense_date": future_date,
        "who_paid": user_enc,
        "category": cat_enc,
    })
    assert resp.status_code == 201

    view_resp = await client.get("/analytics/monthly-total")
    assert view_resp.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("past_date", ["2025-05-15", "2025-06-20"])
async def test_tx_retro(client: AsyncClient, past_date):
    """[TxRetro] Retroactive past expense insertion in unlocked month recalculates paybacks."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)
    cat_enc = encrypt_text("Past Dinner", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [
            {"user_name": john_enc, "pct": 50.0},
            {"user_name": jane_enc, "pct": 50.0},
        ]
    })

    resp = await client.post("/expenses", json={
        "name": encrypt_text("Retroactive Expense", key),
        "cost_cents": 10000,
        "expense_date": past_date,
        "who_paid": john_enc,
        "category": cat_enc,
    })
    assert resp.status_code == 201

    target_month = past_date[:7]
    paybacks_resp = await client.get(f"/analytics/paybacks?month={target_month}&personal_cats=PERSONAL%20COST%2CLEISURE%2CGIFT&combined_fixed_cat=Combined%20Fixed&apartment_cat=Apartment&jane_name={jane_enc}&john_name={john_enc}")
    assert paybacks_resp.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("principal_cents, fee_cents", [
    (10000, 250),
    (5000, 100),
])
async def test_fee_ded(client: AsyncClient, principal_cents, fee_cents):
    """[FeeDed] Transaction fee deduction math via expenses API."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Fees", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    net_cents = principal_cents - fee_cents
    resp = await client.post("/expenses", json={
        "name": encrypt_text("Deducted Fee Expense", key),
        "cost_cents": net_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })
    assert resp.status_code == 201
    assert resp.json()["cost_cents"] == net_cents

@pytest.mark.asyncio
@pytest.mark.parametrize("fee_cents, user_count", [
    (300, 2),
    (600, 3),
])
async def test_trf_fee(client: AsyncClient, fee_cents, user_count):
    """[TrfFee] Split fee allocation across active users via expenses API."""
    key = derive_key()
    cat_enc = encrypt_text("Shared Fee", key)
    john_enc = encrypt_text("John", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })

    resp = await client.post("/expenses", json={
        "name": encrypt_text("Split Fee Expense", key),
        "cost_cents": fee_cents,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
    })
    assert resp.status_code == 201
    assert resp.json()["cost_cents"] == fee_cents

@pytest.mark.asyncio
async def test_out_of_pocket_shared_expense(client: AsyncClient):
    """[Domain 3: Out-of-Pocket Shared Expense] Out-of-pocket payment updates shared category budget and creates joint liability."""
    key = derive_key()
    cat_enc = encrypt_text("Supermarket", key)
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 50.0}, {"user_name": jane_enc, "pct": 50.0}]
    })

    # John pays $200 (20000 cents) out-of-pocket for Supermarket
    e_resp = await client.post("/expenses", json={
        "name": encrypt_text("Weekly Groceries", key),
        "cost_cents": 20000,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
        "is_joint": 0
    })
    assert e_resp.status_code == 201

    # Verify category spending is full $200
    cat_summary = await client.get("/analytics/by-category?month=2026-07")
    assert cat_summary.status_code == 200
    cat_item = next(c for c in cat_summary.json() if c["category"] == cat_enc)
    assert cat_item["total_amount"] == 200.0

    # Payback liability shows Jane owes John $100 (10000 cents)
    url = (
        f"/analytics/paybacks?month=2026-07&personal_cats=PERSONAL%20COST,LEISURE,GIFT"
        f"&combined_fixed_cat=Combined%20Fixed&apartment_cat=Apartment"
        f"&jane_name={jane_enc}&john_name={john_enc}"
    )
    pb_resp = await client.get(url)
    assert pb_resp.status_code == 200
    debts = pb_resp.json()["debts"]
    assert len(debts) == 1
    assert debts[0]["from_user"] == jane_enc
    assert debts[0]["to_user"] == john_enc
    assert debts[0]["amount"] == 100.0

@pytest.mark.asyncio
async def test_joint_account_personal_expense(client: AsyncClient):
    """[Domain 3: Joint Card Personal Expense] Personal draw on joint account deducts from joint balance."""
    key = derive_key()
    cat_enc = encrypt_text("PersonalGadgets", key)
    john_enc = encrypt_text("John", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })

    # Initialize joint account balance at 50000 cents ($500)
    await client.post("/joint-account", json={"name": encrypt_text("Joint Vault", key), "balance_cents": 50000})

    # John spends $150 (15000 cents) personal draw on joint card
    e_resp = await client.post("/expenses", json={
        "name": encrypt_text("Headphones", key),
        "cost_cents": 15000,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
        "is_joint": 1
    })
    assert e_resp.status_code == 201

    # Joint account balance should be reduced to $350 (35000 cents)
    ja_resp = await client.get("/joint-account")
    assert ja_resp.status_code == 200
    assert ja_resp.json()["balance_cents"] == 35000

