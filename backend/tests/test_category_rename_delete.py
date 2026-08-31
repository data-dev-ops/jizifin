import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text, decrypt_text


@pytest.mark.asyncio
async def test_expense_category_rename_cascades_to_expenses_and_allocations(client: AsyncClient):
    """Renaming an expense category updates splits, split_allocations, and expenses."""
    key = derive_key()
    user_john = encrypt_text("John", key)
    user_jane = encrypt_text("Jane", key)

    # Create users
    await client.post("/users", json={"name": user_john, "color": "#6366f1"})
    await client.post("/users", json={"name": user_jane, "color": "#ec4899"})

    # Create category 'RENT'
    cat_rent = encrypt_text("RENT", key)
    allocations = [
        {"user_name": user_john, "pct": 60.0},
        {"user_name": user_jane, "pct": 40.0},
    ]
    resp = await client.post("/splits", json={"category": cat_rent, "allocations": allocations})
    assert resp.status_code == 201

    # Create an expense under 'RENT'
    exp_name = encrypt_text("Monthly Apartment Rent", key)
    exp_resp = await client.post(
        "/expenses",
        json={
            "name": exp_name,
            "cost_cents": 150000,
            "expense_date": "2026-08-01",
            "who_paid": user_john,
            "category": cat_rent,
        },
    )
    assert exp_resp.status_code == 201
    exp_id = exp_resp.json()["id"]

    # Rename 'RENT' to 'HOUSING & RENT'
    cat_housing = encrypt_text("HOUSING & RENT", key)
    rename_resp = await client.put(
        f"/splits/{cat_rent}",
        json={"category": cat_housing},
    )
    assert rename_resp.status_code == 200
    assert rename_resp.json()["category"] == cat_housing
    assert len(rename_resp.json()["allocations"]) == 2

    # Verify expense now reflects new category
    exp_get = await client.get("/expenses")
    assert exp_get.status_code == 200
    matching = [e for e in exp_get.json() if e["id"] == exp_id]
    assert len(matching) == 1
    assert matching[0]["category"] == cat_housing

    # Verify old category is gone from list
    splits_list = await client.get("/splits")
    assert splits_list.status_code == 200
    cats = [s["category"] for s in splits_list.json()]
    assert cat_housing in cats
    assert cat_rent not in cats


@pytest.mark.asyncio
async def test_expense_category_rename_duplicate_conflict(client: AsyncClient):
    """Attempting to rename to an already existing category raises 409."""
    key = derive_key()
    cat_a = encrypt_text("CAT_A", key)
    cat_b = encrypt_text("CAT_B", key)

    await client.post("/splits", json={"category": cat_a, "allocations": []})
    await client.post("/splits", json={"category": cat_b, "allocations": []})

    # Rename CAT_A to CAT_B -> Conflict
    resp = await client.put(f"/splits/{cat_a}", json={"category": cat_b})
    assert resp.status_code == 409


@pytest.mark.asyncio
async def test_expense_category_delete_unused_success(client: AsyncClient):
    """Deleting an unused expense category succeeds with 204."""
    key = derive_key()
    cat_temp = encrypt_text("TEMP_CATEGORY", key)
    create_resp = await client.post("/splits", json={"category": cat_temp, "allocations": []})
    assert create_resp.status_code == 201

    del_resp = await client.delete(f"/splits/{cat_temp}")
    assert del_resp.status_code == 204

    # Verify 404 on subsequent delete
    del_again = await client.delete(f"/splits/{cat_temp}")
    assert del_again.status_code == 404


@pytest.mark.asyncio
async def test_expense_category_delete_blocked_when_in_use(client: AsyncClient):
    """Deleting an expense category with existing expenses is blocked with 409."""
    key = derive_key()
    user = encrypt_text("Alice", key)
    await client.post("/users", json={"name": user, "color": "#10b981"})

    cat = encrypt_text("GROCERIES_IN_USE", key)
    await client.post("/splits", json={"category": cat, "allocations": []})

    exp = await client.post(
        "/expenses",
        json={
            "name": encrypt_text("Supermarket run", key),
            "cost_cents": 5000,
            "expense_date": "2026-08-15",
            "who_paid": user,
            "category": cat,
        },
    )
    assert exp.status_code == 201

    # Attempt delete -> 409
    del_resp = await client.delete(f"/splits/{cat}")
    assert del_resp.status_code == 409
    assert "referenced by existing transactions" in del_resp.json()["detail"]


@pytest.mark.asyncio
async def test_income_category_rename_cascades_to_income(client: AsyncClient):
    """Renaming an income category updates registry and existing income entries."""
    key = derive_key()
    user = encrypt_text("Bob", key)
    await client.post("/users", json={"name": user, "color": "#f59e0b"})

    old_cat = encrypt_text("FREELANCE", key)
    await client.post("/income-categories", json={"category": old_cat})

    # Log income under FREELANCE
    inc_resp = await client.post(
        "/income",
        json=[{
            "name": encrypt_text("Website Design", key),
            "amount_cents": 120000,
            "who": user,
            "category": old_cat,
            "income_date": "2026-08-10",
        }],
    )
    assert inc_resp.status_code == 201
    inc_id = inc_resp.json()[0]["id"]

    # Rename FREELANCE to CONTRACT WORK
    new_cat = encrypt_text("CONTRACT WORK", key)
    rename_resp = await client.put(f"/income-categories/{old_cat}", json={"category": new_cat})
    assert rename_resp.status_code == 200
    assert rename_resp.json()["category"] == new_cat

    # Check income entry is updated
    inc_list = await client.get("/income")
    assert inc_list.status_code == 200
    matching = [i for i in inc_list.json() if i["id"] == inc_id]
    assert len(matching) == 1
    assert matching[0]["category"] == new_cat

    # Check registry list
    list_resp = await client.get("/income-categories")
    cats = [c["category"] for c in list_resp.json()]
    assert new_cat in cats
    assert old_cat not in cats
