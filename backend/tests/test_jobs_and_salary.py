"""
backend/tests/test_jobs_and_salary.py

Tests for the Jobs & Salary Streams system:
- CRUD and frequency normalization (monthly, weekly, biweekly, annual)
- Timeline tracking: promotions, mid-year raises, date boundaries
- Multiple concurrent jobs per person
- Combined analytics with one-off income ledger
- Fallback & compatibility for legacy income entries
"""

import pytest
from httpx import AsyncClient
from tests.conftest import derive_key, encrypt_text, decrypt_text


@pytest.mark.asyncio
async def test_job_crud_and_frequency_normalization(client: AsyncClient):
    key = derive_key()
    user_enc = encrypt_text("John", key)
    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})

    # 1. Create Monthly Job
    res_m = await client.post("/jobs", json={
        "name": encrypt_text("Senior Dev", key),
        "who": user_enc,
        "amount_cents": 450000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
        "notes": encrypt_text("Full-time contract", key),
        "is_active": True,
    })
    assert res_m.status_code == 201
    job_m = res_m.json()
    assert job_m["amount_cents"] == 450000
    assert job_m["frequency"] == "monthly"
    assert job_m["monthly_equivalent_cents"] == 450000
    assert decrypt_text(job_m["name"], key) == "Senior Dev"

    # 2. Create Weekly Job (1000 EUR/week -> ~4333.33 EUR/mo)
    res_w = await client.post("/jobs", json={
        "name": encrypt_text("Consulting", key),
        "who": user_enc,
        "amount_cents": 100000,
        "frequency": "weekly",
        "start_date": "2026-03-01",
        "notes": encrypt_text("Weekly freelance", key),
    })
    assert res_w.status_code == 201
    job_w = res_w.json()
    assert job_w["monthly_equivalent_cents"] == round(100000 * 52 / 12)  # 433333

    # 3. Create Biweekly Job (2000 EUR/2-weeks -> ~4333.33 EUR/mo)
    res_bw = await client.post("/jobs", json={
        "name": encrypt_text("Teaching", key),
        "who": user_enc,
        "amount_cents": 200000,
        "frequency": "biweekly",
        "start_date": "2026-04-01",
    })
    assert res_bw.status_code == 201
    job_bw = res_bw.json()
    assert job_bw["monthly_equivalent_cents"] == round(200000 * 26 / 12)  # 433333

    # 4. Create Annual Job (60000 EUR/year -> 5000 EUR/mo)
    res_a = await client.post("/jobs", json={
        "name": encrypt_text("Annual Stipend", key),
        "who": user_enc,
        "amount_cents": 6000000,
        "frequency": "annual",
        "start_date": "2026-01-01",
    })
    assert res_a.status_code == 201
    job_a = res_a.json()
    assert job_a["monthly_equivalent_cents"] == 500000

    # 5. List jobs
    list_res = await client.get(f"/jobs?who={user_enc}")
    assert list_res.status_code == 200
    jobs = list_res.json()
    assert len(jobs) == 4

    # 6. Update job
    update_res = await client.put(f"/jobs/{job_m['id']}", json={
        "amount_cents": 500000,
        "notes": encrypt_text("Promoted to Lead", key),
    })
    assert update_res.status_code == 200
    assert update_res.json()["amount_cents"] == 500000
    assert update_res.json()["monthly_equivalent_cents"] == 500000

    # 7. Delete job
    del_res = await client.delete(f"/jobs/{job_a['id']}")
    assert del_res.status_code == 204
    list_after_del = await client.get(f"/jobs?who={user_enc}")
    assert len(list_after_del.json()) == 3


@pytest.mark.asyncio
async def test_job_timeline_and_raises(client: AsyncClient):
    key = derive_key()
    john_enc = encrypt_text("John", key)
    jane_enc = encrypt_text("Jane", key)
    salary_cat_enc = encrypt_text("SALARY", key)

    await client.post("/users", json={"name": john_enc, "color": "#6366f1"})
    await client.post("/users", json={"name": jane_enc, "color": "#ec4899"})

    # John: 4000 EUR/mo until 2026-06-30
    await client.post("/jobs", json={
        "name": encrypt_text("John Senior Dev", key),
        "who": john_enc,
        "amount_cents": 400000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
        "end_date": "2026-06-30",
        "notes": encrypt_text("Initial contract", key),
    })

    # John: 4800 EUR/mo promotion starting 2026-07-01
    await client.post("/jobs", json={
        "name": encrypt_text("John Lead Dev", key),
        "who": john_enc,
        "amount_cents": 480000,
        "frequency": "monthly",
        "start_date": "2026-07-01",
        "notes": encrypt_text("Promotion", key),
    })

    # Jane: 3000 EUR/mo ongoing
    await client.post("/jobs", json={
        "name": encrypt_text("Jane UX Lead", key),
        "who": jane_enc,
        "amount_cents": 300000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
    })

    # Query May 2026
    may_res = await client.get(f"/analytics/income-by-person?salary_cat={salary_cat_enc}&month=2026-05")
    assert may_res.status_code == 200
    may_data = {r["who"]: r for r in may_res.json()}
    assert may_data[john_enc]["total_cents"] == 400000
    assert may_data[john_enc]["base_salary_cents"] == 400000
    assert may_data[jane_enc]["total_cents"] == 300000

    # Query August 2026
    aug_res = await client.get(f"/analytics/income-by-person?salary_cat={salary_cat_enc}&month=2026-08")
    assert aug_res.status_code == 200
    aug_data = {r["who"]: r for r in aug_res.json()}
    assert aug_data[john_enc]["total_cents"] == 480000
    assert aug_data[john_enc]["base_salary_cents"] == 480000
    assert aug_data[jane_enc]["total_cents"] == 300000


@pytest.mark.asyncio
async def test_multiple_jobs_and_one_off_income(client: AsyncClient):
    key = derive_key()
    user_enc = encrypt_text("Jane", key)
    salary_cat_enc = encrypt_text("SALARY", key)
    bonus_cat_enc = encrypt_text("BONUS", key)

    await client.post("/users", json={"name": user_enc, "color": "#ec4899"})
    await client.post("/income-categories", json={"category": bonus_cat_enc})

    # Main Job: 3000/mo
    await client.post("/jobs", json={
        "name": encrypt_text("UX Designer", key),
        "who": user_enc,
        "amount_cents": 300000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
    })

    # Freelance Job: 500/week -> ~2166.67/mo
    await client.post("/jobs", json={
        "name": encrypt_text("Freelance", key),
        "who": user_enc,
        "amount_cents": 50000,
        "frequency": "weekly",
        "start_date": "2026-01-01",
    })

    # One-off Bonus in March 2026: 1500 EUR
    await client.post("/income", json=[{
        "name": encrypt_text("Q1 Performance Bonus", key),
        "amount_cents": 150000,
        "who": user_enc,
        "category": bonus_cat_enc,
        "income_date": "2026-03-15",
    }])

    # In March: 300000 (main) + 216667 (weekly) + 150000 (bonus) = 666667
    mar_res = await client.get(f"/analytics/income-by-person?salary_cat={salary_cat_enc}&month=2026-03")
    assert mar_res.status_code == 200
    mar_data = mar_res.json()
    assert len(mar_data) == 1
    assert mar_data[0]["base_salary_cents"] == 300000 + round(50000 * 52 / 12)  # 516667
    assert mar_data[0]["one_off_cents"] == 150000
    assert mar_data[0]["total_cents"] == 516667 + 150000

    # In April (no bonus): 516667
    apr_res = await client.get(f"/analytics/income-by-person?salary_cat={salary_cat_enc}&month=2026-04")
    assert apr_res.status_code == 200
    apr_data = apr_res.json()
    assert apr_data[0]["base_salary_cents"] == 516667
    assert apr_data[0]["one_off_cents"] == 0
    assert apr_data[0]["total_cents"] == 516667


@pytest.mark.asyncio
async def test_latest_salary_endpoint(client: AsyncClient):
    key = derive_key()
    user_enc = encrypt_text("John", key)
    salary_cat_enc = encrypt_text("SALARY", key)

    await client.post("/users", json={"name": user_enc, "color": "#6366f1"})
    await client.post("/jobs", json={
        "name": encrypt_text("Staff Architect", key),
        "who": user_enc,
        "amount_cents": 600000,
        "frequency": "monthly",
        "start_date": "2026-01-01",
    })

    res = await client.get(f"/income/latest-salary?salary_cat={salary_cat_enc}")
    assert res.status_code == 200
    rows = res.json()
    assert len(rows) == 1
    assert rows[0]["who"] == user_enc
    assert rows[0]["amount_cents"] == 600000
    assert decrypt_text(rows[0]["name"], key) == "Staff Architect"
