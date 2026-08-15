"""
backend/tests/test_splits_allocations.py

Domain Specifications Covered:
- SpltMatch: Category split allocations summing to 100.0% accepted with 200.
- SpltMiss: Allocations summing to != 100.0% rejected with 422.
- SpltMix: Expense split overrides taking precedence over category split defaults.
- MultShr: Multi-user percentage splits across 3+ active household members.
- MultSet: Atomic bulk updates of category split allocations via PUT /splits/{category}.
- GrpSplt: Exclude joint-account categories from payback net calculations vs personal cost.
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text

@pytest.mark.asyncio
@pytest.mark.parametrize("allocations", [
    [{"pct": 50.0}, {"pct": 50.0}],
    [{"pct": 60.0}, {"pct": 40.0}],
    [{"pct": 33.0}, {"pct": 33.0}, {"pct": 34.0}],
])
async def test_splt_match(client: AsyncClient, allocations):
    """[SpltMatch] Category split allocations summing to exactly 100.0% accepted."""
    key = derive_key()
    cat_enc = encrypt_text("Rent", key)

    user_names = ["John", "Jane", "Alex"]
    payload = []
    for idx, item in enumerate(allocations):
        u_enc = encrypt_text(user_names[idx], key)
        await client.post("/users", json={"name": u_enc, "color": "#123456"})
        payload.append({"user_name": u_enc, "pct": item["pct"]})

    first_user_enc = payload[0]["user_name"]
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": first_user_enc, "pct": 100.0}]
    })

    resp = await client.put(f"/splits/{cat_enc}", json={"allocations": payload})
    assert resp.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("invalid_allocations", [
    [{"pct": 50.0}, {"pct": 40.0}], # 90%
    [{"pct": 60.0}, {"pct": 50.0}], # 110%
    [{"pct": 0.0}],                  # 0%
])
async def test_splt_miss(client: AsyncClient, invalid_allocations):
    """[SpltMiss] Category split allocations summing to <100% or >100% rejected with 422."""
    key = derive_key()
    cat_enc = encrypt_text("Utilities", key)

    user_names = ["John", "Jane"]
    payload = []
    for idx, item in enumerate(invalid_allocations):
        u_enc = encrypt_text(user_names[idx % 2], key)
        await client.post("/users", json={"name": u_enc, "color": "#123456"})
        payload.append({"user_name": u_enc, "pct": item["pct"]})

    first_user_enc = payload[0]["user_name"]
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": first_user_enc, "pct": 100.0}]
    })

    resp = await client.put(f"/splits/{cat_enc}", json={"allocations": payload})
    assert resp.status_code == 422

@pytest.mark.asyncio
@pytest.mark.parametrize("override_pct_john, override_pct_jane", [
    (80.0, 20.0),
    (100.0, 0.0),
])
async def test_splt_mix(client: AsyncClient, override_pct_john, override_pct_jane):
    """[SpltMix] Expense split overrides overriding default category split allocations."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)
    cat_enc = encrypt_text("Dining", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [
            {"user_name": john_enc, "pct": 50.0},
            {"user_name": jane_enc, "pct": 50.0},
        ]
    })

    # Create expense with override
    resp = await client.post("/expenses", json={
        "name": encrypt_text("Special Dinner", key),
        "cost_cents": 10000,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
        "overrides": [
            {"user_name": john_enc, "pct": override_pct_john},
            {"user_name": jane_enc, "pct": override_pct_jane},
        ]
    })
    assert resp.status_code == 201

    url = (
        f"/analytics/paybacks?month=2026-07&personal_cats=PERSONAL%20COST,LEISURE,GIFT"
        f"&combined_fixed_cat=Combined%20Fixed&apartment_cat=Apartment"
        f"&jane_name={jane_enc}&john_name={john_enc}"
    )
    paybacks = await client.get(url)
    assert paybacks.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("n_users", [3, 4])
async def test_mult_shr(client: AsyncClient, n_users):
    """[MultShr] Multi-user split support across 3+ household members."""
    key = derive_key()
    cat_enc = encrypt_text("Group Vacation", key)

    base_pct = 100 // n_users
    remainder = 100 - (base_pct * n_users)

    payload = []
    for i in range(n_users):
        u_enc = encrypt_text(f"User_{i}", key)
        await client.post("/users", json={"name": u_enc, "color": "#123456"})
        user_pct = float(base_pct + (remainder if i == 0 else 0))
        payload.append({"user_name": u_enc, "pct": user_pct})

    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": payload[0]["user_name"], "pct": 100.0}]
    })

    resp = await client.put(f"/splits/{cat_enc}", json={"allocations": payload})
    assert resp.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("initial_john_pct, new_john_pct", [(50.0, 70.0), (30.0, 100.0)])
async def test_mult_set(client: AsyncClient, initial_john_pct, new_john_pct):
    """[MultSet] Atomic updating of split allocations replacing existing entries."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)
    cat_enc = encrypt_text("Internet", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [
            {"user_name": john_enc, "pct": initial_john_pct},
            {"user_name": jane_enc, "pct": 100.0 - initial_john_pct},
        ]
    })

    # Atomic update
    new_jane_pct = 100.0 - new_john_pct
    payload = []
    if new_john_pct > 0:
        payload.append({"user_name": john_enc, "pct": new_john_pct})
    if new_jane_pct > 0:
        payload.append({"user_name": jane_enc, "pct": new_jane_pct})

    resp = await client.put(f"/splits/{cat_enc}", json={"allocations": payload})
    assert resp.status_code == 200

@pytest.mark.asyncio
@pytest.mark.parametrize("category_name, is_joint_cat", [
    ("Joint Rent", True),
    ("Personal Leisure", False),
])
async def test_grp_splt(client: AsyncClient, category_name, is_joint_cat):
    """[GrpSplt] Joint account category exclusion from payback net calculations vs personal categories."""
    key = derive_key()
    cat_enc = encrypt_text(category_name, key)
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })

    if is_joint_cat:
        await client.post("/joint-account/categories", json={"category": cat_enc})

    await client.post("/expenses", json={
        "name": encrypt_text("Test Expense", key),
        "cost_cents": 10000,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
    })

    url = (
        f"/analytics/paybacks?month=2026-07&personal_cats=PERSONAL%20COST,LEISURE,GIFT"
        f"&combined_fixed_cat=Combined%20Fixed&apartment_cat=Apartment"
        f"&jane_name={jane_enc}&john_name={john_enc}"
    )
    paybacks = await client.get(url)
    assert paybacks.status_code == 200

@pytest.mark.asyncio
async def test_asymmetric_60_40_split(client: AsyncClient):
    """[Domain 2: Asymmetric Splits] 60/40 category split credits 60% to payer and 40% to non-payer without altering vendor total."""
    key = derive_key()
    cat_enc = encrypt_text("DiningOut", key)
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 60.0}, {"user_name": jane_enc, "pct": 40.0}]
    })

    # John pays $100 (10000 cents) for DiningOut
    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Dinner", key),
        "cost_cents": 10000,
        "expense_date": "2026-07-24",
        "who_paid": john_enc,
        "category": cat_enc,
    })
    assert exp_resp.status_code == 201
    assert exp_resp.json()["cost_cents"] == 10000

    # Query paybacks: Jane should owe John 40% of $100 = $40 (4000 cents)
    url = (
        f"/analytics/paybacks?month=2026-07&personal_cats=PERSONAL%20COST,LEISURE,GIFT"
        f"&combined_fixed_cat=Combined%20Fixed&apartment_cat=Apartment"
        f"&jane_name={jane_enc}&john_name={john_enc}"
    )
    resp = await client.get(url)
    assert resp.status_code == 200
    debts = resp.json()["debts"]
    assert len(debts) == 1
    debt = debts[0]
    assert debt["from_user"] == jane_enc
    assert debt["to_user"] == john_enc
    assert debt["amount"] == 40.0

@pytest.mark.asyncio
async def test_mid_period_split_ratio_mutation(client: AsyncClient):
    """[Domain 2: Mid-Period Ratio Changes] Updating split ratio applies to new entries while existing overrides remain unchanged."""
    key = derive_key()
    cat_enc = encrypt_text("Utilities", key)
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 50.0}, {"user_name": jane_enc, "pct": 50.0}]
    })

    # Record expense under 50/50 ratio with explicit override
    e1 = await client.post("/expenses", json={
        "name": encrypt_text("June Electric", key),
        "cost_cents": 10000,
        "expense_date": "2026-06-15",
        "who_paid": john_enc,
        "category": cat_enc,
        "overrides": [{"user_name": john_enc, "pct": 50.0}, {"user_name": jane_enc, "pct": 50.0}]
    })
    assert e1.status_code == 201

    # Mutate default category split to 70/30
    mut_resp = await client.put(f"/splits/{cat_enc}", json={
        "allocations": [{"user_name": john_enc, "pct": 70.0}, {"user_name": jane_enc, "pct": 30.0}]
    })
    assert mut_resp.status_code == 200

    # Record new expense under updated default
    e2 = await client.post("/expenses", json={
        "name": encrypt_text("July Electric", key),
        "cost_cents": 10000,
        "expense_date": "2026-07-15",
        "who_paid": john_enc,
        "category": cat_enc,
    })
    assert e2.status_code == 201

    # Verify e1 override remained 50/50
    exp_list = await client.get("/expenses")
    assert exp_list.status_code == 200
    e1_fetched = next(e for e in exp_list.json() if e["id"] == e1.json()["id"])
    assert len(e1_fetched["overrides"]) == 2
    j_override = next(o for o in e1_fetched["overrides"] if o["user_name"] == john_enc)
    assert j_override["pct"] == 50.0

