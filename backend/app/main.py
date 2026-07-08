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
from contextlib import asynccontextmanager
from datetime import date as _date, datetime as _datetime
from typing import Annotated

import aiosqlite
from apscheduler.schedulers.asyncio import AsyncIOScheduler
from fastapi import Depends, FastAPI, HTTPException, WebSocket, WebSocketDisconnect, status
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

from .database import get_db, init_db
from .models import (
    AllocationEntry,
    BudgetCreate,
    BudgetResponse,
    BudgetStatusRow,
    DebtItem,
    ExpenseCreate,
    ExpenseResponse,
    ExpenseUpdate,
    IncomeByPersonRow,
    IncomeCreate,
    IncomeResponse,
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
    UserCreate,
    UserResponse,
    UserUpdate,
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
# FastAPI app + CORS
# ---------------------------------------------------------------------------

app = FastAPI(
    title="Finance Tracker API",
    version="0.2.0",
    description="Household finance tracking — supports any number of users",
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origin_regex=r"https?://.*",
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

DbDep = Annotated[aiosqlite.Connection, Depends(get_db)]


# ---------------------------------------------------------------------------
# Health check
# ---------------------------------------------------------------------------

@app.get("/", tags=["health"])
async def root() -> dict:
    return {"status": "ok", "service": "finance-tracker"}


# ---------------------------------------------------------------------------
# Auth / First Boot
# ---------------------------------------------------------------------------

class AuthSaltRequest(BaseModel):
    value: str

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
    """Set the encrypted magic word on first boot."""
    async with db.execute("SELECT value FROM app_config WHERE key = 'magic_word'") as cur:
        if await cur.fetchone() is not None:
            raise HTTPException(status_code=409, detail="Already initialized")
    await db.execute("INSERT INTO app_config (key, value) VALUES ('magic_word', ?)", (payload.value,))
    await db.commit()
    return {"status": "ok"}


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
        SELECT id, name, cost_cents, expense_date, who_paid, category, project_id
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

    async with db.execute(
        "INSERT INTO expenses (name, cost_cents, expense_date, who_paid, category, project_id) VALUES (?, ?, ?, ?, ?, ?)",
        (expense.name, expense.cost_cents, expense.expense_date, expense.who_paid, expense.category, expense.project_id),
    ) as cur:
        new_id = cur.lastrowid

    if expense.overrides:
        for alloc in expense.overrides:
            await db.execute(
                "INSERT INTO expense_overrides (expense_id, user_name, pct) VALUES (?, ?, ?)",
                (new_id, alloc.user_name, alloc.pct),
            )

    await db.commit()

    async with db.execute(
        "SELECT id, name, cost_cents, expense_date, who_paid, category, project_id FROM expenses WHERE id = ?",
        (new_id,),
    ) as cur:
        row = await cur.fetchone()

    result = (await _build_expense_responses(db, [row]))[0]
    await manager.broadcast({"event": "expense_created", "payload": {
        "id": new_id, "name": expense.name, "cost_cents": expense.cost_cents,
        "expense_date": expense.expense_date, "who_paid": expense.who_paid,
        "category": expense.category,
    }})
    return result


@app.put("/expenses/{expense_id}", response_model=ExpenseResponse, tags=["expenses"])
async def update_expense(expense_id: int, update: ExpenseUpdate, db: DbDep) -> ExpenseResponse:
    async with db.execute(
        "SELECT id, name, cost_cents, expense_date, who_paid, category, project_id FROM expenses WHERE id = ?",
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

    await db.execute(
        "UPDATE expenses SET name=?, cost_cents=?, expense_date=?, who_paid=?, category=?, project_id=? WHERE id=?",
        (new_name, new_cost_cents, new_expense_date, new_who_paid, new_category, new_project_id, expense_id),
    )

    # Overrides: None = keep as-is; [] = clear all; [items] = replace
    if update.overrides is not None:
        await db.execute("DELETE FROM expense_overrides WHERE expense_id = ?", (expense_id,))
        for alloc in update.overrides:
            await db.execute(
                "INSERT INTO expense_overrides (expense_id, user_name, pct) VALUES (?, ?, ?)",
                (expense_id, alloc.user_name, alloc.pct),
            )

    await db.commit()

    async with db.execute(
        "SELECT id, name, cost_cents, expense_date, who_paid, category, project_id FROM expenses WHERE id = ?",
        (expense_id,),
    ) as cur:
        row = await cur.fetchone()
    return (await _build_expense_responses(db, [row]))[0]


@app.delete("/expenses/{expense_id}", status_code=status.HTTP_204_NO_CONTENT, tags=["expenses"])
async def delete_expense(expense_id: int, db: DbDep) -> None:
    async with db.execute("SELECT id, expense_date FROM expenses WHERE id = ?", (expense_id,)) as cur:
        row = await cur.fetchone()
    if row is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Expense {expense_id} not found.")
    await _check_month_not_locked(db, row["expense_date"])
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
        "SELECT COALESCE(SUM(cost_cents), 0) AS total FROM expenses WHERE project_id = ?",
        (project_id,),
    ) as cur:
        total_row = await cur.fetchone()
    total_spent = total_row["total"] if total_row else 0

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
        estimated_completion_date=est_date,
    )


@app.get("/projects", response_model=list[ProjectResponse], tags=["projects"])
async def list_projects(db: DbDep) -> list[ProjectResponse]:
    async with db.execute("SELECT id, name, target_cents, target_date FROM projects ORDER BY id") as cur:
        rows = await cur.fetchall()
    return [await _build_project_response(db, row) for row in rows]


@app.post("/projects", response_model=ProjectResponse, status_code=status.HTTP_201_CREATED, tags=["projects"])
async def create_project(project: ProjectCreate, db: DbDep) -> ProjectResponse:
    try:
        async with db.execute(
            "INSERT INTO projects (name, target_cents, target_date) VALUES (?, ?, ?)",
            (project.name, project.target_cents, project.target_date),
        ) as cur:
            new_id = cur.lastrowid
        await db.commit()
    except Exception as exc:
        raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail=f"Project '{project.name}' already exists.") from exc
    async with db.execute("SELECT id, name, target_cents, target_date FROM projects WHERE id = ?", (new_id,)) as cur:
        row = await cur.fetchone()
    return await _build_project_response(db, row)


@app.put("/projects/{project_id}", response_model=ProjectResponse, tags=["projects"])
async def update_project(project_id: int, update: ProjectUpdate, db: DbDep) -> ProjectResponse:
    async with db.execute("SELECT id, name, target_cents, target_date FROM projects WHERE id = ?", (project_id,)) as cur:
        existing = await cur.fetchone()
    if existing is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail=f"Project {project_id} not found.")
    new_name   = update.name         if update.name         is not None else existing["name"]
    new_target = update.target_cents if update.target_cents is not None else existing["target_cents"]
    new_date   = update.target_date  if update.target_date  is not None else existing["target_date"]
    await db.execute("UPDATE projects SET name=?, target_cents=?, target_date=? WHERE id=?", (new_name, new_target, new_date, project_id))
    await db.commit()
    async with db.execute("SELECT id, name, target_cents, target_date FROM projects WHERE id = ?", (project_id,)) as cur:
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
            SELECT category, ROUND(SUM(cost_cents) / 100.0, 2) AS total_amount, COUNT(*) AS expense_count
            FROM expenses WHERE strftime('%Y-%m', expense_date) = ? GROUP BY category
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
async def get_paybacks(db: DbDep, month: str | None = None) -> PaybackSummary:
    """
    Compute per-category and overall payback balances for the given month.

    Algorithm:
      1. For each expense, determine the effective split (expense override >
         split_allocations default > equal share across all payers seen).
      2. Accumulate per-user net balance in cents (positive = overpaid = owed money).
      3. Greedy debt simplification: match creditors against debtors to produce
         a minimal list of DebtItem transfers.

    Personal-pay categories (PERSONAL COST, LEISURE, GIFT) are handled by
    renaming the category to "<CAT> <payer>" and assigning the payer a 100%
    effective share, so no cross-user debt is generated for those expenses.
    """
    from datetime import date as _date
    target_month = month or _date.today().strftime("%Y-%m")

    # 1. Load split allocations: {category: {user_name: pct}}
    async with db.execute("SELECT category, user_name, pct FROM split_allocations") as cur:
        alloc_rows = await cur.fetchall()
    splits_dict: dict[str, dict[str, float]] = {}
    for r in alloc_rows:
        splits_dict.setdefault(r["category"], {})[r["user_name"]] = r["pct"]

    # 2. Load expenses for target month
    async with db.execute(
        "SELECT id, name, cost_cents, who_paid, category FROM expenses WHERE strftime('%Y-%m', expense_date) = ?",
        (target_month,),
    ) as cur:
        expense_rows = await cur.fetchall()
    expenses = [dict(r) for r in expense_rows]

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
    PERSONAL_PAY: set[str] = {"PERSONAL COST", "LEISURE", "GIFT"}
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
    query = f"SELECT id, name, amount_cents, who, category, income_date FROM income {where} ORDER BY income_date DESC, id DESC LIMIT ?"
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
            "INSERT INTO income (name, amount_cents, who, category, income_date) VALUES (?, ?, ?, ?, ?)",
            (entry.name, entry.amount_cents, entry.who, entry.category, entry.income_date),
        ) as cur:
            new_id = cur.lastrowid
        async with db.execute("SELECT id, name, amount_cents, who, category, income_date FROM income WHERE id = ?", (new_id,)) as cur:
            row = await cur.fetchone()
        created.append(IncomeResponse(**dict(row)))
    await db.commit()
    return created


@app.get("/income/latest-salary", response_model=list[LatestSalaryRow], tags=["income"])
async def get_latest_salary(db: DbDep) -> list[LatestSalaryRow]:
    async with db.execute(
        """
        SELECT who, amount_cents, income_date, name
        FROM   income
        WHERE  category = 'SALARY'
          AND  (who, income_date) IN (
                   SELECT who, MAX(income_date)
                   FROM   income WHERE category = 'SALARY'
                   GROUP  BY who
               )
        """
    ) as cur:
        rows = await cur.fetchall()
    return [LatestSalaryRow(**dict(r)) for r in rows]


@app.get("/analytics/income-by-person", response_model=list[IncomeByPersonRow], tags=["analytics"])
async def get_income_by_person(db: DbDep, month: str | None = None) -> list[IncomeByPersonRow]:
    """
    Return total income per active user for the requested month.
    Salary entries are append-only; only the latest entry per user per month counts.
    If a user has no income in the target month, their most recent historical
    salary is carried forward (is_carried=True).
    """
    from datetime import date as _date
    target = month or _date.today().strftime("%Y-%m")

    # Iterate over every active user (not just Jim/Zina)
    async with db.execute("SELECT name FROM users WHERE is_active = 1 ORDER BY name") as cur:
        active_users = [r["name"] async for r in cur]

    result: list[IncomeByPersonRow] = []
    for person in active_users:
        async with db.execute(
            "SELECT COALESCE(SUM(amount_cents), 0) AS total_cents FROM income WHERE who=? AND strftime('%Y-%m', income_date)=? AND category!='SALARY'",
            (person, target),
        ) as cur:
            other_row = await cur.fetchone()
        other_cents: int = other_row["total_cents"] if other_row else 0

        async with db.execute(
            "SELECT amount_cents FROM income WHERE who=? AND category='SALARY' AND strftime('%Y-%m', income_date)=? ORDER BY income_date DESC, id DESC LIMIT 1",
            (person, target),
        ) as cur:
            sal_month_row = await cur.fetchone()
        salary_this_month: int = sal_month_row["amount_cents"] if sal_month_row else 0

        is_carried = False
        if salary_this_month == 0:
            async with db.execute(
                "SELECT amount_cents FROM income WHERE who=? AND category='SALARY' AND income_date <= ? || '-31' ORDER BY income_date DESC, id DESC LIMIT 1",
                (person, target),
            ) as cur:
                carry_row = await cur.fetchone()
            if carry_row:
                salary_this_month = carry_row["amount_cents"]
                is_carried = True

        total = other_cents + salary_this_month
        if total > 0:
            result.append(IncomeByPersonRow(who=person, total_cents=total, is_carried=is_carried))

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
    async with db.execute("SELECT id, name, cost_cents, who_paid, category, day_of_month FROM recurring_expenses ORDER BY id") as cur:
        rows = await cur.fetchall()
    return [RecurringResponse(**dict(r)) for r in rows]


@app.post("/recurring", response_model=RecurringResponse, status_code=status.HTTP_201_CREATED, tags=["recurring"])
async def create_recurring(rec: RecurringCreate, db: DbDep) -> RecurringResponse:
    await _assert_active_user(db, rec.who_paid)
    async with db.execute(
        "INSERT INTO recurring_expenses (name, cost_cents, who_paid, category, day_of_month) VALUES (?, ?, ?, ?, ?)",
        (rec.name, rec.cost_cents, rec.who_paid, rec.category, rec.day_of_month),
    ) as cur:
        new_id = cur.lastrowid
    await db.commit()
    async with db.execute("SELECT id, name, cost_cents, who_paid, category, day_of_month FROM recurring_expenses WHERE id=?", (new_id,)) as cur:
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
    settled_at = _datetime.utcnow().isoformat()
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
async def websocket_finance(ws: WebSocket) -> None:
    await manager.connect(ws)
    try:
        while True:
            await ws.receive_text()
    except WebSocketDisconnect:
        manager.disconnect(ws)
