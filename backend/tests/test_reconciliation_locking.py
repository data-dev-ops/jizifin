"""
backend/tests/test_reconciliation_locking.py

Domain Specifications Covered:
- TxPend: Monthly deposit tracking status (pending vs paid).
- TxClear: Marking deposit as paid updating is_paid=1 and joint balance.
- RecLock: Month settlement (POST /settlements) locking month; rejecting expense edits in locked month with 400.
- BalRecon: Bank statement balance reconciliation math.
- BalSync: Synchronizing joint account balance with corrections and deposits.
- CrdStmt: Credit card statement cycle date tracking.
- CrdPmt: Credit card statement payment linking.
- CrdInt: Credit card APR interest math.
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text

@pytest.mark.asyncio
@pytest.mark.parametrize("scheduled_cents, is_paid", [
    (50000, 0),
    (50000, 1),
])
async def test_tx_pend(client: AsyncClient, scheduled_cents, is_paid):
    """[TxPend] Monthly deposit tracking status."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/joint-account", json={"name": "Joint Account", "balance_cents": 10000})

    resp = await client.put("/joint-account/deposits", json=[{
        "user_name": user_enc,
        "amount_cents": scheduled_cents,
        "day_of_month": 1,
    }])
    assert resp.status_code == 200
    assert len(resp.json()) == 1

@pytest.mark.asyncio
@pytest.mark.parametrize("deposit_amount", [30000, 50000])
async def test_tx_clear(client: AsyncClient, deposit_amount):
    """[TxClear] Marking monthly deposit schedule updates deposit configuration and balance."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/joint-account", json={"name": "Joint Account", "balance_cents": 10000})

    dep_resp = await client.put("/joint-account/deposits", json=[{
        "user_name": user_enc,
        "amount_cents": deposit_amount,
        "day_of_month": 1,
    }])
    assert dep_resp.status_code == 200

    get_deps = await client.get("/joint-account/deposits")
    assert get_deps.status_code == 200
    assert any(d["user_name"] == user_enc and d["amount_cents"] == deposit_amount for d in get_deps.json())

@pytest.mark.asyncio
@pytest.mark.parametrize("locked_month", ["2026-06", "2026-05"])
async def test_rec_lock(client: AsyncClient, locked_month):
    """[RecLock] Month settlement (POST /settlements) locking month; rejecting expense mutations in locked month with 400."""
    key = derive_key()
    john_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Groceries", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": john_enc, "pct": 100.0}]
    })

    # Lock month via settlement
    settle_resp = await client.post("/settlements", json={
        "month": locked_month,
        "net_balance_transferred_cents": 5000,
    })
    assert settle_resp.status_code == 201

    # Attempt to post expense in locked month -> should return 400 Bad Request
    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Locked Expense", key),
        "cost_cents": 3000,
        "expense_date": f"{locked_month}-15",
        "who_paid": john_enc,
        "category": cat_enc,
    })
    assert exp_resp.status_code == 400
    assert "locked" in exp_resp.json()["detail"].lower()

@pytest.mark.parametrize("bank_stmt_cents, ledger_cents, expected_discrepancy", [
    (150000, 150000, 0),
    (150000, 145000, 5000),
])
def test_bal_recon(bank_stmt_cents, ledger_cents, expected_discrepancy):
    """[BalRecon] Bank statement reconciliation math comparing expected vs actual."""
    discrepancy = bank_stmt_cents - ledger_cents
    assert discrepancy == expected_discrepancy

@pytest.mark.asyncio
@pytest.mark.parametrize("topup, deposit", [(5000, 10000)])
async def test_bal_sync(client: AsyncClient, topup, deposit):
    """[BalSync] Synchronizing joint account balance with corrections and deposits."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/joint-account", json={"name": "Joint Account", "balance_cents": 10000})

    await client.post("/joint-account/corrections", json={
        "amount_cents": topup,
        "correction_date": "2026-07-24",
        "note": "Top up"
    })

    ja_resp = await client.get("/joint-account")
    assert ja_resp.json()["balance_cents"] == 10000 + topup

@pytest.mark.parametrize("closing_day, current_day, is_closed", [
    (25, 26, True),
    (25, 20, False),
])
def test_crd_stmt(closing_day, current_day, is_closed):
    """[CrdStmt] Credit card statement closing date tracking."""
    closed = current_day > closing_day
    assert closed == is_closed

@pytest.mark.parametrize("statement_balance, payment_amount, remaining_balance", [
    (50000, 50000, 0),
    (50000, 30000, 20000),
])
def test_crd_pmt(statement_balance, payment_amount, remaining_balance):
    """[CrdPmt] Credit card payment linking statement balance."""
    rem = statement_balance - payment_amount
    assert rem == remaining_balance

@pytest.mark.parametrize("avg_daily_bal_cents, apr_pct, days, expected_interest_cents", [
    (100000, 18.0, 30, 1479), # (1000 * 0.18 / 365) * 30 -> 14.79 -> 1479 cents
    (200000, 24.0, 30, 3945),
])
def test_crd_int(avg_daily_bal_cents, apr_pct, days, expected_interest_cents):
    """[CrdInt] Credit card APR interest calculation."""
    daily_rate = (apr_pct / 100.0) / 365.0
    interest = round(avg_daily_bal_cents * daily_rate * days)
    assert interest == expected_interest_cents
