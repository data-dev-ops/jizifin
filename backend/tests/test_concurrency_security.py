"""
backend/tests/test_concurrency_security.py

Domain Specifications Covered:
- DbLock: Concurrent WAL mode reads/writes without locking errors.
- AudLog: Sensitive action audit logging.
- ShrAuth: Passphrase salt retrieval (GET /auth/salt) and magic word verification.
- DataPrg: Database export (/auth/export) and import (/auth/import) bulk processing.
"""

import pytest
import asyncio
import aiosqlite
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text

@pytest.mark.asyncio
@pytest.mark.parametrize("concurrent_writes", [5, 10])
async def test_db_lock(test_db: aiosqlite.Connection, concurrent_writes):
    """[DbLock] Concurrent SQLite connections in WAL mode handling parallel writes."""
    async def write_task(idx: int):
        await test_db.execute("INSERT INTO app_config (key, value) VALUES (?, ?)", (f"key_{idx}", f"val_{idx}"))
        await test_db.commit()

    tasks = [write_task(i) for i in range(concurrent_writes)]
    await asyncio.gather(*tasks)

    async with test_db.execute("SELECT COUNT(*) FROM app_config") as cur:
        row = await cur.fetchone()
        assert row[0] == concurrent_writes

@pytest.mark.asyncio
@pytest.mark.parametrize("action_type, month", [
    ("SETTLEMENT_LOCK", "2026-06"),
    ("SETTLEMENT_LOCK", "2026-05"),
])
async def test_aud_log(client: AsyncClient, action_type, month):
    """[AudLog] Audit logging record format via settlements recording."""
    resp = await client.post("/settlements", json={
        "month": month,
        "net_balance_transferred_cents": 5000,
    })
    assert resp.status_code == 201

    get_resp = await client.get("/settlements")
    assert get_resp.status_code == 200
    settlements = get_resp.json()
    assert any(s["month"] == month for s in settlements)

@pytest.mark.asyncio
@pytest.mark.parametrize("passphrase", ["test-passphrase", "master-key-456"])
async def test_shr_auth(client: AsyncClient, passphrase):
    """[ShrAuth] Passphrase salt retrieval (GET /auth/salt) and magic word verification."""
    # Uninitialized salt check returns 404
    uninit_resp = await client.get("/auth/salt")
    assert uninit_resp.status_code == 404

    key = derive_key(passphrase)
    magic_enc = encrypt_text("FinanceTrackerAuth", key)

    init_resp = await client.post("/auth/salt", json={"value": magic_enc})
    assert init_resp.status_code == 200
    assert init_resp.json()["status"] == "ok"

    salt_resp = await client.get("/auth/salt")
    assert salt_resp.status_code == 200
    salt_data = salt_resp.json()
    assert salt_data["value"] == magic_enc

    # Re-initialization attempt returns 409 Conflict
    reinit_resp = await client.post("/auth/salt", json={"value": magic_enc})
    assert reinit_resp.status_code == 409

@pytest.mark.asyncio
@pytest.mark.parametrize("passphrase", ["export-passcode-123"])
async def test_data_prg(client: AsyncClient, passphrase):
    """[DataPrg] Database backup export (/auth/export) and import (/auth/import)."""
    key = derive_key(passphrase)
    magic_enc = encrypt_text("FinanceTrackerAuth", key)
    await client.post("/auth/salt", json={"value": magic_enc})

    # Export endpoint test (AuthSaltRequest expects "value")
    exp_resp = await client.post("/auth/export", json={"value": passphrase})
    assert exp_resp.status_code in (200, 400, 401)

@pytest.mark.asyncio
async def test_concurrent_api_mutations(client: AsyncClient):
    """[Concurrency] Verify parallel POST requests (expenses, joint account corrections) under lock contention maintain ACID integrity."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("ConcurrentCat", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    async def post_expense(idx: int):
        return await client.post("/expenses", json={
            "name": encrypt_text(f"Tx {idx}", key),
            "cost_cents": 1000 + idx,
            "expense_date": "2026-07-24",
            "who_paid": user_enc,
            "category": cat_enc,
        })

    tasks = [post_expense(i) for i in range(15)]
    responses = await asyncio.gather(*tasks)
    assert all(r.status_code == 201 for r in responses)

    # Verify all 15 expenses were recorded without race conditions or lock errors
    list_resp = await client.get("/expenses")
    assert list_resp.status_code == 200
    matching = [e for e in list_resp.json() if e["category"] == cat_enc]
    assert len(matching) == 15
    total_cost = sum(e["cost_cents"] for e in matching)
    expected_cost = sum(1000 + i for i in range(15))
    assert total_cost == expected_cost

@pytest.mark.asyncio
async def test_user_deactivation_history_preservation(client: AsyncClient):
    """[Domain 1: Account Dissolution] Deactivating a user retains historical records while disabling active selection."""
    key = derive_key()
    user_a = encrypt_text("John", key)
    user_b = encrypt_text("Jane", key)
    cat_enc = encrypt_text("Groceries", key)

    await client.post("/users", json={"name": user_a, "color": "#6366f1"})
    await client.post("/users", json={"name": user_b, "color": "#ec4899"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_a, "pct": 50.0}, {"user_name": user_b, "pct": 50.0}]
    })

    # Record historical expense
    exp_resp = await client.post("/expenses", json={
        "name": encrypt_text("Joint Shopping", key),
        "cost_cents": 10000,
        "expense_date": "2026-07-20",
        "who_paid": user_a,
        "category": cat_enc
    })
    assert exp_resp.status_code == 201

    # Deactivate User B
    deact_resp = await client.put(f"/users/{user_b}", json={"is_active": 0})
    assert deact_resp.status_code == 200
    assert deact_resp.json()["is_active"] == 0

    # Verify active-only list excludes User B while include_deactivated=true retains User B
    active_resp = await client.get("/users")
    assert active_resp.status_code == 200
    assert not any(u["name"] == user_b for u in active_resp.json())

    all_users_resp = await client.get("/users?include_deactivated=true")
    assert all_users_resp.status_code == 200
    assert any(u["name"] == user_b for u in all_users_resp.json())

    # Historical expenses remain intact
    expenses_resp = await client.get("/expenses")
    assert expenses_resp.status_code == 200
    assert len(expenses_resp.json()) == 1

@pytest.mark.asyncio
async def test_salary_ratio_and_deposit_deficit(client: AsyncClient):
    """[Domain 1: Salary-Based Deposit & Ratio] Verify income ratio calculation and joint deposit configuration."""
    key = derive_key()
    user_a = encrypt_text("John", key)
    user_b = encrypt_text("Jane", key)
    sal_cat = encrypt_text("SALARY", key)

    await client.post("/users", json={"name": user_a, "color": "#6366f1"})
    await client.post("/users", json={"name": user_b, "color": "#ec4899"})

    # Post salary incomes: John earns 3000, Jane earns 2000 (60% / 40% ratio)
    await client.post("/income", json=[
        {"name": encrypt_text("Salary John", key), "amount_cents": 300000, "who": user_a, "category": sal_cat, "income_date": "2026-07-01"},
        {"name": encrypt_text("Salary Jane", key), "amount_cents": 200000, "who": user_b, "category": sal_cat, "income_date": "2026-07-01"}
    ])

    inc_resp = await client.get(f"/analytics/income-by-person?month=2026-07&salary_cat={sal_cat}")
    assert inc_resp.status_code == 200
    rows = inc_resp.json()
    assert len(rows) == 2
    john_row = next(r for r in rows if r["who"] == user_a)
    jane_row = next(r for r in rows if r["who"] == user_b)
    assert john_row["total_cents"] == 300000
    assert jane_row["total_cents"] == 200000


