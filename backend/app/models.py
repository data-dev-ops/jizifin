"""
models.py — Pydantic v2 request/response schemas and analytics result types.

v4 multi-user changes:
  • New User models (UserCreate, UserUpdate, UserResponse).
  • AllocationEntry helper (user_name + pct) replaces the old Jim/Zina named fields.
  • Split models refactored to carry a list of AllocationEntry instead of
    the old cost_pct_jim / cost_pct_zina fields.
  • Expense models: who_paid is now a plain str validated against the DB;
    per-expense split overrides use list[AllocationEntry] instead of two columns.
  • Income and Recurring models: who/who_paid relaxed from Literal to str.
  • MonthlyCategoryRow: removed user-specific pct fields.
  • PaybackRow / PaybackSummary generalised for N users via dict fields and
    a simplified DebtItem list.
"""

from __future__ import annotations

import re
from typing import Annotated, Optional

from pydantic import BaseModel, Field, field_validator, model_validator


# ---------------------------------------------------------------------------
# Shared helper
# ---------------------------------------------------------------------------

_DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


class AllocationEntry(BaseModel):
    """A single user's percentage share in a split or override."""
    user_name: Annotated[str, Field(min_length=1, max_length=64)]
    pct:       Annotated[float, Field(ge=0.0, le=100.0)]


# ---------------------------------------------------------------------------
# Users
# ---------------------------------------------------------------------------

class UserCreate(BaseModel):
    name:      Annotated[str, Field(min_length=1, max_length=64)]
    color:     str = Field(default="#6366f1", description="CSS hex colour, e.g. #0ea5e9")
    is_active: bool = True


class UserUpdate(BaseModel):
    color:     Optional[str]  = None
    is_active: Optional[bool] = None


class UserResponse(BaseModel):
    name:       str
    color:      str
    is_active:  bool
    created_at: str

    model_config = {"from_attributes": True}


# ---------------------------------------------------------------------------
# Projects
# ---------------------------------------------------------------------------

class ProjectCreate(BaseModel):
    name:         Annotated[str, Field(min_length=1, max_length=96)]
    target_cents: Annotated[int, Field(gt=0, description="Total target in whole cents")]
    target_date:  str = Field(..., description="ISO date YYYY-MM-DD")

    @field_validator("target_date")
    @classmethod
    def validate_date_format(cls, v: str) -> str:
        if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", v):
            raise ValueError("target_date must be in YYYY-MM-DD format")
        return v


class ProjectUpdate(BaseModel):
    name:         Optional[Annotated[str, Field(min_length=1, max_length=96)]] = None
    target_cents: Optional[Annotated[int, Field(gt=0)]] = None
    target_date:  Optional[str] = None

    @field_validator("target_date")
    @classmethod
    def validate_date_format(cls, v: str | None) -> str | None:
        if v is not None and not re.fullmatch(r"\d{4}-\d{2}-\d{2}", v):
            raise ValueError("target_date must be in YYYY-MM-DD format")
        return v


class LastPaymentInfo(BaseModel):
    cost_cents:   int
    expense_date: str
    who_paid:     str
    name:         str


class ProjectResponse(BaseModel):
    id:                        int
    name:                      str
    target_cents:              int
    target_date:               str
    total_spent_cents:         int
    avg_monthly_payment_cents: int
    last_payment:              Optional[LastPaymentInfo] = None
    estimated_completion_date: Optional[str] = None

    model_config = {"from_attributes": True}


# ---------------------------------------------------------------------------
# Splits
# ---------------------------------------------------------------------------

def _validate_allocations(allocations: list[AllocationEntry]) -> list[AllocationEntry]:
    """Shared validator: no empty list, no duplicate names, must sum to 100.0."""
    if not allocations:
        raise ValueError("At least one allocation entry is required.")
    names = [a.user_name for a in allocations]
    if len(names) != len(set(names)):
        raise ValueError("Duplicate user_name entries in allocations.")
    total = round(sum(a.pct for a in allocations), 4)
    if total != 100.0:
        raise ValueError(f"Allocations must sum to 100.0 (got {total})")
    return allocations


class SplitCreate(BaseModel):
    category:    Annotated[str, Field(min_length=1, max_length=32)]
    allocations: list[AllocationEntry]

    @model_validator(mode="after")
    def validate_allocations(self) -> "SplitCreate":
        _validate_allocations(self.allocations)
        return self


class SplitUpdate(BaseModel):
    allocations: list[AllocationEntry]

    @model_validator(mode="after")
    def validate_allocations(self) -> "SplitUpdate":
        _validate_allocations(self.allocations)
        return self


class SplitResponse(BaseModel):
    category:    str
    allocations: list[AllocationEntry]


# ---------------------------------------------------------------------------
# Expenses
# ---------------------------------------------------------------------------

class ExpenseCreate(BaseModel):
    name:         Annotated[str, Field(min_length=1, max_length=96)]
    cost_cents:   Annotated[int, Field(gt=0, description="Whole cents — e.g. €12.50 → 1250")]
    expense_date: str = Field(..., description="ISO date YYYY-MM-DD")
    who_paid:     Annotated[str, Field(min_length=1, max_length=64)]
    category:     Annotated[str, Field(max_length=32)]
    project_id:   Optional[int] = None
    overrides:    Optional[list[AllocationEntry]] = None

    @field_validator("expense_date")
    @classmethod
    def validate_date_format(cls, v: str) -> str:
        if not _DATE_RE.fullmatch(v):
            raise ValueError("expense_date must be in YYYY-MM-DD format")
        return v

    @model_validator(mode="after")
    def overrides_must_sum_to_100(self) -> "ExpenseCreate":
        if self.overrides is not None:
            _validate_allocations(self.overrides)
        return self


class ExpenseUpdate(BaseModel):
    """Partial update — all fields optional."""
    name:         Optional[Annotated[str, Field(min_length=1, max_length=96)]] = None
    cost_cents:   Optional[Annotated[int, Field(gt=0)]] = None
    expense_date: Optional[str] = None
    who_paid:     Optional[Annotated[str, Field(min_length=1, max_length=64)]] = None
    category:     Optional[Annotated[str, Field(max_length=32)]] = None
    project_id:   Optional[int] = None
    overrides:    Optional[list[AllocationEntry]] = None  # None = keep existing; [] = clear all

    @field_validator("expense_date")
    @classmethod
    def validate_date_format(cls, v: str | None) -> str | None:
        if v is not None and not _DATE_RE.fullmatch(v):
            raise ValueError("expense_date must be in YYYY-MM-DD format")
        return v

    @model_validator(mode="after")
    def overrides_must_sum_to_100(self) -> "ExpenseUpdate":
        if self.overrides is not None and len(self.overrides) > 0:
            _validate_allocations(self.overrides)
        return self


class ExpenseResponse(BaseModel):
    id:           int
    name:         str
    cost_cents:   int
    expense_date: str
    who_paid:     str
    category:     str
    project_id:   Optional[int] = None
    overrides:    list[AllocationEntry] = []

    model_config = {"from_attributes": True}


# ---------------------------------------------------------------------------
# Analytics view result types
# ---------------------------------------------------------------------------

class MonthlyTotal(BaseModel):
    """Maps 1-to-1 onto view_monthly_total."""
    total_amount:  float
    expense_count: int
    month:         str


class MonthlyCategoryRow(BaseModel):
    """One row from view_monthly_by_category (user-specific pct removed in v4)."""
    category:      str
    total_amount:  float
    expense_count: int


class MonthlyPayerRow(BaseModel):
    """One row from view_monthly_by_payer."""
    who_paid:      str
    total_amount:  float
    expense_count: int


# ---------------------------------------------------------------------------
# Income
# ---------------------------------------------------------------------------

class IncomeCreate(BaseModel):
    name:         Annotated[str, Field(min_length=1, max_length=96)]
    amount_cents: Annotated[int, Field(gt=0, description="Whole cents")]
    who:          Annotated[str, Field(min_length=1, max_length=64)]
    category:     Annotated[str, Field(min_length=1, max_length=32)]
    income_date:  str = Field(..., description="ISO date YYYY-MM-DD")

    @field_validator("income_date")
    @classmethod
    def validate_date_format(cls, v: str) -> str:
        if not _DATE_RE.fullmatch(v):
            raise ValueError("income_date must be in YYYY-MM-DD format")
        return v


class IncomeResponse(BaseModel):
    id:           int
    name:         str
    amount_cents: int
    who:          str
    category:     str
    income_date:  str

    model_config = {"from_attributes": True}


class LatestSalaryRow(BaseModel):
    """The most recent SALARY income row for one person."""
    who:          str
    amount_cents: int
    income_date:  str
    name:         str


class IncomeByPersonRow(BaseModel):
    """Income total per person for an analytics query (with carry-forward)."""
    who:        str
    total_cents: int
    is_carried:  bool = False


# ---------------------------------------------------------------------------
# Paybacks (N-user generalisation)
# ---------------------------------------------------------------------------

class DebtItem(BaseModel):
    """A single simplified debt transfer: from_user owes to_user this amount."""
    from_user: str
    to_user:   str
    amount:    float  # euros (already converted from cents)


class PaybackRow(BaseModel):
    """Per-category payback breakdown — user fields are dicts keyed by user name."""
    category:           str
    total_amount:       float
    per_user_paid:      dict[str, float]  # {user_name: euros_actually_paid}
    per_user_share_pct: dict[str, float]  # {user_name: effective_expected_%}
    net_per_user:       dict[str, float]  # {user_name: euros_net (+ = overpaid)}


class PaybackSummary(BaseModel):
    rows:  list[PaybackRow]
    debts: list[DebtItem]  # greedy-simplified debt transfers
    month: str


# ---------------------------------------------------------------------------
# Recurring Expenses
# ---------------------------------------------------------------------------

class RecurringCreate(BaseModel):
    name:         Annotated[str, Field(min_length=1, max_length=96)]
    cost_cents:   Annotated[int, Field(gt=0, description="Whole cents")]
    who_paid:     Annotated[str, Field(min_length=1, max_length=64)]
    category:     Annotated[str, Field(max_length=32)]
    day_of_month: Annotated[int, Field(ge=1, le=31)]


class RecurringResponse(BaseModel):
    id:           int
    name:         str
    cost_cents:   int
    who_paid:     str
    category:     str
    day_of_month: int

    model_config = {"from_attributes": True}


# ---------------------------------------------------------------------------
# Budgets
# ---------------------------------------------------------------------------

class BudgetCreate(BaseModel):
    category:    Annotated[str, Field(max_length=32)]
    month:       str = Field(..., description="YYYY-MM or 'ALL'")
    limit_cents: Annotated[int, Field(ge=0, description="Monthly limit in whole cents")]

    @field_validator("month")
    @classmethod
    def validate_month(cls, v: str) -> str:
        import re as _re
        if v != "ALL" and not _re.fullmatch(r"\d{4}-\d{2}", v):
            raise ValueError("month must be YYYY-MM or 'ALL'")
        return v


class BudgetResponse(BaseModel):
    category:    str
    month:       str
    limit_cents: int

    model_config = {"from_attributes": True}


class BudgetStatusRow(BaseModel):
    """Actuals vs. limits for a single category in a given month."""
    category:     str
    limit_cents:  int
    actual_cents: int
    pct_used:     float


# ---------------------------------------------------------------------------
# Settlements
# ---------------------------------------------------------------------------

class SettlementCreate(BaseModel):
    month:                         str = Field(..., description="YYYY-MM")
    net_balance_transferred_cents: int

    @field_validator("month")
    @classmethod
    def validate_month(cls, v: str) -> str:
        import re as _re
        if not _re.fullmatch(r"\d{4}-\d{2}", v):
            raise ValueError("month must be YYYY-MM")
        return v


class SettlementResponse(BaseModel):
    month:                         str
    settled_at:                    str
    net_balance_transferred_cents: int

    model_config = {"from_attributes": True}
