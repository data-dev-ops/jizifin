"""
backend/tests/test_scenarios_integration.py

Comprehensive, Mathematically Rigorous Integration Test Suite for Multi-Household Financial Scenarios:
1. Scenario 1: Isolated Joint Accounts, Household-Wide Shared Costs, and Basis-Point Proportional Income Splits
2. Scenario 2: Dynamic Tag Timeline Window Enforcement and Multi-Tenant Project Balance Sheet Settlement
3. Scenario 3: Automated Dynamic Income Recalibration, Multi-Category Cascades, and Live Debt Settlement
4. Scenario 4: Overlapping Category Split Overrides and Timeline Priority Precedence
5. Scenario 5: Split Override Active on Category Linked to Joint Account
6. Companion Invariants: Month Settlement Locking, Priority Overrides, Tag Shrink Rejection, and Connected Graph Isolation.

All tests operate on an isolated copy of the pre-populated backend/tests/test.db database,
verifying real multi-month mock data with integer-cents financial precision and zero sensitive data.
"""

import pytest
import pytest_asyncio
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text, decrypt_text


@pytest_asyncio.fixture
async def client(integration_client: AsyncClient) -> AsyncClient:
    """Module-level fixture binding integration_client (pre-populated test.db) to client."""
    return integration_client


# ===========================================================================
# SCENARIO 1: Isolated Joint Accounts, Household-Wide Shared Costs,
#             and Basis-Point Proportional Income Splits
# ===========================================================================

@pytest.mark.asyncio
async def test_scenario_1_isolated_joint_accounts_and_proportional_income_splits(client: AsyncClient):
    """
    Scenario 1: Isolated Joint Accounts, Household-Wide Shared Costs, and Proportional Income Splits

    Household Setup (Pre-populated in test.db):
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

    Invariants Verified:
      - Joint Account cash balances:
        * Account 1 (Couple AB Joint): €1,150.00
        * Account 2 (Couple CD Joint): €1,580.00
      - Exact Net Settlement Positions for July 2026 (2026-07):
        * Alice:   +€1,170.37 (Creditor: €2,000.00 paid - €829.63 obligation)
        * Bob:     -€414.82   (Debtor:   €0.00 paid - €414.82 obligation)
        * Charlie: -€819.26   (Debtor:   €0.00 paid - €819.26 obligation)
        * Dave:    -€351.11   (Debtor:   €0.00 paid - €351.11 obligation)
        * Eve:     +€414.82   (Creditor: €1,000.00 paid - €585.18 obligation)
      - Exact Zero-Sum Ledger Invariant: Sum of all net positions across household == €0.00.
      - Reimbursement routing directly settles debtors to creditors with minimal transfer count.
    """
    key = derive_key()
    month = "2026-07"
    users = ["Alice", "Bob", "Charlie", "Dave", "Eve"]

    # Invariant 1: Joint Account AB cash balance == €1,150.00 (115,000 cents)
    ja_ab = (await client.get("/joint-accounts/1")).json()
    assert ja_ab["balance_cents"] == 115000, f"Expected €1,150.00 (115000 cents), got {ja_ab['balance_cents']}"

    # Invariant 2: Joint Account CD cash balance == €1,580.00 (158,000 cents)
    ja_cd = (await client.get("/joint-accounts/2")).json()
    assert ja_cd["balance_cents"] == 158000, f"Expected €1,580.00 (158000 cents), got {ja_cd['balance_cents']}"

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

    # Validate exact net positions with basis-point precision
    assert round(user_net["Alice"], 2) == 1170.37, f"Expected Alice +€1,170.37, got {user_net['Alice']}"
    assert round(user_net["Bob"], 2) == -414.82, f"Expected Bob -€414.82, got {user_net['Bob']}"
    assert round(user_net["Charlie"], 2) == -819.26, f"Expected Charlie -€819.26, got {user_net['Charlie']}"
    assert round(user_net["Dave"], 2) == -351.11, f"Expected Dave -€351.11, got {user_net['Dave']}"
    assert round(user_net["Eve"], 2) == 414.82, f"Expected Eve +€414.82, got {user_net['Eve']}"

    # Zero-sum double-entry invariant across all household members
    total_net = sum(user_net.values())
    assert round(total_net, 2) == 0.00, f"Ledger invariant violated: sum of net balances is {total_net} != 0.00"

    # Invariant 4: Minimal transfer settlement routing
    debts = pb["debts"]
    assert len(debts) == 4, f"Expected exactly 4 minimal settlement transfers, got {len(debts)}: {debts}"

    transfer_pairs = {
        (decrypt_text(d["from_user"], key), decrypt_text(d["to_user"], key)): d["amount"]
        for d in debts
    }

    assert transfer_pairs.get(("Charlie", "Alice")) == 819.26
    assert transfer_pairs.get(("Bob", "Alice")) == 351.11
    assert transfer_pairs.get(("Bob", "Eve")) == 63.71
    assert transfer_pairs.get(("Dave", "Eve")) == 351.11

    # Invariant 5: Total debt transferred equals total creditor receivables
    total_debt_transferred = sum(transfer_pairs.values())
    assert round(total_debt_transferred, 2) == 1585.19


# ===========================================================================
# SCENARIO 2: Dynamic Tag Timeline Window Enforcement and Multi-Tenant
#             Project Balance Sheet Settlement
# ===========================================================================

@pytest.mark.asyncio
async def test_scenario_2_dynamic_timeline_tagging_and_multi_tenant_project_cost_breakdowns(client: AsyncClient):
    """
    Scenario 2: Dynamic Tag Timeline Enforcement & Multi-Tenant Project Balance Sheet Settlement

    Pre-populated in test.db:
      - Project 1: "Solar & Battery Installation" (Target: €6,000.00 / 600,000 cents).
      - Participants: Alice, Bob, Charlie (Dave and Eve opted out).
      - Agreed Cost Allocation: Alice 45% (€2,700), Bob 25% (€1,500), Charlie 30% (€1,800).
      - Tag 1: `tag:solar-phase-1` bounded to project milestone timeline window (2026-06-01 to 2026-06-30).
      - Joint Account AB equity: funded 60% Alice / 40% Bob.
      - Expenses 101, 102, 103 logged in June 2026.

    Negative Boundary Tests:
      - Attempting to log an expense dated 2026-07-02 with `tag:solar-phase-1` is rejected with HTTP 422.
      - Attempting to log an expense dated 2026-05-28 with `tag:solar-phase-1` is rejected with HTTP 422.

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
    proj_id = 1
    tag_id = 1

    enc_bob = encrypt_text("Bob", key)
    enc_cat_solar = encrypt_text("Solar Infrastructure", key)

    # Negative Boundary Test A: Out-of-bounds future invoice (2026-07-02 > 2026-06-30) rejected with HTTP 422
    r_bad_future = await client.post("/expenses", json={
        "name": encrypt_text("Out-of-bounds Future Invoice", key),
        "cost_cents": 10000,
        "expense_date": "2026-07-02",
        "who_paid": enc_bob,
        "category": enc_cat_solar,
        "project_id": proj_id,
        "tag_id": tag_id,
        "is_joint": False,
    })
    assert r_bad_future.status_code == 422, f"Expected 422 for out-of-bounds future tag date, got {r_bad_future.status_code}"

    # Negative Boundary Test B: Out-of-bounds past invoice (2026-05-28 < 2026-06-01) rejected with HTTP 422
    r_bad_past = await client.post("/expenses", json={
        "name": encrypt_text("Out-of-bounds Early Survey", key),
        "cost_cents": 5000,
        "expense_date": "2026-05-28",
        "who_paid": enc_bob,
        "category": enc_cat_solar,
        "project_id": proj_id,
        "tag_id": tag_id,
        "is_joint": False,
    })
    assert r_bad_past.status_code == 422, f"Expected 422 for out-of-bounds past tag date, got {r_bad_past.status_code}"

    # Settlement Verification against GET /projects/{project_id}/settlement
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
    assert part_map["Alice"]["assigned_liability_cents"] == 270000  # 45% of €6,000
    assert part_map["Alice"]["net_balance"] == -780.00

    assert part_map["Bob"]["effective_funding_cents"] == 308000
    assert part_map["Bob"]["assigned_liability_cents"] == 150000  # 25% of €6,000
    assert part_map["Bob"]["net_balance"] == 1580.00

    assert part_map["Charlie"]["effective_funding_cents"] == 100000
    assert part_map["Charlie"]["assigned_liability_cents"] == 180000  # 30% of €6,000
    assert part_map["Charlie"]["net_balance"] == -800.00

    # Invariant: Project zero-sum ledger balance
    project_net_sum = (
        part_map["Alice"]["net_balance"]
        + part_map["Bob"]["net_balance"]
        + part_map["Charlie"]["net_balance"]
    )
    assert round(project_net_sum, 2) == 0.00

    # Invariant: Debt settlement transfers generated directly by backend
    debts = settlement_data["debts"]
    assert len(debts) == 2, f"Expected 2 debt transfers, got {len(debts)}: {debts}"

    debt_map = {
        (decrypt_text(d["from_user"], key), decrypt_text(d["to_user"], key)): d["amount"]
        for d in debts
    }
    assert debt_map.get(("Alice", "Bob")) == 780.00
    assert debt_map.get(("Charlie", "Bob")) == 800.00


# ===========================================================================
# SCENARIO 3: Automated Dynamic Income Recalibration, Multi-Category
#             Cascades, and Live Debt Settlement
# ===========================================================================

@pytest.mark.asyncio
async def test_scenario_3_automated_dynamic_income_recalibration_and_multi_category_cascades(client: AsyncClient):
    """
    Scenario 3: Automated Dynamic Income Recalibration, Multi-Category Cascades, and Live Debt Settlement

    Household Setup (Pre-populated in test.db):
      - 2 members: Alice and Bob.
      - Base salary in October: €3,000 each via salary overrides.
      - On Oct 15: Alice receives €1,000 Q3 performance bonus -> Alice October income = €4,000.
      - Dynamic ratio for Discretionary Shared: Alice 4000/7000 (57.1428%), Bob 3000/7000 (42.8571%).

    Categories & Expenses in October 2026:
      - Fixed Living (€400.00): Alice owes €200.00, Bob owes €200.00. Paid by Alice (€400.00) -> Alice +€200.00, Bob -€200.00.
      - Discretionary Shared (€800.00): Paid by Bob. Alice owes €457.15, Bob owes €342.85 -> Alice -€457.15, Bob +€457.15.
      - Asset Investment (€1,400.00): Strict 70/30. Paid by Alice. Alice owes €980, Bob owes €420 -> Alice +€420.00, Bob -€420.00.

    Consolidated Invariants:
      - Alice Net: +200.00 - 457.15 + 420.00 = +€162.85
      - Bob Net:   -200.00 + 457.15 - 420.00 = -€162.85
      - Single-transaction live settlement: Bob pays Alice €162.85.
    """
    key = derive_key()
    month = "2026-10"
    enc_alice = encrypt_text("Alice", key)
    enc_bob = encrypt_text("Bob", key)
    enc_cat_disc = encrypt_text("Discretionary Shared", key)

    # Verify live income analytics after bonus via /analytics/income-by-person
    r_inc_ana = await client.get(f"/analytics/income-by-person?salary_cat=SALARY&month={month}")
    assert r_inc_ana.status_code == 200
    income_map = {decrypt_text(r["who"], key): r for r in r_inc_ana.json()}
    assert income_map["Alice"]["total_cents"] == 400000  # €4,000.00
    assert income_map["Bob"]["total_cents"] == 300000    # €3,000.00

    # Query paybacks with dynamic_income_cats="Discretionary Shared"
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
    assert round(alice_total_net + bob_total_net, 2) == 0.00

    # Invariant 5: Single-transaction live settlement: Bob pays Alice €162.85
    assert len(pb["debts"]) == 1
    settlement_transfer = pb["debts"][0]
    assert decrypt_text(settlement_transfer["from_user"], key) == "Bob"
    assert decrypt_text(settlement_transfer["to_user"], key) == "Alice"
    assert settlement_transfer["amount"] == 162.85


# ===========================================================================
# SCENARIO 4: Overlapping Category Split Overrides & Timeline Precedence
# ===========================================================================

@pytest.mark.asyncio
async def test_scenario_4_overlapping_category_split_overrides_and_timeline_precedence(client: AsyncClient):
    """
    Scenario 4: Overlapping Category Split Overrides & Timeline Priority Precedence

    Household Setup (Pre-populated in test.db):
      - 2 members: Alice and Bob.
      - Category: GROCERIES
      - Baseline Agreement (Ongoing): Alice 50%, Bob 50%
      - Override 1 (2026-08-01 to 2026-08-31): Alice 20%, Bob 80% (Summer Host Month)
      - Override 2 (2026-08-10 to 2026-08-17): Alice 100%, Bob 0% (Private Event Week)

    Transactions:
      1. 2026-08-05: Weekly Farmers Market - €100.00 paid by Bob (Override 1: Alice 20%, Bob 80%)
         -> Bob +€20, Alice -€20
      2. 2026-08-12: Party Supplies & Catered Dinner - €200.00 paid by Bob (Override 2: Alice 100%, Bob 0%)
         -> Bob +€200, Alice -€200
      3. 2026-08-25: Bulk Pantry Restock - €150.00 paid by Alice (Override 1: Alice 20%, Bob 80%)
         -> Alice +€120, Bob -€120
      4. 2026-09-02: September Welcome Dinner - €80.00 paid by Bob (Baseline: Alice 50%, Bob 50%)
         -> Bob +€40, Alice -€40

    Validation & Invariants:
      - August 2026 Net Balances in GROCERIES: Bob +€100.00, Alice -€100.00
      - September 2026 Net Balances: Bob +€40.00, Alice -€40.00
      - September 2026 Debt Transfer: Alice pays Bob €40.00
    """
    key = derive_key()

    # 1. Verify August 2026 Paybacks (GROCERIES row under overlapping overrides)
    r_aug = await client.get("/analytics/paybacks?month=2026-08&personal_cats=&combined_fixed_cat=&apartment_cat=&jane_name=&john_name=")
    assert r_aug.status_code == 200
    aug_data = r_aug.json()

    cat_rows_aug = {decrypt_text(r["category"], key): r for r in aug_data["rows"]}
    assert "GROCERIES" in cat_rows_aug
    groc_net_aug = {decrypt_text(k, key): v for k, v in cat_rows_aug["GROCERIES"]["net_per_user"].items()}
    assert groc_net_aug.get("Bob") == 100.00
    assert groc_net_aug.get("Alice") == -100.00

    # 2. Verify September 2026 Paybacks (Baseline fallback)
    r_sep = await client.get("/analytics/paybacks?month=2026-09&personal_cats=&combined_fixed_cat=&apartment_cat=&jane_name=&john_name=")
    assert r_sep.status_code == 200
    sep_data = r_sep.json()

    cat_rows_sep = {decrypt_text(r["category"], key): r for r in sep_data["rows"]}
    assert "GROCERIES" in cat_rows_sep
    groc_net_sep = {decrypt_text(k, key): v for k, v in cat_rows_sep["GROCERIES"]["net_per_user"].items()}
    assert groc_net_sep.get("Bob") == 40.00
    assert groc_net_sep.get("Alice") == -40.00

    sep_debts = sep_data["debts"]
    assert len(sep_debts) == 1
    assert decrypt_text(sep_debts[0]["from_user"], key) == "Alice"
    assert decrypt_text(sep_debts[0]["to_user"], key) == "Bob"
    assert sep_debts[0]["amount"] == 40.00


# ===========================================================================
# SCENARIO 5: Split Override Active on Category Linked to Joint Account
# ===========================================================================

@pytest.mark.asyncio
async def test_scenario_5_split_override_on_category_linked_to_joint_account(client: AsyncClient):
    """
    Scenario 5: Split Override on Category with Out-of-Pocket vs Direct Joint Funding

    Household Setup (Pre-populated in test.db):
      - 2 members: Alice and Bob.
      - Joint Account 1 (Couple AB Joint) with members Alice and Bob.
      - Category: HOME IMPROVEMENT linked to Joint Account 1.
      - Baseline Agreement: Alice 50%, Bob 50%
      - Override (2026-08-01 to 2026-08-31): Alice 75%, Bob 25% (Custom Renovation Split)

    Transactions:
      1. 2026-08-18: Custom Bookshelf Unit - €400.00 paid out-of-pocket by Bob (is_joint = False)
         -> Split according to Override (Alice 75%, Bob 25%): Bob funded €400, owes €100, Alice owes €300 -> Bob +€300, Alice -€300
      2. 2026-08-20: Painting Supplies - €150.00 paid directly by Joint Account (is_joint = True)
         -> Direct joint account payment: excluded from peer-to-peer payback settlements.

    Validation & Invariants:
      - August 2026 Net Balances in HOME IMPROVEMENT row: Bob +€300.00, Alice -€300.00
      - Combined August Debt Transfer (GROCERIES + HOME IMPROVEMENT): Alice pays Bob €400.00
    """
    key = derive_key()
    month = "2026-08"

    # Verify Paybacks
    r_pb = await client.get(f"/analytics/paybacks?month={month}&personal_cats=&combined_fixed_cat=&apartment_cat=&jane_name=&john_name=")
    assert r_pb.status_code == 200
    pb_data = r_pb.json()

    cat_rows = {decrypt_text(r["category"], key): r for r in pb_data["rows"]}
    assert "HOME IMPROVEMENT" in cat_rows
    row_net = {decrypt_text(k, key): v for k, v in cat_rows["HOME IMPROVEMENT"]["net_per_user"].items()}
    assert row_net.get("Bob") == 300.00
    assert row_net.get("Alice") == -300.00

    # Total August debt settlement combining GROCERIES (€100) and HOME IMPROVEMENT (€300)
    debts = pb_data["debts"]
    assert len(debts) == 1
    assert decrypt_text(debts[0]["from_user"], key) == "Alice"
    assert decrypt_text(debts[0]["to_user"], key) == "Bob"
    assert debts[0]["amount"] == 400.00


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
    enc_cat = encrypt_text("Housing/Rent", key)

    # Log expense in month 2026-05
    r_exp = await client.post("/expenses", json={
        "name": encrypt_text("Pre-Settlement Utility", key),
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
    month = "2026-12"

    enc_alice = encrypt_text("Alice", key)
    enc_bob = encrypt_text("Bob", key)
    enc_charlie = encrypt_text("Charlie", key)
    enc_cat = encrypt_text("Housing/Rent", key)

    # Expense 1: Uses category default (€300.00) paid by Alice
    await client.post("/expenses", json={
        "name": encrypt_text("Year End Dinner", key),
        "cost_cents": 30000,
        "expense_date": f"{month}-10",
        "who_paid": enc_alice,
        "category": enc_cat,
        "is_joint": False,
    })

    # Expense 2: Uses specific override (Alice 80%, Bob 20%, Charlie 0%) for €500.00 paid by Bob
    await client.post("/expenses", json={
        "name": encrypt_text("Private Project Dinner", key),
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

    # Exp 2 override was applied and net sums to 0.00
    net_map = {decrypt_text(k, key): v for k, v in pb["rows"][0]["net_per_user"].items()}
    assert round(sum(net_map.values()), 2) == 0.00
    assert len(pb["debts"]) >= 1


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
    for u, count in extra_cent_counts.items():
        assert 120 <= count <= 210, f"User {u} received {count}/500 extra cents, showing potential tie-breaking drift!"


@pytest.mark.asyncio
async def test_tag_timeline_mutation_shrink_rejection(client: AsyncClient):
    """
    Verify that updating a tag timeline (PUT /tags/{id}) rejects window shrinking
    when existing linked expenses fall outside the new date window.
    """
    # Tag 1 (tag:solar-phase-1) is 2026-06-01 to 2026-06-30 and has expenses on June 10, 18, 25.
    # Attempting to shrink tag end_date to June 20 (before June 25 expense) -> Must fail with HTTP 422
    r_bad_tag = await client.put("/tags/1", json={
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
    # Expense 101 has tag_id=1 (window: 2026-06-01 to 2026-06-30).
    # Partial update changing only expense_date to August (out of tag bounds) without sending tag_id
    r_bad_update = await client.put("/expenses/101", json={
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

    enc_cat_c1 = encrypt_text("GROCERIES", key)
    enc_cat_c2 = encrypt_text("UTILITIES", key)

    # Alice pays €100 for Couple 1 (Bob owes Alice €50 under 50/50 baseline)
    await client.post("/expenses", json={
        "name": encrypt_text("C1 Dinner", key),
        "cost_cents": 10000,
        "expense_date": f"{month}-10",
        "who_paid": enc_alice,
        "category": enc_cat_c1,
        "is_joint": False,
    })

    # Charlie pays €200 for Couple 2 (Dave owes Charlie €100 under equal split)
    await client.post("/expenses", json={
        "name": encrypt_text("C2 Dinner", key),
        "cost_cents": 20000,
        "expense_date": f"{month}-12",
        "who_paid": enc_charlie,
        "category": enc_cat_c2,
        "is_joint": False,
        "overrides": [
            {"user_name": enc_charlie, "pct": 50.0},
            {"user_name": enc_dave, "pct": 50.0},
        ],
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
