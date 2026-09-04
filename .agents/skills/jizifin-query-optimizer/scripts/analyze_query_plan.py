#!/usr/bin/env python3
"""
analyze_query_plan.py — Jizifin Query Execution Plan Inspector & Optimizer.

Evaluates SQLite query execution plans (EXPLAIN QUERY PLAN) against the Jizifin
schema, detects performance bottlenecks (table scans, temp b-trees, missing FK indexes),
and outputs actionable indexing and query rewrite recommendations conforming to
AGENTS.md and README.md.

Usage:
    # Run built-in benchmark of all 7 Jizifin views and core queries:
    uv run --directory backend python ../.agents/skills/jizifin-query-optimizer/scripts/analyze_query_plan.py

    # Analyze a specific query:
    uv run --directory backend python ../.agents/skills/jizifin-query-optimizer/scripts/analyze_query_plan.py "SELECT * FROM expenses WHERE expense_date >= '2025-01-01' ORDER BY expense_date DESC"

    # Analyze against the persistent finance.db rather than an in-memory test schema:
    uv run --directory backend python ../.agents/skills/jizifin-query-optimizer/scripts/analyze_query_plan.py --live
"""

from __future__ import annotations

import asyncio
import re
import sys
from pathlib import Path
from typing import NamedTuple

import aiosqlite

# Add backend directory to sys.path so app modules can be resolved
BACKEND_DIR = Path(__file__).resolve().parents[4] / "backend"
if str(BACKEND_DIR) not in sys.path:
    sys.path.insert(0, str(BACKEND_DIR))

try:
    from app.database import DB_PATH, _init_db_schema
except ImportError:
    # Fallback when running from backend directory
    from backend.app.database import DB_PATH, _init_db_schema  # type: ignore


class PlanIssue(NamedTuple):
    severity: str  # "HIGH", "MEDIUM", "INFO"
    detail: str
    remedy: str


CORE_BENCHMARK_QUERIES: list[tuple[str, str]] = [
    ("View: Monthly Total", "SELECT * FROM view_monthly_total"),
    ("View: Monthly by Category", "SELECT * FROM view_monthly_by_category"),
    ("View: Expenses by Month & Category", "SELECT * FROM view_expenses_by_month_category"),
    ("View: Monthly by Payer", "SELECT * FROM view_monthly_by_payer"),
    ("View: Project Summary (JOIN)", "SELECT * FROM view_project_summary"),
    ("View: Tag Totals (JOIN)", "SELECT * FROM view_tag_totals"),
    ("View: Joint Account Monthly", "SELECT * FROM view_joint_account_monthly"),
    (
        "Query: Recent Expenses Timeline",
        "SELECT id, name, cost_cents, expense_date, who_paid, category FROM expenses ORDER BY expense_date DESC, id DESC LIMIT 50",
    ),
    (
        "Query: Member Filter & Date Sort",
        "SELECT * FROM expenses WHERE who_paid = 'sample_encrypted_user' ORDER BY expense_date DESC LIMIT 50",
    ),
    (
        "Query: Category Budget Spending",
        "SELECT * FROM expenses WHERE category = 'sample_encrypted_cat' AND expense_date >= '2025-01-01' AND expense_date <= '2025-01-31'",
    ),
    (
        "Query: Project Settlement Aggregation",
        "SELECT who_paid, is_joint, SUM(cost_cents) FROM expenses WHERE project_id = 1 GROUP BY who_paid, is_joint",
    ),
    (
        "Query: Tag Active Window Expenses",
        "SELECT * FROM expenses WHERE tag_id = 1 ORDER BY expense_date DESC",
    ),
]


def detect_issues(eqp_details: list[str]) -> list[PlanIssue]:
    """Inspects EQP detail lines and identifies architectural hazards in Jizifin."""
    issues: list[PlanIssue] = []
    full_text = " ".join(eqp_details)

    for detail in eqp_details:
        # Full Table Scan
        if "SCAN" in detail and "SCAN TABLE" in detail or ("SCAN " in detail and "USING" not in detail and "view_" not in detail):
            match = re.search(r"SCAN\s+(?:TABLE\s+)?(\w+)", detail)
            tbl = match.group(1) if match else "table"
            issues.append(
                PlanIssue(
                    severity="HIGH",
                    detail=f"Full table scan detected: `{detail}`",
                    remedy=f"Create a supporting B-tree index on filter/join columns of `{tbl}` in `_init_db_schema()`.",
                )
            )

        # Temp B-Tree for ORDER BY
        if "USE TEMP B-TREE FOR ORDER BY" in detail:
            issues.append(
                PlanIssue(
                    severity="MEDIUM",
                    detail=f"Sorting in temporary B-tree: `{detail}`",
                    remedy="Provide an index with matching column sort order (e.g. `(expense_date DESC, id DESC)`).",
                )
            )

        # Temp B-Tree for GROUP BY
        if "USE TEMP B-TREE FOR GROUP BY" in detail:
            issues.append(
                PlanIssue(
                    severity="MEDIUM",
                    detail=f"Temporary B-tree for aggregation grouping: `{detail}`",
                    remedy="Add composite index covering the grouped columns.",
                )
            )

        # Automatic Index Construction
        if "AUTOMATIC COVERING INDEX" in detail or "AUTOMATIC INDEX" in detail:
            issues.append(
                PlanIssue(
                    severity="HIGH",
                    detail=f"Ephemeral index built on-the-fly: `{detail}`",
                    remedy="SQLite is creating an index in memory. Add an explicit persistent index on the foreign key column.",
                )
            )

        # Bloom Filter
        if "BLOOM FILTER" in detail:
            issues.append(
                PlanIssue(
                    severity="MEDIUM",
                    detail=f"Bloom filter pruning: `{detail}`",
                    remedy="Add an explicit index on the joined foreign key column to replace bloom filter pruning with direct B-tree search.",
                )
            )

    return issues


async def run_analysis(sql: str, db: aiosqlite.Connection, title: str = "Custom Query") -> None:
    print(f"\n{'=' * 80}")
    print(f"📊 {title}")
    print(f"{'=' * 80}")
    print(f"SQL:\n  {sql.strip()}\n")

    try:
        async with db.execute(f"EXPLAIN QUERY PLAN {sql}") as cur:
            rows = await cur.fetchall()
    except Exception as exc:
        print(f"❌ Error explaining query: {exc}")
        return

    details = [r["detail"] for r in rows]
    print("Execution Plan Steps:")
    for r in rows:
        row_dict = dict(r)
        step_id = row_dict.get("id", 0)
        parent = row_dict.get("parent", 0)
        indent = "  " * (1 if parent != 0 else 0)
        print(f"  {indent}• [{step_id}->{parent}] {row_dict.get('detail')}")

    issues = detect_issues(details)
    if not issues:
        print("\n✅ Plan Status: OPTIMAL (Using indexed searches with zero temporary B-trees or scans)")
    else:
        print(f"\n⚠️ Plan Status: {len(issues)} Performance Hazard(s) Detected:")
        for idx, issue in enumerate(issues, start=1):
            color_badge = "🔴" if issue.severity == "HIGH" else "🟡" if issue.severity == "MEDIUM" else "ℹ️"
            print(f"  {color_badge} [{issue.severity}] {issue.detail}")
            print(f"     👉 Remedy: {issue.remedy}")


async def main() -> None:
    args = sys.argv[1:]
    use_live = "--live" in args
    filtered_args = [a for a in args if a != "--live"]

    if use_live:
        print(f"🔌 Connecting to live database at: {DB_PATH}")
        db_conn = await aiosqlite.connect(DB_PATH)
    else:
        print("🧪 Initializing clean in-memory Jizifin v4 schema...")
        db_conn = await aiosqlite.connect(":memory:")
        await _init_db_schema(db_conn)

    db_conn.row_factory = aiosqlite.Row

    try:
        if filtered_args:
            custom_sql = " ".join(filtered_args)
            await run_analysis(custom_sql, db_conn, "Custom Query Analysis")
        else:
            print(f"🚀 Running benchmark suite across {len(CORE_BENCHMARK_QUERIES)} core Jizifin queries...")
            for title, query_sql in CORE_BENCHMARK_QUERIES:
                await run_analysis(query_sql, db_conn, title)
    finally:
        await db_conn.close()


if __name__ == "__main__":
    asyncio.run(main())
