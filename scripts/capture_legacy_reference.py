"""Phase B reference capture. Runs inside the legacy Python image against a COPY of finance.db.

Writes <out>/<month>.json (legacy /analytics/paybacks and /analytics/monthly-total responses),
<out>/totals.json (cents per month and kind, read BEFORE the legacy app starts, i.e. what the migrator sees) and
<out>/startup_materialized.json (recurring expenses the legacy app inserted into the copy while starting). Reads the household passphrase from JIZIFIN_PASSPHRASE
at run time only; it is never written anywhere.
"""
import base64
import json
import os
import sqlite3
import subprocess
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC

DB = Path("/app/finance.db")
OUT = Path("/out")
PORT = 8765


def encryptor(passphrase: str):
    kdf = PBKDF2HMAC(algorithm=hashes.SHA256(), length=32, salt=b"jizifin-salt-pbkdf2", iterations=100_000)
    aes = AESGCM(kdf.derive(passphrase.encode()))
    return lambda text: base64.urlsafe_b64encode(aes.encrypt(b"jizifin-cryp", text.encode(), None)).rstrip(b"=").decode()


def months(conn: sqlite3.Connection) -> list[str]:
    sql = (
        "SELECT substr(expense_date,1,7) FROM expenses UNION SELECT substr(income_date,1,7) FROM income "
        "UNION SELECT month FROM joint_account_monthly_deposits WHERE is_paid = 1 ORDER BY 1"
    )
    return [r[0] for r in conn.execute(sql)]


def totals(conn: sqlite3.Connection, month: str, salary: str) -> dict:
    one = lambda sql, *p: conn.execute(sql, p).fetchone()
    expense = one("SELECT COALESCE(SUM(cost_cents),0), COUNT(*) FROM expenses WHERE substr(expense_date,1,7)=?", month)
    income = one("SELECT COALESCE(SUM(amount_cents),0), COUNT(*) FROM income WHERE substr(income_date,1,7)=? AND category<>?", month, salary)
    deposit = one(
        "SELECT COALESCE(SUM(CASE WHEN actual_cents>0 THEN actual_cents ELSE scheduled_cents END),0), COUNT(*) "
        "FROM joint_account_monthly_deposits WHERE is_paid=1 AND month=?", month)
    return {"expense": {"cents": expense[0], "count": expense[1]}, "income": {"cents": income[0], "count": income[1]},
            "deposit": {"cents": deposit[0], "count": deposit[1]}}


def startup_materialized(conn: sqlite3.Connection, last_id: int) -> dict:
    """Expenses the legacy lifespan inserted from recurring templates; they are in the paybacks, not in totals."""
    sql = "SELECT substr(expense_date,1,7), COUNT(*), SUM(cost_cents) FROM expenses WHERE id > ? GROUP BY 1 ORDER BY 1"
    return {m: {"count": n, "cents": c} for m, n, c in conn.execute(sql, (last_id,))}


def get(path: str, params: dict) -> dict:
    url = f"http://127.0.0.1:{PORT}{path}?{urllib.parse.urlencode(params)}"
    with urllib.request.urlopen(url, timeout=30) as res:
        return json.load(res)


def wait_for_server() -> None:
    for _ in range(60):
        try:
            urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1)
            return
        except Exception:
            time.sleep(0.5)
    sys.exit("legacy server did not start")


def main() -> None:
    enc = encryptor(os.environ["JIZIFIN_PASSPHRASE"])
    conn = sqlite3.connect(DB)
    conn.execute("DELETE FROM app_config WHERE key = 'magic_word'")  # copy only: lets the capture skip login
    conn.commit()
    salary = enc("SALARY")
    pristine = {m: totals(conn, m, salary) for m in months(conn)}  # before startup: what the migrator reads
    last_id = conn.execute("SELECT COALESCE(MAX(id), 0) FROM expenses").fetchone()[0]
    server = subprocess.Popen(["uvicorn", "app.main:app", "--port", str(PORT), "--log-level", "warning"])
    try:
        wait_for_server()
        base = {
            "personal_cats": ",".join(enc(c) for c in ("PERSONAL COST", "LEISURE", "GIFT")),
            "combined_fixed_cat": enc("Combined Fixed"), "apartment_cat": enc("Apartment"),
            "jane_name": enc("Jane"), "john_name": enc("John"),
        }
        OUT.mkdir(parents=True, exist_ok=True)
        startup = startup_materialized(conn, last_id)
        for month in months(conn):
            paybacks = get("/analytics/paybacks", {**base, "month": month})
            monthly = get("/analytics/monthly-total", {"month": month})
            (OUT / f"{month}.json").write_text(json.dumps({"paybacks": paybacks, "monthly_total": monthly}, indent=1, sort_keys=True))
            print(f"{month}: {len(paybacks['debts'])} debts")
        (OUT / "totals.json").write_text(json.dumps(pristine, indent=1, sort_keys=True))
        (OUT / "startup_materialized.json").write_text(json.dumps(startup, indent=1, sort_keys=True))
        print("recurring expenses the legacy app generated at startup:", json.dumps(startup))
    finally:
        server.terminate()


main()
