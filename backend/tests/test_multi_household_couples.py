"""
backend/tests/test_multi_household_couples.py — Comprehensive tests for multi-household multi-user setups.

Verifies:
  1. Two couples (Alice & Bob, Charlie & Dana) in a 4-person household.
  2. Project membership isolation and user exclusion.
  3. Multiple independent joint accounts with dedicated member subsets.
  4. Joint account deposits, expected costs, corrections, and settlements per account.
  5. Dashboard/Analytics filtering by user subsets (radio switcher scope).
  6. Database export and import cryptography integrity with multi-user project & multi-joint account schemas.
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text, decrypt_text


@pytest.mark.asyncio
async def test_two_couples_project_membership_isolation(client: AsyncClient):
    key = derive_key()

    # 1. Create 4 users: Couple 1 (Alice, Bob), Couple 2 (Charlie, Dana)
    for u in ["Alice", "Bob", "Charlie", "Dana"]:
        r = await client.post("/users", json={"name": encrypt_text(u, key), "color": "#6366f1", "is_active": 1})
        assert r.status_code == 201

    enc_alice = encrypt_text("Alice", key)
    enc_bob = encrypt_text("Bob", key)
    enc_charlie = encrypt_text("Charlie", key)
    enc_dana = encrypt_text("Dana", key)

    # 2. Couple 1 creates "Kitchen Remodel" with Alice & Bob
    r_p1 = await client.post("/projects", json={
        "name": encrypt_text("Kitchen Remodel", key),
        "target_cents": 500000,
        "target_date": "2026-12-31",
        "is_joint": False,
        "user_names": [enc_alice, enc_bob],
    })
    assert r_p1.status_code == 201
    p1 = r_p1.json()
    assert set(p1["user_names"]) == {enc_alice, enc_bob}

    # 3. Couple 2 creates "Trip to Japan" with Charlie & Dana
    r_p2 = await client.post("/projects", json={
        "name": encrypt_text("Trip to Japan", key),
        "target_cents": 350000,
        "target_date": "2026-10-15",
        "is_joint": False,
        "user_names": [enc_charlie, enc_dana],
    })
    assert r_p2.status_code == 201
    p2 = r_p2.json()
    assert set(p2["user_names"]) == {enc_charlie, enc_dana}

    # 4. List projects and verify user_names
    r_list = await client.get("/projects")
    assert r_list.status_code == 200
    projects = r_list.json()
    assert len(projects) == 2

    p1_found = next(p for p in projects if p["id"] == p1["id"])
    assert set(p1_found["user_names"]) == {enc_alice, enc_bob}

    p2_found = next(p for p in projects if p["id"] == p2["id"])
    assert set(p2_found["user_names"]) == {enc_charlie, enc_dana}

    # 5. Update project members
    r_up = await client.put(f"/projects/{p1['id']}", json={
        "user_names": [enc_alice],
    })
    assert r_up.status_code == 200
    assert r_up.json()["user_names"] == [enc_alice]


@pytest.mark.asyncio
async def test_multiple_joint_accounts_coexistence(client: AsyncClient):
    key = derive_key()

    # Create 4 users
    users = ["Alice", "Bob", "Charlie", "Dana"]
    for u in users:
        await client.post("/users", json={"name": encrypt_text(u, key), "color": "#6366f1", "is_active": 1})

    enc_alice = encrypt_text("Alice", key)
    enc_bob = encrypt_text("Bob", key)
    enc_charlie = encrypt_text("Charlie", key)
    enc_dana = encrypt_text("Dana", key)

    # 1. Create Joint Account 1 for Couple 1 (Alice & Bob)
    r_ja1 = await client.post("/joint-accounts", json={
        "name": encrypt_text("Alice & Bob Joint", key),
        "balance_cents": 100000,
        "safety_margin_pct": 10,
        "deposit_split_mode": "even",
        "expected_total_cents": 80000,
        "member_names": [enc_alice, enc_bob],
    })
    assert r_ja1.status_code == 201
    ja1 = r_ja1.json()
    ja1_id = ja1["id"]
    assert ja1["balance_cents"] == 100000
    assert set(ja1["member_names"]) == {enc_alice, enc_bob}

    # 2. Create Joint Account 2 for Couple 2 (Charlie & Dana)
    r_ja2 = await client.post("/joint-accounts", json={
        "name": encrypt_text("Charlie & Dana Joint", key),
        "balance_cents": 250000,
        "safety_margin_pct": 15,
        "deposit_split_mode": "even",
        "expected_total_cents": 200000,
        "member_names": [enc_charlie, enc_dana],
    })
    assert r_ja2.status_code == 201
    ja2 = r_ja2.json()
    ja2_id = ja2["id"]
    assert ja2["balance_cents"] == 250000
    assert set(ja2["member_names"]) == {enc_charlie, enc_dana}

    # 3. Verify listing joint accounts
    r_list = await client.get("/joint-accounts")
    assert r_list.status_code == 200
    all_jas = r_list.json()
    assert len(all_jas) >= 2
    acc1 = next(a for a in all_jas if a["id"] == ja1_id)
    acc2 = next(a for a in all_jas if a["id"] == ja2_id)
    assert set(acc1["member_names"]) == {enc_alice, enc_bob}
    assert set(acc2["member_names"]) == {enc_charlie, enc_dana}

    # 4. Configure categories for each joint account
    enc_c1_rent = encrypt_text("C1_Rent", key)
    enc_c2_rent = encrypt_text("C2_Rent", key)

    await client.post("/splits", json={
        "category": enc_c1_rent,
        "allocations": [{"user_name": enc_alice, "pct": 50.0}, {"user_name": enc_bob, "pct": 50.0}],
    })
    await client.post("/splits", json={
        "category": enc_c2_rent,
        "allocations": [{"user_name": enc_charlie, "pct": 50.0}, {"user_name": enc_dana, "pct": 50.0}],
    })

    await client.post("/joint-account/categories", json={"category": enc_c1_rent, "account_id": ja1_id})
    await client.post("/joint-account/categories", json={"category": enc_c2_rent, "account_id": ja2_id})

    # Categories per account
    c1_cats = (await client.get(f"/joint-account/categories?account_id={ja1_id}")).json()
    c2_cats = (await client.get(f"/joint-account/categories?account_id={ja2_id}")).json()
    assert enc_c1_rent in c1_cats
    assert enc_c1_rent not in c2_cats
    assert enc_c2_rent in c2_cats

    # 5. Configure scheduled deposits for each account
    await client.put(f"/joint-account/deposits?account_id={ja1_id}", json=[
        {"user_name": enc_alice, "amount_cents": 44000, "day_of_month": 1, "account_id": ja1_id},
        {"user_name": enc_bob, "amount_cents": 44000, "day_of_month": 1, "account_id": ja1_id},
    ])
    await client.put(f"/joint-account/deposits?account_id={ja2_id}", json=[
        {"user_name": enc_charlie, "amount_cents": 115000, "day_of_month": 5, "account_id": ja2_id},
        {"user_name": enc_dana, "amount_cents": 115000, "day_of_month": 5, "account_id": ja2_id},
    ])

    ja1_deps = (await client.get(f"/joint-account/deposits?account_id={ja1_id}")).json()
    ja2_deps = (await client.get(f"/joint-account/deposits?account_id={ja2_id}")).json()
    assert len(ja1_deps) == 2
    assert len(ja2_deps) == 2
    assert {d["user_name"] for d in ja1_deps} == {enc_alice, enc_bob}
    assert {d["user_name"] for d in ja2_deps} == {enc_charlie, enc_dana}

    # 6. Balance corrections per account
    await client.post("/joint-account/corrections", json={
        "amount_cents": 5000,
        "correction_date": "2026-07-02",
        "note": encrypt_text("C1 top-up", key),
        "account_id": ja1_id,
    })
    await client.post("/joint-account/corrections", json={
        "amount_cents": -10000,
        "correction_date": "2026-07-02",
        "note": encrypt_text("C2 withdrawal", key),
        "account_id": ja2_id,
    })

    # Check updated balances
    acc1_after = (await client.get(f"/joint-accounts/{ja1_id}")).json()
    acc2_after = (await client.get(f"/joint-accounts/{ja2_id}")).json()
    assert acc1_after["balance_cents"] == 100000 + 5000
    assert acc2_after["balance_cents"] == 250000 - 10000

    # 7. Settle per account
    s1 = (await client.post("/joint-account/settle", json={"month": "2026-07", "account_id": ja1_id})).json()
    s2 = (await client.post("/joint-account/settle", json={"month": "2026-07", "account_id": ja2_id})).json()
    assert s1["account_id"] == ja1_id
    assert s2["account_id"] == ja2_id


@pytest.mark.asyncio
async def test_analytics_scope_filtering_by_user_subset(client: AsyncClient):
    key = derive_key()

    # Create 4 users: Couple 1 (Alice, Bob), Couple 2 (Charlie, Dana)
    for u in ["Alice", "Bob", "Charlie", "Dana"]:
        await client.post("/users", json={"name": encrypt_text(u, key), "color": "#6366f1", "is_active": 1})

    enc_alice = encrypt_text("Alice", key)
    enc_bob = encrypt_text("Bob", key)
    enc_charlie = encrypt_text("Charlie", key)
    enc_dana = encrypt_text("Dana", key)

    enc_groc = encrypt_text("Groceries", key)
    enc_dining = encrypt_text("Dining", key)

    await client.post("/splits", json={
        "category": enc_groc,
        "allocations": [{"user_name": enc_alice, "pct": 50.0}, {"user_name": enc_bob, "pct": 50.0}],
    })
    await client.post("/splits", json={
        "category": enc_dining,
        "allocations": [{"user_name": enc_charlie, "pct": 50.0}, {"user_name": enc_dana, "pct": 50.0}],
    })

    # Alice spends 100 on Groceries
    r_exp1 = await client.post("/expenses", json={
        "name": encrypt_text("Supermarket", key),
        "cost_cents": 10000,
        "expense_date": "2026-07-10",
        "who_paid": enc_alice,
        "category": enc_groc,
        "is_joint": False,
    })
    assert r_exp1.status_code == 201

    # Bob spends 50 on Groceries
    r_exp2 = await client.post("/expenses", json={
        "name": encrypt_text("Bakery", key),
        "cost_cents": 5000,
        "expense_date": "2026-07-11",
        "who_paid": enc_bob,
        "category": enc_groc,
        "is_joint": False,
    })
    assert r_exp2.status_code == 201

    # Charlie spends 200 on Dining
    r_exp3 = await client.post("/expenses", json={
        "name": encrypt_text("Steakhouse", key),
        "cost_cents": 20000,
        "expense_date": "2026-07-12",
        "who_paid": enc_charlie,
        "category": enc_dining,
        "is_joint": False,
    })
    assert r_exp3.status_code == 201

    # Dana spends 80 on Dining
    r_exp4 = await client.post("/expenses", json={
        "name": encrypt_text("Sushi", key),
        "cost_cents": 8000,
        "expense_date": "2026-07-13",
        "who_paid": enc_dana,
        "category": enc_dining,
        "is_joint": False,
    })
    assert r_exp4.status_code == 201

    # Overall monthly total
    all_tot = (await client.get("/analytics/monthly-total?month=2026-07")).json()
    assert all_tot["total_amount"] == 430.0  # 100 + 50 + 200 + 80
    assert all_tot["expense_count"] == 4

    # Filter by Couple 1 (Alice & Bob)
    c1_users = f"{enc_alice},{enc_bob}"
    c1_tot = (await client.get(f"/analytics/monthly-total?month=2026-07&users={c1_users}")).json()
    assert c1_tot["total_amount"] == 150.0  # 100 + 50
    assert c1_tot["expense_count"] == 2

    # Filter by Couple 2 (Charlie & Dana)
    c2_users = f"{enc_charlie},{enc_dana}"
    c2_tot = (await client.get(f"/analytics/monthly-total?month=2026-07&users={c2_users}")).json()
    assert c2_tot["total_amount"] == 280.0  # 200 + 80
    assert c2_tot["expense_count"] == 2

    # By category with scope filter
    c1_cats = (await client.get(f"/analytics/by-category?month=2026-07&users={c1_users}")).json()
    assert len(c1_cats) == 1
    assert c1_cats[0]["category"] == enc_groc
    assert c1_cats[0]["total_amount"] == 150.0

    # By payer with scope filter
    c1_payers = (await client.get(f"/analytics/by-payer?month=2026-07&users={c1_users}")).json()
    assert len(c1_payers) == 2
    payer_map = {p["who_paid"]: p["total_amount"] for p in c1_payers}
    assert payer_map[enc_alice] == 100.0
    assert payer_map[enc_bob] == 50.0


@pytest.mark.asyncio
async def test_database_backup_export_import_with_multi_household(client: AsyncClient, test_db):
    passphrase = "test-master-passphrase"
    key = derive_key(passphrase)

    # Set up auth
    magic_enc = encrypt_text("FinanceTrackerAuth", key)
    await client.post("/auth/salt", json={"value": magic_enc})

    # Seed users, projects, and joint accounts
    for u in ["Alice", "Bob"]:
        await client.post("/users", json={"name": encrypt_text(u, key), "color": "#6366f1", "is_active": 1})
    enc_alice = encrypt_text("Alice", key)
    enc_bob = encrypt_text("Bob", key)

    await client.post("/projects", json={
        "name": encrypt_text("Solar Panels", key),
        "target_cents": 1200000,
        "target_date": "2027-01-01",
        "is_joint": False,
        "user_names": [enc_alice, enc_bob],
    })

    await client.post("/joint-accounts", json={
        "name": encrypt_text("Household Pool", key),
        "balance_cents": 50000,
        "safety_margin_pct": 10,
        "deposit_split_mode": "even",
        "expected_total_cents": 40000,
        "member_names": [enc_alice, enc_bob],
    })

    # Export backup
    r_exp = await client.post("/auth/export", json={"value": passphrase})
    assert r_exp.status_code == 200
    export_content = r_exp.content
    assert len(export_content) > 0

    # Import back
    r_imp = await client.post(
        "/auth/import",
        files={"file": ("backup.db", export_content, "application/octet-stream")},
        data={"saltText": passphrase},
    )
    assert r_imp.status_code in (200, 409)

    # Verify restored state
    r_proj = await client.get("/projects")
    assert r_proj.status_code == 200
    p = r_proj.json()[0]
    assert decrypt_text(p["name"], key) == "Solar Panels"
    assert len(p["user_names"]) == 2

    r_jas = await client.get("/joint-accounts")
    assert r_jas.status_code == 200
    jas = r_jas.json()
    assert len(jas) >= 1
    matching_ja = next(j for j in jas if decrypt_text(j["name"], key) == "Household Pool")
    assert len(matching_ja["member_names"]) == 2


@pytest.mark.asyncio
async def test_joint_account_id_in_expenses_and_recurring(client: AsyncClient):
    key = derive_key()

    # Create users
    for u in ["Alice", "Bob", "Charlie", "Dana"]:
        await client.post("/users", json={"name": encrypt_text(u, key), "color": "#6366f1", "is_active": 1})
    enc_alice = encrypt_text("Alice", key)
    enc_bob = encrypt_text("Bob", key)
    enc_charlie = encrypt_text("Charlie", key)
    enc_dana = encrypt_text("Dana", key)

    await client.post("/splits", json={
        "category": encrypt_text("Groceries", key),
        "allocations": [
            {"user_name": enc_alice, "pct": 50},
            {"user_name": enc_bob, "pct": 50},
        ],
    })
    enc_groc = encrypt_text("Groceries", key)

    # 1. Create two separate joint accounts
    # JA1: Alice & Bob ($100.00 = 10000 cents)
    r1 = await client.post("/joint-accounts", json={
        "name": encrypt_text("AlBob Joint", key),
        "balance_cents": 10000,
        "safety_margin_pct": 10,
        "deposit_split_mode": "even",
        "member_names": [enc_alice, enc_bob],
    })
    assert r1.status_code == 201
    ja1_id = r1.json()["id"]

    # JA2: Charlie & Dana ($200.00 = 20000 cents)
    r2 = await client.post("/joint-accounts", json={
        "name": encrypt_text("ChaDia Joint", key),
        "balance_cents": 20000,
        "safety_margin_pct": 10,
        "deposit_split_mode": "even",
        "member_names": [enc_charlie, enc_dana],
    })
    assert r2.status_code == 201
    ja2_id = r2.json()["id"]

    # 2. Create recurring expense attached to JA2
    r_rec = await client.post("/recurring", json={
        "name": encrypt_text("Internet ChaDia", key),
        "cost_cents": 4000,
        "who_paid": enc_charlie,
        "category": enc_groc,
        "frequency": "monthly",
        "day_of_month": 15,
        "start_date": "2026-01-01",
        "is_joint": True,
        "joint_account_id": ja2_id,
    })
    assert r_rec.status_code == 201
    rec = r_rec.json()
    assert rec["joint_account_id"] == ja2_id
    assert rec["is_joint"] is True

    # 3. Create expense attached to JA1 ($25.00)
    r_exp1 = await client.post("/expenses", json={
        "name": encrypt_text("Dinner AlBob", key),
        "cost_cents": 2500,
        "expense_date": "2026-08-10",
        "who_paid": enc_alice,
        "category": enc_groc,
        "is_joint": True,
        "joint_account_id": ja1_id,
    })
    assert r_exp1.status_code == 201
    exp1 = r_exp1.json()
    assert exp1["joint_account_id"] == ja1_id

    # Verify JA1 balance deducted, JA2 intact
    ja1_data = (await client.get(f"/joint-accounts/{ja1_id}")).json()
    ja2_data = (await client.get(f"/joint-accounts/{ja2_id}")).json()
    assert ja1_data["balance_cents"] == 7500  # 10000 - 2500
    assert ja2_data["balance_cents"] == 20000

    # 4. Create expense attached to JA2 ($50.00)
    r_exp2 = await client.post("/expenses", json={
        "name": encrypt_text("Dinner ChaDia", key),
        "cost_cents": 5000,
        "expense_date": "2026-08-12",
        "who_paid": enc_charlie,
        "category": enc_groc,
        "is_joint": True,
        "joint_account_id": ja2_id,
    })
    assert r_exp2.status_code == 201
    exp2 = r_exp2.json()
    assert exp2["joint_account_id"] == ja2_id

    ja1_data = (await client.get(f"/joint-accounts/{ja1_id}")).json()
    ja2_data = (await client.get(f"/joint-accounts/{ja2_id}")).json()
    assert ja1_data["balance_cents"] == 7500
    assert ja2_data["balance_cents"] == 15000  # 20000 - 5000

    # 5. Update exp1: reassign to JA2
    r_up = await client.put(f"/expenses/{exp1['id']}", json={
        "name": encrypt_text("Dinner AlBob (Moved to ChaDia)", key),
        "cost_cents": 2500,
        "expense_date": "2026-08-10",
        "who_paid": enc_alice,
        "category": enc_groc,
        "is_joint": True,
        "joint_account_id": ja2_id,
    })
    assert r_up.status_code == 200

    # JA1 refunded 2500 -> 10000; JA2 charged 2500 -> 12500
    ja1_data = (await client.get(f"/joint-accounts/{ja1_id}")).json()
    ja2_data = (await client.get(f"/joint-accounts/{ja2_id}")).json()
    assert ja1_data["balance_cents"] == 10000
    assert ja2_data["balance_cents"] == 12500

    # 6. Delete exp2: JA2 refunded 5000 -> 17500
    r_del = await client.delete(f"/expenses/{exp2['id']}")
    assert r_del.status_code == 204
    ja2_data = (await client.get(f"/joint-accounts/{ja2_id}")).json()
    assert ja2_data["balance_cents"] == 17500

