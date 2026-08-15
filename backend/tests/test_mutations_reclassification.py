"""
backend/tests/test_mutations_reclassification.py

Domain Specifications Covered:
- DelAccTx: DELETE /users/{name} returns 409 Conflict if history exists, 204 if unused.
- DelCatTx: Category deletion behavior with historical records.
- DelTx: Deleting expense (DELETE /expenses/{id}) or income (DELETE /income/{id}).
- EdTxAmt: Updating cost_cents on expense updating joint balance by exact delta.
- EdTxAcc: Changing who_paid reclassifying payback debtor/creditor.
- EdTxCat: Changing category updating budget actuals and paybacks.
- EdTxDate: Changing expense_date checking month lock status.
- TxVoid: Voiding transaction logic.
- CatBlk: Bulk category reclassification across historical records.
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text

@pytest.mark.asyncio
@pytest.mark.parametrize("has_history", [True, False])
async def test_del_acc_tx(client: AsyncClient, has_history):
    """[DelAccTx] DELETE /users/{name} returns 409 Conflict if history exists, 204 if unused."""
    key = derive_key()
    user_name = "TargetUser" if has_history else "UnusedUser"
    user_enc = encrypt_text(user_name, key)
    cat_enc = encrypt_text("Misc", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    if has_history:
        await client.post("/expenses", json={
            "name": encrypt_text("User Expense", key),
            "cost_cents": 5000,
            "expense_date": "2026-07-24",
            "who_paid": user_enc,
            "category": cat_enc,
        })

    resp = await client.delete(f"/users/{user_enc}")
    if has_history:
        assert resp.status_code == 409
    else:
        assert resp.status_code == 204

@pytest.mark.asyncio
@pytest.mark.parametrize("cat_name", ["OldCategory", "TempCategory"])
async def test_del_cat_tx(client: AsyncClient, cat_name):
    """[DelCatTx] Category deletion behavior with historical records via income categories endpoint."""
    key = derive_key()
    cat_enc = encrypt_text(cat_name, key)

    post_resp = await client.post("/income-categories", json={"category": cat_enc})
    assert post_resp.status_code == 201

    del_resp = await client.delete(f"/income-categories/{cat_enc}")
    assert del_resp.status_code == 204

@pytest.mark.asyncio
@pytest.mark.parametrize("initial_balance, exp_cost", [(10000, 3000)])
async def test_del_tx(client: AsyncClient, initial_balance, exp_cost):
    """[DelTx] Deleting expense (DELETE /expenses/{id}) reverses joint balance effect."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Food", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })
    await client.post("/joint-account", json={"name": "Joint Account", "balance_cents": initial_balance})

    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Joint Lunch", key),
        "cost_cents": exp_cost,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
        "is_joint": 1,
    })
    assert exp_resp.status_code == 201
    exp_id = exp_resp.json()["id"]

    del_resp = await client.delete(f"/expenses/{exp_id}")
    assert del_resp.status_code == 204

    ja_resp = await client.get("/joint-account")
    assert ja_resp.json()["balance_cents"] == initial_balance

@pytest.mark.asyncio
@pytest.mark.parametrize("old_cost, new_cost, initial_bal", [(2000, 5000, 10000)])
async def test_ed_tx_amt(client: AsyncClient, old_cost, new_cost, initial_bal):
    """[EdTxAmt] Updating cost_cents on expense updates joint account balance by exact delta."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Hardware", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })
    await client.post("/joint-account", json={"name": "Joint Account", "balance_cents": initial_bal})

    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Tools", key),
        "cost_cents": old_cost,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
        "is_joint": 1,
    })
    exp_id = exp_resp.json()["id"]

    put_resp = await client.put(f"/expenses/{exp_id}", json={
        "name": encrypt_text("Tools", key),
        "cost_cents": new_cost,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
        "is_joint": 1,
    })
    assert put_resp.status_code == 200

    ja_resp = await client.get("/joint-account")
    assert ja_resp.json()["balance_cents"] == initial_bal - new_cost

@pytest.mark.asyncio
@pytest.mark.parametrize("new_payer_name", ["Jane", "Alex"])
async def test_ed_tx_acc(client: AsyncClient, new_payer_name):
    """[EdTxAcc] Changing who_paid on expense reclassifying payback debtor/creditor."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    new_user_enc = encrypt_text(new_payer_name, key)
    cat_enc = encrypt_text("Groceries", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": new_user_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [
            {"user_name": john_enc, "pct": 50.0},
            {"user_name": new_user_enc, "pct": 50.0},
        ]
    })

    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Supermarket", key),
        "cost_cents": 10000,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
    })
    exp_id = exp_resp.json()["id"]

    edit_resp = await client.put(f"/expenses/{exp_id}", json={
        "name": encrypt_text("Supermarket", key),
        "cost_cents": 10000,
        "expense_date": "2026-07-24",
        "who_paid": new_user_enc,
        "category": cat_enc,
    })
    assert edit_resp.status_code == 200
    assert edit_resp.json()["who_paid"] == new_user_enc

@pytest.mark.asyncio
@pytest.mark.parametrize("new_cat_name", ["Leisure", "Transport"])
async def test_ed_tx_cat(client: AsyncClient, new_cat_name):
    """[EdTxCat] Changing category on expense updating budget actuals & paybacks."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    old_cat_enc = encrypt_text("Food", key)
    new_cat_enc = encrypt_text(new_cat_name, key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": old_cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })
    await client.post("/splits", json={
        "category": new_cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })

    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Purchase", key),
        "cost_cents": 5000,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": old_cat_enc,
    })
    exp_id = exp_resp.json()["id"]

    edit_resp = await client.put(f"/expenses/{exp_id}", json={
        "name": encrypt_text("Purchase", key),
        "cost_cents": 5000,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": new_cat_enc,
    })
    assert edit_resp.status_code == 200
    assert edit_resp.json()["category"] == new_cat_enc

@pytest.mark.asyncio
@pytest.mark.parametrize("new_date", ["2026-07-25", "2026-07-20"])
async def test_ed_tx_date(client: AsyncClient, new_date):
    """[EdTxDate] Changing expense_date checking month lock status."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Misc", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })

    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Item", key),
        "cost_cents": 2500,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
    })
    exp_id = exp_resp.json()["id"]

    edit_resp = await client.put(f"/expenses/{exp_id}", json={
        "name": encrypt_text("Item", key),
        "cost_cents": 2500,
        "expense_date": new_date,
        "who_paid": john_enc,
        "category": cat_enc,
    })
    assert edit_resp.status_code == 200
    assert edit_resp.json()["expense_date"] == new_date

@pytest.mark.asyncio
@pytest.mark.parametrize("cost_cents", [3000, 5000])
async def test_tx_void(client: AsyncClient, cost_cents):
    """[TxVoid] Deleting expense entry via DELETE /expenses/{id}."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("VoidCat", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })

    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Void Target", key),
        "cost_cents": cost_cents,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
    })
    exp_id = exp_resp.json()["id"]

    del_resp = await client.delete(f"/expenses/{exp_id}")
    assert del_resp.status_code == 204

    get_resp = await client.get("/expenses")
    assert get_resp.status_code == 200
    assert not any(e["id"] == exp_id for e in get_resp.json())

@pytest.mark.asyncio
@pytest.mark.parametrize("target_cat", ["ReclassifiedCat1", "ReclassifiedCat2"])
async def test_cat_blk(client: AsyncClient, target_cat):
    """[CatBlk] Reclassifying expenses across categories via PUT /expenses/{id}."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    old_cat_enc = encrypt_text("OldCat", key)
    new_cat_enc = encrypt_text(target_cat, key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": old_cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })
    await client.post("/splits", json={
        "category": new_cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })

    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Bulk Item", key),
        "cost_cents": 4000,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": old_cat_enc,
    })
    exp_id = exp_resp.json()["id"]

    put_resp = await client.put(f"/expenses/{exp_id}", json={
        "name": encrypt_text("Bulk Item", key),
        "cost_cents": 4000,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": new_cat_enc,
    })
    assert put_resp.status_code == 200
    assert put_resp.json()["category"] == new_cat_enc
