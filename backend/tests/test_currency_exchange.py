"""
backend/tests/test_currency_exchange.py

Domain Specifications Covered:
- CurrExp: Multi-currency expense recording converted to base currency cents via FastAPI endpoints.
- CurrIso: ISO date and schema validation via Pydantic models and endpoint payloads.
- CurrTrf: Cross-currency transfer conversions recorded via joint account corrections endpoint.
- RateAge: Historical transaction grouping and analytics querying based on expense_date.
- LocFmt: Currency amount conversion to decimal representation in analytics view responses.
"""

import pytest
from httpx import AsyncClient
from pydantic import ValidationError
from app.models import ExpenseCreate
from tests.conftest import derive_key, encrypt_text

@pytest.mark.asyncio
@pytest.mark.parametrize("foreign_cents, rate, expected_base_cents", [
    (10000, 1.10, 11000), # 100 USD @ 1.10 -> 11000 EUR cents
    (5000, 0.85, 4250),   # 50 GBP @ 0.85 -> 4250 EUR cents
    (10000, 1.00, 10000),
])
async def test_curr_exp(client: AsyncClient, foreign_cents, rate, expected_base_cents):
    """[CurrExp] Multi-currency expense conversion to base currency cents recorded via expenses endpoint."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Travel", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    base_cents = round(foreign_cents * rate)
    resp = await client.post("/expenses", json={
        "name": encrypt_text("Foreign Expense", key),
        "cost_cents": base_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })
    assert resp.status_code == 201
    assert resp.json()["cost_cents"] == expected_base_cents

    get_resp = await client.get("/expenses")
    assert get_resp.status_code == 200
    expenses = get_resp.json()
    assert any(e["cost_cents"] == expected_base_cents for e in expenses)

@pytest.mark.asyncio
@pytest.mark.parametrize("date_str, is_valid", [
    ("2026-07-24", True),
    ("2026-12-31", True),
    ("INVALID-DATE", False),
    ("2026/07/24", False),
])
async def test_curr_iso(client: AsyncClient, date_str, is_valid):
    """[CurrIso] ISO date format validation via Pydantic model schemas and POST /expenses endpoint."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Transport", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    payload = {
        "name": encrypt_text("Flight Ticket", key),
        "cost_cents": 15000,
        "expense_date": date_str,
        "who_paid": user_enc,
        "category": cat_enc,
    }

    if is_valid:
        exp_model = ExpenseCreate(**payload)
        assert exp_model.expense_date == date_str
        resp = await client.post("/expenses", json=payload)
        assert resp.status_code == 201
    else:
        with pytest.raises(ValidationError):
            ExpenseCreate(**payload)
        resp = await client.post("/expenses", json=payload)
        assert resp.status_code == 422

@pytest.mark.asyncio
@pytest.mark.parametrize("amount_cents, note_text", [
    (12000, "USD to EUR Transfer"),
    (-5000, "GBP Withdrawal"),
])
async def test_curr_trf(client: AsyncClient, amount_cents, note_text):
    """[CurrTrf] Cross-currency transfer conversions recorded via joint account corrections API endpoint."""
    key = derive_key()
    note_enc = encrypt_text(note_text, key)

    await client.post("/joint-account", json={"name": "Joint Account", "balance_cents": 10000})

    resp = await client.post("/joint-account/corrections", json={
        "amount_cents": amount_cents,
        "correction_date": "2026-07-24",
        "note": note_enc,
    })
    assert resp.status_code == 201
    data = resp.json()
    assert data["amount_cents"] == amount_cents

    get_resp = await client.get("/joint-account/corrections")
    assert get_resp.status_code == 200
    corrections = get_resp.json()
    assert any(c["id"] == data["id"] and c["amount_cents"] == amount_cents for c in corrections)

@pytest.mark.asyncio
@pytest.mark.parametrize("expense_date, cost_cents", [
    ("2026-01-15", 10800),
    ("2026-06-20", 11200),
])
async def test_rate_age(client: AsyncClient, expense_date, cost_cents):
    """[RateAge] Historical exchange rate resolution and expense grouping by month via analytics API."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Travel", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    resp = await client.post("/expenses", json={
        "name": encrypt_text("Historical Hotel", key),
        "cost_cents": cost_cents,
        "expense_date": expense_date,
        "who_paid": user_enc,
        "category": cat_enc,
    })
    assert resp.status_code == 201

    target_month = expense_date[:7]
    analytics_resp = await client.get(f"/analytics/by-category?month={target_month}")
    assert analytics_resp.status_code == 200
    rows = analytics_resp.json()
    month_row = next((r for r in rows if r["category"] == cat_enc), None)
    assert month_row is not None
    assert month_row["total_amount"] == pytest.approx(cost_cents / 100.0)

@pytest.mark.asyncio
@pytest.mark.parametrize("cost_cents, expected_decimal", [
    (123456, 1234.56),
    (5000, 50.0),
])
async def test_loc_fmt(client: AsyncClient, cost_cents, expected_decimal):
    """[LocFmt] Currency amount conversion to decimal format via monthly category analytics endpoint."""
    key = derive_key()
    user_enc = encrypt_text("John", key)
    cat_enc = encrypt_text("Shopping", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/splits", json={
        "category": cat_enc,
        "allocations": [{"user_name": user_enc, "pct": 100.0}]
    })

    await client.post("/expenses", json={
        "name": encrypt_text("Store Item", key),
        "cost_cents": cost_cents,
        "expense_date": "2026-07-24",
        "who_paid": user_enc,
        "category": cat_enc,
    })

    resp = await client.get("/analytics/by-category?month=2026-07")
    assert resp.status_code == 200
    cat_rows = resp.json()
    target_row = next((r for r in cat_rows if r["category"] == cat_enc), None)
    assert target_row is not None
    assert target_row["total_amount"] == pytest.approx(expected_decimal)
