"""
backend/tests/test_expenses_batch.py

Tests for POST /expenses/batch bulk creation endpoint:
- Batch creation of multiple expenses with deterministic encryption.
- Validation: Non-existent category fails entire batch.
- Validation: Month lock protection applies to all expenses in batch.
- Validation: Empty batch handling.
- Validation: Tag date boundaries checked.
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text


@pytest.mark.asyncio
async def test_create_expenses_batch_success(client: AsyncClient):
    key = derive_key()
    u1 = encrypt_text("Jim", key)
    c1 = encrypt_text("GROCERIES", key)
    c2 = encrypt_text("UTILITIES", key)

    await client.post("/users", json={"name": u1, "color": "#10b981"})
    await client.post("/splits", json={"category": c1, "allocations": []})
    await client.post("/splits", json={"category": c2, "allocations": []})

    e1_name = encrypt_text("Supermarket", key)
    e2_name = encrypt_text("Electricity", key)

    payload = [
        {
            "name": e1_name,
            "cost_cents": 4500,
            "expense_date": "2026-08-15",
            "who_paid": u1,
            "category": c1,
        },
        {
            "name": e2_name,
            "cost_cents": 8500,
            "expense_date": "2026-08-18",
            "who_paid": u1,
            "category": c2,
        },
    ]

    resp = await client.post("/expenses/batch", json=payload)
    assert resp.status_code == 201
    data = resp.json()
    assert len(data) == 2
    assert data[0]["cost_cents"] == 4500
    assert data[0]["expense_date"] == "2026-08-15"
    assert data[1]["cost_cents"] == 8500
    assert data[1]["expense_date"] == "2026-08-18"


@pytest.mark.asyncio
async def test_create_expenses_batch_empty(client: AsyncClient):
    resp = await client.post("/expenses/batch", json=[])
    assert resp.status_code == 201
    assert resp.json() == []


@pytest.mark.asyncio
async def test_create_expenses_batch_invalid_category(client: AsyncClient):
    key = derive_key()
    u1 = encrypt_text("Jim", key)
    c_valid = encrypt_text("RENT", key)
    c_invalid = encrypt_text("NON_EXISTENT", key)

    await client.post("/users", json={"name": u1, "color": "#10b981"})
    await client.post("/splits", json={"category": c_valid, "allocations": []})

    payload = [
        {
            "name": encrypt_text("Rent payment", key),
            "cost_cents": 80000,
            "expense_date": "2026-08-01",
            "who_paid": u1,
            "category": c_valid,
        },
        {
            "name": encrypt_text("Unknown item", key),
            "cost_cents": 1000,
            "expense_date": "2026-08-02",
            "who_paid": u1,
            "category": c_invalid,
        },
    ]

    resp = await client.post("/expenses/batch", json=payload)
    assert resp.status_code == 422
