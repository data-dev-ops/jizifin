"""
main.py — FastAPI application entry point.

Routes
------
GET    /                              Health check
GET    /users                         List users (add ?include_deactivated=true)
POST   /users                         Create user
PUT    /users/{name}                  Update user color / active flag
DELETE /users/{name}                  Hard-delete user (blocked if history exists)
GET    /expenses                      List expenses (newest first)
POST   /expenses                      Create expense + broadcast WS tick
PUT    /expenses/{id}                 Update expense (blocked if month locked)
DELETE /expenses/{id}                 Delete expense (blocked if month locked)
GET    /splits                        List all split categories with allocations
POST   /splits                        Create a new split category
PUT    /splits/{category}             Replace allocations for a category
GET    /analytics/monthly-total       Query view_monthly_total
GET    /analytics/by-category         Query view_monthly_by_category
GET    /analytics/by-payer            Query view_monthly_by_payer
GET    /analytics/paybacks            N-user payback balances via debt simplification
GET    /analytics/budgets             Budget actuals vs. limits
GET    /analytics/income-by-person    Income per active user with salary carry-forward
WS     /ws/finance                    Real-time expense ticker (fan-out broadcast)
GET    /projects                      List all projects with computed stats
POST   /projects                      Create new project
PUT    /projects/{project_id}         Update project details
DELETE /projects/{project_id}         Delete project (expenses retain history)
GET    /recurring                     List recurring expenses
POST   /recurring                     Create recurring expense
DELETE /recurring/{id}                Delete recurring expense
GET    /budgets                       List all budget rows
POST   /budgets                       Upsert a budget limit
DELETE /budgets/{category}/{month}    Remove a budget row
GET    /settlements                   List locked months
POST   /settlements                   Lock a month
POST   /query                         Execute raw SQL (dev utility)
"""

from __future__ import annotations

import json
import logging
import os
import secrets
import shutil
import time
from contextlib import asynccontextmanager
from datetime import date as _date, datetime as _datetime, timezone as _timezone
from pathlib import Path
from typing import Annotated

import aiosqlite
from apscheduler.schedulers.asyncio import AsyncIOScheduler
from fastapi import (
    BackgroundTasks,
    Depends,
    FastAPI,
    File,
    Form,
    Header,
    HTTPException,
    Query,
    Request,
    Response,
    UploadFile,
    WebSocket,
    WebSocketDisconnect,
    status,
)
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from pydantic import BaseModel
from starlette.middleware.base import BaseHTTPMiddleware

from .database import get_db, init_db
from .models import (
    AllocationEntry,
    AuthLoginRequest,
    AuthLoginResponse,
    AuthSaltRequest,
    AuthStatusResponse,
    BudgetCreate,
    BudgetResponse,
    BudgetStatusRow,
    DebtItem,
    ExpenseCreate,
    ExpenseResponse,
    ExpenseUpdate,
    IncomeByPersonRow,
    IncomeCategoryCreate,
    IncomeCategoryResponse,
    IncomeCreate,
    IncomeResponse,
    JobCreate,
    JobResponse,
    JobUpdate,
    LatestSalaryRow,
    MonthlyCategoryRow,
    MonthlyPayerRow,
    MonthlyTotal,
    PaybackRow,
    PaybackSummary,
    ProjectCreate,
    ProjectResponse,
    ProjectUpdate,
    RecurringCreate,
    RecurringResponse,
    SettlementCreate,
    SettlementResponse,
    SplitCreate,
    SplitResponse,
    SplitUpdate,
    TagCategoryRow,
    TagCreate,
    TagDetailResponse,
    TagMonthRow,
    TagResponse,
    TagTotalRow,
    TagUpdate,
    UserCreate,
    UserResponse,
    UserUpdate,
    JointAccountCreate,
    JointAccountUpdate,
    JointAccountResponse,
    JointAccountDepositCreate,
    JointAccountDepositResponse,
    JointAccountExpectedCostCreate,
    JointAccountExpectedCostResponse,
    JointAccountCorrectionCreate,
    JointAccountCorrectionResponse,
    JointAccountCategoryRow,
    JointAccountDashboardResponse,
    JointAccountMonthlyDepositUpdate,
    JointAccountMonthlyDepositRow,
)
from .crypto_utils import (
    derive_key,
    encrypt_database,
    decrypt_database,
    encrypt_text,
    decrypt_text,
    decrypt_database_connection,
)


# ---------------------------------------------------------------------------
# WebSocket connection manager
# ---------------------------------------------------------------------------

class ConnectionManager:
    """Thread-safe (asyncio-safe) fan-out broadcaster for WebSocket clients."""

    def __init__(self) -> None:
        self._active: set[WebSocket] = set()

    async def connect(self, ws: WebSocket) -> None:
        await ws.accept()
        self._active.add(ws)

    def disconnect(self, ws: WebSocket) -> None:
        self._active.discard(ws)

    async def broadcast(self, payload: dict) -> None:
        dead: set[WebSocket] = set()
        message = json.dumps(payload)
        for ws in self._active:
            try:
                await ws.send_text(message)
            except Exception:
                dead.add(ws)
        self._active -= dead


manager = ConnectionManager()
logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Recurring expense scheduler
# ---------------------------------------------------------------------------

async def process_recurring_expenses() -> None:
    """
    Daily cron: insert expense rows for every recurring_expenses whose
    day_of_month matches today.  Broadcasts a WS tick for each insertion.
    """
    today = _date.today()
    today_str = today.isoformat()
    day = today.day

    async with aiosqlite.connect(__import__("pathlib").Path(__file__).resolve().parent.parent / "finance.db") as conn:
        await conn.execute("PRAGMA journal_mode=WAL;")
        await conn.execute("PRAGMA foreign_keys=ON;")
        conn.row_factory = aiosqlite.Row

        async with conn.execute(
            "SELECT id, name, cost_cents, who_paid, category FROM recurring_expenses WHERE day_of_month = ?",
            (day,),
        ) as cur:
            due = await cur.fetchall()

        for rec in due:
            async with conn.execute(
                """
                SELECT id FROM expenses
                WHERE name = ? AND expense_date = ? AND who_paid = ? AND cost_cents = ? AND category = ?
                LIMIT 1
                """,
                (rec["name"], today_str, rec["who_paid"], rec["cost_cents"], rec["category"]),
            ) as check_cur:
                exists = await check_cur.fetchone()
            if exists:
                continue

            await conn.execute(
                "INSERT INTO expenses (name, cost_cents, expense_date, who_paid, category) VALUES (?, ?, ?, ?, ?)",
                (rec["name"], rec["cost_cents"], today_str, rec["who_paid"], rec["category"]),
            )

        await conn.commit()

        for rec in due:
            await manager.broadcast({
                "event": "expense_created",
                "payload": {
                    "name":         rec["name"],
                    "cost_cents":   rec["cost_cents"],
                    "expense_date": today_str,
                    "who_paid":     rec["who_paid"],
                    "category":     rec["category"],
                    "source":       "recurring",
                },
            })


# ---------------------------------------------------------------------------
# Application lifespan
# ---------------------------------------------------------------------------

_scheduler = AsyncIOScheduler()


@asynccontextmanager
async def lifespan(app: FastAPI):
    await init_db()
    _scheduler.add_job(
        process_recurring_expenses,
        trigger="cron",
        hour=0,
        minute=0,
        id="recurring_daily",
        replace_existing=True,
    )
    _scheduler.start()
    yield
    _scheduler.shutdown(wait=False)


# ---------------------------------------------------------------------------
# Sessions & Authentication Store
# ---------------------------------------------------------------------------

_ACTIVE_SESSIONS: dict[str, float] = {}  # token -> last_active_timestamp
SESSION_TTL_SECONDS = 7 * 24 * 3600  # 7 days


def touch_session(token: str) -> bool:
    """Validate and refresh active session token in O(1) memory lookup (< 0.01ms)."""
    if not token or token not in _ACTIVE_SESSIONS:
        return False
    now = time.time()
    if now - _ACTIVE_SESSIONS[token] < SESSION_TTL_SECONDS:
        _ACTIVE_SESSIONS[token] = now
        return True
    else:
        _ACTIVE_SESSIONS.pop(token, None)
        return False


def create_session() -> str:
    """Generate a cryptographically secure 256-bit ephemeral session token."""
    token = secrets.token_hex(32)
    _ACTIVE_SESSIONS[token] = time.time()
    return token


PUBLIC_PATHS = {
    "/",
    "/docs",
    "/openapi.json",
    "/redoc",
    "/auth/salt",
    "/auth/login",
    "/auth/import",
    "/auth/export",
    "/auth/status",
}


class AuthMiddleware(BaseHTTPMiddleware):
    """
    Sub-0.05ms authentication gateway.
    Protects all API resources unless the database is uninitialized or the caller provides
    a valid ephemeral session token.
    """
    async def dispatch(self, request: Request, call_next):
        path = request.url.path.rstrip("/")
        if not path:
            path = "/"

        normalized_path = path[4:] if path.startswith("/api") else path
        if not normalized_path:
            normalized_path = "/"

        # Allow public paths, docs, and CORS preflight OPTIONS requests
        if request.method == "OPTIONS" or normalized_path in PUBLIC_PATHS:
            return await call_next(request)

        # Allow test bypass if test mode flag is set
        if getattr(request.app.state, "testing", False):
            return await call_next(request)

        # Check if auth header or query token is provided
        auth_header = request.headers.get("authorization")
        token = None
        if auth_header and auth_header.startswith("Bearer "):
            token = auth_header[7:].strip()
        elif "token" in request.query_params:
            token = request.query_params["token"].strip()

        if token and touch_session(token):
            return await call_next(request)

        # If no valid token, check if database is initialized
        db_getter = request.app.dependency_overrides.get(get_db, get_db)
        try:
            async for conn in db_getter():
                async with conn.execute("SELECT value FROM app_config WHERE key = 'magic_word'") as cur:
                    row = await cur.fetchone()
                if row is None:
                    # Database is uninitialized; allow request for first-boot setup/tests
                    return await call_next(request)
                break
        except Exception:
            # Database or table might not exist yet
            return await call_next(request)

        # Block unauthenticated request
        return JSONResponse(
            status_code=status.HTTP_401_UNAUTHORIZED,
            content={"detail": "Authentication required. Please unlock with master password."},
            headers={"WWW-Authenticate": "Bearer"},
        )


# ---------------------------------------------------------------------------
# FastAPI app + Hardened CORS & Auth Middleware
# ---------------------------------------------------------------------------

app = FastAPI(
    title="Finance Tracker API",
    version="0.2.0",
    description="Household finance tracking — supports any number of users",
    lifespan=lifespan,
)

ALLOWED_ORIGINS = [
    "http://localhost:5173",
    "http://localhost:4173",
    "http://localhost:8000",
    "http://localhost",
    "http://127.0.0.1:5173",
    "http://127.0.0.1:8000",
    "http://127.0.0.1",
    "https://jizifin.duckdns.org",
]
env_origins = os.getenv("ALLOWED_ORIGINS")
if env_origins:
    for orig in env_origins.split(","):
        orig = orig.strip()
        if orig and orig not in ALLOWED_ORIGINS:
            ALLOWED_ORIGINS.append(orig)

app.add_middleware(
    CORSMiddleware,
    allow_origins=ALLOWED_ORIGINS,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
app.add_middleware(AuthMiddleware)

DbDep = Annotated[aiosqlite.Connection, Depends(get_db)]


# ---------------------------------------------------------------------------
# Health check
# ---------------------------------------------------------------------------

@app.get("/", tags=["health"])
async def root() -> dict:
    return {"status": "ok", "service": "finance-tracker"}


# ---------------------------------------------------------------------------
# Auth / Session Endpoints
# ---------------------------------------------------------------------------

@app.get("/auth/status", response_model=AuthStatusResponse, tags=["auth"])
async def auth_status(request: Request, db: DbDep) -> AuthStatusResponse:
    """Return instance initialization state and current session authentication status."""
    async with db.execute("SELECT value FROM app_config WHERE key = 'magic_word'") as cur:
        row = await cur.fetchone()
    initialized = row is not None
    auth_header = request.headers.get("authorization")
    token = None
    if auth_header and auth_header.startswith("Bearer "):
        token = auth_header[7:].strip()
    authenticated = bool(token and touch_session(token))
    return AuthStatusResponse(initialized=initialized, authenticated=authenticated)


@app.get("/auth/salt", tags=["auth"])
async def get_auth_salt(db: DbDep) -> dict:
    """Return the encrypted magic word to validate the salt on login."""
    async with db.execute("SELECT value FROM app_config WHERE key = 'magic_word'") as cur:
        row = await cur.fetchone()
    if row is None:
        raise HTTPException(status_code=404, detail="Not initialized")
    return {"value": row["value"]}


@app.post("/auth/salt", tags=["auth"])
async def set_auth_salt(payload: AuthSaltRequest, db: DbDep) -> dict:
    """Set the encrypted magic word on first boot and return an initial session token."""
    async with db.execute("SELECT value FROM app_config WHERE key = 'magic_word'") as cur:
        if await cur.fetchone() is not None:
            raise HTTPException(status_code=409, detail="Already initialized")
    await db.execute("INSERT INTO app_config (key, value) VALUES ('magic_word', ?)", (payload.value,))
    await db.commit()
    token = create_session()
    return {"status": "ok", "token": token}


@app.post("/auth/login", response_model=AuthLoginResponse, tags=["auth"])
async def login(payload: AuthLoginRequest, db: DbDep) -> AuthLoginResponse:
    """Authenticate with encryption key proof and receive an ephemeral session token."""
    async with db.execute("SELECT value FROM app_config WHERE key = 'magic_word'") as cur:
        row = await cur.fetchone()
    if row is None:
        raise HTTPException(status_code=400, detail="Database not initialized")
    if payload.proof != row["value"]:
        raise HTTPException(status_code=401, detail="Incorrect master password")
    token = create_session()
    return AuthLoginResponse(status="ok", token=token)


@app.post("/auth/logout", tags=["auth"])
async def logout(authorization: Annotated[str | None, Header()] = None) -> dict:
    """Invalidate active session token."""
    if authorization and authorization.startswith("Bearer "):
        token = authorization[7:].strip()
        _ACTIVE_SESSIONS.pop(token, None)
    return {"status": "ok"}


@app.post("/auth/export", tags=["auth"])
async def export_database(payload: AuthSaltRequest, db: DbDep):
    """Decrypt a copy of the database in RAM and stream it directly with zero disk footprint."""
    # Verify password
    async with db.execute("SELECT value FROM app_config WHERE key = 'magic_word'") as cur:
        row = await cur.fetchone()
    if row is None:
        raise HTTPException(status_code=400, detail="Database not initialized")
        
    # Verify salt by checking magic word using local crypto_utils
    key = derive_key(payload.value)
    if decrypt_text(row["value"], key) != "FinanceTrackerAuth":
        raise HTTPException(status_code=401, detail="Incorrect master password")

    # In-memory zero-disk database decryption
    db_path = Path(__file__).resolve().parent.parent / "finance.db"
    import sqlite3
    mem_conn = sqlite3.connect(":memory:")
    try:
        with sqlite3.connect(db_path) as src:
            src.backup(mem_conn)
        decrypt_database_connection(mem_conn, key)
        db_bytes = mem_conn.serialize()
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Decryption failed: {e}")
    finally:
        mem_conn.close()

    return Response(
        content=db_bytes,
        media_type="application/octet-stream",
        headers={
            "Content-Disposition": 'attachment; filename="finance_decrypted.db"',
        }
    )


@app.post("/auth/import", tags=["auth"])
async def import_database(db: DbDep, file: UploadFile = File(...), saltText: str = Form(...)):
    """Import an unencrypted database file on first boot and encrypt it."""
    try:
        async with db.execute("SELECT value FROM app_config WHERE key = 'magic_word'") as cur:
            if await cur.fetchone() is not None:
                raise HTTPException(status_code=409, detail="Database already initialized")
    except Exception:
        pass # Table doesn't exist, which means it's uninitialized. Proceed.
            
    db_path = Path(__file__).resolve().parent.parent / "finance.db"
    
    # Clear any old WAL/SHM files to prevent corruption with the new database
    wal_path = db_path.with_name(db_path.name + "-wal")
    shm_path = db_path.with_name(db_path.name + "-shm")
    if wal_path.exists(): wal_path.unlink()
    if shm_path.exists(): shm_path.unlink()
    
    # Save the uploaded file over the current DB
    with open(db_path, "wb") as f:
        shutil.copyfileobj(file.file, f)
        
    key = derive_key(saltText)
    
    # Encrypt the data
    try:
        encrypt_database(db_path, key)
    except Exception as e:
        # If encryption fails, remove the db to stay uninitialized
        if db_path.exists():
            os.remove(db_path)
        raise HTTPException(status_code=500, detail=f"Encryption failed: {e}")
        
    # Set the magic word
    magic_val = encrypt_text("FinanceTrackerAuth", key)
    
    # Needs a new connection since we replaced the file under the hood
    async with aiosqlite.connect(db_path) as new_conn:
        await new_conn.execute("PRAGMA journal_mode=WAL;")
        await new_conn.execute("PRAGMA foreign_keys=ON;")
        await new_conn.execute("CREATE TABLE IF NOT EXISTS app_config (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
        await new_conn.execute("INSERT OR REPLACE INTO app_config (key, value) VALUES ('magic_word', ?)", (magic_val,))
        await new_conn.commit()

    token = create_session()
    return {"status": "ok", "token": token}



# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------

async def _check_month_not_locked(db: aiosqlite.Connection, date_str: str) -> None:
    month = date_str[:7]
    async with db.execute("SELECT month FROM settlements WHERE month = ?", (month,)) as cur:
        if await cur.fetchone() is not None:
            raise HTTPException(
                status_code=status.HTTP_400_BAD_REQUEST,
                detail=f"Month {month} is locked. No modifications allowed.",
            )


async def _assert_active_user(db: aiosqlite.Connection, name: str) -> None:
    """Raise 422 if *name* is not a known active user."""
    if name in ("Joint Account", "JOINT"):
        return
    async with db.execute(
        "SELECT name FROM users WHERE name = ? AND is_active = 1", (name,)
    ) as cur:
        if await cur.fetchone() is None:
            raise HTTPException(
                status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
                detail=f"User '{name}' is not an active user.",
            )


async def _assert_user_exists(db: aiosqlite.Connection, name: str) -> None:
    """Raise 422 if *name* is not in the users table at all."""
    async with db.execute("SELECT name FROM users WHERE name = ?", (name,)) as cur:
        if await cur.fetchone() is None:
            raise HTTPException(
                status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
                detail=f"User '{name}' not found.",
            )


async def _batch_fetch_overrides(
    db: aiosqlite.Connection, expense_ids: list[int]
) -> dict[int, list[AllocationEntry]]:
    """Return a dict mapping expense_id → list[AllocationEntry] in one query."""
    result: dict[int, list[AllocationEntry]] = {eid: [] for eid in expense_ids}
    if not expense_ids:
        return result
    placeholders = ",".join("?" * len(expense_ids))
    async with db.execute(
        f"SELECT expense_id, user_name, pct FROM expense_overrides "
        f"WHERE expense_id IN ({placeholders}) ORDER BY expense_id, user_name",
        expense_ids,
    ) as cur:
        for row in await cur.fetchall():
            result[row["expense_id"]].append(
                AllocationEntry(user_name=row["user_name"], pct=row["pct"])
            )
    return result


async def _build_expense_responses(
    db: aiosqlite.Connection, rows: list[aiosqlite.Row]
) -> list[ExpenseResponse]:
    """Build ExpenseResponse objects for a list of expense DB rows, batching override lookups."""
    if not rows:
        return []
    ids = [r["id"] for r in rows]
    overrides_map = await _batch_fetch_overrides(db, ids)
    return [
        ExpenseResponse(
            id=r["id"],
            name=r["name"],
            cost_cents=r["cost_cents"],
            expense_date=r["expense_date"],
            who_paid=r["who_paid"],
            category=r["category"],
            project_id=r["project_id"],
            tag_id=r["tag_id"],
            is_joint=bool(r["is_joint"]) if "is_joint" in r.keys() else False,
            overrides=overrides_map[r["id"]],
        )
        for r in rows
    ]


# ---------------------------------------------------------------------------
# Users
# ---------------------------------------------------------------------------

@app.get("/users", response_model=list[UserResponse], tags=["users"])
async def list_users(
    db: DbDep,
    include_deactivated: bool = False,
) -> list[UserResponse]:
    """Return household users.  Pass include_deactivated=true to include inactive users."""
    q = (
        "SELECT name, color, is_active, created_at FROM users ORDER BY name"
        if include_deactivated
        else "SELECT name, color, is_active, created_at FROM users WHERE is_active = 1 ORDER BY name"
    )
    async with db.execute(q) as cur:
        rows = await cur.fetchall()
    return [UserResponse(**dict(r)) for r in rows]


@app.post("/users", response_model=UserResponse, status_code=status.HTTP_201_CREATED, tags=["users"])
async def create_user(user: UserCreate, db: DbDep) -> UserResponse:
    """Add a new household member."""
    try:
        await db.execute(
            "INSERT INTO users (name, color, is_active) VALUES (?, ?, ?)",
            (user.name, user.color, 1 if user.is_active else 0),
        )
        await db.commit()
    except Exception as exc:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail=f"User '{user.name}' already exists.",
        ) from exc
    async with db.execute(
        "SELECT name, color, is_active, created_at FROM users WHERE name = ?", (user.name,)
    ) as cur:
        row = await cur.fetchone()
    return UserResponse(**dict(row))


@app.put("/users/{name}", response_model=UserResponse, tags=["users"])
async def update_user(name: str, update: UserUpdate, db: DbDep) -> UserResponse:
    """Update a user's color and/or active flag."""
    async with db.execute("SELECT name FROM users WHERE name = ?", (name,)) as cur:
        if await cur.fetchone() is None:
            raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"User '{name}' not found.")
    sets, params = [], []
    if update.color is not None:
        sets.append("color = ?")
        params.append(update.color)
    if update.is_active is not None:
        sets.append("is_active = ?")
        params.append(1 if update.is_active else 0)
    if not sets:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="No fields to update.")
    params.append(name)
    await db.execute(f"UPDATE users SET {', '.join(sets)} WHERE name = ?", params)
    await db.commit()
    async with db.execute(
        "SELECT name, color, is_active, created_at FROM users WHERE name = ?", (name,)
    ) as cur:
        row = await cur.fetchone()
    return UserResponse(**dict(row))


@app.delete("/users/{name}", status_code=status.HTTP_204_NO_CONTENT, tags=["users"])
async def delete_user(name: str, db: DbDep) -> None:
    """
    Hard-delete a user.  Blocked if the user has any expense or income history
    (deactivate them instead to preserve data integrity).
    """
    async with db.execute("SELECT name FROM users WHERE name = ?", (name,)) as cur:
        if await cur.fetchone() is None:
            raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"User '{name}' not found.")
    async with db.execute("SELECT 1 FROM expenses WHERE who_paid = ? LIMIT 1", (name,)) as cur:
        has_expenses = await cur.fetchone() is not None
    async with db.execute("SELECT 1 FROM income WHERE who = ? LIMIT 1", (name,)) as cur:
        has_income = await cur.fetchone() is not None
    if has_expenses or has_income:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail=(
                f"User '{name}' has expense/income history and cannot be deleted. "
                "Deactivate them instead to preserve data integrity."
            ),
        )
    await db.execute("DELETE FROM users WHERE name = ?", (name,))
    await db.commit()


# ---------------------------------------------------------------------------
# Expenses
# ---------------------------------------------------------------------------

@app.get("/expenses", response_model=list[ExpenseResponse], tags=["expenses"])
async def list_expenses(
    db: DbDep,
    limit: int = 200,
    offset: int = 0,
    who_paid: str | None = None,
    category: str | None = None,
) -> list[ExpenseResponse]:
    conditions: list[str] = []
    params: list[object] = []
    if who_paid:
        conditions.append("who_paid = ?")
        params.append(who_paid)
    if category:
        conditions.append("category = ?")
        params.append(category)
    where = ("WHERE " + " AND ".join(conditions)) if conditions else ""
    query = f"""
        SELECT id, name, cost_cents, expense_date, who_paid, category, project_id, tag_id, is_joint
        FROM   expenses
        {where}
        ORDER  BY expense_date DESC, id DESC
        LIMIT  ? OFFSET ?
    """
    params.extend([limit, offset])
    async with db.execute(query, params) as cur:
        rows = await cur.fetchall()
    return await _build_expense_responses(db, rows)


@app.post(
    "/expenses",
    response_model=ExpenseResponse,
    status_code=status.HTTP_201_CREATED,
    tags=["expenses"],
)
async def create_expense(expense: ExpenseCreate, db: DbDep) -> ExpenseResponse:
    await _check_month_not_locked(db, expense.expense_date)
    await _assert_active_user(db, expense.who_paid)

    async with db.execute(
        "SELECT category FROM splits WHERE category = ?", (expense.category,)
    ) as cur:
        if await cur.fetchone() is None:
            raise HTTPException(
                status_code=status.HTTP_422_UNPROCESSABLE_ENTITY,
                detail=f"Category '{expense.category}' does not exist in splits table.",
            )

    is_joint_val = 1 if expense.is_joint else 0
    async with db.execute(
        "INSERT INTO expenses (name, cost_cents, expense_date, who_paid, category, project_id, tag_id, is_joint) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        (expense.name, expense.cost_cents, expense.expense_date, expense.who_paid, expense.category, expense.project_id, expense.tag_id, is_joint_val),
    ) as cur:
        new_id = cur.lastrowid

    if expense.overrides:
        for alloc in expense.overrides:
            await db.execute(
                "INSERT INTO expense_overrides (expense_id, user_name, pct) VALUES (?, ?, ?)",
                (new_id, alloc.user_name, alloc.pct),
            )

    if expense.is_joint:
        await db.execute("UPDATE joint_account SET balance_cents = balance_cents - ? WHERE id = 1", (expense.cost_cents,))

    await db.commit()

    async with db.execute(
        "SELECT id, name, cost_cents, expense_date, who_paid, category, project_id, tag_id, is_joint FROM expenses WHERE id = ?",
        (new_id,),
    ) as cur:
        row = await cur.fetchone()

    result = (await _build_expense_responses(db, [row]))[0]
    await manager.broadcast({"event": "expense_created", "payload": {
        "id": new_id, "name": expense.name, "cost_cents": expense.cost_cents,
        "expense_date": expense.expense_date, "who_paid": expense.who_paid,
        "category": expense.category, "is_joint": expense.is_joint,
    }})
    return result


@app.put("/expenses/{expense_id}", response_model=ExpenseResponse, tags=["expenses"])
async def update_expense(expense_id: int, update: ExpenseUpdate, db: DbDep) -> ExpenseResponse:
    async with db.execute(
        "SELECT id, name, cost_cents, expense_date, who_paid, category, project_id, tag_id, is_joint FROM expenses WHERE id = ?",
        (expense_id,),
    ) as cur:
        existing = await cur.fetchone()
    if existing is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Expense {expense_id} not found.")

    await _check_month_not_locked(db, existing["expense_date"])
    if update.expense_date:
        await _check_month_not_locked(db, update.expense_date)
    if update.who_paid:
        await _assert_user_exists(db, update.who_paid)

    new_name         = update.name         if update.name         is not None else existing["name"]
    new_cost_cents   = update.cost_cents   if update.cost_cents   is not None else existing["cost_cents"]
    new_expense_date = update.expense_date if update.expense_date is not None else existing["expense_date"]
    new_who_paid     = update.who_paid     if update.who_paid     is not None else existing["who_paid"]
    new_category     = update.category     if update.category     is not None else existing["category"]
    new_project_id   = update.project_id   if update.project_id   is not None else existing["project_id"]
    new_tag_id       = update.tag_id       if update.tag_id       is not None else existing["tag_id"]
    new_is_joint     = update.is_joint     if update.is_joint     is not None else bool(existing["is_joint"])
    new_is_joint_val = 1 if new_is_joint else 0

    old_is_joint = bool(existing["is_joint"])
    old_cost = existing["cost_cents"]

    await db.execute(
        "UPDATE expenses SET name=?, cost_cents=?, expense_date=?, who_paid=?, category=?, project_id=?, tag_id=?, is_joint=? WHERE id=?",
        (new_name, new_cost_cents, new_expense_date, new_who_paid, new_category, new_project_id, new_tag_id, new_is_joint_val, expense_id),
    )

    # Overrides: None = keep as-is; [] = clear all; [items] = replace
    if update.overrides is not None:
        await db.execute("DELETE FROM expense_overrides WHERE expense_id = ?", (expense_id,))
        for alloc in update.overrides:
            await db.execute(
                "INSERT INTO expense_overrides (expense_id, user_name, pct) VALUES (?, ?, ?)",
                (expense_id, alloc.user_name, alloc.pct),
            )

    # Adjust joint account balance if is_joint or cost changed
    if old_is_joint and not new_is_joint:
        await db.execute("UPDATE joint_account SET balance_cents = balance_cents + ? WHERE id = 1", (old_cost,))
    elif not old_is_joint and new_is_joint:
        await db.execute("UPDATE joint_account SET balance_cents = balance_cents - ? WHERE id = 1", (new_cost_cents,))
    elif old_is_joint and new_is_joint and old_cost != new_cost_cents:
        diff = new_cost_cents - old_cost
        await db.execute("UPDATE joint_account SET balance_cents = balance_cents - ? WHERE id = 1", (diff,))

    await db.commit()

    async with db.execute(
        "SELECT id, name, cost_cents, expense_date, who_paid, category, project_id, tag_id, is_joint FROM expenses WHERE id = ?",
        (expense_id,),
    ) as cur:
        row = await cur.fetchone()
    return (await _build_expense_responses(db, [row]))[0]


@app.delete("/expenses/{expense_id}", status_code=status.HTTP_204_NO_CONTENT, tags=["expenses"])
async def delete_expense(expense_id: int, db: DbDep) -> None:
    async with db.execute("SELECT id, expense_date, cost_cents, is_joint FROM expenses WHERE id = ?", (expense_id,)) as cur:
        row = await cur.fetchone()
    if row is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Expense {expense_id} not found.")
    await _check_month_not_locked(db, row["expense_date"])
    if row["is_joint"]:
        await db.execute("UPDATE joint_account SET balance_cents = balance_cents + ? WHERE id = 1", (row["cost_cents"],))
    await db.execute("DELETE FROM expenses WHERE id = ?", (expense_id,))
    await db.commit()


# ---------------------------------------------------------------------------
# Projects
# ---------------------------------------------------------------------------

async def _build_project_response(db: aiosqlite.Connection, row: aiosqlite.Row) -> ProjectResponse:
    from datetime import date as _date
    import calendar

    project_id   = row["id"]
    target_cents = row["target_cents"]

    async with db.execute(
        "SELECT total_spent_cents FROM view_project_summary WHERE id = ?",
        (project_id,),
    ) as cur:
        total_row = await cur.fetchone()
    total_spent = total_row["total_spent_cents"] if total_row else 0

    async with db.execute(
        "SELECT cost_cents, expense_date, who_paid, name FROM expenses WHERE project_id = ? ORDER BY expense_date DESC, id DESC LIMIT 1",
        (project_id,),
    ) as cur:
        last_row = await cur.fetchone()
    last_payment = __import__("app.models", fromlist=["LastPaymentInfo"]).LastPaymentInfo(**dict(last_row)) if last_row else None

    async with db.execute(
        "SELECT strftime('%Y-%m', expense_date) AS month, SUM(cost_cents) AS monthly_total FROM expenses WHERE project_id = ? GROUP BY month",
        (project_id,),
    ) as cur:
        month_rows = await cur.fetchall()

    if month_rows:
        avg_monthly = round(sum(r["monthly_total"] for r in month_rows) / len(month_rows))
    else:
        avg_monthly = 0

    remaining = target_cents - total_spent
    if remaining <= 0:
        est_date = "Completed"
    elif avg_monthly == 0:
        est_date = "Indefinite"
    else:
        months_int = int(remaining / avg_monthly) + (1 if remaining % avg_monthly else 0)
        today = _date.today()
        y, m = today.year, today.month
        m += months_int
        y += (m - 1) // 12
        m = (m - 1) % 12 + 1
        last_day = calendar.monthrange(y, m)[1]
        est_date = _date(y, m, min(today.day, last_day)).isoformat()

    return ProjectResponse(
        id=project_id,
        name=row["name"],
        target_cents=target_cents,
        target_date=row["target_date"],
        total_spent_cents=total_spent,
        avg_monthly_payment_cents=avg_monthly,
        last_payment=last_payment,
        is_joint=bool(row["is_joint"]) if "is_joint" in row.keys() else False,
        allow_subcategories=bool(row["allow_subcategories"]) if "allow_subcategories" in row.keys() else True,
        estimated_completion_date=est_date,
    )


@app.get("/projects", response_model=list[ProjectResponse], tags=["projects"])
async def list_projects(db: DbDep) -> list[ProjectResponse]:
    async with db.execute("SELECT id, name, target_cents, target_date, is_joint, allow_subcategories FROM projects ORDER BY id") as cur:
        rows = await cur.fetchall()
    return [await _build_project_response(db, row) for row in rows]


@app.post("/projects", response_model=ProjectResponse, status_code=status.HTTP_201_CREATED, tags=["projects"])
async def create_project(project: ProjectCreate, db: DbDep) -> ProjectResponse:
    try:
        async with db.execute(
            "INSERT INTO projects (name, target_cents, target_date, is_joint, allow_subcategories) VALUES (?, ?, ?, ?, ?)",
            (project.name, project.target_cents, project.target_date, 1 if project.is_joint else 0, 1 if project.allow_subcategories else 0),
        ) as cur:
            new_id = cur.lastrowid
        await db.commit()
    except Exception as exc:
        raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail=f"Project '{project.name}' already exists.") from exc
    async with db.execute("SELECT id, name, target_cents, target_date, is_joint, allow_subcategories FROM projects WHERE id = ?", (new_id,)) as cur:
        row = await cur.fetchone()
    return await _build_project_response(db, row)


@app.put("/projects/{project_id}", response_model=ProjectResponse, tags=["projects"])
async def update_project(project_id: int, update: ProjectUpdate, db: DbDep) -> ProjectResponse:
    async with db.execute("SELECT id, name, target_cents, target_date, is_joint, allow_subcategories FROM projects WHERE id = ?", (project_id,)) as cur:
        existing = await cur.fetchone()
    if existing is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Project {project_id} not found.")
    new_name   = update.name         if update.name         is not None else existing["name"]
    new_target = update.target_cents if update.target_cents is not None else existing["target_cents"]
    new_date   = update.target_date  if update.target_date  is not None else existing["target_date"]
    new_joint  = 1 if update.is_joint else (0 if update.is_joint is False else existing["is_joint"])
    new_allow_subcats = 1 if update.allow_subcategories else (0 if update.allow_subcategories is False else existing["allow_subcategories"])
    await db.execute("UPDATE projects SET name=?, target_cents=?, target_date=?, is_joint=?, allow_subcategories=? WHERE id=?", (new_name, new_target, new_date, new_joint, new_allow_subcats, project_id))
    await db.commit()
    async with db.execute("SELECT id, name, target_cents, target_date, is_joint, allow_subcategories FROM projects WHERE id = ?", (project_id,)) as cur:
        row = await cur.fetchone()
    return await _build_project_response(db, row)


@app.delete("/projects/{project_id}", status_code=status.HTTP_204_NO_CONTENT, tags=["projects"])
async def delete_project(project_id: int, db: DbDep) -> None:
    async with db.execute("SELECT id FROM projects WHERE id = ?", (project_id,)) as cur:
        if await cur.fetchone() is None:
            raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Project {project_id} not found.")
    await db.execute("UPDATE expenses SET project_id = NULL WHERE project_id = ?", (project_id,))
    await db.execute("DELETE FROM projects WHERE id = ?", (project_id,))
    await db.commit()


# ---------------------------------------------------------------------------
# Tags
# ---------------------------------------------------------------------------

@app.get("/tags", response_model=list[TagTotalRow], tags=["tags"])
async def list_tags(db: DbDep) -> list[TagTotalRow]:
    """Return all tags with all-time aggregated totals from view_tag_totals."""
    async with db.execute(
        "SELECT id, name, color, description, is_joint, is_active, total_amount, expense_count, first_date, last_date"
        " FROM view_tag_totals ORDER BY name"
    ) as cur:
        rows = await cur.fetchall()
    return [
        TagTotalRow(
            id=r["id"],
            name=r["name"],
            color=r["color"],
            description=r["description"],
            total_amount=r["total_amount"],
            expense_count=r["expense_count"],
            first_date=r["first_date"],
            last_date=r["last_date"],
            is_joint=bool(r["is_joint"]) if "is_joint" in r.keys() else False,
            is_active=bool(r["is_active"]) if "is_active" in r.keys() else True,
        )
        for r in rows
    ]


@app.post("/tags", response_model=TagResponse, status_code=status.HTTP_201_CREATED, tags=["tags"])
async def create_tag(tag: TagCreate, db: DbDep) -> TagResponse:
    """Create a new expense tag."""
    try:
        async with db.execute(
            "INSERT INTO tags (name, color, description, is_joint, is_active) VALUES (?, ?, ?, ?, ?)",
            (tag.name, tag.color, tag.description, 1 if tag.is_joint else 0, 1 if tag.is_active else 0),
        ) as cur:
            new_id = cur.lastrowid
    except Exception as exc:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail=f"Tag name '{tag.name}' already exists.",
        ) from exc
    await db.commit()
    async with db.execute(
        "SELECT id, name, color, description, is_joint, is_active, created_at FROM tags WHERE id = ?", (new_id,)
    ) as cur:
        row = await cur.fetchone()
    return TagResponse(
        id=row["id"],
        name=row["name"],
        color=row["color"],
        description=row["description"],
        created_at=row["created_at"],
        is_joint=bool(row["is_joint"]),
        is_active=bool(row["is_active"]),
    )


@app.put("/tags/{tag_id}", response_model=TagResponse, tags=["tags"])
async def update_tag(tag_id: int, update: TagUpdate, db: DbDep) -> TagResponse:
    """Rename, recolor, toggle active state, or update the description of a tag."""
    async with db.execute(
        "SELECT id, name, color, description, is_joint, is_active, created_at FROM tags WHERE id = ?", (tag_id,)
    ) as cur:
        existing = await cur.fetchone()
    if existing is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Tag {tag_id} not found.")

    new_name        = update.name        if update.name        is not None else existing["name"]
    new_color       = update.color       if update.color       is not None else existing["color"]
    new_description = update.description if update.description is not None else existing["description"]
    new_joint       = (1 if update.is_joint else 0) if update.is_joint is not None else (existing["is_joint"] if existing["is_joint"] is not None else 0)
    new_active      = (1 if update.is_active else 0) if update.is_active is not None else (existing["is_active"] if existing["is_active"] is not None else 1)

    await db.execute(
        "UPDATE tags SET name=?, color=?, description=?, is_joint=?, is_active=? WHERE id=?",
        (new_name, new_color, new_description, new_joint, new_active, tag_id),
    )
    await db.commit()
    async with db.execute(
        "SELECT id, name, color, description, is_joint, is_active, created_at FROM tags WHERE id = ?", (tag_id,)
    ) as cur:
        row = await cur.fetchone()
    return TagResponse(
        id=row["id"],
        name=row["name"],
        color=row["color"],
        description=row["description"],
        created_at=row["created_at"],
        is_joint=bool(row["is_joint"]),
        is_active=bool(row["is_active"]),
    )


@app.delete("/tags/{tag_id}", status_code=status.HTTP_204_NO_CONTENT, tags=["tags"])
async def delete_tag(tag_id: int, db: DbDep) -> None:
    """Delete a tag. Linked expenses become untagged (tag_id SET NULL) but are not deleted."""
    async with db.execute("SELECT id FROM tags WHERE id = ?", (tag_id,)) as cur:
        if await cur.fetchone() is None:
            raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Tag {tag_id} not found.")
    await db.execute("UPDATE expenses SET tag_id = NULL WHERE tag_id = ?", (tag_id,))
    await db.execute("DELETE FROM tags WHERE id = ?", (tag_id,))
    await db.commit()


@app.get("/analytics/tags/{tag_id}", response_model=TagDetailResponse, tags=["tags"])
async def get_tag_detail(tag_id: int, db: DbDep) -> TagDetailResponse:
    """
    Full cross-month breakdown for a single tag.
    Returns the tag summary row, spending by calendar month, and spending by category.
    This endpoint is unconstrained by the sidebar month-picker — it aggregates all time.
    """
    async with db.execute(
        "SELECT id, name, color, description, is_joint, is_active, total_amount, expense_count, first_date, last_date"
        " FROM view_tag_totals WHERE id = ?",
        (tag_id,),
    ) as cur:
        tag_row = await cur.fetchone()
    if tag_row is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Tag {tag_id} not found.")

    tag_summary = TagTotalRow(
        id=tag_row["id"],
        name=tag_row["name"],
        color=tag_row["color"],
        description=tag_row["description"],
        total_amount=tag_row["total_amount"],
        expense_count=tag_row["expense_count"],
        first_date=tag_row["first_date"],
        last_date=tag_row["last_date"],
        is_joint=bool(tag_row["is_joint"]),
        is_active=bool(tag_row["is_active"]) if "is_active" in tag_row.keys() else True,
    )

    # ── Spending by calendar month (chronological) ────────────────────────────────
    async with db.execute(
        """
        SELECT
            strftime('%Y-%m', expense_date) AS month,
            ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount,
            COUNT(*) AS expense_count
        FROM expenses
        WHERE tag_id = ?
        GROUP BY month
        ORDER BY month
        """,
        (tag_id,),
    ) as cur:
        by_month = [
            TagMonthRow(
                month=r["month"],
                total_amount=r["total_amount"],
                expense_count=r["expense_count"],
            )
            async for r in cur
        ]

    # ── Spending by category ─────────────────────────────────────────────────
    async with db.execute(
        """
        SELECT
            category,
            ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount,
            COUNT(*) AS expense_count
        FROM expenses
        WHERE tag_id = ?
        GROUP BY category
        ORDER BY total_amount DESC
        """,
        (tag_id,),
    ) as cur:
        by_category = [
            TagCategoryRow(
                category=r["category"],
                total_amount=r["total_amount"],
                expense_count=r["expense_count"],
            )
            async for r in cur
        ]

    return TagDetailResponse(tag=tag_summary, by_month=by_month, by_category=by_category)


# ---------------------------------------------------------------------------
# Splits
# ---------------------------------------------------------------------------

@app.get("/splits", response_model=list[SplitResponse], tags=["splits"])
async def list_splits(db: DbDep) -> list[SplitResponse]:
    """Return all split categories with their per-user allocation percentages."""
    async with db.execute("SELECT category FROM splits ORDER BY category") as cur:
        categories = [r["category"] async for r in cur]
    result = []
    for cat in categories:
        async with db.execute(
            "SELECT user_name, pct FROM split_allocations WHERE category = ? ORDER BY user_name",
            (cat,),
        ) as cur:
            allocs = [AllocationEntry(user_name=r["user_name"], pct=r["pct"]) async for r in cur]
        result.append(SplitResponse(category=cat, allocations=allocs))
    return result


@app.post("/splits", response_model=SplitResponse, status_code=status.HTTP_201_CREATED, tags=["splits"])
async def create_split(split: SplitCreate, db: DbDep) -> SplitResponse:
    try:
        await db.execute("INSERT INTO splits (category) VALUES (?)", (split.category,))
    except Exception as exc:
        raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail=f"Category '{split.category}' already exists.") from exc
    for alloc in split.allocations:
        await db.execute(
            "INSERT INTO split_allocations (category, user_name, pct) VALUES (?, ?, ?)",
            (split.category, alloc.user_name, alloc.pct),
        )
    await db.commit()
    return SplitResponse(category=split.category, allocations=split.allocations)


@app.put("/splits/{category}", response_model=SplitResponse, tags=["splits"])
async def update_split(category: str, update: SplitUpdate, db: DbDep) -> SplitResponse:
    async with db.execute("SELECT category FROM splits WHERE category = ?", (category,)) as cur:
        if await cur.fetchone() is None:
            raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Category '{category}' not found.")
    await db.execute("DELETE FROM split_allocations WHERE category = ?", (category,))
    for alloc in update.allocations:
        await db.execute(
            "INSERT INTO split_allocations (category, user_name, pct) VALUES (?, ?, ?)",
            (category, alloc.user_name, alloc.pct),
        )
    await db.commit()
    return SplitResponse(category=category, allocations=update.allocations)


# ---------------------------------------------------------------------------
# Analytics
# ---------------------------------------------------------------------------

@app.get("/analytics/monthly-total", response_model=MonthlyTotal, tags=["analytics"])
async def get_monthly_total(db: DbDep, month: str | None = None) -> MonthlyTotal:
    if month:
        async with db.execute(
            """
            SELECT COALESCE(ROUND(SUM(cost_cents) / 100.0, 2), 0.0) AS total_amount,
                   COUNT(*) AS expense_count, ? AS month
            FROM expenses WHERE strftime('%Y-%m', expense_date) = ?
            """,
            (month, month),
        ) as cur:
            row = await cur.fetchone()
        return MonthlyTotal(**(dict(row) if row else {"total_amount": 0.0, "expense_count": 0, "month": month}))
    async with db.execute("SELECT total_amount, expense_count, month FROM view_monthly_total") as cur:
        row = await cur.fetchone()
    if row is None:
        from datetime import date
        return MonthlyTotal(total_amount=0.0, expense_count=0, month=date.today().strftime("%Y-%m"))
    return MonthlyTotal(**dict(row))


@app.get("/analytics/by-category", response_model=list[MonthlyCategoryRow], tags=["analytics"])
async def get_monthly_by_category(db: DbDep, month: str | None = None) -> list[MonthlyCategoryRow]:
    if month:
        async with db.execute(
            """
            SELECT category, total_amount, expense_count
            FROM view_expenses_by_month_category WHERE month = ?
            """,
            (month,),
        ) as cur:
            rows = await cur.fetchall()
        return [MonthlyCategoryRow(**dict(r)) for r in rows]
    async with db.execute("SELECT category, total_amount, expense_count FROM view_monthly_by_category") as cur:
        rows = await cur.fetchall()
    return [MonthlyCategoryRow(**dict(r)) for r in rows]


@app.get("/analytics/by-payer", response_model=list[MonthlyPayerRow], tags=["analytics"])
async def get_monthly_by_payer(db: DbDep, month: str | None = None) -> list[MonthlyPayerRow]:
    if month:
        async with db.execute(
            """
            SELECT who_paid, ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount, COUNT(*) AS expense_count
            FROM expenses WHERE strftime('%Y-%m', expense_date) = ? GROUP BY who_paid
            """,
            (month,),
        ) as cur:
            rows = await cur.fetchall()
        return [MonthlyPayerRow(**dict(r)) for r in rows]
    async with db.execute("SELECT who_paid, total_amount, expense_count FROM view_monthly_by_payer") as cur:
        rows = await cur.fetchall()
    return [MonthlyPayerRow(**dict(r)) for r in rows]


@app.get("/analytics/paybacks", response_model=PaybackSummary, tags=["analytics"])
async def get_paybacks(
    personal_cats: str,
    combined_fixed_cat: str,
    apartment_cat: str,
    jane_name: str,
    john_name: str,
    db: DbDep,
    month: str | None = None
) -> PaybackSummary:
    """
    Compute per-category and overall payback balances for the given month.

    Algorithm:
      1. For each expense, determine the effective split (expense override >
         split_allocations default > equal share across all payers seen).
      2. Accumulate per-user net balance in cents (positive = overpaid = owed money).
      3. Greedy debt simplification: match creditors against debtors to produce
         a minimal list of DebtItem transfers.

    Personal-pay categories are handled by renaming the category to "<CAT> <payer>" 
    and assigning the payer a 100% effective share.
    """
    from datetime import date as _date
    target_month = month or _date.today().strftime("%Y-%m")

    # 1. Load split allocations: {category: {user_name: pct}}
    async with db.execute("SELECT category, user_name, pct FROM split_allocations") as cur:
        alloc_rows = await cur.fetchall()
    splits_dict: dict[str, dict[str, float]] = {}
    for r in alloc_rows:
        splits_dict.setdefault(r["category"], {})[r["user_name"]] = r["pct"]

    # 2. Load joint-account categories to exclude from paybacks
    async with db.execute("SELECT category FROM joint_account_categories") as cur:
        ja_cat_rows = await cur.fetchall()
    joint_categories: set[str] = {r["category"] for r in ja_cat_rows}

    # 3. Load expenses for target month (excluding joint-account categories and is_joint expenses)
    async with db.execute(
        "SELECT id, name, cost_cents, who_paid, category, is_joint FROM expenses WHERE strftime('%Y-%m', expense_date) = ?",
        (target_month,),
    ) as cur:
        expense_rows = await cur.fetchall()
    expenses = [dict(r) for r in expense_rows if r["category"] not in joint_categories and not r["is_joint"]]

    if not expenses:
        return PaybackSummary(rows=[], debts=[], month=target_month)

    # 3. Load expense overrides in batch: {expense_id: {user_name: pct}}
    exp_ids = [e["id"] for e in expenses]
    placeholders = ",".join("?" * len(exp_ids))
    async with db.execute(
        f"SELECT expense_id, user_name, pct FROM expense_overrides WHERE expense_id IN ({placeholders})",
        exp_ids,
    ) as cur:
        override_rows_raw = await cur.fetchall()
    override_map: dict[int, dict[str, float]] = {}
    for r in override_rows_raw:
        override_map.setdefault(r["expense_id"], {})[r["user_name"]] = r["pct"]

    # 4. Personal-pay category rename: payer bears 100%, others 0%
    PERSONAL_PAY: set[str] = set(personal_cats.split(",")) if personal_cats else set()
    for e in expenses:
        if e["category"] in PERSONAL_PAY:
            e["category"] = f"{e['category']} {e['who_paid']}"
            e["_personal"] = True
        else:
            e["_personal"] = False

    # Collect all users that appear in expenses or allocations
    all_users: set[str] = {e["who_paid"] for e in expenses}
    for cat_alloc in splits_dict.values():
        all_users.update(cat_alloc.keys())

    # 5. Per-category accumulation
    cat_paid:     dict[str, dict[str, int]] = {}  # {cat: {user: cents_paid}}
    cat_expected: dict[str, dict[str, int]] = {}  # {cat: {user: cents_expected}}

    for e in expenses:
        cat   = e["category"]
        payer = e["who_paid"]
        cost  = e["cost_cents"]
        eid   = e["id"]

        # Determine effective split for this expense
        if e["_personal"]:
            # 100% borne by the payer
            eff_split: dict[str, float] = {payer: 100.0}
        elif eid in override_map:
            eff_split = override_map[eid]
        else:
            # Strip personal-pay suffix before looking up allocations
            base_cat = cat.rsplit(" ", 1)[0]
            eff_split = splits_dict.get(cat) or splits_dict.get(base_cat) or {}
            if not eff_split:
                # Fallback: equal share across all known users
                n = len(all_users) or 1
                eff_split = {u: round(100.0 / n, 4) for u in all_users}

        if cat not in cat_paid:
            cat_paid[cat]     = {}
            cat_expected[cat] = {}

        cat_paid[cat][payer] = cat_paid[cat].get(payer, 0) + cost

        for user, pct in eff_split.items():
            exp_cents = round(cost * pct / 100.0)
            cat_expected[cat][user] = cat_expected[cat].get(user, 0) + exp_cents

    # 6. Build PaybackRow per category; accumulate global net balance (in cents)
    rows: list[PaybackRow] = []
    net_balance: dict[str, int] = {}  # user → net cents (positive = overpaid = owed)

    for cat, paid_map in cat_paid.items():
        exp_map     = cat_expected.get(cat, {})
        total_cents = sum(paid_map.values())
        if total_cents == 0:
            continue

        cat_users = set(paid_map) | set(exp_map)
        per_user_paid_eur:  dict[str, float] = {}
        per_user_share_pct: dict[str, float] = {}
        net_per_user_eur:   dict[str, float] = {}

        for u in sorted(cat_users):
            paid_c = paid_map.get(u, 0)
            exp_c  = exp_map.get(u, 0)
            per_user_paid_eur[u]  = round(paid_c / 100.0, 2)
            per_user_share_pct[u] = round(exp_c / total_cents * 100.0, 2) if total_cents else 0.0
            net_c  = paid_c - exp_c
            net_per_user_eur[u] = round(net_c / 100.0, 2)
            net_balance[u] = net_balance.get(u, 0) + net_c

        rows.append(PaybackRow(
            category=cat,
            total_amount=round(total_cents / 100.0, 2),
            per_user_paid=per_user_paid_eur,
            per_user_share_pct=per_user_share_pct,
            net_per_user=net_per_user_eur,
        ))

    # Apply special deduction rule: "Combined Fixed" paid by Jane is subtracted from "Apartment" paid by John
    jane_combined_fixed_paid = cat_paid.get(combined_fixed_cat, {}).get(jane_name, 0)
    john_apartment_paid = cat_paid.get(apartment_cat, {}).get(john_name, 0)
    
    # If Jane paid Combined Fixed and John paid Apartment
    if jane_combined_fixed_paid > 0 and john_apartment_paid > 0:
        # Subtract from John's net balance, add to Jane's net balance (simulating Jane paying John)
        deduction = min(jane_combined_fixed_paid, john_apartment_paid)
        net_balance[john_name] = net_balance.get(john_name, 0) - deduction
        net_balance[jane_name] = net_balance.get(jane_name, 0) + deduction

    # 7. Greedy debt simplification
    creditors = sorted(((u, v) for u, v in net_balance.items() if v > 0), key=lambda x: -x[1])
    debtors   = sorted(((u, -v) for u, v in net_balance.items() if v < 0), key=lambda x: -x[1])

    # Work with mutable lists of (user, cents)
    cred_list = list(creditors)
    debt_list = list(debtors)
    debts: list[DebtItem] = []
    ci = di = 0

    while ci < len(cred_list) and di < len(debt_list):
        creditor, credit_c = cred_list[ci]
        debtor,   debt_c   = debt_list[di]
        transfer = min(credit_c, debt_c)
        if transfer > 0:
            debts.append(DebtItem(from_user=debtor, to_user=creditor, amount=round(transfer / 100.0, 2)))
        credit_c -= transfer
        debt_c   -= transfer
        cred_list[ci] = (creditor, credit_c)
        debt_list[di] = (debtor,   debt_c)
        if credit_c <= 0:
            ci += 1
        if debt_c <= 0:
            di += 1

    return PaybackSummary(rows=rows, debts=debts, month=target_month)


# ---------------------------------------------------------------------------
# Income
# ---------------------------------------------------------------------------

@app.get("/income", response_model=list[IncomeResponse], tags=["income"])
async def list_income(
    db: DbDep,
    who: str | None = None,
    category: str | None = None,
    month: str | None = None,
    limit: int = 500,
) -> list[IncomeResponse]:
    conditions: list[str] = []
    params: list[object] = []
    if who:
        conditions.append("who = ?"); params.append(who)
    if category:
        conditions.append("category = ?"); params.append(category)
    if month:
        conditions.append("strftime('%Y-%m', income_date) = ?"); params.append(month)
    where = ("WHERE " + " AND ".join(conditions)) if conditions else ""
    query = f"SELECT id, name, amount_cents, who, category, income_date, is_joint FROM income {where} ORDER BY income_date DESC, id DESC LIMIT ?"
    params.append(limit)
    async with db.execute(query, params) as cur:
        rows = await cur.fetchall()
    return [IncomeResponse(**dict(r)) for r in rows]


@app.post("/income", response_model=list[IncomeResponse], status_code=status.HTTP_201_CREATED, tags=["income"])
async def create_income(entries: list[IncomeCreate], db: DbDep) -> list[IncomeResponse]:
    created: list[IncomeResponse] = []
    for entry in entries:
        await _assert_active_user(db, entry.who)
        async with db.execute(
            "INSERT INTO income (name, amount_cents, who, category, income_date, is_joint) VALUES (?, ?, ?, ?, ?, ?)",
            (entry.name, entry.amount_cents, entry.who, entry.category, entry.income_date, 1 if entry.is_joint else 0),
        ) as cur:
            new_id = cur.lastrowid
        if entry.is_joint:
            await db.execute("UPDATE joint_account SET balance_cents = balance_cents + ? WHERE id = 1", (entry.amount_cents,))
        async with db.execute("SELECT id, name, amount_cents, who, category, income_date, is_joint FROM income WHERE id = ?", (new_id,)) as cur:
            row = await cur.fetchone()
        created.append(IncomeResponse(**dict(row)))
    await db.commit()
    return created


@app.delete("/income/{income_id}", status_code=status.HTTP_204_NO_CONTENT, tags=["income"])
async def delete_income(income_id: int, db: DbDep) -> None:
    """Hard-delete a single income entry by id."""
    async with db.execute("SELECT id, amount_cents, is_joint FROM income WHERE id = ?", (income_id,)) as cur:
        row = await cur.fetchone()
        if row is None:
            raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Income entry {income_id} not found.")
    if row["is_joint"]:
        await db.execute("UPDATE joint_account SET balance_cents = balance_cents - ? WHERE id = 1", (row["amount_cents"],))
    await db.execute("DELETE FROM income WHERE id = ?", (income_id,))
    await db.commit()


# ---------------------------------------------------------------------------
# Income Categories
# ---------------------------------------------------------------------------

@app.get("/income-categories", response_model=list[IncomeCategoryResponse], tags=["income"])
async def list_income_categories(db: DbDep) -> list[IncomeCategoryResponse]:
    """Return all user-defined income category labels."""
    async with db.execute("SELECT category FROM income_categories ORDER BY category") as cur:
        rows = await cur.fetchall()
    return [IncomeCategoryResponse(category=r["category"]) for r in rows]


@app.post("/income-categories", response_model=IncomeCategoryResponse,
          status_code=status.HTTP_201_CREATED, tags=["income"])
async def create_income_category(payload: IncomeCategoryCreate, db: DbDep) -> IncomeCategoryResponse:
    """Add a new income category to the registry."""
    try:
        await db.execute("INSERT INTO income_categories (category) VALUES (?)", (payload.category,))
    except Exception as exc:
        raise HTTPException(
            status_code=status.HTTP_409_CONFLICT,
            detail=f"Income category already exists.",
        ) from exc
    await db.commit()
    return IncomeCategoryResponse(category=payload.category)


@app.delete("/income-categories/{category}", status_code=status.HTTP_204_NO_CONTENT, tags=["income"])
async def delete_income_category(category: str, db: DbDep) -> None:
    """Remove an income category from the registry."""
    async with db.execute("SELECT category FROM income_categories WHERE category = ?", (category,)) as cur:
        if await cur.fetchone() is None:
            raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Income category not found.")
    await db.execute("DELETE FROM income_categories WHERE category = ?", (category,))
    await db.commit()



def to_monthly_cents(amount_cents: int, frequency: str) -> int:
    """Convert pay frequency to standard monthly equivalent in whole cents."""
    if frequency == "monthly":
        return amount_cents
    elif frequency == "weekly":
        return round(amount_cents * 52 / 12)
    elif frequency == "biweekly":
        return round(amount_cents * 26 / 12)
    elif frequency == "annual":
        return round(amount_cents / 12)
    return amount_cents


# ---------------------------------------------------------------------------
# Jobs & Salary Streams
# ---------------------------------------------------------------------------

@app.get("/jobs", response_model=list[JobResponse], tags=["jobs"])
async def list_jobs(
    db: DbDep,
    who: str | None = None,
    month: str | None = None,
    is_active: bool | None = None,
) -> list[JobResponse]:
    """
    List jobs/salary streams, optionally filtered by person, month, or active status.
    If month (YYYY-MM) is provided, filters for jobs active during that month.
    """
    conditions: list[str] = []
    params: list[object] = []
    if who:
        conditions.append("who = ?")
        params.append(who)
    if is_active is not None:
        conditions.append("is_active = ?")
        params.append(1 if is_active else 0)
    if month:
        month_start = f"{month}-01"
        month_end = f"{month}-31"
        conditions.append("start_date <= ? AND (end_date IS NULL OR end_date >= ?)")
        params.extend([month_end, month_start])

    where = ("WHERE " + " AND ".join(conditions)) if conditions else ""
    query = f"SELECT id, name, who, amount_cents, frequency, start_date, end_date, notes, is_active FROM jobs {where} ORDER BY start_date DESC, id DESC"
    async with db.execute(query, params) as cur:
        rows = await cur.fetchall()

    return [
        JobResponse(
            **dict(r),
            monthly_equivalent_cents=to_monthly_cents(r["amount_cents"], r["frequency"]),
        )
        for r in rows
    ]


@app.post("/jobs", response_model=JobResponse, status_code=status.HTTP_201_CREATED, tags=["jobs"])
async def create_job(payload: JobCreate, db: DbDep) -> JobResponse:
    """Create a new job / salary stream for an active user."""
    await _assert_active_user(db, payload.who)
    async with db.execute(
        """
        INSERT INTO jobs (name, who, amount_cents, frequency, start_date, end_date, notes, is_active)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            payload.name,
            payload.who,
            payload.amount_cents,
            payload.frequency,
            payload.start_date,
            payload.end_date,
            payload.notes,
            1 if payload.is_active else 0,
        ),
    ) as cur:
        new_id = cur.lastrowid
    await db.commit()

    async with db.execute(
        "SELECT id, name, who, amount_cents, frequency, start_date, end_date, notes, is_active FROM jobs WHERE id = ?",
        (new_id,),
    ) as cur:
        row = await cur.fetchone()

    return JobResponse(
        **dict(row),
        monthly_equivalent_cents=to_monthly_cents(row["amount_cents"], row["frequency"]),
    )


@app.put("/jobs/{job_id}", response_model=JobResponse, tags=["jobs"])
async def update_job(job_id: int, payload: JobUpdate, db: DbDep) -> JobResponse:
    """Update an existing job or adjust its rate/dates/notes."""
    async with db.execute("SELECT * FROM jobs WHERE id = ?", (job_id,)) as cur:
        row = await cur.fetchone()
        if row is None:
            raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Job {job_id} not found.")

    updates: list[str] = []
    params: list[object] = []

    if payload.name is not None:
        updates.append("name = ?")
        params.append(payload.name)
    if payload.who is not None:
        await _assert_active_user(db, payload.who)
        updates.append("who = ?")
        params.append(payload.who)
    if payload.amount_cents is not None:
        updates.append("amount_cents = ?")
        params.append(payload.amount_cents)
    if payload.frequency is not None:
        updates.append("frequency = ?")
        params.append(payload.frequency)
    if payload.start_date is not None:
        updates.append("start_date = ?")
        params.append(payload.start_date)
    if payload.end_date is not None:
        updates.append("end_date = ?")
        params.append(payload.end_date)
    if payload.notes is not None:
        updates.append("notes = ?")
        params.append(payload.notes)
    if payload.is_active is not None:
        updates.append("is_active = ?")
        params.append(1 if payload.is_active else 0)

    if updates:
        params.append(job_id)
        await db.execute(f"UPDATE jobs SET {", ".join(updates)} WHERE id = ?", params)
        await db.commit()

    async with db.execute("SELECT id, name, who, amount_cents, frequency, start_date, end_date, notes, is_active FROM jobs WHERE id = ?", (job_id,)) as cur:
        updated = await cur.fetchone()

    return JobResponse(
        **dict(updated),
        monthly_equivalent_cents=to_monthly_cents(updated["amount_cents"], updated["frequency"]),
    )


@app.delete("/jobs/{job_id}", status_code=status.HTTP_204_NO_CONTENT, tags=["jobs"])
async def delete_job(job_id: int, db: DbDep) -> None:
    """Delete a job stream."""
    async with db.execute("SELECT id FROM jobs WHERE id = ?", (job_id,)) as cur:
        if await cur.fetchone() is None:
            raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Job {job_id} not found.")
    await db.execute("DELETE FROM jobs WHERE id = ?", (job_id,))
    await db.commit()

@app.get("/income/latest-salary", response_model=list[LatestSalaryRow], tags=["income"])
async def get_latest_salary(salary_cat: str, db: DbDep) -> list[LatestSalaryRow]:
    """
    Return effective monthly salary per person.
    Prioritizes active jobs; falls back to legacy SALARY income rows.
    """
    from datetime import date as _date
    current_month = _date.today().strftime("%Y-%m")
    month_start = f"{current_month}-01"
    month_end = f"{current_month}-31"

    async with db.execute("SELECT name FROM users WHERE is_active = 1 ORDER BY name") as cur:
        active_users = [r["name"] async for r in cur]

    rows: list[LatestSalaryRow] = []
    for person in active_users:
        # Check active jobs first
        async with db.execute(
            "SELECT amount_cents, frequency, name, start_date FROM jobs WHERE who = ? AND is_active = 1 AND start_date <= ? AND (end_date IS NULL OR end_date >= ?)",
            (person, month_end, month_start),
        ) as cur:
            user_jobs = await cur.fetchall()

        if user_jobs:
            total_monthly = sum(to_monthly_cents(j["amount_cents"], j["frequency"]) for j in user_jobs)
            primary_name = user_jobs[0]["name"]
            latest_date = max(j["start_date"] for j in user_jobs)
            rows.append(LatestSalaryRow(who=person, amount_cents=total_monthly, income_date=latest_date, name=primary_name))
        else:
            # Fallback to legacy income table
            async with db.execute(
                """
                SELECT who, amount_cents, income_date, name
                FROM   income
                WHERE  who = ? AND category = ?
                ORDER  BY income_date DESC, id DESC
                LIMIT 1
                """,
                (person, salary_cat),
            ) as cur:
                leg_row = await cur.fetchone()
            if leg_row:
                rows.append(LatestSalaryRow(**dict(leg_row)))

    return rows


@app.get("/analytics/income-by-person", response_model=list[IncomeByPersonRow], tags=["analytics"])
async def get_income_by_person(salary_cat: str, db: DbDep, month: str | None = None) -> list[IncomeByPersonRow]:
    """
    Return total income per active user for the requested month.
    Combines effective base salary from active jobs (or legacy salary carry-forward)
    with one-off income entries for that target month.
    """
    from datetime import date as _date
    target = month or _date.today().strftime("%Y-%m")
    month_start = f"{target}-01"
    month_end = f"{target}-31"

    # Iterate over every active user
    async with db.execute("SELECT name FROM users WHERE is_active = 1 ORDER BY name") as cur:
        active_users = [r["name"] async for r in cur]

    result: list[IncomeByPersonRow] = []
    for person in active_users:
        # 1. Check one-off income logged for this month
        async with db.execute(
            "SELECT COALESCE(SUM(amount_cents), 0) AS total_cents FROM income WHERE who=? AND strftime('%Y-%m', income_date)=? AND category!=?",
            (person, target, salary_cat),
        ) as cur:
            other_row = await cur.fetchone()
        other_cents: int = other_row["total_cents"] if other_row else 0

        # 2. Check jobs active during this target month
        async with db.execute(
            "SELECT amount_cents, frequency FROM jobs WHERE who=? AND is_active=1 AND start_date <= ? AND (end_date IS NULL OR end_date >= ?)",
            (person, month_end, month_start),
        ) as cur:
            active_jobs = await cur.fetchall()

        # Also check if user has ANY jobs in DB (to distinguish between zero income from jobs vs legacy user)
        async with db.execute("SELECT 1 FROM jobs WHERE who = ? LIMIT 1", (person,)) as cur:
            has_any_jobs = (await cur.fetchone()) is not None

        salary_this_month = 0
        is_carried = False

        if has_any_jobs:
            # User uses the jobs system
            salary_this_month = sum(to_monthly_cents(j["amount_cents"], j["frequency"]) for j in active_jobs)
            is_carried = False
        else:
            # Legacy fallback: check income table for SALARY
            async with db.execute(
                "SELECT amount_cents FROM income WHERE who=? AND category=? AND strftime('%Y-%m', income_date)=? ORDER BY income_date DESC, id DESC LIMIT 1",
                (person, salary_cat, target),
            ) as cur:
                sal_month_row = await cur.fetchone()
            salary_this_month = sal_month_row["amount_cents"] if sal_month_row else 0

            if salary_this_month == 0:
                async with db.execute(
                    "SELECT amount_cents FROM income WHERE who=? AND category=? AND income_date <= ? || '-31' ORDER BY income_date DESC, id DESC LIMIT 1",
                    (person, salary_cat, target),
                ) as cur:
                    carry_row = await cur.fetchone()
                if carry_row:
                    salary_this_month = carry_row["amount_cents"]
                    is_carried = True

        total = other_cents + salary_this_month
        if total > 0 or has_any_jobs:
            result.append(
                IncomeByPersonRow(
                    who=person,
                    total_cents=total,
                    is_carried=is_carried,
                    base_salary_cents=salary_this_month,
                    one_off_cents=other_cents,
                )
            )

    return result


# ---------------------------------------------------------------------------
# Budgets
# ---------------------------------------------------------------------------

@app.get("/analytics/budgets", response_model=list[BudgetStatusRow], tags=["analytics"])
async def get_budget_status(db: DbDep, month: str | None = None) -> list[BudgetStatusRow]:
    from datetime import date as _date
    target_month = month or _date.today().strftime("%Y-%m")
    async with db.execute(
        "SELECT category, COALESCE(SUM(cost_cents), 0) AS actual_cents FROM expenses WHERE strftime('%Y-%m', expense_date)=? GROUP BY category",
        (target_month,),
    ) as cur:
        actuals = {r["category"]: r["actual_cents"] for r in await cur.fetchall()}
    # Capture both the winning limit_cents and which budget row won (specific month vs ALL)
    async with db.execute(
        """
        SELECT
            category,
            COALESCE(MAX(CASE WHEN month=? THEN limit_cents END),
                     MAX(CASE WHEN month='ALL' THEN limit_cents END), 0) AS limit_cents,
            CASE
                WHEN MAX(CASE WHEN month=? THEN 1 END) = 1 THEN ?
                WHEN MAX(CASE WHEN month='ALL' THEN 1 END) = 1 THEN 'ALL'
                ELSE 'ALL'
            END AS budget_month
        FROM budgets
        GROUP BY category
        """,
        (target_month, target_month, target_month),
    ) as cur:
        budget_rows = {r["category"]: (r["limit_cents"], r["budget_month"]) for r in await cur.fetchall()}
    all_cats = set(actuals) | set(budget_rows)
    result: list[BudgetStatusRow] = []
    for cat in sorted(all_cats):
        limit, bmonth = budget_rows.get(cat, (0, 'ALL'))
        actual = actuals.get(cat, 0)
        pct    = round(actual / limit * 100.0, 2) if limit > 0 else 0.0
        result.append(BudgetStatusRow(category=cat, limit_cents=limit, actual_cents=actual, pct_used=pct, budget_month=bmonth))
    return result


@app.get("/budgets", response_model=list[BudgetResponse], tags=["budgets"])
async def list_budgets(db: DbDep) -> list[BudgetResponse]:
    async with db.execute("SELECT category, month, limit_cents FROM budgets ORDER BY category, month") as cur:
        rows = await cur.fetchall()
    return [BudgetResponse(**dict(r)) for r in rows]


@app.post("/budgets", response_model=BudgetResponse, status_code=status.HTTP_201_CREATED, tags=["budgets"])
async def upsert_budget(budget: BudgetCreate, db: DbDep) -> BudgetResponse:
    await db.execute(
        "INSERT INTO budgets (category, month, limit_cents) VALUES (?, ?, ?) ON CONFLICT(category, month) DO UPDATE SET limit_cents=excluded.limit_cents",
        (budget.category, budget.month, budget.limit_cents),
    )
    await db.commit()
    async with db.execute("SELECT category, month, limit_cents FROM budgets WHERE category=? AND month=?", (budget.category, budget.month)) as cur:
        row = await cur.fetchone()
    return BudgetResponse(**dict(row))


@app.delete("/budgets/{category}/{month}", status_code=status.HTTP_204_NO_CONTENT, tags=["budgets"])
async def delete_budget(category: str, month: str, db: DbDep) -> None:
    await db.execute("DELETE FROM budgets WHERE category=? AND month=?", (category, month))
    await db.commit()


# ---------------------------------------------------------------------------
# Recurring Expenses
# ---------------------------------------------------------------------------

@app.get("/recurring", response_model=list[RecurringResponse], tags=["recurring"])
async def list_recurring(db: DbDep) -> list[RecurringResponse]:
    async with db.execute("SELECT id, name, cost_cents, who_paid, category, day_of_month, is_joint FROM recurring_expenses ORDER BY id") as cur:
        rows = await cur.fetchall()
    return [RecurringResponse(**dict(r)) for r in rows]


@app.post("/recurring", response_model=RecurringResponse, status_code=status.HTTP_201_CREATED, tags=["recurring"])
async def create_recurring(rec: RecurringCreate, db: DbDep) -> RecurringResponse:
    await _assert_active_user(db, rec.who_paid)
    async with db.execute(
        "INSERT INTO recurring_expenses (name, cost_cents, who_paid, category, day_of_month, is_joint) VALUES (?, ?, ?, ?, ?, ?)",
        (rec.name, rec.cost_cents, rec.who_paid, rec.category, rec.day_of_month, 1 if rec.is_joint else 0),
    ) as cur:
        new_id = cur.lastrowid
    await db.commit()
    async with db.execute("SELECT id, name, cost_cents, who_paid, category, day_of_month, is_joint FROM recurring_expenses WHERE id=?", (new_id,)) as cur:
        row = await cur.fetchone()
    return RecurringResponse(**dict(row))


@app.delete("/recurring/{rec_id}", status_code=status.HTTP_204_NO_CONTENT, tags=["recurring"])
async def delete_recurring(rec_id: int, db: DbDep) -> None:
    async with db.execute("SELECT id FROM recurring_expenses WHERE id=?", (rec_id,)) as cur:
        if await cur.fetchone() is None:
            raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Recurring expense {rec_id} not found.")
    await db.execute("DELETE FROM recurring_expenses WHERE id=?", (rec_id,))
    await db.commit()


# ---------------------------------------------------------------------------
# Settlements
# ---------------------------------------------------------------------------

@app.get("/settlements", response_model=list[SettlementResponse], tags=["settlements"])
async def list_settlements(db: DbDep) -> list[SettlementResponse]:
    async with db.execute("SELECT month, settled_at, net_balance_transferred_cents FROM settlements ORDER BY month DESC") as cur:
        rows = await cur.fetchall()
    return [SettlementResponse(**dict(r)) for r in rows]


@app.post("/settlements", response_model=SettlementResponse, status_code=status.HTTP_201_CREATED, tags=["settlements"])
async def create_settlement(payload: SettlementCreate, db: DbDep) -> SettlementResponse:
    settled_at = _datetime.now(_timezone.utc).isoformat()
    try:
        await db.execute(
            "INSERT INTO settlements (month, settled_at, net_balance_transferred_cents) VALUES (?, ?, ?)",
            (payload.month, settled_at, payload.net_balance_transferred_cents),
        )
        await db.commit()
    except Exception as exc:
        raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail=f"Month {payload.month} is already settled.") from exc
    async with db.execute("SELECT month, settled_at, net_balance_transferred_cents FROM settlements WHERE month=?", (payload.month,)) as cur:
        row = await cur.fetchone()
    return SettlementResponse(**dict(row))


# ---------------------------------------------------------------------------
# Joint Account
# ---------------------------------------------------------------------------

async def _get_joint_account(db: aiosqlite.Connection) -> dict | None:
    """Return the singleton joint account row or None."""
    async with db.execute(
        "SELECT id, name, balance_cents, safety_margin_pct, deposit_split_mode, expected_total_cents FROM joint_account WHERE id = 1"
    ) as cur:
        row = await cur.fetchone()
    return dict(row) if row else None


@app.get("/joint-account", response_model=JointAccountResponse | None, tags=["joint-account"])
async def get_joint_account(db: DbDep):
    """Return singleton joint account config, or null if not configured."""
    return await _get_joint_account(db)


@app.post("/joint-account", response_model=JointAccountResponse, status_code=status.HTTP_201_CREATED, tags=["joint-account"])
async def create_joint_account(payload: JointAccountCreate, db: DbDep) -> JointAccountResponse:
    """Create the singleton joint account (id=1). Fails if one already exists."""
    try:
        await db.execute(
            """
            INSERT INTO joint_account (id, name, balance_cents, safety_margin_pct, deposit_split_mode, expected_total_cents)
            VALUES (1, ?, ?, ?, ?, ?)
            """,
            (payload.name, payload.balance_cents, payload.safety_margin_pct,
             payload.deposit_split_mode, payload.expected_total_cents),
        )
        await db.commit()
    except Exception as exc:
        raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="Joint account already exists.") from exc
    row = await _get_joint_account(db)
    return JointAccountResponse(**row)


@app.patch("/joint-account", response_model=JointAccountResponse, tags=["joint-account"])
async def update_joint_account(payload: JointAccountUpdate, db: DbDep) -> JointAccountResponse:
    """Partially update the joint account config."""
    row = await _get_joint_account(db)
    if row is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="No joint account configured.")
    updates: list[str] = []
    params: list = []
    for field, col in [
        ("name", "name"),
        ("balance_cents", "balance_cents"),
        ("safety_margin_pct", "safety_margin_pct"),
        ("deposit_split_mode", "deposit_split_mode"),
        ("expected_total_cents", "expected_total_cents"),
    ]:
        val = getattr(payload, field)
        if val is not None:
            updates.append(f"{col} = ?")
            params.append(val)
    if not updates:
        return JointAccountResponse(**row)
    params.append(1)
    await db.execute(f"UPDATE joint_account SET {', '.join(updates)} WHERE id = ?", params)
    await db.commit()
    row = await _get_joint_account(db)
    return JointAccountResponse(**row)


@app.delete("/joint-account", status_code=status.HTTP_204_NO_CONTENT, tags=["joint-account"])
async def delete_joint_account(db: DbDep) -> None:
    """Remove the joint account and all associated config."""
    await db.execute("DELETE FROM joint_account_corrections")
    await db.execute("DELETE FROM joint_account_expected_costs")
    await db.execute("DELETE FROM joint_account_deposits")
    await db.execute("DELETE FROM joint_account_categories")
    await db.execute("DELETE FROM joint_account WHERE id = 1")
    await db.commit()


# ── Joint Account Categories ─────────────────────────────────────────────────

@app.get("/joint-account/categories", response_model=list[str], tags=["joint-account"])
async def list_joint_account_categories(db: DbDep) -> list[str]:
    async with db.execute("SELECT category FROM joint_account_categories") as cur:
        rows = await cur.fetchall()
    return [r["category"] for r in rows]


@app.post("/joint-account/categories", status_code=status.HTTP_201_CREATED, tags=["joint-account"])
async def add_joint_account_category(payload: dict, db: DbDep):
    """Add a category to the joint account. Payload: {category: str}"""
    category = payload.get("category")
    if not category:
        raise HTTPException(status_code=400, detail="category required")
    try:
        await db.execute("INSERT INTO joint_account_categories (category) VALUES (?)", (category,))
        await db.commit()
    except Exception as exc:
        raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="Category already assigned.") from exc
    return {"category": category}


@app.delete("/joint-account/categories/{category}", status_code=status.HTTP_204_NO_CONTENT, tags=["joint-account"])
async def remove_joint_account_category(category: str, db: DbDep) -> None:
    await db.execute("DELETE FROM joint_account_categories WHERE category = ?", (category,))
    await db.commit()


# ── Joint Account Deposits ────────────────────────────────────────────────────

@app.get("/joint-account/deposits", response_model=list[JointAccountDepositResponse], tags=["joint-account"])
async def list_joint_account_deposits(db: DbDep) -> list[JointAccountDepositResponse]:
    async with db.execute("SELECT user_name, amount_cents, day_of_month FROM joint_account_deposits") as cur:
        rows = await cur.fetchall()
    return [JointAccountDepositResponse(**dict(r)) for r in rows]


@app.put("/joint-account/deposits", response_model=list[JointAccountDepositResponse], tags=["joint-account"])
async def set_joint_account_deposits(
    deposits: list[JointAccountDepositCreate], db: DbDep
) -> list[JointAccountDepositResponse]:
    """Replace all deposit records (upsert per user)."""
    for d in deposits:
        await db.execute(
            """
            INSERT INTO joint_account_deposits (user_name, amount_cents, day_of_month)
            VALUES (?, ?, ?)
            ON CONFLICT(user_name) DO UPDATE SET
                amount_cents = excluded.amount_cents,
                day_of_month = excluded.day_of_month
            """,
            (d.user_name, d.amount_cents, d.day_of_month),
        )
    await db.commit()
    async with db.execute("SELECT user_name, amount_cents, day_of_month FROM joint_account_deposits") as cur:
        rows = await cur.fetchall()
    return [JointAccountDepositResponse(**dict(r)) for r in rows]


# ── Joint Account Expected Costs ─────────────────────────────────────────────

@app.get("/joint-account/expected-costs", response_model=list[JointAccountExpectedCostResponse], tags=["joint-account"])
async def list_joint_account_expected_costs(db: DbDep) -> list[JointAccountExpectedCostResponse]:
    async with db.execute("SELECT category, expected_cents FROM joint_account_expected_costs") as cur:
        rows = await cur.fetchall()
    return [JointAccountExpectedCostResponse(**dict(r)) for r in rows]


@app.put("/joint-account/expected-costs", response_model=list[JointAccountExpectedCostResponse], tags=["joint-account"])
async def set_joint_account_expected_costs(
    costs: list[JointAccountExpectedCostCreate], db: DbDep
) -> list[JointAccountExpectedCostResponse]:
    """Replace all expected cost records."""
    # Clear and rewrite
    await db.execute("DELETE FROM joint_account_expected_costs")
    for c in costs:
        await db.execute(
            "INSERT INTO joint_account_expected_costs (category, expected_cents) VALUES (?, ?)",
            (c.category, c.expected_cents),
        )
    await db.commit()
    async with db.execute("SELECT category, expected_cents FROM joint_account_expected_costs") as cur:
        rows = await cur.fetchall()
    return [JointAccountExpectedCostResponse(**dict(r)) for r in rows]


# ── Joint Account Corrections ────────────────────────────────────────────────

@app.get("/joint-account/corrections", response_model=list[JointAccountCorrectionResponse], tags=["joint-account"])
async def list_joint_account_corrections(db: DbDep) -> list[JointAccountCorrectionResponse]:
    async with db.execute(
        "SELECT id, amount_cents, correction_date, note FROM joint_account_corrections ORDER BY correction_date DESC, id DESC"
    ) as cur:
        rows = await cur.fetchall()
    return [JointAccountCorrectionResponse(**dict(r)) for r in rows]


@app.post("/joint-account/corrections", response_model=JointAccountCorrectionResponse, status_code=status.HTTP_201_CREATED, tags=["joint-account"])
async def create_joint_account_correction(payload: JointAccountCorrectionCreate, db: DbDep) -> JointAccountCorrectionResponse:
    """Log a balance correction (positive = top-up, negative = withdrawal)."""
    row = await _get_joint_account(db)
    if row is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="No joint account configured.")
    async with db.execute(
        "INSERT INTO joint_account_corrections (amount_cents, correction_date, note) VALUES (?, ?, ?)",
        (payload.amount_cents, payload.correction_date, payload.note),
    ) as cur:
        new_id = cur.lastrowid
    # Update balance
    new_balance = row["balance_cents"] + payload.amount_cents
    await db.execute("UPDATE joint_account SET balance_cents = ? WHERE id = 1", (new_balance,))
    await db.commit()
    async with db.execute(
        "SELECT id, amount_cents, correction_date, note FROM joint_account_corrections WHERE id = ?", (new_id,)
    ) as cur:
        corr_row = await cur.fetchone()
    return JointAccountCorrectionResponse(**dict(corr_row))


@app.delete("/joint-account/corrections/{correction_id}", status_code=status.HTTP_204_NO_CONTENT, tags=["joint-account"])
async def delete_joint_account_correction(correction_id: int, db: DbDep) -> None:
    """Delete a correction and reverse its effect on balance."""
    async with db.execute(
        "SELECT amount_cents FROM joint_account_corrections WHERE id = ?", (correction_id,)
    ) as cur:
        corr = await cur.fetchone()
    if corr is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Correction not found.")
    row = await _get_joint_account(db)
    if row:
        new_balance = row["balance_cents"] - corr["amount_cents"]
        await db.execute("UPDATE joint_account SET balance_cents = ? WHERE id = 1", (new_balance,))
    await db.execute("DELETE FROM joint_account_corrections WHERE id = ?", (correction_id,))
    await db.commit()


# ── Joint Account Dashboard ──────────────────────────────────────────────────

# ── Joint Account Monthly Deposit Tracking ──────────────────────────────

async def _get_monthly_deposits_data(db: DbConnection, target_month: str):
    """Fetch scheduled vs actual deposit status for all active users for a month."""
    from datetime import date as _date
    today = _date.today()
    current_month_str = today.strftime("%Y-%m")
    current_day = today.day

    async with db.execute("SELECT name FROM users WHERE is_active = 1 ORDER BY name") as cur:
        u_rows = await cur.fetchall()
    active_users = [r["name"] for r in u_rows]

    async with db.execute("SELECT user_name, amount_cents, day_of_month FROM joint_account_deposits") as cur:
        cfg_rows = await cur.fetchall()
    cfg_map = {r["user_name"]: (r["amount_cents"], r["day_of_month"]) for r in cfg_rows}

    async with db.execute(
        "SELECT user_name, scheduled_cents, actual_cents, is_paid, paid_date FROM joint_account_monthly_deposits WHERE month = ?",
        (target_month,),
    ) as cur:
        log_rows = await cur.fetchall()
    log_map = {r["user_name"]: dict(r) for r in log_rows}

    user_rows: list[JointAccountMonthlyDepositRow] = []
    total_logged_paid_cents = 0

    for u in active_users:
        sched_cents, day_m = cfg_map.get(u, (0, 1))
        log = log_map.get(u, {})

        is_paid = bool(log.get("is_paid", 0))
        actual_cents = log.get("actual_cents", sched_cents if is_paid else 0)
        paid_date = log.get("paid_date", None)

        if is_paid:
            total_logged_paid_cents += actual_cents
            if actual_cents == sched_cents:
                status_str = "paid"
            else:
                status_str = "paid_diverted"
        else:
            if target_month == current_month_str:
                if current_day < day_m:
                    status_str = "pending"
                else:
                    status_str = "overdue"
            elif target_month < current_month_str:
                status_str = "overdue"
            else:
                status_str = "pending"

        user_rows.append(JointAccountMonthlyDepositRow(
            user_name=u,
            scheduled_cents=sched_cents,
            day_of_month=day_m,
            actual_cents=actual_cents,
            is_paid=is_paid,
            paid_date=paid_date,
            status=status_str,
        ))

    return user_rows, total_logged_paid_cents


@app.get("/joint-account/monthly-deposits", response_model=list[JointAccountMonthlyDepositRow], tags=["joint-account"])
async def get_joint_account_monthly_deposits(db: DbDep, month: str | None = None) -> list[JointAccountMonthlyDepositRow]:
    from datetime import date as _date
    target_month = month or _date.today().strftime("%Y-%m")
    user_rows, _ = await _get_monthly_deposits_data(db, target_month)
    return user_rows


@app.post("/joint-account/monthly-deposits", response_model=JointAccountMonthlyDepositRow, tags=["joint-account"])
async def update_joint_account_monthly_deposit(db: DbDep, update: JointAccountMonthlyDepositUpdate) -> JointAccountMonthlyDepositRow:
    """Mark a user's monthly deposit as paid/unpaid, with actual amount paid and date."""
    from datetime import date as _date
    paid_date = update.paid_date or _date.today().strftime("%Y-%m-%d")

    async with db.execute(
        "SELECT amount_cents, day_of_month FROM joint_account_deposits WHERE user_name = ?",
        (update.user_name,),
    ) as cur:
        cfg = await cur.fetchone()
    sched_cents = cfg["amount_cents"] if cfg else update.actual_cents
    day_m = cfg["day_of_month"] if cfg else 1

    async with db.execute(
        "SELECT is_paid, actual_cents FROM joint_account_monthly_deposits WHERE month = ? AND user_name = ?",
        (update.month, update.user_name),
    ) as cur:
        existing = await cur.fetchone()

    was_paid = bool(existing["is_paid"]) if existing else False
    old_actual = existing["actual_cents"] if existing else 0

    await db.execute(
        """
        INSERT INTO joint_account_monthly_deposits (month, user_name, scheduled_cents, actual_cents, is_paid, paid_date)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(month, user_name) DO UPDATE SET
            scheduled_cents = excluded.scheduled_cents,
            actual_cents    = excluded.actual_cents,
            is_paid         = excluded.is_paid,
            paid_date       = excluded.paid_date
        """,
        (update.month, update.user_name, sched_cents, update.actual_cents, 1 if update.is_paid else 0, paid_date if update.is_paid else None),
    )

    row = await _get_joint_account(db)
    if row:
        current_bal = row["balance_cents"]
        diff = 0
        if update.is_paid:
            if was_paid:
                diff = update.actual_cents - old_actual
            else:
                diff = update.actual_cents
        else:
            if was_paid:
                diff = -old_actual
        if diff != 0:
            await db.execute("UPDATE joint_account SET balance_cents = ? WHERE id = 1", (current_bal + diff,))

    await db.commit()

    if update.is_paid:
        st = "paid" if update.actual_cents == sched_cents else "paid_diverted"
    else:
        st = "pending"

    return JointAccountMonthlyDepositRow(
        user_name=update.user_name,
        scheduled_cents=sched_cents,
        day_of_month=day_m,
        actual_cents=update.actual_cents if update.is_paid else 0,
        is_paid=update.is_paid,
        paid_date=paid_date if update.is_paid else None,
        status=st,
    )


# ── Joint Account Dashboard ──────────────────────────────────────────────────

@app.get("/joint-account/dashboard", response_model=JointAccountDashboardResponse, tags=["joint-account"])
async def get_joint_account_dashboard(db: DbDep, month: str | None = None) -> JointAccountDashboardResponse:
    """Compute actual vs expected spending for joint account categories in a month."""
    from datetime import date as _date
    target_month = month or _date.today().strftime("%Y-%m")

    row = await _get_joint_account(db)
    if row is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="No joint account configured.")

    # Actuals per category for the month
    async with db.execute(
        "SELECT category, total_amount, expense_count FROM view_joint_account_monthly WHERE month = ?",
        (target_month,),
    ) as cur:
        actual_rows = await cur.fetchall()
    actual_map: dict[str, int] = {r["category"]: round(r["total_amount"] * 100) for r in actual_rows}

    # Expected costs per category
    async with db.execute("SELECT category, expected_cents FROM joint_account_expected_costs") as cur:
        expected_rows = await cur.fetchall()
    expected_map: dict[str, int] = {r["category"]: r["expected_cents"] for r in expected_rows}

    # All joint categories
    async with db.execute("SELECT category FROM joint_account_categories") as cur:
        cat_rows = await cur.fetchall()
    all_cats = [r["category"] for r in cat_rows]

    # Sum of expected (per-cat or global override)
    expected_total = row["expected_total_cents"]
    if expected_total is None:
        expected_total = sum(expected_map.values())

    actual_total_cents = sum(actual_map.values())

    safety = row["safety_margin_pct"]
    target_deposit_cents = round(expected_total * (1 + safety / 100.0))

    # User deposits & total deposits for month
    user_deposits, total_logged_paid_cents = await _get_monthly_deposits_data(db, target_month)

    async with db.execute(
        "SELECT COALESCE(SUM(amount_cents), 0) AS total FROM joint_account_corrections WHERE strftime('%Y-%m', correction_date) = ? AND amount_cents > 0",
        (target_month,),
    ) as cur:
        dep_row = await cur.fetchone()
    corr_deposits = dep_row["total"] if dep_row else 0

    total_deposits_cents = total_logged_paid_cents + corr_deposits

    today = _date.today()
    current_month_str = today.strftime("%Y-%m")
    current_day = today.day

    if total_deposits_cents >= target_deposit_cents:
        deposit_status = "target_met"
        pending_due_day = None
    else:
        pending_items = [ud for ud in user_deposits if not ud.is_paid and ud.day_of_month > current_day and target_month == current_month_str]
        if pending_items:
            deposit_status = "pending"
            pending_due_day = min(ud.day_of_month for ud in pending_items)
        else:
            deposit_status = "below_target"
            pending_due_day = None

    categories: list[JointAccountCategoryRow] = []
    for cat in all_cats:
        actual_c   = actual_map.get(cat, 0)
        expected_c = expected_map.get(cat, 0)
        pct_used   = round(actual_c / expected_c * 100.0, 1) if expected_c > 0 else 0.0
        categories.append(JointAccountCategoryRow(
            category=cat,
            actual_cents=actual_c,
            expected_cents=expected_c,
            pct_used=pct_used,
        ))

    return JointAccountDashboardResponse(
        month=target_month,
        balance_cents=row["balance_cents"],
        expected_total_cents=expected_total,
        actual_total_cents=actual_total_cents,
        total_deposits_cents=total_deposits_cents,
        safety_margin_pct=safety,
        target_deposit_cents=target_deposit_cents,
        categories=categories,
        user_deposits=user_deposits,
        deposit_status=deposit_status,
        pending_due_day=pending_due_day,
    )


# ── Joint Account Settle ─────────────────────────────────────────────────────

@app.post("/joint-account/settle", tags=["joint-account"])
async def settle_joint_account(
    payload: dict,
    db: DbDep,
) -> dict:
    """
    Settle the joint account balance.
    Payload: {mode: 'direct_pay' | 'adjust_deposits', month: str, correction_note: str}
    - direct_pay:       log a correction for the difference; returns suggested transfers.
    - adjust_deposits:  rebalance deposit amounts to recover deficit/surplus over next month.
    """
    from datetime import date as _date
    mode  = payload.get("mode", "direct_pay")
    month = payload.get("month") or _date.today().strftime("%Y-%m")
    note  = payload.get("correction_note", "")

    row = await _get_joint_account(db)
    if row is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="No joint account configured.")

    # Actual spending this month
    async with db.execute(
        "SELECT COALESCE(ROUND(SUM(cost_cents) / 100.0, 2), 0.0) AS total FROM expenses e INNER JOIN joint_account_categories jac ON jac.category = e.category WHERE strftime('%Y-%m', e.expense_date) = ?",
        (month,),
    ) as cur:
        spend_row = await cur.fetchone()
    actual_cents = round((spend_row["total"] or 0) * 100)

    # Deposits this month
    async with db.execute(
        "SELECT COALESCE(SUM(amount_cents), 0) AS total FROM joint_account_corrections WHERE strftime('%Y-%m', correction_date) = ? AND amount_cents > 0",
        (month,),
    ) as cur:
        dep_row = await cur.fetchone()
    deposit_cents = dep_row["total"] if dep_row else 0

    difference_cents = deposit_cents - actual_cents  # positive = surplus, negative = deficit

    if mode == "direct_pay":
        return {
            "mode": "direct_pay",
            "month": month,
            "difference_cents": difference_cents,
            "message": "Surplus" if difference_cents >= 0 else "Deficit",
        }
    elif mode == "adjust_deposits":
        # Distribute difference proportionally across deposits
        async with db.execute(
            "SELECT user_name, amount_cents, day_of_month FROM joint_account_deposits"
        ) as cur:
            dep_rows = await cur.fetchall()
        deposits = [dict(r) for r in dep_rows]
        total_deposit = sum(d["amount_cents"] for d in deposits) or 1
        adjustments = []
        for d in deposits:
            share = d["amount_cents"] / total_deposit
            adj = round(-difference_cents * share)  # distribute recovery
            new_amt = max(0, d["amount_cents"] + adj)
            await db.execute(
                "UPDATE joint_account_deposits SET amount_cents = ? WHERE user_name = ?",
                (new_amt, d["user_name"]),
            )
            adjustments.append({"user_name": d["user_name"], "old_cents": d["amount_cents"], "new_cents": new_amt})
        await db.commit()
        return {
            "mode": "adjust_deposits",
            "month": month,
            "difference_cents": difference_cents,
            "adjustments": adjustments,
        }
    else:
        raise HTTPException(status_code=400, detail="mode must be direct_pay or adjust_deposits")


# ---------------------------------------------------------------------------
# Raw SQL query console
# ---------------------------------------------------------------------------


class QueryRequest(BaseModel):
    sql: str

class QueryResponse(BaseModel):
    columns:   list[str]
    rows:      list[list]
    row_count: int
    truncated: bool


@app.post("/query", response_model=QueryResponse, tags=["query"])
async def run_query(req: QueryRequest, db: DbDep) -> QueryResponse:
    LIMIT = 50
    try:
        async with db.execute(req.sql) as cur:
            raw_rows = await cur.fetchmany(LIMIT + 1)
            truncated = len(raw_rows) > LIMIT
            raw_rows  = raw_rows[:LIMIT]
            columns: list[str] = [d[0] for d in (cur.description or [])]
            rows = [list(r) for r in raw_rows]
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    return QueryResponse(columns=columns, rows=rows, row_count=len(rows), truncated=truncated)


# ---------------------------------------------------------------------------
# WebSocket
# ---------------------------------------------------------------------------

@app.websocket("/ws/finance")
async def websocket_finance(ws: WebSocket, token: str | None = None) -> None:
    if not getattr(app.state, "testing", False):
        db_path = Path(__file__).resolve().parent.parent / "finance.db"
        try:
            async with aiosqlite.connect(db_path) as conn:
                async with conn.execute("SELECT value FROM app_config WHERE key = 'magic_word'") as cur:
                    row = await cur.fetchone()
                if row is not None:
                    if not (token and touch_session(token)):
                        await ws.close(code=status.WS_1008_POLICY_VIOLATION)
                        return
        except Exception:
            pass

    await manager.connect(ws)
    try:
        while True:
            await ws.receive_text()
    except WebSocketDisconnect:
        manager.disconnect(ws)

