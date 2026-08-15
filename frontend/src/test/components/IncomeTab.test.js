import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, fireEvent, screen } from "@testing-library/svelte";
import IncomeTab from "../../lib/IncomeTab.svelte";
import { selectedMonth, users, incomeEntries, incomeCategories, jobs } from "../../lib/stores.js";
import * as api from "../../lib/api.js";

describe("IncomeTab.svelte — Income Ledger & Category Management", () => {
  beforeEach(() => {
    selectedMonth.set("2026-07");
    users.set([{ name: "John", color: "#6366f1", is_active: true }]);
    incomeCategories.set([{ category: "SALARY" }, { category: "FREELANCE" }]);
    incomeEntries.set([
      { id: 1, name: "July Salary", amount_cents: 350000, who: "John", category: "SALARY", income_date: "2026-07-01" },
    ]);
    jobs.set([
      {
        id: 10,
        name: "Senior Software Engineer",
        who: "John",
        amount_cents: 450000,
        frequency: "monthly",
        start_date: "2026-01-01",
        end_date: null,
        notes: "Full-time contract",
        is_active: true,
      },
    ]);

    vi.restoreAllMocks();
  });

  it.each([
    { entryName: "July Salary", amountFormatted: "€3500.00", catName: "SALARY" },
  ])("renders income entries and categories correctly ($entryName)", async ({ entryName, amountFormatted, catName }) => {
    render(IncomeTab);

    expect(screen.getByText(entryName)).toBeInTheDocument();
    expect(screen.getAllByText(amountFormatted).length).toBeGreaterThan(0);
    expect(screen.getAllByText(catName).length).toBeGreaterThan(0);
  });

  it.each([
    { expectedErr: "Name is required." },
  ])("validates form input before logging income", async ({ expectedErr }) => {
    render(IncomeTab);

    const submitBtn = document.getElementById("income-submit");
    await fireEvent.click(submitBtn);

    expect(screen.getByText(expectedErr)).toBeInTheDocument();
  });

  it.each([
    { nameVal: "Consulting Fee", amountVal: "500.00" },
  ])("logs income successfully when form is valid ($nameVal)", async ({ nameVal, amountVal }) => {
    const createSpy = vi.spyOn(api, "createIncome").mockResolvedValue({});
    vi.spyOn(api, "fetchIncome").mockResolvedValue([]);

    render(IncomeTab);

    const nameInput = document.getElementById("income-name");
    await fireEvent.input(nameInput, { target: { value: nameVal } });

    const amountInput = document.getElementById("income-amount");
    await fireEvent.input(amountInput, { target: { value: amountVal } });

    const submitBtn = document.getElementById("income-submit");
    await fireEvent.click(submitBtn);

    expect(createSpy).toHaveBeenCalled();
  });

  it.each([
    { linkId: "link-manage-categories" },
  ])("triggers navigateCategories event when link card is clicked ($linkId)", async ({ linkId }) => {
    const { component } = render(IncomeTab);
    let navTriggered = false;
    component.$on("navigateCategories", () => { navTriggered = true; });

    const linkBtn = document.getElementById(linkId);
    expect(linkBtn).toBeInTheDocument();
    await fireEvent.click(linkBtn);

    expect(navTriggered).toBe(true);
  });

  it.each([
    { deleteId: "delete-income-1", targetId: 1 },
  ])("deletes an income entry on double click confirm ($deleteId)", async ({ deleteId, targetId }) => {
    const delSpy = vi.spyOn(api, "deleteIncome").mockResolvedValue({});
    vi.spyOn(api, "fetchIncome").mockResolvedValue([]);

    render(IncomeTab);

    const delBtn = document.getElementById(deleteId);
    await fireEvent.click(delBtn); // first click -> pending confirmation
    await fireEvent.click(delBtn); // second click -> confirm delete

    expect(delSpy).toHaveBeenCalledWith(targetId);
  });

  it("renders jobs list and allows opening add job modal", async () => {
    render(IncomeTab);

    expect(screen.getByText("Senior Software Engineer")).toBeInTheDocument();
    expect(screen.getByText("Full-time contract")).toBeInTheDocument();

    const addBtn = document.getElementById("btn-add-job-top");
    expect(addBtn).toBeInTheDocument();
    await fireEvent.click(addBtn);

    expect(screen.getByText("Add Employment Stream")).toBeInTheDocument();
  });
});
