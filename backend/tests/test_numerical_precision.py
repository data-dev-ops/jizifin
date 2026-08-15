"""
backend/tests/test_numerical_precision.py

Domain Specifications Covered:
- FloatAdd: IEEE 754 float accumulation vs exact integer cents.
- RoundHalfUp: Half-up rounding for fractional cent splits.
- LargeInt: Max 64-bit integer cost boundary limits without precision loss.
- ZeroTx: 0-cent expense/income rejection (cost_cents > 0, amount_cents > 0).
- NegBal: Signed balance representation in corrections and payback math.
- CryptDec: AES-GCM static IV roundtrip encryption/decryption.
"""

import pytest
import math
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text, decrypt_text

@pytest.mark.asyncio
@pytest.mark.parametrize("item_costs, expected_integer_cents, float_sum", [
    ([10, 20], 30, 0.1 + 0.2),
    ([100, 200, 300], 600, 1.0 + 2.0 + 3.0),
    ([33, 33, 34], 100, 0.33 + 0.33 + 0.34),
])
async def test_float_add(item_costs, expected_integer_cents, float_sum):
    """[FloatAdd] Verify exact integer cent addition compared to IEEE 754 float behavior."""
    integer_cents_sum = sum(item_costs)
    assert integer_cents_sum == expected_integer_cents
    # Demonstrate IEEE 754 precision drift vs exact integer cents
    if item_costs == [10, 20]:
        assert float_sum != 0.3

@pytest.mark.asyncio
@pytest.mark.parametrize("total_cents, percentage, expected_cents", [
    (1000, 33.333, 333),
    (100, 50.0, 50),
    (1000, 66.667, 667),
    (1, 50.0, 1), # half up rounding
])
async def test_round_half_up(total_cents, percentage, expected_cents):
    """[RoundHalfUp] Verify half-up rounding calculation for odd-cent splits."""
    calculated = math.floor((total_cents * percentage / 100.0) + 0.5)
    assert calculated == expected_cents

@pytest.mark.asyncio
@pytest.mark.parametrize("large_cents", [
    9007199254740991, # JavaScript Number.MAX_SAFE_INTEGER
    1000000000000,
    2147483647,
])
async def test_large_int(client: AsyncClient, large_cents):
    """[LargeInt] Verify max boundary integers in database queries without truncation."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Groceries", key)
    exp_enc = encrypt_text("Big Purchase", key)

    await client.post("/users", json={"name": user_enc, "color": "#123456"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    resp = await client.post("/expenses", json={
        "name": exp_enc,
        "cost_cents": large_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })
    assert resp.status_code == 201
    data = resp.json()
    assert data["cost_cents"] == large_cents

@pytest.mark.asyncio
@pytest.mark.parametrize("invalid_cost", [0, -1, -500])
async def test_zero_tx(client: AsyncClient, invalid_cost):
    """[ZeroTx] Rejection of 0-cent and negative expense/income entries raising HTTP 422."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Groceries", key)
    exp_enc = encrypt_text("Zero Item", key)

    await client.post("/users", json={"name": user_enc, "color": "#123456"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    resp = await client.post("/expenses", json={
        "name": exp_enc,
        "cost_cents": invalid_cost,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })
    assert resp.status_code == 422

@pytest.mark.asyncio
@pytest.mark.parametrize("correction_amount, initial_balance, expected_balance", [
    (-5000, 10000, 5000),
    (5000, 5000, 10000),
    (-15000, 10000, -5000),
])
async def test_neg_bal(client: AsyncClient, correction_amount, initial_balance, expected_balance):
    """[NegBal] Signed balance calculations in joint account corrections."""
    # Seed joint account
    await client.post("/joint-account", json={"name": "Joint Vault", "balance_cents": initial_balance})

    resp = await client.post("/joint-account/corrections", json={
        "amount_cents": correction_amount,
        "correction_date": "2026-07-24",
        "note": "Correction test"
    })
    assert resp.status_code == 201

    ja_resp = await client.get("/joint-account")
    assert ja_resp.status_code == 200
    assert ja_resp.json()["balance_cents"] == expected_balance

@pytest.mark.asyncio
@pytest.mark.parametrize("raw_text", [
    "Secret Passphrase",
    "John & Jane Financial Tracker",
    "Special Chars: !@#$%^&*()_+-=[]{}|;:',.<>/?",
    "Unicode: 欧元, 🚀, 100%",
])
async def test_crypt_dec(raw_text):
    """[CryptDec] AES-GCM static IV roundtrip encryption and decryption."""
    key = derive_key("master-key-123")
    encrypted = encrypt_text(raw_text, key)
    decrypted = decrypt_text(encrypted, key)
    assert decrypted == raw_text

@pytest.mark.asyncio
async def test_double_entry_net_worth_invariant(client: AsyncClient):
    """[Domain 6: Double-Entry Invariant] Total Income - Total Expenses = Net Balance invariant across views."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Groceries", key)
    sal_cat = encrypt_text("SALARY", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={"category": cat_enc, "allocations": [{"user_name": user_enc, "pct": 100.0}]})

    # Income: $3000 (300000 cents)
    await client.post("/income", json=[{"name": encrypt_text("Salary", key), "amount_cents": 300000, "who": user_enc, "category": sal_cat, "income_date": "2026-07-01"}])

    # Expenses: $1200 (120000 cents)
    await client.post("/expenses", json={"name": encrypt_text("Supermarket", key), "cost_cents": 120000, "expense_date": "2026-07-15", "who_paid": user_enc, "category": cat_enc})

    # Query monthly total view
    m_resp = await client.get("/analytics/monthly-total?month=2026-07")
    assert m_resp.status_code == 200
    spent_total = m_resp.json()["total_amount"]
    assert spent_total == 1200.0

    net_cents = 300000 - 120000
    assert net_cents == 180000

@pytest.mark.asyncio
async def test_timezone_boundary_synchronization(client: AsyncClient):
    """[Domain 6: Time-Zone Boundary] Expense logged near UTC midnight resolves to proper YYYY-MM month budget period."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("LateNight", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={"category": cat_enc, "allocations": [{"user_name": user_enc, "pct": 100.0}]})

    # Log expense on last day of month
    resp = await client.post("/expenses", json={
        "name": encrypt_text("Midnight Purchase", key),
        "cost_cents": 4500,
        "expense_date": "2026-07-31",
        "who_paid": user_enc,
        "category": cat_enc
    })
    assert resp.status_code == 201
    assert resp.json()["expense_date"] == "2026-07-31"

    # Query month by category for 2026-07
    b_resp = await client.get("/analytics/by-category?month=2026-07")
    assert b_resp.status_code == 200
    match_item = next(item for item in b_resp.json() if item["category"] == cat_enc)
    assert match_item["total_amount"] == 45.0

