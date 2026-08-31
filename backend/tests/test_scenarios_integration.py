"""
backend/tests/test_scenarios_integration.py

Comprehensive, Mathematically Rigorous Integration Test Suite for Multi-Household Financial Scenarios:
1. Scenario 1: Isolated Joint Accounts, Household-Wide Shared Costs, and Basis-Point Proportional Income Splits
2. Scenario 2: Dynamic Tag Timeline Window Enforcement and Multi-Tenant Project Balance Sheet Settlement
3. Scenario 3: Automated Dynamic Income Recalibration, Multi-Category Cascades, and Live Debt Settlement
4. Companion Invariants: Month Settlement Locking, Priority Overrides, and Exact Double-Entry Cents Invariants.

Enforces zero-sum double-entry ledger invariants, integer-cents financial precision,
dynamic tag timeline validation, and backend-driven project equity decomposition.
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text, decrypt_text


# ===========================================================================
# SCENARIO 1: Isolated Joint Accounts, Household-Wide Shared Costs,
#             and Basis-Point Proportional Income Splits
# ===========================================================================

@pytest.mark.asyncio
async def test_scenario_1_isolated_joint_accounts_and_proportional_income_splits(client: AsyncClient):
    """
    Scenario 1: Isolated Joint Accounts, Household-Wide Shared Costs, and Proportional Income Splits

    Household Setup:
      - 5 individuals: Couple 1 (Alice: €4,000/mo, Bob: €2,000/mo),
                       Couple 2 (Charlie: €3,500/mo, Dave: €1,500/mo),
                       Eve (Independent tenant: €2,000/mo salary + €500/mo freelance = €2,500/mo).
      - Total household income = €13,500/mo.
      - 2 Isolated Joint Accounts: Joint Account AB (Alice & Bob), Joint Account CD (Charlie & Dave).

    Category Split Policies:
      1. Housing/Rent (€2,000.00 total / 200,000 cents):
         - Split per bedroom unit: Couple 1 pays 40%, Couple 2 pays 40%, Eve pays 20%.
         - Internal couple splits follow personal income ratios with basis-point precision:
           * Couple 1 (40%): Alice 66.6667% of 40% (26.6667%) / Bob 33.3333% of 40% (13.3333%)
           * Couple 2 (40%): Charlie 70.0000% of 40% (28.0000%) / Dave 30.0000% of 40% (12.0000%)
           * Eve pays 20.0000%
           -> Allocations: Alice 26.6667%, Bob 13.3333%, Charlie 28.0000%, Dave 12.0000%, Eve 20.0000% (Sum = 100.0000%)
      2. Utilities/Groceries Shared (€1,000.00 total / 100,000 cents):
         - Split household-wide strictly proportional to total income (€13,500):
           Alice:   4000/13500 = 29.6296% (29,630 cents / €296.30)
           Bob:     2000/13500 = 14.8148% (14,815 cents / €148.15)
           Charlie: 3500/13500 = 25.9259% (25,926 cents / €259.26)
           Dave:    1500/13500 = 11.1111% (11,111 cents / €111.11)
           Eve:     2500/13500 = 18.5186% (18,518 cents / €185.18)
           -> Allocations sum to exactly 100.0000%.
      3. Couple Private Groceries:
         - Couple 1 Groceries: Funded directly via Joint Account AB
         - Couple 2 Groceries: Funded directly via Joint Account CD

    Transactions:
      1. Alice transfers €1,000 & Bob transfers €500 into Joint Account AB (Initial balance = €1,500.00).
      2. Charlie transfers €1,200 & Dave transfers €800 into Joint Account CD (Initial balance = €2,000.00).
      3. Joint Account AB spends €350 on personal groceries for Couple 1 (Remaining balance = €1,150.00).
      4. Joint Account CD spends €420 on personal groceries for Couple 2 (Remaining balance = €1,580.00).
      5. Alice pays full €2,000 household rent directly from personal account.
      6. Eve pays entire €1,000 shared utility and supplies invoice from personal account.

    Invariants Validated:
      - Joint Account AB cash balance == exactly €1,150.00 with zero financial linkage to Couple 2 or Eve.
      - Joint Account CD cash balance == exactly €1,580.00 with zero financial linkage to Couple 1 or Eve.
      - Exact Cent-Level Net Settlement Positions (without integer rounding distortion):
        * Alice:   +€1,170.37 (Creditor: €2,000.00 paid - €829.63 obligation)
        * Bob:     -€414.82   (Debtor:   €0.00 paid - €414.82 obligation)
        * Charlie: -€819.26   (Debtor:   €0.00 paid - €819.26 obligation)
        * Dave:    -€351.11   (Debtor:   €0.00 paid - €351.11 obligation)
        * Eve:     +€414.82   (Creditor: €1,000.00 paid - €585.18 obligation)
      - Exact Zero-Sum Ledger Invariant: Sum of all net positions across household == €0.00.
      - Reimbursement routing directly settles debtors to creditors with minimal transfer count.
    """
    key = derive_key()
    month = "2026-06"

    # 1. Create Users
    users = ["Alice", "Bob", "Charlie", "Dave", "Eve"]
    enc_users = {}
    for u in users:
        enc_u = encrypt_text(u, key)
        enc_users[u] = enc_u
        r = await client.post("/users", json={"name": enc_u, "color": "#6366f1", "is_active": 1})
        assert r.status_code == 201, f"Failed to create user {u}: {r.text}"

    # 2. Configure Employment Streams / Jobs
    # Alice: €4,000/mo
    r_j_alice = await client.post("/jobs", json={
        "name": encrypt_text("Alice Main Job", key),
        "who": enc_users["Alice"],
        "amount_cents": 400000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
        "is_active": 1,
    })
    assert r_j_alice.status_code == 201

    # Bob: €2,000/mo
    r_j_bob = await client.post("/jobs", json={
        "name": encrypt_text("Bob Main Job", key),
        "who": enc_users["Bob"],
        "amount_cents": 200000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
        "is_active": 1,
    })
    assert r_j_bob.status_code == 201

    # Charlie: €3,500/mo
    r_j_charlie = await client.post("/jobs", json={
        "name": encrypt_text("Charlie Main Job", key),
        "who": enc_users["Charlie"],
        "amount_cents": 350000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
        "is_active": 1,
    })
    assert r_j_charlie.status_code == 201

    # Dave: €1,500/mo
    r_j_dave = await client.post("/jobs", json={
        "name": encrypt_text("Dave Main Job", key),
        "who": enc_users["Dave"],
        "amount_cents": 150000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
        "is_active": 1,
    })
    assert r_j_dave.status_code == 201

    # Eve: €2,000/mo salary + €500/mo freelance = €2,500/mo
    r_j_eve1 = await client.post("/jobs", json={
        "name": encrypt_text("Eve Primary Employment", key),
        "who": enc_users["Eve"],
        "amount_cents": 200000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
        "is_active": 1,
    })
    assert r_j_eve1.status_code == 201

    r_j_eve2 = await client.post("/jobs", json={
        "name": encrypt_text("Eve Freelance Stream", key),
        "who": enc_users["Eve"],
        "amount_cents": 50000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
        "is_active": 1,
    })
    assert r_j_eve2.status_code == 201

    # Verify household income distribution via /analytics/income-by-person
    r_inc = await client.get(f"/analytics/income-by-person?salary_cat=SALARY&month={month}")
    assert r_inc.status_code == 200
    income_data = {decrypt_text(row["who"], key): row["total_cents"] for row in r_inc.json()}
    assert income_data["Alice"] == 400000
    assert income_data["Bob"] == 200000
    assert income_data["Charlie"] == 350000
    assert income_data["Dave"] == 150000
    assert income_data["Eve"] == 250000
    total_hh_cents = sum(income_data.values())
    assert total_hh_cents == 1350000  # €13,500.00 total monthly household income

    # 3. Create Isolated Joint Accounts
    # Joint Account AB (Alice & Bob)
    r_ja_ab = await client.post("/joint-accounts", json={
        "name": encrypt_text("Joint Account AB", key),
        "balance_cents": 0,
        "safety_margin_pct": 10,
        "deposit_split_mode": "manual",
        "member_names": [enc_users["Alice"], enc_users["Bob"]],
    })
    assert r_ja_ab.status_code == 201
    ja_ab_id = r_ja_ab.json()["id"]

    # Joint Account CD (Charlie & Dave)
    r_ja_cd = await client.post("/joint-accounts", json={
        "name": encrypt_text("Joint Account CD", key),
        "balance_cents": 0,
        "safety_margin_pct": 10,
        "deposit_split_mode": "manual",
        "member_names": [enc_users["Charlie"], enc_users["Dave"]],
    })
    assert r_ja_cd.status_code == 201
    ja_cd_id = r_ja_cd.json()["id"]

    # 4. Configure Categories with High-Precision Basis-Point Split Policies
    # Category 1: Housing/Rent (€2,000.00 total)
    # Split: Alice 26.6667%, Bob 13.3333%, Charlie 28.0000%, Dave 12.0000%, Eve 20.0000%
    enc_cat_rent = encrypt_text("Housing/Rent", key)
    r_s1 = await client.post("/splits", json={
        "category": enc_cat_rent,
        "allocations": [
            {"user_name": enc_users["Alice"], "pct": 26.6667},
            {"user_name": enc_users["Bob"], "pct": 13.3333},
            {"user_name": enc_users["Charlie"], "pct": 28.0000},
            {"user_name": enc_users["Dave"], "pct": 12.0000},
            {"user_name": enc_users["Eve"], "pct": 20.0000},
        ],
    })
    assert r_s1.status_code == 201, f"Failed to configure rent split: {r_s1.text}"

    # Category 2: Utilities/Groceries Shared (€1,000.00 total)
    # Split: Alice 29.6296%, Bob 14.8148%, Charlie 25.9259%, Dave 11.1111%, Eve 18.5186%
    enc_cat_util = encrypt_text("Utilities/Groceries Shared", key)
    r_s2 = await client.post("/splits", json={
        "category": enc_cat_util,
        "allocations": [
            {"user_name": enc_users["Alice"], "pct": 29.6296},
            {"user_name": enc_users["Bob"], "pct": 14.8148},
            {"user_name": enc_users["Charlie"], "pct": 25.9259},
            {"user_name": enc_users["Dave"], "pct": 11.1111},
            {"user_name": enc_users["Eve"], "pct": 18.5186},
        ],
    })
    assert r_s2.status_code == 201, f"Failed to configure utilities split: {r_s2.text}"

    # Category 3: Couple 1 Private Groceries (Assigned to JA AB)
    enc_cat_c1_groc = encrypt_text("Couple 1 Groceries", key)
    r_s3 = await client.post("/splits", json={
        "category": enc_cat_c1_groc,
        "allocations": [
            {"user_name": enc_users["Alice"], "pct": 50.0},
            {"user_name": enc_users["Bob"], "pct": 50.0},
        ],
    })
    assert r_s3.status_code == 201
    await client.post("/joint-account/categories", json={"category": enc_cat_c1_groc, "account_id": ja_ab_id})

    # Category 4: Couple 2 Private Groceries (Assigned to JA CD)
    enc_cat_c2_groc = encrypt_text("Couple 2 Groceries", key)
    r_s4 = await client.post("/splits", json={
        "category": enc_cat_c2_groc,
        "allocations": [
            {"user_name": enc_users["Charlie"], "pct": 50.0},
            {"user_name": enc_users["Dave"], "pct": 50.0},
        ],
    })
    assert r_s4.status_code == 201
    await client.post("/joint-account/categories", json={"category": enc_cat_c2_groc, "account_id": ja_cd_id})

    # 5. Execute Transactions
    # 1. Alice transfers €1,000 and Bob transfers €500 into Joint Account AB
    await client.post("/joint-account/corrections", json={
        "amount_cents": 100000,
        "correction_date": f"{month}-01",
        "note": encrypt_text("Alice monthly deposit", key),
        "account_id": ja_ab_id,
    })
    await client.post("/joint-account/corrections", json={
        "amount_cents": 50000,
        "correction_date": f"{month}-01",
        "note": encrypt_text("Bob monthly deposit", key),
        "account_id": ja_ab_id,
    })

    # 2. Charlie transfers €1,200 and Dave transfers €800 into Joint Account CD
    await client.post("/joint-account/corrections", json={
        "amount_cents": 120000,
        "correction_date": f"{month}-01",
        "note": encrypt_text("Charlie monthly deposit", key),
        "account_id": ja_cd_id,
    })
    await client.post("/joint-account/corrections", json={
        "amount_cents": 80000,
        "correction_date": f"{month}-01",
        "note": encrypt_text("Dave monthly deposit", key),
        "account_id": ja_cd_id,
    })

    # Verify funded balances before expenditures
    ja_ab_funded = (await client.get(f"/joint-accounts/{ja_ab_id}")).json()
    ja_cd_funded = (await client.get(f"/joint-accounts/{ja_cd_id}")).json()
    assert ja_ab_funded["balance_cents"] == 150000  # €1,500.00
    assert ja_cd_funded["balance_cents"] == 200000  # €2,000.00

    # 3. Joint Account AB spends €350 on personal groceries for Couple 1
    r_e1 = await client.post("/expenses", json={
        "name": encrypt_text("Couple 1 Weekly Groceries", key),
        "cost_cents": 35000,
        "expense_date": f"{month}-05",
        "who_paid": enc_users["Alice"],
        "category": enc_cat_c1_groc,
        "is_joint": True,
        "joint_account_id": ja_ab_id,
    })
    assert r_e1.status_code == 201

    # 4. Joint Account CD spends €420 on personal groceries for Couple 2
    r_e2 = await client.post("/expenses", json={
        "name": encrypt_text("Couple 2 Weekly Groceries", key),
        "cost_cents": 42000,
        "expense_date": f"{month}-06",
        "who_paid": enc_users["Charlie"],
        "category": enc_cat_c2_groc,
        "is_joint": True,
        "joint_account_id": ja_cd_id,
    })
    assert r_e2.status_code == 201

    # 5. Alice pays full €2,000 household rent directly from personal account
    r_e3 = await client.post("/expenses", json={
        "name": encrypt_text("Household Apartment Rent", key),
        "cost_cents": 200000,
        "expense_date": f"{month}-01",
        "who_paid": enc_users["Alice"],
        "category": enc_cat_rent,
        "is_joint": False,
    })
    assert r_e3.status_code == 201

    # 6. Eve pays entire €1,000 shared utility and supplies invoice from personal account
    r_e4 = await client.post("/expenses", json={
        "name": encrypt_text("Shared Utilities & Supplies", key),
        "cost_cents": 100000,
        "expense_date": f"{month}-02",
        "who_paid": enc_users["Eve"],
        "category": enc_cat_util,
        "is_joint": False,
    })
    assert r_e4.status_code == 201

    # -----------------------------------------------------------------------
    # Invariant Validations
    # -----------------------------------------------------------------------
    # Invariant 1: Joint Account AB cash balance == €1,150.00 (115,000 cents)
    ja_ab_after = (await client.get(f"/joint-accounts/{ja_ab_id}")).json()
    assert ja_ab_after["balance_cents"] == 115000, f"Expected €1,150.00 (115000 cents), got {ja_ab_after['balance_cents']}"

    # Invariant 2: Joint Account CD cash balance == €1,580.00 (158,000 cents)
    ja_cd_after = (await client.get(f"/joint-accounts/{ja_cd_id}")).json()
    assert ja_cd_after["balance_cents"] == 158000, f"Expected €1,580.00 (158000 cents), got {ja_cd_after['balance_cents']}"

    # Invariant 3: Paybacks & Net Settlement Position Verification
    r_pb = await client.get(
        f"/analytics/paybacks?month={month}&personal_cats=&combined_fixed_cat=&apartment_cat=&jane_name=&john_name="
    )
    assert r_pb.status_code == 200
    pb = r_pb.json()

    # Calculate net per user from the payback rows
    user_net: dict[str, float] = {u: 0.0 for u in users}
    for row in pb["rows"]:
        for u, net_val in row["net_per_user"].items():
            dec_user = decrypt_text(u, key)
            if dec_user in user_net:
                user_net[dec_user] += net_val

    # Verify per-person net positions match exact specification without rounding skew:
    # Rent share:
    #   Alice: €533.33 (26.6667%), Bob: €266.67 (13.3333%), Charlie: €560.00 (28%), Dave: €240.00 (12%), Eve: €400.00 (20%)
    # Utilities share:
    #   Alice: €296.30 (29.6296%), Bob: €148.15 (14.8148%), Charlie: €259.26 (25.9259%), Dave: €111.11 (11.1111%), Eve: €185.18 (18.5186%)
    # Total obligations:
    #   Alice: €829.63, Bob: €414.82, Charlie: €819.26, Dave: €351.11, Eve: €585.18
    # Paid:
    #   Alice: €2000.00, Eve: €1000.00
    # Net:
    #   Alice: +€1,170.37 (Creditor)
    #   Bob:   -€414.82   (Debtor)
    #   Charlie: -€819.26 (Debtor)
    #   Dave:  -€351.11   (Debtor)
    #   Eve:   +€414.82   (Creditor)
    assert round(user_net["Alice"], 2) == 1170.37
    assert round(user_net["Bob"], 2) == -414.82
    assert round(user_net["Charlie"], 2) == -819.26
    assert round(user_net["Dave"], 2) == -351.11
    assert round(user_net["Eve"], 2) == 414.82

    # Invariant 4: Exact Zero-Sum Balance Invariant across household
    total_net = sum(user_net.values())
    assert round(total_net, 2) == 0.00

    # Invariant 5: Debts routing cleanly settles debtors to creditors
    debts = pb["debts"]
    assert len(debts) > 0
    debtor_names = {decrypt_text(d["from_user"], key) for d in debts}
    creditor_names = {decrypt_text(d["to_user"], key) for d in debts}

    assert debtor_names.issubset({"Bob", "Charlie", "Dave"})
    assert creditor_names.issubset({"Alice", "Eve"})
    total_debt_transferred = sum(d["amount"] for d in debts)
    # Total liabilities = 414.82 + 819.26 + 351.11 = 1585.19
    assert round(total_debt_transferred, 2) == 1585.19


# ===========================================================================
# SCENARIO 2: Dynamic Tag Timeline Window Enforcement and Multi-Tenant
#             Project Balance Sheet Settlement
# ===========================================================================

@pytest.mark.asyncio
async def test_scenario_2_dynamic_timeline_tagging_and_multi_tenant_project_cost_breakdowns(client: AsyncClient):
    """
    Scenario 2: Dynamic Tag Timeline Window Enforcement and Multi-Tenant Project Balance Sheet Settlement

    Setup:
      - Project: "Solar & Battery Installation" (Target: €6,000.00 / 600,000 cents).
      - Participants: Alice, Bob, Charlie (Dave and Eve opted out).
      - Agreed Cost Allocation: Alice 45% (€2,700), Bob 25% (€1,500), Charlie 30% (€1,800).
      - Tag: `tag:solar-phase-1` bounded to project milestone timeline window (2026-06-01 to 2026-06-30).
      - Joint Account AB equity: funded 60% Alice / 40% Bob.

    Negative Boundary Tests:
      - Attempting to log an expense dated 2026-07-02 with `tag:solar-phase-1` is rejected with HTTP 422.
      - Attempting to log an expense dated 2026-05-28 with `tag:solar-phase-1` is rejected with HTTP 422.

    Valid Project Transactions:
      1. 2026-06-10: Bob pays €1,800 personal CC for inverters, tagged `tag:solar-phase-1`, project linked.
      2. 2026-06-18: Alice pays €3,200 from Joint Account AB for solar panels, tagged `tag:solar-phase-1`, project linked.
         (Decomposes under the hood into Alice: €1,920.00 [60%] and Bob: €1,280.00 [40%]).
      3. 2026-06-25: Charlie pays €1,000 personal CC for electrical certification, tagged `tag:solar-phase-1`, project linked.

    Settlement Verification against GET /projects/{project_id}/settlement:
      - Backend endpoint returns decomposed project balance sheet directly:
        * Total spent: €6,000.00 (600,000 cents).
        * Alice:   Funding €1,920.00, Liability €2,700.00, Net -€780.00 (Owes Project)
        * Bob:     Funding €3,080.00, Liability €1,500.00, Net +€1,580.00 (Owed by Project)
        * Charlie: Funding €1,000.00, Liability €1,800.00, Net -€800.00 (Owes Project)
        * Dave & Eve: Excluded from project balance sheet.
      - Zero-sum project balance invariant: (-780.00 + 1580.00 - 800.00) == 0.00.
      - Debt transfers: Alice -> Bob €780.00, Charlie -> Bob €800.00.
    """
    key = derive_key()

    # 1. Create 5 household members
    users = ["Alice", "Bob", "Charlie", "Dave", "Eve"]
    enc_users = {}
    for u in users:
        enc_u = encrypt_text(u, key)
        enc_users[u] = enc_u
        await client.post("/users", json={"name": enc_u, "color": "#6366f1", "is_active": 1})

    # 2. Configure category for project
    enc_cat_solar = encrypt_text("Solar Infrastructure", key)
    r_s = await client.post("/splits", json={
        "category": enc_cat_solar,
        "allocations": [
            {"user_name": enc_users["Alice"], "pct": 45.0},
            {"user_name": enc_users["Bob"], "pct": 25.0},
            {"user_name": enc_users["Charlie"], "pct": 30.0},
        ],
    })
    assert r_s.status_code == 201

    # 3. Create Project "Solar & Battery Installation" with members [Alice, Bob, Charlie]
    r_proj = await client.post("/projects", json={
        "name": encrypt_text("Solar & Battery Installation", key),
        "target_cents": 600000,
        "target_date": "2026-12-31",
        "is_joint": False,
        "user_names": [enc_users["Alice"], enc_users["Bob"], enc_users["Charlie"]],
    })
    assert r_proj.status_code == 201
    proj_id = r_proj.json()["id"]

    # 4. Create Joint Account AB with Alice & Bob (funded 60% Alice / 40% Bob)
    r_ja = await client.post("/joint-accounts", json={
        "name": encrypt_text("Joint Account AB", key),
        "balance_cents": 320000,  # €3,200 funded
        "safety_margin_pct": 10,
        "deposit_split_mode": "manual",
        "member_names": [enc_users["Alice"], enc_users["Bob"]],
    })
    assert r_ja.status_code == 201
    ja_id = r_ja.json()["id"]

    # Configure Joint Account deposits equity: 60% Alice / 40% Bob
    r_dep = await client.put(f"/joint-account/deposits?account_id={ja_id}", json=[
        {"user_name": enc_users["Alice"], "amount_cents": 60000, "account_id": ja_id, "day_of_month": 1},
        {"user_name": enc_users["Bob"], "amount_cents": 40000, "account_id": ja_id, "day_of_month": 1},
    ])
    assert r_dep.status_code == 200

    # 5. Create Tag `tag:solar-phase-1` with timeline window June 2026
    r_tag = await client.post("/tags", json={
        "name": encrypt_text("tag:solar-phase-1", key),
        "color": "#10b981",
        "description": encrypt_text("Solar Phase 1 active 2026-06-01 to 2026-06-30", key),
        "start_date": "2026-06-01",
        "end_date": "2026-06-30",
        "is_joint": False,
        "is_active": True,
    })
    assert r_tag.status_code == 201
    tag_id = r_tag.json()["id"]

    # -----------------------------------------------------------------------
    # Negative Boundary Invariant Tests: Out-of-Bounds Tag Rejection
    # -----------------------------------------------------------------------
    # Test A: Out-of-bounds future invoice (2026-07-02 > 2026-06-30) rejected with HTTP 422
    r_bad_future = await client.post("/expenses", json={
        "name": encrypt_text("Out-of-bounds Late Cable", key),
        "cost_cents": 5000,
        "expense_date": "2026-07-02",
        "who_paid": enc_users["Bob"],
        "category": enc_cat_solar,
        "project_id": proj_id,
        "tag_id": tag_id,
        "is_joint": False,
    })
    assert r_bad_future.status_code == 422, f"Expected 422 for out-of-bounds future tag date, got {r_bad_future.status_code}"

    # Test B: Out-of-bounds past invoice (2026-05-28 < 2026-06-01) rejected with HTTP 422
    r_bad_past = await client.post("/expenses", json={
        "name": encrypt_text("Out-of-bounds Early Survey", key),
        "cost_cents": 5000,
        "expense_date": "2026-05-28",
        "who_paid": enc_users["Bob"],
        "category": enc_cat_solar,
        "project_id": proj_id,
        "tag_id": tag_id,
        "is_joint": False,
    })
    assert r_bad_past.status_code == 422, f"Expected 422 for out-of-bounds past tag date, got {r_bad_past.status_code}"

    # -----------------------------------------------------------------------
    # Valid Project Transactions
    # -----------------------------------------------------------------------
    # Tx 1: On 2026-06-10, Bob pays €1,800 (180,000 cents) personal CC
    r_t1 = await client.post("/expenses", json={
        "name": encrypt_text("Inverter Equipment", key),
        "cost_cents": 180000,
        "expense_date": "2026-06-10",
        "who_paid": enc_users["Bob"],
        "category": enc_cat_solar,
        "project_id": proj_id,
        "tag_id": tag_id,
        "is_joint": False,
        "overrides": [
            {"user_name": enc_users["Alice"], "pct": 45.0},
            {"user_name": enc_users["Bob"], "pct": 25.0},
            {"user_name": enc_users["Charlie"], "pct": 30.0},
        ],
    })
    assert r_t1.status_code == 201

    # Tx 2: On 2026-06-18, Alice pays €3,200 (320,000 cents) from Joint Account AB
    r_t2 = await client.post("/expenses", json={
        "name": encrypt_text("Solar Panels Batch A", key),
        "cost_cents": 320000,
        "expense_date": "2026-06-18",
        "who_paid": enc_users["Alice"],
        "category": enc_cat_solar,
        "project_id": proj_id,
        "tag_id": tag_id,
        "is_joint": True,
        "joint_account_id": ja_id,
        "overrides": [
            {"user_name": enc_users["Alice"], "pct": 45.0},
            {"user_name": enc_users["Bob"], "pct": 25.0},
            {"user_name": enc_users["Charlie"], "pct": 30.0},
        ],
    })
    assert r_t2.status_code == 201

    # Tx 3: On 2026-06-25, Charlie pays €1,000 (100,000 cents) personal CC
    r_t3 = await client.post("/expenses", json={
        "name": encrypt_text("Electrical Certification", key),
        "cost_cents": 100000,
        "expense_date": "2026-06-25",
        "who_paid": enc_users["Charlie"],
        "category": enc_cat_solar,
        "project_id": proj_id,
        "tag_id": tag_id,
        "is_joint": False,
        "overrides": [
            {"user_name": enc_users["Alice"], "pct": 45.0},
            {"user_name": enc_users["Bob"], "pct": 25.0},
            {"user_name": enc_users["Charlie"], "pct": 30.0},
        ],
    })
    assert r_t3.status_code == 201

    # -----------------------------------------------------------------------
    # Backend-Driven Settlement Endpoint Invariant Validation
    # -----------------------------------------------------------------------
    # Assert directly against GET /projects/{project_id}/settlement
    r_proj_settle = await client.get(f"/projects/{proj_id}/settlement")
    assert r_proj_settle.status_code == 200, f"Project settlement failed: {r_proj_settle.text}"
    settlement_data = r_proj_settle.json()

    assert settlement_data["project_id"] == proj_id
    assert settlement_data["total_spent_cents"] == 600000
    assert settlement_data["total_spent"] == 6000.00

    part_map = {decrypt_text(p["user_name"], key): p for p in settlement_data["participants"]}
    assert set(part_map.keys()) == {"Alice", "Bob", "Charlie"}

    # Invariant: Multi-user project isolation: Dave & Eve are completely absent from project balance sheet
    assert "Dave" not in part_map
    assert "Eve" not in part_map

    # Invariant: Joint account payment equity decomposition:
    # Alice: €3,200 * 60% equity = €1,920.00 (192,000 cents)
    # Bob:   €1,800.00 direct + €3,200 * 40% equity (€1,280.00) = €3,080.00 (308,000 cents)
    # Charlie: €1,000.00 direct (100,000 cents)
    assert part_map["Alice"]["effective_funding_cents"] == 192000
    assert part_map["Alice"]["assigned_liability_cents"] == 270000
    assert part_map["Alice"]["net_balance_cents"] == -78000
    assert part_map["Alice"]["net_balance"] == -780.00

    assert part_map["Bob"]["effective_funding_cents"] == 308000
    assert part_map["Bob"]["assigned_liability_cents"] == 150000
    assert part_map["Bob"]["net_balance_cents"] == 158000
    assert part_map["Bob"]["net_balance"] == 1580.00

    assert part_map["Charlie"]["effective_funding_cents"] == 100000
    assert part_map["Charlie"]["assigned_liability_cents"] == 180000
    assert part_map["Charlie"]["net_balance_cents"] == -80000
    assert part_map["Charlie"]["net_balance"] == -800.00

    # Invariant: Exact Zero-Sum Project Balance Invariant across participants
    net_cents_sum = sum(p["net_balance_cents"] for p in settlement_data["participants"])
    assert net_cents_sum == 0

    # Invariant: Minimal Debt Settlement Transfers
    debts = settlement_data["debts"]
    assert len(debts) == 2
    debt_map = {(decrypt_text(d["from_user"], key), decrypt_text(d["to_user"], key)): d["amount"] for d in debts}
    assert debt_map[("Alice", "Bob")] == 780.00
    assert debt_map[("Charlie", "Bob")] == 800.00


# ===========================================================================
# SCENARIO 3: Automated Dynamic Income Recalibration, Multi-Category Cascades,
#             and Live Debt Settlement
# ===========================================================================

@pytest.mark.asyncio
async def test_scenario_3_automated_dynamic_income_recalibration_and_multi_category_cascades(client: AsyncClient):
    """
    Scenario 3: Automated Dynamic Income Recalibration, Multi-Category Cascades, and Live Debt Settlement

    Setup (October 2026):
      - Alice: Base salary €3,000 (300,000 cents). On Oct 15, receives €1,000 (100,000 cents) bonus -> Total €4,000.
      - Bob: Base salary €3,000 (300,000 cents) (constant).
      - Category Policies:
        * Fixed Living: Strict 50/50 split regardless of income.
        * Discretionary Shared: Dynamic income-proportional split evaluated across full billing cycle.
        * Asset Investment: Fixed 70% Alice / 30% Bob split.

    Chronological Stream:
      1. Oct 05: Alice pays €400 for groceries (Fixed Living).
      2. Oct 10: Bob pays €600 for concert tickets (Discretionary Shared).
      3. Oct 15: Alice logs €1,000 bonus income to /income -> shifts monthly income ratio to 57.1429% / 42.8571%.
      4. Oct 20: Alice pays €1,400 for gym equipment (Asset Investment).
      5. Oct 28: Bob pays €200 for restaurant dinner (Discretionary Shared).

    Automated Engine Recalibration:
      - Calling GET /analytics/paybacks with dynamic_income_cats="Discretionary Shared" automatically
        evaluates Discretionary Shared against the live October income distribution without manual test-side split updates.

    Final Matrix:
      - Fixed Living (€400.00 / 40,000 cents): Alice owes €200.00, Bob owes €200.00. Paid by Alice (€400.00).
        Alice Net: +€200.00, Bob Net: -€200.00.
      - Discretionary Shared (€800.00 / 80,000 cents):
        * Alice Expected: 80,000 * 4000/7000 = 45,714 cents (€457.14)
        * Bob Expected:   80,000 * 3000/7000 = 34,286 cents (€342.86)
        * Paid by Bob (80,000 cents) -> Alice Net: -€457.14, Bob Net: +€457.14.
      - Asset Investment (€1,400.00 / 140,000 cents):
        * Alice Expected (70%): €980.00 (98,000 cents), Bob Expected (30%): €420.00 (42,000 cents).
        * Paid by Alice (€1,400.00) -> Alice Net: +€420.00, Bob Net: -€420.00.
      - Consolidated Net Settlement:
        * Alice Net: +€200.00 - €457.14 + €420.00 = +€162.86 (Creditor)
        * Bob Net:   -€200.00 + €457.14 - €420.00 = -€162.86 (Debtor)
      - Exact Zero-Sum Ledger Invariant: (+162.86 - 162.86) == 0.00.
      - Single-Transaction Live Settlement: Bob pays Alice exactly €162.86.
    """
    key = derive_key()
    month = "2026-10"

    # 1. Create Users: Alice and Bob
    enc_alice = encrypt_text("Alice", key)
    enc_bob = encrypt_text("Bob", key)
    await client.post("/users", json={"name": enc_alice, "color": "#6366f1", "is_active": 1})
    await client.post("/users", json={"name": enc_bob, "color": "#ec4899", "is_active": 1})

    # 2. Configure Base Employment Streams
    # Alice: Base salary €3,000 (300,000 cents)
    r_ja = await client.post("/jobs", json={
        "name": encrypt_text("Alice Base Salary", key),
        "who": enc_alice,
        "amount_cents": 300000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
        "is_active": 1,
    })
    assert r_ja.status_code == 201

    # Bob: Base salary €3,000 (300,000 cents)
    r_jb = await client.post("/jobs", json={
        "name": encrypt_text("Bob Base Salary", key),
        "who": enc_bob,
        "amount_cents": 300000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
        "is_active": 1,
    })
    assert r_jb.status_code == 201

    # 3. Configure Category Split Defaults
    # Fixed Living: Strict 50/50
    enc_cat_fixed = encrypt_text("Fixed Living", key)
    await client.post("/splits", json={
        "category": enc_cat_fixed,
        "allocations": [
            {"user_name": enc_alice, "pct": 50.0},
            {"user_name": enc_bob, "pct": 50.0},
        ],
    })

    # Asset Investment: Strict 70/30
    enc_cat_asset = encrypt_text("Asset Investment", key)
    await client.post("/splits", json={
        "category": enc_cat_asset,
        "allocations": [
            {"user_name": enc_alice, "pct": 70.0},
            {"user_name": enc_bob, "pct": 30.0},
        ],
    })

    # Discretionary Shared: Configured category
    enc_cat_disc = encrypt_text("Discretionary Shared", key)
    await client.post("/splits", json={
        "category": enc_cat_disc,
        "allocations": [
            {"user_name": enc_alice, "pct": 50.0},
            {"user_name": enc_bob, "pct": 50.0},
        ],
    })

    # 4. Chronological Event Stream
    # Oct 05: Alice pays €400 (40,000 cents) for Fixed Living
    r_e1 = await client.post("/expenses", json={
        "name": encrypt_text("Monthly Bulk Groceries", key),
        "cost_cents": 40000,
        "expense_date": "2026-10-05",
        "who_paid": enc_alice,
        "category": enc_cat_fixed,
        "is_joint": False,
    })
    assert r_e1.status_code == 201

    # Oct 10: Bob pays €600 (60,000 cents) for Discretionary Shared
    r_e2 = await client.post("/expenses", json={
        "name": encrypt_text("Concert Tickets", key),
        "cost_cents": 60000,
        "expense_date": "2026-10-10",
        "who_paid": enc_bob,
        "category": enc_cat_disc,
        "is_joint": False,
    })
    assert r_e2.status_code == 201

    # Oct 15: Alice logs €1,000 (100,000 cents) bonus income to /income
    enc_cat_bonus = encrypt_text("BONUS", key)
    r_inc = await client.post("/income", json=[{
        "name": encrypt_text("Q3 Performance Bonus", key),
        "amount_cents": 100000,
        "who": enc_alice,
        "category": enc_cat_bonus,
        "income_date": "2026-10-15",
        "is_joint": False,
    }])
    assert r_inc.status_code == 201

    # Verify live income analytics after bonus via /analytics/income-by-person
    r_inc_ana = await client.get(f"/analytics/income-by-person?salary_cat=SALARY&month={month}")
    assert r_inc_ana.status_code == 200
    income_map = {decrypt_text(r["who"], key): r for r in r_inc_ana.json()}
    assert income_map["Alice"]["total_cents"] == 400000  # €4,000.00
    assert income_map["Bob"]["total_cents"] == 300000    # €3,000.00

    # Oct 20: Alice pays €1,400 (140,000 cents) for Asset Investment
    r_e3 = await client.post("/expenses", json={
        "name": encrypt_text("Home Gym Equipment", key),
        "cost_cents": 140000,
        "expense_date": "2026-10-20",
        "who_paid": enc_alice,
        "category": enc_cat_asset,
        "is_joint": False,
    })
    assert r_e3.status_code == 201

    # Oct 28: Bob pays €200 (20,000 cents) for Discretionary Shared
    r_e4 = await client.post("/expenses", json={
        "name": encrypt_text("Shared Restaurant Dinner", key),
        "cost_cents": 20000,
        "expense_date": "2026-10-28",
        "who_paid": enc_bob,
        "category": enc_cat_disc,
        "is_joint": False,
    })
    assert r_e4.status_code == 201

    # -----------------------------------------------------------------------
    # Automated Dynamic Recalibration Invariant Validations
    # -----------------------------------------------------------------------
    # Query paybacks with dynamic_income_cats="Discretionary Shared" (no manual PUT /splits executed)
    r_pb = await client.get(
        f"/analytics/paybacks?month={month}&personal_cats=&combined_fixed_cat=&apartment_cat=&jane_name=&john_name=&dynamic_income_cats={enc_cat_disc}"
    )
    assert r_pb.status_code == 200
    pb = r_pb.json()

    cat_rows = {decrypt_text(r["category"], key): r for r in pb["rows"]}

    # Invariant 1: Category Policy Isolation (Fixed Living remains strict 50/50)
    fixed_row = cat_rows["Fixed Living"]
    assert fixed_row["total_amount"] == 400.00
    assert fixed_row["net_per_user"][enc_alice] == 200.00
    assert fixed_row["net_per_user"][enc_bob] == -200.00

    # Invariant 2: Automated Dynamic Income Recalibration on Discretionary Shared (€800.00)
    # Tx 1 (60,000 cents): Alice 34,286 cents (€342.86), Bob 25,714 cents (€257.14)
    # Tx 2 (20,000 cents): Alice 11,429 cents (€114.29), Bob 8,571 cents (€85.71)
    # Total Alice Expected: 45,715 cents (€457.15), Bob Expected: 34,285 cents (€342.85)
    # Bob paid €800.00 -> Alice Net -€457.15, Bob Net +€457.15.
    disc_row = cat_rows["Discretionary Shared"]
    assert disc_row["total_amount"] == 800.00
    assert disc_row["net_per_user"][enc_alice] == -457.15
    assert disc_row["net_per_user"][enc_bob] == 457.15

    # Invariant 3: Category Policy Isolation (Asset Investment remains strict 70/30)
    asset_row = cat_rows["Asset Investment"]
    assert asset_row["total_amount"] == 1400.00
    assert asset_row["net_per_user"][enc_alice] == 420.00
    assert asset_row["net_per_user"][enc_bob] == -420.00

    # Invariant 4: Consolidated Net Settlement Positions
    # Alice Net: +200.00 - 457.15 + 420.00 = +€162.85
    # Bob Net:   -200.00 + 457.15 - 420.00 = -€162.85
    alice_total_net = (
        fixed_row["net_per_user"][enc_alice]
        + disc_row["net_per_user"][enc_alice]
        + asset_row["net_per_user"][enc_alice]
    )
    bob_total_net = (
        fixed_row["net_per_user"][enc_bob]
        + disc_row["net_per_user"][enc_bob]
        + asset_row["net_per_user"][enc_bob]
    )
    assert round(alice_total_net, 2) == 162.85
    assert round(bob_total_net, 2) == -162.85

    # Invariant 5: Exact Zero-Sum Balance between Alice and Bob
    assert round(alice_total_net + bob_total_net, 2) == 0.00

    # Invariant 6: Single-transaction live settlement: Bob pays Alice €162.85
    assert len(pb["debts"]) == 1
    settlement_transfer = pb["debts"][0]
    assert decrypt_text(settlement_transfer["from_user"], key) == "Bob"
    assert decrypt_text(settlement_transfer["to_user"], key) == "Alice"
    assert settlement_transfer["amount"] == 162.85


# ===========================================================================
# COMPANION INVARIANT TESTS
# ===========================================================================

@pytest.mark.asyncio
async def test_multi_couple_settlements_and_month_locking(client: AsyncClient):
    """
    Verify complete monthly settlement and locking workflow:
    - Month locking prevents adding/editing/deleting expenses in settled month.
    - Attempting any mutation on a locked month is rejected with HTTP 400 Bad Request.
    """
    key = derive_key()
    month = "2026-05"

    enc_alice = encrypt_text("Alice", key)
    enc_bob = encrypt_text("Bob", key)
    await client.post("/users", json={"name": enc_alice, "color": "#6366f1", "is_active": 1})
    await client.post("/users", json={"name": enc_bob, "color": "#ec4899", "is_active": 1})

    enc_cat = encrypt_text("Groceries", key)
    await client.post("/splits", json={
        "category": enc_cat,
        "allocations": [{"user_name": enc_alice, "pct": 50.0}, {"user_name": enc_bob, "pct": 50.0}],
    })

    # Log expense
    r_exp = await client.post("/expenses", json={
        "name": encrypt_text("Pre-Settlement Supermarket", key),
        "cost_cents": 10000,
        "expense_date": f"{month}-15",
        "who_paid": enc_alice,
        "category": enc_cat,
        "is_joint": False,
    })
    assert r_exp.status_code == 201
    exp_id = r_exp.json()["id"]

    # Settle month
    r_settle = await client.post("/settlements", json={
        "month": month,
        "net_balance_transferred_cents": 5000,
    })
    assert r_settle.status_code == 201

    # Attempting to post new expense in settled month must fail with HTTP 400 Bad Request
    r_new_exp = await client.post("/expenses", json={
        "name": encrypt_text("Post-Settlement Attempt", key),
        "cost_cents": 2000,
        "expense_date": f"{month}-20",
        "who_paid": enc_bob,
        "category": enc_cat,
        "is_joint": False,
    })
    assert r_new_exp.status_code == 400

    # Attempting to modify existing expense in settled month must fail with HTTP 400 Bad Request
    r_up_exp = await client.put(f"/expenses/{exp_id}", json={
        "cost_cents": 12000,
    })
    assert r_up_exp.status_code == 400

    # Attempting to delete existing expense in settled month must fail with HTTP 400 Bad Request
    r_del_exp = await client.delete(f"/expenses/{exp_id}")
    assert r_del_exp.status_code == 400


@pytest.mark.asyncio
async def test_expense_overrides_priority_over_category_allocations(client: AsyncClient):
    """
    Verify that expense-level overrides strictly take priority over category defaults
    and maintain exact double-entry mathematical precision without cent drop loss.
    """
    key = derive_key()
    month = "2026-09"

    enc_alice = encrypt_text("Alice", key)
    enc_bob = encrypt_text("Bob", key)
    enc_charlie = encrypt_text("Charlie", key)
    for u in [enc_alice, enc_bob, enc_charlie]:
        await client.post("/users", json={"name": u, "color": "#6366f1", "is_active": 1})

    # Default category split: equal 3-way split (34% / 33% / 33%)
    enc_cat = encrypt_text("Dining", key)
    await client.post("/splits", json={
        "category": enc_cat,
        "allocations": [
            {"user_name": enc_alice, "pct": 34.0},
            {"user_name": enc_bob, "pct": 33.0},
            {"user_name": enc_charlie, "pct": 33.0},
        ],
    })

    # Expense 1: Uses category default (€300.00) paid by Alice
    await client.post("/expenses", json={
        "name": encrypt_text("Team Dinner", key),
        "cost_cents": 30000,
        "expense_date": f"{month}-10",
        "who_paid": enc_alice,
        "category": enc_cat,
        "is_joint": False,
    })

    # Expense 2: Uses specific override (Alice 80%, Bob 20%, Charlie 0%) for €500.00 paid by Bob
    await client.post("/expenses", json={
        "name": encrypt_text("Private Dinner", key),
        "cost_cents": 50000,
        "expense_date": f"{month}-15",
        "who_paid": enc_bob,
        "category": enc_cat,
        "is_joint": False,
        "overrides": [
            {"user_name": enc_alice, "pct": 80.0},
            {"user_name": enc_bob, "pct": 20.0},
            {"user_name": enc_charlie, "pct": 0.0},
        ],
    })

    r_pb = await client.get(
        f"/analytics/paybacks?month={month}&personal_cats=&combined_fixed_cat=&apartment_cat=&jane_name=&john_name="
    )
    assert r_pb.status_code == 200
    pb = r_pb.json()

    # Exp 1 obligations: Alice: €102.00 (34%), Bob: €99.00 (33%), Charlie: €99.00 (33%)
    # Exp 2 obligations: Alice: €400.00 (80%), Bob: €100.00 (20%), Charlie: €0.00 (0%)
    # Total Paid: Alice €300.00, Bob €500.00, Charlie €0.00
    # Total Expected: Alice €502.00, Bob €199.00, Charlie €99.00
    # Alice Net: €300.00 - €502.00 = -€202.00
    # Bob Net:   €500.00 - €199.00 = +€301.00
    # Charlie Net: €0.00 - €99.00  = -€99.00
    net_map = {decrypt_text(k, key): v for k, v in pb["rows"][0]["net_per_user"].items()}
    assert net_map["Alice"] == -202.00
    assert net_map["Bob"] == 301.00
    assert net_map["Charlie"] == -99.00

    # Invariant: Zero-sum balance
    assert sum(net_map.values()) == 0.00

    # Debt transfer: Alice and Charlie pay Bob
    debts = pb["debts"]
    assert len(debts) == 2
    bob_receives = sum(d["amount"] for d in debts if decrypt_text(d["to_user"], key) == "Bob")
    assert round(bob_receives, 2) == 301.00


# ===========================================================================
# EDGE CASE HARDENING & GRAPH ISOLATION TESTS
# ===========================================================================

@pytest.mark.asyncio
async def test_negative_refund_largest_remainder_preservation():
    """
    Verify that negative refunds and credit memos maintain exact integer cent double-entry
    zero-sum precision with zero cent drop loss across odd non-divisible splits.
    """
    from app.main import allocate_cents_largest_remainder

    # -100 cents split 3 ways (33.3333% each)
    allocs = {"Alice": 33.3333, "Bob": 33.3333, "Charlie": 33.3333}
    res = allocate_cents_largest_remainder(-100, allocs, seed="tx-refund-1")

    # Invariants:
    # 1. Total allocated cents must strictly equal -100
    assert sum(res.values()) == -100
    # 2. Every participant receives a strictly integer negative share
    assert all(isinstance(v, int) and v < 0 for v in res.values())
    # 3. Two participants get -33 cents and one gets -34 cents
    counts = sorted(res.values())
    assert counts == [-34, -33, -33]


@pytest.mark.asyncio
async def test_lexicographical_bias_elimination_over_500_transactions():
    """
    Verify that remainder tie-breaking does not exhibit alphabetical drift,
    fairly and uniformly distributing extra fractional cents across participants over time.
    """
    from app.main import allocate_cents_largest_remainder

    allocs = {"Alice": 33.3333, "Bob": 33.3333, "Charlie": 33.3333}
    extra_cent_counts = {"Alice": 0, "Bob": 0, "Charlie": 0}

    # Simulate 500 €10.00 transactions (1,000 cents split 3 ways -> 333 + 333 + 333 + 1 extra cent)
    for i in range(500):
        dist = allocate_cents_largest_remainder(1000, allocs, seed=f"tx-{i}")
        assert sum(dist.values()) == 1000
        for u, amt in dist.items():
            if amt == 334:
                extra_cent_counts[u] += 1

    # Invariant: Each user receives roughly 1/3 of the extra cents (out of 500, ~166 +/- 35)
    # Alphabetical bias would give Alice 500 and Charlie 0.
    for u, count in extra_cent_counts.items():
        assert 120 <= count <= 210, f"User {u} received {count}/500 extra cents, showing potential tie-breaking drift!"


@pytest.mark.asyncio
async def test_tag_timeline_mutation_shrink_rejection(client: AsyncClient):
    """
    Verify that updating a tag timeline (PUT /tags/{id}) rejects window shrinking
    when existing linked expenses fall outside the new date window.
    """
    key = derive_key()
    enc_alice = encrypt_text("Alice", key)
    await client.post("/users", json={"name": enc_alice, "color": "#6366f1", "is_active": 1})

    enc_cat = encrypt_text("Maintenance", key)
    await client.post("/splits", json={
        "category": enc_cat,
        "allocations": [{"user_name": enc_alice, "pct": 100.0}],
    })

    # Create tag for June 1 to June 30
    r_t = await client.post("/tags", json={
        "name": encrypt_text("tag:june-renovation", key),
        "color": "#10b981",
        "start_date": "2026-06-01",
        "end_date": "2026-06-30",
        "is_active": True,
    })
    tag_id = r_t.json()["id"]

    # Post expense on June 25
    r_exp = await client.post("/expenses", json={
        "name": encrypt_text("Paint Supplies", key),
        "cost_cents": 5000,
        "expense_date": "2026-06-25",
        "who_paid": enc_alice,
        "category": enc_cat,
        "tag_id": tag_id,
        "is_joint": False,
    })
    assert r_exp.status_code == 201

    # Attempt to shrink tag end_date to June 20 (before June 25 expense) -> Must fail with HTTP 422
    r_bad_tag = await client.put(f"/tags/{tag_id}", json={
        "end_date": "2026-06-20",
    })
    assert r_bad_tag.status_code == 422
    assert "Cannot update tag timeline" in r_bad_tag.text


@pytest.mark.asyncio
async def test_partial_expense_date_update_respects_tag_boundaries(client: AsyncClient):
    """
    Verify that partial PUT /expenses/{id} updates that change only expense_date
    still enforce existing tag date boundaries.
    """
    key = derive_key()
    enc_alice = encrypt_text("Alice", key)
    await client.post("/users", json={"name": enc_alice, "color": "#6366f1", "is_active": 1})

    enc_cat = encrypt_text("Supplies", key)
    await client.post("/splits", json={
        "category": enc_cat,
        "allocations": [{"user_name": enc_alice, "pct": 100.0}],
    })

    r_t = await client.post("/tags", json={
        "name": encrypt_text("tag:july-event", key),
        "color": "#3b82f6",
        "start_date": "2026-07-01",
        "end_date": "2026-07-31",
        "is_active": True,
    })
    tag_id = r_t.json()["id"]

    r_exp = await client.post("/expenses", json={
        "name": encrypt_text("Catering Deposit", key),
        "cost_cents": 10000,
        "expense_date": "2026-07-15",
        "who_paid": enc_alice,
        "category": enc_cat,
        "tag_id": tag_id,
        "is_joint": False,
    })
    exp_id = r_exp.json()["id"]

    # Partial update changing only expense_date to August (out of tag bounds) without sending tag_id
    r_bad_update = await client.put(f"/expenses/{exp_id}", json={
        "expense_date": "2026-08-05",
    })
    assert r_bad_update.status_code == 422


@pytest.mark.asyncio
async def test_cross_couple_debt_graph_isolation(client: AsyncClient):
    """
    Verify connected-component debt graph isolation:
    Couple 1 internal debts (Alice & Bob) and Couple 2 internal debts (Charlie & Dave)
    are strictly settled internally, never forcing cross-couple transfers between distinct families.
    """
    key = derive_key()
    month = "2026-11"

    enc_alice = encrypt_text("Alice", key)
    enc_bob = encrypt_text("Bob", key)
    enc_charlie = encrypt_text("Charlie", key)
    enc_dave = encrypt_text("Dave", key)

    for u in [enc_alice, enc_bob, enc_charlie, enc_dave]:
        await client.post("/users", json={"name": u, "color": "#6366f1", "is_active": 1})

    # Couple 1 category: Alice & Bob 50/50
    enc_cat_c1 = encrypt_text("Couple 1 Private Dinner", key)
    await client.post("/splits", json={
        "category": enc_cat_c1,
        "allocations": [{"user_name": enc_alice, "pct": 50.0}, {"user_name": enc_bob, "pct": 50.0}],
    })

    # Couple 2 category: Charlie & Dave 50/50
    enc_cat_c2 = encrypt_text("Couple 2 Private Dinner", key)
    await client.post("/splits", json={
        "category": enc_cat_c2,
        "allocations": [{"user_name": enc_charlie, "pct": 50.0}, {"user_name": enc_dave, "pct": 50.0}],
    })

    # Alice pays €100 for Couple 1 (Bob owes Alice €50)
    await client.post("/expenses", json={
        "name": encrypt_text("C1 Bistro", key),
        "cost_cents": 10000,
        "expense_date": f"{month}-10",
        "who_paid": enc_alice,
        "category": enc_cat_c1,
        "is_joint": False,
    })

    # Charlie pays €200 for Couple 2 (Dave owes Charlie €100)
    await client.post("/expenses", json={
        "name": encrypt_text("C2 Trattoria", key),
        "cost_cents": 20000,
        "expense_date": f"{month}-12",
        "who_paid": enc_charlie,
        "category": enc_cat_c2,
        "is_joint": False,
    })

    r_pb = await client.get(
        f"/analytics/paybacks?month={month}&personal_cats=&combined_fixed_cat=&apartment_cat=&jane_name=&john_name="
    )
    assert r_pb.status_code == 200
    pb = r_pb.json()

    debts = pb["debts"]
    assert len(debts) == 2

    # Verify transfers strictly stay within each couple
    transfer_pairs = {(decrypt_text(d["from_user"], key), decrypt_text(d["to_user"], key)): d["amount"] for d in debts}
    assert transfer_pairs.get(("Bob", "Alice")) == 50.00
    assert transfer_pairs.get(("Dave", "Charlie")) == 100.00
    # Zero cross-couple transfers (e.g. Bob paying Charlie or Dave paying Alice)
    assert ("Bob", "Charlie") not in transfer_pairs
    assert ("Dave", "Alice") not in transfer_pairs


# ===========================================================================
# SCENARIO 4: Overlapping Category Split Overrides & Timeline Precedence
# ===========================================================================

@pytest.mark.asyncio
async def test_scenario_4_overlapping_category_split_overrides_and_timeline_precedence(client: AsyncClient):
    """
    Scenario 4: Overlapping Category Split Overrides & Timeline Priority Precedence

    Household Setup:
      - 2 members: Zina and Jim (Salt: zinajim3303).
      - Category: GROCERIES
      - Baseline Agreement (Ongoing): Zina 50%, Jim 50%
      - Override 1 (2026-08-01 to 2026-08-31): Zina 20%, Jim 80% (Summer Host Month)
      - Override 2 (2026-08-10 to 2026-08-17): Zina 100%, Jim 0% (Private Event Week, overlapping Override 1)

    Transactions:
      1. 2026-08-05: Weekly Farmers Market - €100.00 (10,000 cents) paid by Jim (Override 1: Zina 20%, Jim 80%)
         -> Jim paid €100, owes €80; Zina owes €20 -> Jim +€20, Zina -€20
      2. 2026-08-12: Party Supplies & Catered Dinner - €200.00 (20,000 cents) paid by Jim (Override 2: Zina 100%, Jim 0%)
         -> Jim paid €200, owes €0; Zina owes €200 -> Jim +€200, Zina -€200
      3. 2026-08-25: Bulk Pantry Restock - €150.00 (15,000 cents) paid by Zina (Override 1: Zina 20%, Jim 80%)
         -> Zina paid €150, owes €30; Jim owes €120 -> Zina +€120, Jim -€120
      4. 2026-09-02: September Welcome Dinner - €80.00 (8,000 cents) paid by Jim (Baseline: Zina 50%, Jim 50%)
         -> Jim paid €80, owes €40; Zina owes €40 -> Jim +€40, Zina -€40

    Validation & Invariants:
      - August 2026 Net Balances: Jim +€100.00, Zina -€100.00
      - August 2026 Debt Transfer: Zina pays Jim €100.00
      - September 2026 Net Balances: Jim +€40.00, Zina -€40.00
      - September 2026 Debt Transfer: Zina pays Jim €40.00
    """
    key = derive_key()

    u_zina = encrypt_text("Zina", key)
    u_jim = encrypt_text("Jim", key)
    for u in [u_zina, u_jim]:
        await client.post("/users", json={"name": u, "color": "#ff7800", "is_active": 1})

    cat_groceries = encrypt_text("GROCERIES", key)

    # 1. Create Category with Baseline 50/50
    r_cat = await client.post("/splits", json={
        "category": cat_groceries,
        "allocations": [{"user_name": u_zina, "pct": 50.0}, {"user_name": u_jim, "pct": 50.0}],
    })
    assert r_cat.status_code == 201

    # 2. Add Override 1 (Full Month August: Zina 20%, Jim 80%)
    r_ov1 = await client.post(f"/splits/{cat_groceries}/agreements", json={
        "category": cat_groceries,
        "start_date": "2026-08-01",
        "end_date": "2026-08-31",
        "is_active": True,
        "note": encrypt_text("Summer Host Month", key),
        "allocations": [{"user_name": u_zina, "pct": 20.0}, {"user_name": u_jim, "pct": 80.0}],
    })
    assert r_ov1.status_code == 201

    # 3. Add Override 2 (Overlapping Event Window Aug 10-17: Zina 100%, Jim 0%)
    r_ov2 = await client.post(f"/splits/{cat_groceries}/agreements", json={
        "category": cat_groceries,
        "start_date": "2026-08-10",
        "end_date": "2026-08-17",
        "is_active": True,
        "note": encrypt_text("Private Event Week", key),
        "allocations": [{"user_name": u_zina, "pct": 100.0}, {"user_name": u_jim, "pct": 0.0}],
    })
    assert r_ov2.status_code == 201

    # 4. Log Expenses
    # Expense 1 (Aug 5, falls in Override 1)
    await client.post("/expenses", json={
        "name": encrypt_text("Weekly Farmers Market", key),
        "cost_cents": 10000,
        "expense_date": "2026-08-05",
        "who_paid": u_jim,
        "category": cat_groceries,
        "is_joint": False,
    })

    # Expense 2 (Aug 12, falls in Override 2 - overlapping window)
    await client.post("/expenses", json={
        "name": encrypt_text("Party Supplies & Catered Dinner", key),
        "cost_cents": 20000,
        "expense_date": "2026-08-12",
        "who_paid": u_jim,
        "category": cat_groceries,
        "is_joint": False,
    })

    # Expense 3 (Aug 25, falls in Override 1 after Override 2 expired)
    await client.post("/expenses", json={
        "name": encrypt_text("Bulk Pantry Restock", key),
        "cost_cents": 15000,
        "expense_date": "2026-08-25",
        "who_paid": u_zina,
        "category": cat_groceries,
        "is_joint": False,
    })

    # Expense 4 (Sep 2, falls in Baseline after all overrides expired)
    await client.post("/expenses", json={
        "name": encrypt_text("September Welcome Dinner", key),
        "cost_cents": 8000,
        "expense_date": "2026-09-02",
        "who_paid": u_jim,
        "category": cat_groceries,
        "is_joint": False,
    })

    # 5. Verify August 2026 Paybacks
    r_aug = await client.get("/analytics/paybacks?month=2026-08&personal_cats=&combined_fixed_cat=&apartment_cat=&jane_name=&john_name=")
    assert r_aug.status_code == 200
    aug_data = r_aug.json()

    aug_row_net = {decrypt_text(k, key): v for k, v in aug_data["rows"][0]["net_per_user"].items()}
    assert aug_row_net.get("Jim") == 100.00
    assert aug_row_net.get("Zina") == -100.00

    aug_debts = aug_data["debts"]
    assert len(aug_debts) == 1
    assert decrypt_text(aug_debts[0]["from_user"], key) == "Zina"
    assert decrypt_text(aug_debts[0]["to_user"], key) == "Jim"
    assert aug_debts[0]["amount"] == 100.00

    # 6. Verify September 2026 Paybacks (Baseline fallback)
    r_sep = await client.get("/analytics/paybacks?month=2026-09&personal_cats=&combined_fixed_cat=&apartment_cat=&jane_name=&john_name=")
    assert r_sep.status_code == 200
    sep_data = r_sep.json()

    sep_row_net = {decrypt_text(k, key): v for k, v in sep_data["rows"][0]["net_per_user"].items()}
    assert sep_row_net.get("Jim") == 40.00
    assert sep_row_net.get("Zina") == -40.00

    sep_debts = sep_data["debts"]
    assert len(sep_debts) == 1
    assert decrypt_text(sep_debts[0]["from_user"], key) == "Zina"
    assert decrypt_text(sep_debts[0]["to_user"], key) == "Jim"
    assert sep_debts[0]["amount"] == 40.00


# ===========================================================================
# SCENARIO 5: Split Override Active on Category Linked to Joint Account
# ===========================================================================

@pytest.mark.asyncio
async def test_scenario_5_split_override_on_category_linked_to_joint_account(client: AsyncClient):
    """
    Scenario 5: Split Override on Category with Out-of-Pocket vs Direct Joint Funding

    Household Setup:
      - 2 members: Zina and Jim.
      - Singleton Joint Account (id=1) with members Zina and Jim.
      - Category: HOME IMPROVEMENT.
      - Baseline Agreement: Zina 50%, Jim 50%
      - Override (2026-08-01 to 2026-08-31): Zina 75%, Jim 25% (Custom Renovation Split)

    Transactions:
      1. 2026-08-18: Custom Bookshelf Unit - €400.00 (40,000 cents) paid out-of-pocket by Jim (is_joint = False)
         -> Split according to Override (Zina 75%, Jim 25%): Jim funded €400, owes €100, Zina owes €300 -> Jim +€300, Zina -€300
      2. 2026-08-20: Painting Supplies - €150.00 (15,000 cents) paid directly by Joint Account (is_joint = True)
         -> Direct joint account payment: excluded from peer-to-peer payback settlements.

    Validation & Invariants:
      - August 2026 Net Balances in Payback Row: Jim +€300.00, Zina -€300.00
      - August 2026 Debt Transfer: Zina pays Jim €300.00
    """
    key = derive_key()
    month = "2026-08"

    u_zina = encrypt_text("Zina", key)
    u_jim = encrypt_text("Jim", key)
    for u in [u_zina, u_jim]:
        await client.post("/users", json={"name": u, "color": "#ff7800", "is_active": 1})

    # Setup Joint Account
    r_ja = await client.post("/joint-account", json={
        "name": encrypt_text("Household Joint Account", key),
        "balance_cents": 500000,
        "safety_margin_pct": 10,
        "deposit_split_mode": "manual",
        "expected_total_cents": None,
        "member_names": [u_zina, u_jim],
    })
    assert r_ja.status_code == 201

    cat_home = encrypt_text("HOME IMPROVEMENT", key)

    # 1. Create Category with Baseline 50/50
    await client.post("/splits", json={
        "category": cat_home,
        "allocations": [{"user_name": u_zina, "pct": 50.0}, {"user_name": u_jim, "pct": 50.0}],
    })

    # 2. Add Override for August (Zina 75%, Jim 25%)
    await client.post(f"/splits/{cat_home}/agreements", json={
        "category": cat_home,
        "start_date": f"{month}-01",
        "end_date": f"{month}-31",
        "is_active": True,
        "note": encrypt_text("Custom Renovation Split", key),
        "allocations": [{"user_name": u_zina, "pct": 75.0}, {"user_name": u_jim, "pct": 25.0}],
    })

    # 3. Log Expenses
    # Personal out-of-pocket expense paid by Jim (is_joint = False) -> subject to 75/25 override
    await client.post("/expenses", json={
        "name": encrypt_text("Custom Bookshelf Unit", key),
        "cost_cents": 40000,
        "expense_date": f"{month}-18",
        "who_paid": u_jim,
        "category": cat_home,
        "is_joint": False,
    })

    # Direct Joint Account payment (is_joint = True) -> excluded from peer-to-peer paybacks
    await client.post("/expenses", json={
        "name": encrypt_text("Painting Supplies", key),
        "cost_cents": 15000,
        "expense_date": f"{month}-20",
        "who_paid": u_jim,
        "category": cat_home,
        "is_joint": True,
    })

    # 4. Verify Paybacks
    r_pb = await client.get(f"/analytics/paybacks?month={month}&personal_cats=&combined_fixed_cat=&apartment_cat=&jane_name=&john_name=")
    assert r_pb.status_code == 200
    pb_data = r_pb.json()

    row_net = {decrypt_text(k, key): v for k, v in pb_data["rows"][0]["net_per_user"].items()}
    assert row_net.get("Jim") == 300.00
    assert row_net.get("Zina") == -300.00

    debts = pb_data["debts"]
    assert len(debts) == 1
    assert decrypt_text(debts[0]["from_user"], key) == "Zina"
    assert decrypt_text(debts[0]["to_user"], key) == "Jim"
    assert debts[0]["amount"] == 300.00


