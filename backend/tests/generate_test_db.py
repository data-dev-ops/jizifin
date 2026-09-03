"""
backend/tests/generate_test_db.py

Generates a realistic, deterministic, and fully populated test.db database
for the Jizifin integration test suite.
Contains:
  - 5-person household:
      * Couple 1: Alice & Bob (Joint Account 1: Couple AB Joint)
      * Couple 2: Charlie & Dave (Joint Account 2: Couple CD Joint)
      * Single Adult: Eve
  - 2 isolated joint accounts with members, categories, deposit schedules, and logs
  - Employment contracts (jobs) with frequency normalization and income entries
  - SCD2 category split agreements (baseline and date-bounded overrides)
  - Target projects with user membership (project_users)
  - Tags with active timeline windows
  - Multi-month expense transactions (2026-06, 2026-07, 2026-08, 2026-09, 2026-10)
  - Recurring expense templates and category budgets
All sensitive columns are encrypted using the standard test master passphrase:
  'test-master-passphrase'
"""

import asyncio
import base64
from pathlib import Path
import aiosqlite
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC

from app.database import init_db

STATIC_IV = b"jizifin-cryp"
SALT = b"jizifin-salt-pbkdf2"
TEST_PASSWORD = "test-master-passphrase"
OUTPUT_DB_PATH = Path(__file__).parent / "test.db"


def derive_key(password: str = TEST_PASSWORD) -> bytes:
    kdf = PBKDF2HMAC(
        algorithm=hashes.SHA256(),
        length=32,
        salt=SALT,
        iterations=100000,
    )
    return kdf.derive(password.encode("utf-8"))


def encrypt_text(plaintext: str, key: bytes) -> str:
    if not plaintext:
        return plaintext
    aesgcm = AESGCM(key)
    ciphertext = aesgcm.encrypt(STATIC_IV, str(plaintext).encode("utf-8"), None)
    return base64.urlsafe_b64encode(ciphertext).decode("ascii").rstrip("=")


async def populate_test_db(db_path: Path):
    if db_path.exists():
        db_path.unlink()

    key = derive_key()

    async with aiosqlite.connect(db_path) as db:
        db.row_factory = aiosqlite.Row
        await db.execute("PRAGMA journal_mode=WAL;")
        await db.execute("PRAGMA foreign_keys=ON;")

        # 1. Initialize complete schema & views
        await init_db(conn=db)

        # 2. app_config
        magic_val = encrypt_text("FinanceTrackerAuth", key)
        await db.execute(
            "INSERT OR REPLACE INTO app_config (key, value) VALUES ('magic_word', ?)",
            (magic_val,),
        )

        # 3. users (5 members)
        members = [
            ("Alice", "#6366f1"),
            ("Bob", "#3b82f6"),
            ("Charlie", "#10b981"),
            ("Dave", "#f59e0b"),
            ("Eve", "#ec4899"),
        ]
        enc_users = {name: encrypt_text(name, key) for name, _ in members}
        for name, color in members:
            await db.execute(
                "INSERT INTO users (name, color, is_active, created_at) VALUES (?, ?, 1, '2026-01-01 00:00:00')",
                (enc_users[name], color),
            )

        # 4. joint_accounts & joint_account_members
        # Account 1: Couple AB Joint (Alice & Bob)
        # Account 2: Couple CD Joint (Charlie & Dave)
        enc_ja1 = encrypt_text("Couple AB Joint", key)
        enc_ja2 = encrypt_text("Couple CD Joint", key)

        await db.execute(
            """INSERT INTO joint_accounts (id, name, balance_cents, safety_margin_pct, deposit_split_mode, expected_total_cents)
               VALUES (1, ?, 115000, 10, 'even', 70000)""",
            (enc_ja1,),
        )
        await db.execute(
            """INSERT INTO joint_accounts (id, name, balance_cents, safety_margin_pct, deposit_split_mode, expected_total_cents)
               VALUES (2, ?, 158000, 10, 'even', 84000)""",
            (enc_ja2,),
        )

        for u in ["Alice", "Bob"]:
            await db.execute(
                "INSERT INTO joint_account_members (account_id, user_name) VALUES (1, ?)",
                (enc_users[u],),
            )
        for u in ["Charlie", "Dave"]:
            await db.execute(
                "INSERT INTO joint_account_members (account_id, user_name) VALUES (2, ?)",
                (enc_users[u],),
            )

        # Legacy singleton table parity
        await db.execute(
            """INSERT OR REPLACE INTO joint_account (id, name, balance_cents, safety_margin_pct, deposit_split_mode, expected_total_cents)
               VALUES (1, ?, 115000, 10, 'even', 70000)""",
            (enc_ja1,),
        )

        # 5. splits (Categories)
        category_names = [
            "Housing/Rent",
            "Utilities/Groceries Shared",
            "GROCERIES",
            "HOME IMPROVEMENT",
            "Solar Infrastructure",
            "UTILITIES",
            "Fixed Living",
            "Asset Investment",
            "Discretionary Shared",
            "PERSONAL COST",
            "LEISURE",
            "GIFT",
        ]
        enc_cats = {cat: encrypt_text(cat, key) for cat in category_names}
        for cat in category_names:
            await db.execute(
                "INSERT INTO splits (category) VALUES (?)",
                (enc_cats[cat],),
            )

        # 6. joint_account_categories & expected costs
        # Only dedicated joint-only pool category is excluded from peer paybacks
        enc_ja_pool = encrypt_text("Joint Household Reserve", key)
        await db.execute(
            "INSERT INTO splits (category) VALUES (?)",
            (enc_ja_pool,),
        )
        await db.execute(
            "INSERT INTO joint_account_categories (category, account_id) VALUES (?, 1)",
            (enc_ja_pool,),
        )

        await db.execute(
            "INSERT INTO joint_account_expected_costs (category, expected_cents, account_id) VALUES (?, 35000, 1)",
            (enc_cats["GROCERIES"],),
        )
        await db.execute(
            "INSERT INTO joint_account_expected_costs (category, expected_cents, account_id) VALUES (?, 42000, 2)",
            (enc_cats["UTILITIES"],),
        )

        # 7. joint_account_deposits & monthly logs
        # Account 1: Alice €1,200 (60%), Bob €800 (40%)
        # Account 2: Charlie €1,200 (60%), Dave €800 (40%)
        deposits = [
            ("Alice", 120000, 1, 1),
            ("Bob", 80000, 1, 1),
            ("Charlie", 120000, 1, 2),
            ("Dave", 80000, 1, 2),
        ]
        for u, amt, day, ja_id in deposits:
            await db.execute(
                "INSERT INTO joint_account_deposits (user_name, amount_cents, day_of_month, account_id) VALUES (?, ?, ?, ?)",
                (enc_users[u], amt, day, ja_id),
            )

        # Monthly execution logs for 2026-06, 2026-07, 2026-08
        for month in ["2026-06", "2026-07", "2026-08"]:
            for u, amt, _, ja_id in deposits:
                await db.execute(
                    """INSERT INTO joint_account_monthly_deposits
                       (month, user_name, scheduled_cents, actual_cents, is_paid, paid_date, account_id)
                       VALUES (?, ?, ?, ?, 1, ?, ?)""",
                    (month, enc_users[u], amt, amt, f"{month}-01", ja_id),
                )

        # 8. joint_account_corrections
        note1 = encrypt_text("Quarterly Reserve Top-up", key)
        await db.execute(
            "INSERT INTO joint_account_corrections (amount_cents, correction_date, note, account_id) VALUES (50000, '2026-07-01', ?, 1)",
            (note1,),
        )

        # 9. jobs (Employment streams)
        # Charlie, Dave, and Eve contracts end in July 2026 so October dynamic recalibration (Scenario 3) isolates Alice & Bob
        job_defs = [
            ("Alice Main Job", "Alice", 400000, "monthly", "2026-01-01", None),
            ("Bob Logistics", "Bob", 200000, "monthly", "2026-01-01", None),
            ("Charlie Engineering", "Charlie", 350000, "monthly", "2026-01-01", "2026-07-31"),
            ("Dave Marketing", "Dave", 150000, "monthly", "2026-01-01", "2026-07-31"),
            ("Eve Primary Employment", "Eve", 200000, "monthly", "2026-01-01", "2026-07-31"),
            ("Eve Freelance Stream", "Eve", 50000, "monthly", "2026-01-01", "2026-07-31"),
        ]
        for jname, who, amt, freq, sdate, edate in job_defs:
            await db.execute(
                """INSERT INTO jobs (name, who, amount_cents, frequency, start_date, end_date, is_active)
                   VALUES (?, ?, ?, ?, ?, ?, 1)""",
                (encrypt_text(jname, key), enc_users[who], amt, freq, sdate, edate),
            )

        # 10. income (One-off streams)
        one_off_income = [
            ("Eve Freelance Retainer", 50000, "Eve", "FREELANCE", "2026-08-15"),
            ("Alice Q3 Bonus", 100000, "Alice", "BONUS", "2026-10-15"),
        ]
        for iname, amt, who, cat, idate in one_off_income:
            await db.execute(
                """INSERT INTO income (name, amount_cents, who, category, income_date, is_joint)
                   VALUES (?, ?, ?, ?, ?, 0)""",
                (encrypt_text(iname, key), amt, enc_users[who], encrypt_text(cat, key), idate),
            )

        # 10b. salary_overrides for October 2026 (Alice €3,000, Bob €3,000)
        await db.execute(
            """INSERT INTO salary_overrides (user_name, month, amount_cents, note)
               VALUES (?, '2026-10', 300000, ?)""",
            (enc_users["Alice"], encrypt_text("October Contract Recalibration", key)),
        )
        await db.execute(
            """INSERT INTO salary_overrides (user_name, month, amount_cents, note)
               VALUES (?, '2026-10', 300000, ?)""",
            (enc_users["Bob"], encrypt_text("October Contract Recalibration", key)),
        )

        # 11. SCD2 split_agreements & split_allocations
        # A) Housing/Rent (40% Couple 1 / 40% Couple 2 / 20% Eve)
        # Alice 26.6667, Bob 13.3333, Charlie 28.0000, Dave 12.0000, Eve 20.0000
        cursor = await db.execute(
            """INSERT INTO split_agreements (category, start_date, end_date, is_active, note)
               VALUES (?, '2026-01-01', NULL, 1, ?)""",
            (enc_cats["Housing/Rent"], encrypt_text("Household Bedroom Unit Baseline", key)),
        )
        agr_rent_id = cursor.lastrowid
        rent_allocs = [
            ("Alice", 26.6667),
            ("Bob", 13.3333),
            ("Charlie", 28.0000),
            ("Dave", 12.0000),
            ("Eve", 20.0000),
        ]
        for u, pct in rent_allocs:
            await db.execute(
                """INSERT INTO split_allocations (agreement_id, category, user_name, pct)
                   VALUES (?, ?, ?, ?)""",
                (agr_rent_id, enc_cats["Housing/Rent"], enc_users[u], pct),
            )

        # B) Utilities/Groceries Shared (Proportional to €13,500 total income)
        # Alice 29.6296, Bob 14.8148, Charlie 25.9259, Dave 11.1111, Eve 18.5186
        cursor = await db.execute(
            """INSERT INTO split_agreements (category, start_date, end_date, is_active, note)
               VALUES (?, '2026-01-01', NULL, 1, ?)""",
            (enc_cats["Utilities/Groceries Shared"], encrypt_text("Income Proportional Baseline", key)),
        )
        agr_util_id = cursor.lastrowid
        util_allocs = [
            ("Alice", 29.6296),
            ("Bob", 14.8148),
            ("Charlie", 25.9259),
            ("Dave", 11.1111),
            ("Eve", 18.5186),
        ]
        for u, pct in util_allocs:
            await db.execute(
                """INSERT INTO split_allocations (agreement_id, category, user_name, pct)
                   VALUES (?, ?, ?, ?)""",
                (agr_util_id, enc_cats["Utilities/Groceries Shared"], enc_users[u], pct),
            )

        # C) Solar Infrastructure (Scenario 2: Alice 45%, Bob 25%, Charlie 30%)
        cursor = await db.execute(
            """INSERT INTO split_agreements (category, start_date, end_date, is_active, note)
               VALUES (?, '2026-01-01', NULL, 1, ?)""",
            (enc_cats["Solar Infrastructure"], encrypt_text("Solar Infrastructure Baseline", key)),
        )
        agr_solar_id = cursor.lastrowid
        for u, pct in [("Alice", 45.0), ("Bob", 25.0), ("Charlie", 30.0)]:
            await db.execute(
                """INSERT INTO split_allocations (agreement_id, category, user_name, pct)
                   VALUES (?, ?, ?, ?)""",
                (agr_solar_id, enc_cats["Solar Infrastructure"], enc_users[u], pct),
            )

        # D) GROCERIES (Couple 1: Baseline 50/50, Overrides in August)
        cursor = await db.execute(
            """INSERT INTO split_agreements (category, start_date, end_date, is_active, note)
               VALUES (?, '2026-01-01', NULL, 1, ?)""",
            (enc_cats["GROCERIES"], encrypt_text("Couple 1 Baseline 50/50", key)),
        )
        agr_groc_base = cursor.lastrowid
        for u, pct in [("Alice", 50.0), ("Bob", 50.0)]:
            await db.execute(
                "INSERT INTO split_allocations (agreement_id, category, user_name, pct) VALUES (?, ?, ?, ?)",
                (agr_groc_base, enc_cats["GROCERIES"], enc_users[u], pct),
            )

        # Override 1 (Full Month August: Alice 20%, Bob 80%)
        cursor = await db.execute(
            """INSERT INTO split_agreements (category, start_date, end_date, is_active, note)
               VALUES (?, '2026-08-01', '2026-08-31', 1, ?)""",
            (enc_cats["GROCERIES"], encrypt_text("Summer Host Month Override (Alice 20, Bob 80)", key)),
        )
        agr_groc_ov1 = cursor.lastrowid
        for u, pct in [("Alice", 20.0), ("Bob", 80.0)]:
            await db.execute(
                "INSERT INTO split_allocations (agreement_id, category, user_name, pct) VALUES (?, ?, ?, ?)",
                (agr_groc_ov1, enc_cats["GROCERIES"], enc_users[u], pct),
            )

        # Override 2 (Event Window Aug 10-17: Alice 100%, Bob 0%)
        cursor = await db.execute(
            """INSERT INTO split_agreements (category, start_date, end_date, is_active, note)
               VALUES (?, '2026-08-10', '2026-08-17', 1, ?)""",
            (enc_cats["GROCERIES"], encrypt_text("Private Event Week Override (Alice 100, Bob 0)", key)),
        )
        agr_groc_ov2 = cursor.lastrowid
        for u, pct in [("Alice", 100.0), ("Bob", 0.0)]:
            await db.execute(
                "INSERT INTO split_allocations (agreement_id, category, user_name, pct) VALUES (?, ?, ?, ?)",
                (agr_groc_ov2, enc_cats["GROCERIES"], enc_users[u], pct),
            )

        # E) HOME IMPROVEMENT (Couple 1: Baseline 50/50, August Override 75/25)
        cursor = await db.execute(
            """INSERT INTO split_agreements (category, start_date, end_date, is_active, note)
               VALUES (?, '2026-01-01', NULL, 1, ?)""",
            (enc_cats["HOME IMPROVEMENT"], encrypt_text("Home Improvement Baseline 50/50", key)),
        )
        agr_home_base = cursor.lastrowid
        for u, pct in [("Alice", 50.0), ("Bob", 50.0)]:
            await db.execute(
                "INSERT INTO split_allocations (agreement_id, category, user_name, pct) VALUES (?, ?, ?, ?)",
                (agr_home_base, enc_cats["HOME IMPROVEMENT"], enc_users[u], pct),
            )

        cursor = await db.execute(
            """INSERT INTO split_agreements (category, start_date, end_date, is_active, note)
               VALUES (?, '2026-08-01', '2026-08-31', 1, ?)""",
            (enc_cats["HOME IMPROVEMENT"], encrypt_text("Custom Bookshelf Renovation (Alice 75, Bob 25)", key)),
        )
        agr_home_ov = cursor.lastrowid
        for u, pct in [("Alice", 75.0), ("Bob", 25.0)]:
            await db.execute(
                "INSERT INTO split_allocations (agreement_id, category, user_name, pct) VALUES (?, ?, ?, ?)",
                (agr_home_ov, enc_cats["HOME IMPROVEMENT"], enc_users[u], pct),
            )

        # F) Fixed Living, Asset Investment, Discretionary Shared (October Scenario 3)
        for cat, allocs in [
            ("Fixed Living", [("Alice", 50.0), ("Bob", 50.0)]),
            ("Asset Investment", [("Alice", 70.0), ("Bob", 30.0)]),
            ("Discretionary Shared", [("Alice", 57.14), ("Bob", 42.86)]),
        ]:
            cursor = await db.execute(
                "INSERT INTO split_agreements (category, start_date, end_date, is_active, note) VALUES (?, '2026-01-01', NULL, 1, 'Baseline')",
                (enc_cats[cat],),
            )
            a_id = cursor.lastrowid
            for u, pct in allocs:
                await db.execute(
                    "INSERT INTO split_allocations (agreement_id, category, user_name, pct) VALUES (?, ?, ?, ?)",
                    (a_id, enc_cats[cat], enc_users[u], pct),
                )

        # 12. projects & project_users
        cursor = await db.execute(
            """INSERT INTO projects (id, name, target_cents, target_date, is_joint, allow_subcategories)
               VALUES (1, ?, 600000, '2026-12-31', 0, 1)""",
            (encrypt_text("Solar & Battery Installation", key),),
        )
        for u in ["Alice", "Bob", "Charlie"]:
            await db.execute(
                "INSERT INTO project_users (project_id, user_name) VALUES (1, ?)",
                (enc_users[u],),
            )

        # 13. tags
        await db.execute(
            """INSERT INTO tags (id, name, color, description, start_date, end_date, is_joint, is_active)
               VALUES (1, ?, '#f59e0b', ?, '2026-06-01', '2026-06-30', 0, 1)""",
            (encrypt_text("tag:solar-phase-1", key), encrypt_text("Phase 1 Solar Installation", key)),
        )
        await db.execute(
            """INSERT INTO tags (id, name, color, description, start_date, end_date, is_joint, is_active)
               VALUES (2, ?, '#10b981', ?, '2026-08-01', '2026-08-31', 0, 1)""",
            (encrypt_text("tag:summer-festival", key), encrypt_text("Summer Festival Events", key)),
        )

        # 14. expenses across months
        expenses_data = [
            # June 2026: Scenario 2 Solar Project
            (101, "Inverter Equipment", 180000, "2026-06-10", "Bob", "Solar Infrastructure", 1, 1, 0, None),
            (102, "Solar Panels", 320000, "2026-06-18", "Alice", "Solar Infrastructure", 1, 1, 1, 1),
            (103, "Electrical Certification", 100000, "2026-06-25", "Charlie", "Solar Infrastructure", 1, 1, 0, None),

            # July 2026: Scenario 1 Household Shared & Joint Costs
            (201, "Household Apartment Rent", 200000, "2026-07-01", "Alice", "Housing/Rent", None, None, 0, None),
            (202, "Shared Utilities & Supplies", 100000, "2026-07-02", "Eve", "Utilities/Groceries Shared", None, None, 0, None),
            (203, "Couple 1 Personal Groceries", 35000, "2026-07-05", "Alice", "GROCERIES", None, None, 1, 1),
            (204, "Couple 2 Personal Groceries", 42000, "2026-07-06", "Charlie", "UTILITIES", None, None, 1, 2),

            # August 2026: Scenario 4 Overlapping Category Overrides on GROCERIES (Alice & Bob)
            (301, "Weekly Farmers Market", 10000, "2026-08-05", "Bob", "GROCERIES", None, None, 0, None),
            (302, "Party Supplies & Catered Dinner", 20000, "2026-08-12", "Bob", "GROCERIES", None, None, 0, None),
            (303, "Bulk Pantry Restock", 15000, "2026-08-25", "Alice", "GROCERIES", None, None, 0, None),

            # September 2026: Scenario 4 Post-Override Fallback to Baseline
            (304, "September Welcome Dinner", 8000, "2026-09-02", "Bob", "GROCERIES", None, None, 0, None),

            # August 2026: Scenario 5 HOME IMPROVEMENT Out-of-Pocket vs Direct Joint (Alice & Bob)
            (401, "Custom Bookshelf Unit", 40000, "2026-08-18", "Bob", "HOME IMPROVEMENT", None, None, 0, None),
            (402, "Painting Supplies", 15000, "2026-08-20", "Alice", "HOME IMPROVEMENT", None, None, 1, 1),

            # October 2026: Scenario 3 Multi-Category Recalibration
            (501, "Supermarket Run", 40000, "2026-10-05", "Alice", "Fixed Living", None, None, 0, None),
            (502, "Concert Tickets", 60000, "2026-10-10", "Bob", "Discretionary Shared", None, None, 0, None),
            (503, "Home Gym Equipment", 140000, "2026-10-20", "Alice", "Asset Investment", None, None, 0, None),
            (504, "Restaurant Dinner", 20000, "2026-10-28", "Bob", "Discretionary Shared", None, None, 0, None),
        ]

        for eid, ename, cost, edate, who, cat, pid, tid, is_j, ja_id in expenses_data:
            await db.execute(
                """INSERT INTO expenses (id, name, cost_cents, expense_date, who_paid, category, project_id, tag_id, is_joint, joint_account_id)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (
                    eid,
                    encrypt_text(ename, key),
                    cost,
                    edate,
                    enc_users[who],
                    enc_cats[cat],
                    pid,
                    tid,
                    is_j,
                    ja_id,
                ),
            )

        # 15. recurring_expenses (Templates)
        recurring_data = [
            ("Monthly Internet Fiber", 6000, "Eve", "Utilities/Groceries Shared", 1, "monthly", "2026-01-01", None, 0, None),
            ("Shared House Cleaning", 12000, "Alice", "Housing/Rent", 15, "monthly", "2026-01-01", None, 0, None),
        ]
        for rname, cost, who, cat, dom, freq, sdate, edate, is_j, ja_id in recurring_data:
            await db.execute(
                """INSERT INTO recurring_expenses (name, cost_cents, who_paid, category, day_of_month, frequency, start_date, end_date, is_active, is_joint, joint_account_id)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)""",
                (encrypt_text(rname, key), cost, enc_users[who], enc_cats[cat], dom, freq, sdate, edate, is_j, ja_id),
            )

        # 16. budgets
        await db.execute(
            "INSERT INTO budgets (category, month, limit_cents) VALUES (?, 'ALL', 50000)",
            (enc_cats["GROCERIES"],),
        )

        await db.commit()
        print(f"Successfully generated populated test.db at {db_path} ({db_path.stat().st_size} bytes).")


if __name__ == "__main__":
    asyncio.run(populate_test_db(OUTPUT_DB_PATH))
