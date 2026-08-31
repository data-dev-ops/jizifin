import pytest
from httpx import AsyncClient, ASGITransport
from app.main import app
from app.database import init_db
from tests.conftest import derive_key, encrypt_text


@pytest.fixture(autouse=True)
async def setup_db(tmp_path, monkeypatch):
    test_db = tmp_path / "test.db"
    monkeypatch.setattr("app.database.DB_PATH", test_db)
    monkeypatch.setattr("app.main.DB_PATH", test_db)
    await init_db(test_db)


@pytest.mark.asyncio
async def test_create_split_creates_baseline_agreement():
    key = derive_key("test-passphrase")
    u_alice = encrypt_text("Alice", key)
    u_bob = encrypt_text("Bob", key)
    cat_enc = encrypt_text("GROCERIES", key)

    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        await client.post("/users", json={"name": u_alice, "color": "#ff0000"})
        await client.post("/users", json={"name": u_bob, "color": "#00ff00"})

        # Create category
        res = await client.post("/splits", json={
            "category": cat_enc,
            "allocations": [
                {"user_name": u_alice, "pct": 50.0},
                {"user_name": u_bob, "pct": 50.0},
            ]
        })
        assert res.status_code == 201
        data = res.json()
        assert data["category"] == cat_enc
        assert len(data["allocations"]) == 2
        assert len(data["agreements"]) == 1
        baseline = data["agreements"][0]
        assert baseline["start_date"] == "2000-01-01"
        assert baseline["end_date"] is None
        assert baseline["is_active"] is True


@pytest.mark.asyncio
async def test_scd2_temporary_override_crud_and_payback_resolution():
    key = derive_key("test-passphrase")
    u_alice = encrypt_text("Alice", key)
    u_bob = encrypt_text("Bob", key)
    cat_enc = encrypt_text("GROCERIES", key)
    note_enc = encrypt_text("Summer guest stay", key)

    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        await client.post("/users", json={"name": u_alice, "color": "#ff0000"})
        await client.post("/users", json={"name": u_bob, "color": "#00ff00"})

        # 1. Create category with baseline 50/50 split
        await client.post("/splits", json={
            "category": cat_enc,
            "allocations": [
                {"user_name": u_alice, "pct": 50.0},
                {"user_name": u_bob, "pct": 50.0},
            ]
        })

        # 2. Add temporary override for June 2026: 70% Alice, 30% Bob
        res_ov = await client.post(f"/splits/{cat_enc}/agreements", json={
            "category": cat_enc,
            "start_date": "2026-06-01",
            "end_date": "2026-06-30",
            "is_active": True,
            "note": note_enc,
            "allocations": [
                {"user_name": u_alice, "pct": 70.0},
                {"user_name": u_bob, "pct": 30.0},
            ]
        })
        assert res_ov.status_code == 201
        ov_data = res_ov.json()
        override_id = ov_data["id"]
        assert ov_data["start_date"] == "2026-06-01"
        assert ov_data["end_date"] == "2026-06-30"

        # Check list_splits includes both baseline and override
        res_list = await client.get("/splits")
        assert res_list.status_code == 200
        cat_data = [s for s in res_list.json() if s["category"] == cat_enc][0]
        assert len(cat_data["agreements"]) == 2

        # 3. Add expense in May 2026: 100.00 EUR (10000 cents) paid by Alice
        exp_may = encrypt_text("Groceries May", key)
        await client.post("/expenses", json={
            "name": exp_may,
            "cost_cents": 10000,
            "expense_date": "2026-05-15",
            "who_paid": u_alice,
            "category": cat_enc,
        })

        # 4. Add expense in June 2026: 100.00 EUR (10000 cents) paid by Alice
        exp_june = encrypt_text("Groceries June", key)
        await client.post("/expenses", json={
            "name": exp_june,
            "cost_cents": 10000,
            "expense_date": "2026-06-15",
            "who_paid": u_alice,
            "category": cat_enc,
        })

        # 5. Add expense in July 2026: 100.00 EUR (10000 cents) paid by Alice
        exp_july = encrypt_text("Groceries July", key)
        await client.post("/expenses", json={
            "name": exp_july,
            "cost_cents": 10000,
            "expense_date": "2026-07-15",
            "who_paid": u_alice,
            "category": cat_enc,
        })

        # 6. Verify May paybacks: Baseline 50/50 -> Bob owes Alice 50.00 EUR
        res_pb_may = await client.get("/analytics/paybacks", params={
            "month": "2026-05",
            "personal_cats": "",
            "combined_fixed_cat": "",
            "apartment_cat": "",
            "jane_name": u_alice,
            "john_name": u_bob,
        })
        assert res_pb_may.status_code == 200
        pb_may = res_pb_may.json()
        assert len(pb_may["debts"]) == 1
        assert pb_may["debts"][0]["from_user"] == u_bob
        assert pb_may["debts"][0]["to_user"] == u_alice
        assert pb_may["debts"][0]["amount"] == 50.0

        # 7. Verify June paybacks: Temporary Override 70/30 -> Bob owes Alice 30.00 EUR
        res_pb_june = await client.get("/analytics/paybacks", params={
            "month": "2026-06",
            "personal_cats": "",
            "combined_fixed_cat": "",
            "apartment_cat": "",
            "jane_name": u_alice,
            "john_name": u_bob,
        })
        assert res_pb_june.status_code == 200
        pb_june = res_pb_june.json()
        assert len(pb_june["debts"]) == 1
        assert pb_june["debts"][0]["from_user"] == u_bob
        assert pb_june["debts"][0]["to_user"] == u_alice
        assert pb_june["debts"][0]["amount"] == 30.0

        # 8. Verify July paybacks: Reverted to Baseline 50/50 -> Bob owes Alice 50.00 EUR
        res_pb_july = await client.get("/analytics/paybacks", params={
            "month": "2026-07",
            "personal_cats": "",
            "combined_fixed_cat": "",
            "apartment_cat": "",
            "jane_name": u_alice,
            "john_name": u_bob,
        })
        assert res_pb_july.status_code == 200
        pb_july = res_pb_july.json()
        assert len(pb_july["debts"]) == 1
        assert pb_july["debts"][0]["from_user"] == u_bob
        assert pb_july["debts"][0]["to_user"] == u_alice
        assert pb_july["debts"][0]["amount"] == 50.0

        # 9. Update the temporary override to 80/20
        res_up = await client.put(f"/splits/agreements/{override_id}", json={
            "allocations": [
                {"user_name": u_alice, "pct": 80.0},
                {"user_name": u_bob, "pct": 20.0},
            ]
        })
        assert res_up.status_code == 200

        # Verify June paybacks after update: Bob owes 20.00 EUR
        res_pb_june2 = await client.get("/analytics/paybacks", params={
            "month": "2026-06",
            "personal_cats": "",
            "combined_fixed_cat": "",
            "apartment_cat": "",
            "jane_name": u_alice,
            "john_name": u_bob,
        })
        pb_june2 = res_pb_june2.json()
        assert pb_june2["debts"][0]["amount"] == 20.0

        # 10. Delete the temporary override
        res_del = await client.delete(f"/splits/agreements/{override_id}")
        assert res_del.status_code == 204

        # Verify June paybacks after deletion: reverts to baseline 50/50 (50.0 EUR)
        res_pb_june3 = await client.get("/analytics/paybacks", params={
            "month": "2026-06",
            "personal_cats": "",
            "combined_fixed_cat": "",
            "apartment_cat": "",
            "jane_name": u_alice,
            "john_name": u_bob,
        })
        pb_june3 = res_pb_june3.json()
        assert pb_june3["debts"][0]["amount"] == 50.0


@pytest.mark.asyncio
async def test_category_rename_cascades_to_split_agreements_and_allocations():
    key = derive_key("test-passphrase")
    u_alice = encrypt_text("Alice", key)
    cat_old = encrypt_text("FOOD", key)
    cat_new = encrypt_text("GROCERIES", key)

    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        await client.post("/users", json={"name": u_alice, "color": "#ff0000"})
        await client.post("/splits", json={
            "category": cat_old,
            "allocations": [{"user_name": u_alice, "pct": 100.0}]
        })

        # Add temporary override for FOOD
        await client.post(f"/splits/{cat_old}/agreements", json={
            "category": cat_old,
            "start_date": "2026-01-01",
            "end_date": "2026-01-31",
            "allocations": [{"user_name": u_alice, "pct": 100.0}]
        })

        # Rename FOOD to GROCERIES
        res_ren = await client.put(f"/splits/{cat_old}", json={"category": cat_new})
        assert res_ren.status_code == 200
        data = res_ren.json()
        assert data["category"] == cat_new
        assert len(data["agreements"]) == 2
        for agr in data["agreements"]:
            assert agr["category"] == cat_new
