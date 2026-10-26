/**
 * entryInput.js — an entry row (as listed) back to the `EntryInput` the API takes,
 * with optional field overrides. PUT replaces the whole entry, so every edit sends this.
 */
export function entryInput(entry, patch = {}) {
  return {
    kind: entry.kind,
    date: entry.date,
    amount: entry.amount,
    member: entry.member,
    category: entry.category,
    account: entry.account,
    project: entry.project,
    tag: entry.tag,
    name: entry.name,
    note: entry.note,
    overrides: entry.overrides ?? [],
    ...patch,
  };
}
