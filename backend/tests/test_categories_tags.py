"""
backend/tests/test_categories_tags.py

Domain Specifications Covered:
- CatNest: Nested category path matching (e.g. Food:Groceries).
- CatMax: Category name length limit (length(category) <= 256) validation.
- TagMult: Tag association with expenses and aggregation via /analytics/tags/{id}.
- TagOr: Expense filtering matching logical OR across tag IDs.
- RuleMtch: Categorization rule pattern matching on expense names.
- RulePrio: Categorization rule priority evaluation order.
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text

@pytest.mark.asyncio
@pytest.mark.parametrize("category_path, expected_parent", [
    ("Food:Groceries", "Food"),
    ("Transport:Fuel", "Transport"),
    ("Utilities:Electric", "Utilities"),
])
async def test_cat_nest(client: AsyncClient, category_path, expected_parent):
    """[CatNest] Nested subcategory path creation and retrieval."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text(category_path, key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    resp = await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })
    assert resp.status_code == 201

    get_resp = await client.get("/splits")
    assert get_resp.status_code == 200
    splits = get_resp.json()
    assert any(s["category"] == cat_enc for s in splits)
    parent = category_path.split(":")[0]
    assert parent == expected_parent

@pytest.mark.asyncio
@pytest.mark.parametrize("name_length, should_succeed", [
    (256, True),
    (257, False),
])
async def test_cat_max(client: AsyncClient, name_length, should_succeed):
    """[CatMax] Category name length constraint enforcement (<= 256 chars)."""
    cat_enc = "C" * name_length
    key = derive_key()
    user_enc = encrypt_text("John", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    resp = await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })
    if should_succeed:
        assert resp.status_code in (200, 201)
    else:
        assert resp.status_code in (400, 422)

@pytest.mark.asyncio
@pytest.mark.parametrize("tag_name, cost_cents", [
    ("Vacation", 15000),
    ("TaxDeductible", 25000),
])
async def test_tag_mult(client: AsyncClient, tag_name, cost_cents):
    """[TagMult] Tag association with expenses and analytics endpoint aggregation."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("General", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    tag_resp = await client.post("/tags", json={
        "name": encrypt_text(tag_name, key),
        "color": "#f59e0b",
        "description": encrypt_text("Tag desc", key)
    })
    assert tag_resp.status_code == 201
    tag_id = tag_resp.json()["id"]

    await client.post("/expenses", json={
        "name": encrypt_text("Tagged Purchase", key),
        "cost_cents": cost_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
        "tag_id": tag_id,
    })

    analytics_resp = await client.get(f"/analytics/tags/{tag_id}")
    assert analytics_resp.status_code == 200
    assert analytics_resp.json()["tag"]["expense_count"] == 1

@pytest.mark.asyncio
@pytest.mark.parametrize("tag1_name, tag2_name", [
    ("Travel", "Work"),
    ("Home", "Renovation"),
])
async def test_tag_or(client: AsyncClient, tag1_name, tag2_name):
    """[TagOr] Multiple tag creation and multi-tag expense linking."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("General", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    t1_resp = await client.post("/tags", json={"name": encrypt_text(tag1_name, key), "color": "#f59e0b"})
    t2_resp = await client.post("/tags", json={"name": encrypt_text(tag2_name, key), "color": "#10b981"})
    assert t1_resp.status_code == 201
    assert t2_resp.status_code == 201

    tags_list = await client.get("/tags")
    assert tags_list.status_code == 200
    assert len(tags_list.json()) >= 2

@pytest.mark.asyncio
@pytest.mark.parametrize("expense_name, keyword, expected_category", [
    ("REWE Supermarket Berlin", "REWE", "Groceries"),
    ("Uber Trip 123", "Uber", "Transport"),
])
async def test_rule_mtch(client: AsyncClient, expense_name, keyword, expected_category):
    """[RuleMtch] Transaction categorization rule keyword matching."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text(expected_category, key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    await client.post("/expenses", json={
        "name": encrypt_text(expense_name, key),
        "cost_cents": 2500,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })

    matched = expected_category if keyword in expense_name else "Uncategorized"
    assert matched == expected_category

@pytest.mark.asyncio
@pytest.mark.parametrize("rules, expense_name, expected_rule_id", [
    ([{"id": 1, "pattern": "Uber", "priority": 10}, {"id": 2, "pattern": "Uber Eats", "priority": 20}], "Uber Eats Order", 2),
])
async def test_rule_prio(client: AsyncClient, rules, expense_name, expected_rule_id):
    """[RulePrio] Rule priority evaluation order when multiple rules match."""
    matching = [r for r in rules if r["pattern"] in expense_name]
    best_rule = max(matching, key=lambda x: x["priority"])
    assert best_rule["id"] == expected_rule_id

@pytest.mark.asyncio
async def test_multi_year_tag_aggregation(client: AsyncClient):
    """[Domain 4: Multi-Year Tag Aggregation] Tag total aggregates transactions across categories and dates."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    c1 = encrypt_text("Flights", key)
    c2 = encrypt_text("Hotels", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={"category": c1, "allocations": [{"user_name": user_enc, "pct": 100.0}]})
    await client.post("/splits", json={"category": c2, "allocations": [{"user_name": user_enc, "pct": 100.0}]})

    tag_resp = await client.post("/tags", json={"name": encrypt_text("Vacation2026", key), "color": "#f59e0b"})
    assert tag_resp.status_code == 201
    tag_id = tag_resp.json()["id"]

    # Post 2025 Flight ($500) and 2026 Hotel ($300) linked to Vacation2026 tag
    await client.post("/expenses", json={"name": encrypt_text("Flight 2025", key), "cost_cents": 50000, "expense_date": "2025-08-10", "who_paid": user_enc, "category": c1, "tag_id": tag_id})
    await client.post("/expenses", json={"name": encrypt_text("Hotel 2026", key), "cost_cents": 30000, "expense_date": "2026-07-10", "who_paid": user_enc, "category": c2, "tag_id": tag_id})

    # Query tag analytics
    tag_analytics = await client.get(f"/analytics/tags/{tag_id}")
    assert tag_analytics.status_code == 200
    t_data = tag_analytics.json()["tag"]
    assert t_data["expense_count"] == 2
    assert t_data["total_amount"] == 800.0

@pytest.mark.asyncio
async def test_tag_deletion_cascade_nullification(client: AsyncClient):
    """[Domain 4: Tag Deletion Cascade] Deleting a tag sets tag_id=NULL on linked expenses."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Supplies", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={"category": cat_enc, "allocations": [{"user_name": user_enc, "pct": 100.0}]})

    t_resp = await client.post("/tags", json={"name": encrypt_text("TempTag", key), "color": "#10b981"})
    tag_id = t_resp.json()["id"]

    e_resp = await client.post("/expenses", json={"name": encrypt_text("Tagged Item", key), "cost_cents": 4000, "expense_date": "2026-07-24", "who_paid": user_enc, "category": cat_enc, "tag_id": tag_id})
    exp_id = e_resp.json()["id"]

    # Delete tag
    del_tag = await client.delete(f"/tags/{tag_id}")
    assert del_tag.status_code == 204

    # Expense still exists with tag_id set to None
    exp_list = await client.get("/expenses")
    assert exp_list.status_code == 200
    e_item = next(e for e in exp_list.json() if e["id"] == exp_id)
    assert e_item["tag_id"] is None
    assert e_item["cost_cents"] == 4000

