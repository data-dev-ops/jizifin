/**
 * csvParser.js
 *
 * Client-side Bank CSV statement parser for Jizifin.
 * Supports:
 * - ING Bank (Belgium) format
 * - KBC Bank (Belgium) placeholder / future hook
 * - Tokenization handling quotes, multiline fields, commas, UTF-8 BOM
 * - Clean Beneficiary/Merchant extraction
 * - Category cascading across identical beneficiaries
 * - Consolidation of "Group to Other (semi-hidden)" expenses
 */

/**
 * Robust CSV tokenizer that parses standard RFC 4180 CSV with quotes, commas, and newlines.
 *
 * @param {string} text Raw CSV text
 * @param {string} delimiter Default ','
 * @returns {string[][]} 2D array of rows and column values
 */
export function parseCsvRows(text, delimiter = ',') {
  if (!text) return [];

  // Strip UTF-8 BOM if present
  let cleanText = text.charCodeAt(0) === 0xFEFF ? text.slice(1) : text;
  // Also strip any Windows-1252/ISO-8859-1 artifact of UTF-8 BOM: ï»¿
  if (cleanText.startsWith('\xEF\xBB\xBF') || cleanText.startsWith('ï»¿')) {
    cleanText = cleanText.slice(3);
  }

  const rows = [];
  let currentRow = [];
  let currentField = '';
  let inQuotes = false;
  let i = 0;
  const len = cleanText.length;

  while (i < len) {
    const char = cleanText[i];
    const nextChar = i + 1 < len ? cleanText[i + 1] : '';

    if (inQuotes) {
      if (char === '"') {
        if (nextChar === '"') {
          // Escaped quote
          currentField += '"';
          i += 2;
          continue;
        } else {
          // End of quotes
          inQuotes = false;
          i++;
          continue;
        }
      } else {
        currentField += char;
        i++;
        continue;
      }
    } else {
      if (char === '"') {
        inQuotes = true;
        i++;
        continue;
      } else if (char === delimiter) {
        currentRow.push(currentField);
        currentField = '';
        i++;
        continue;
      } else if (char === '\r') {
        if (nextChar === '\n') {
          i++;
        }
        currentRow.push(currentField);
        currentField = '';
        if (currentRow.length > 1 || (currentRow.length === 1 && currentRow[0] !== '')) {
          rows.push(currentRow);
        }
        currentRow = [];
        i++;
        continue;
      } else if (char === '\n') {
        currentRow.push(currentField);
        currentField = '';
        if (currentRow.length > 1 || (currentRow.length === 1 && currentRow[0] !== '')) {
          rows.push(currentRow);
        }
        currentRow = [];
        i++;
        continue;
      } else {
        currentField += char;
        i++;
        continue;
      }
    }
  }

  // Trailing field
  if (currentField !== '' || currentRow.length > 0) {
    currentRow.push(currentField);
    if (currentRow.length > 1 || (currentRow.length === 1 && currentRow[0] !== '')) {
      rows.push(currentRow);
    }
  }

  return rows;
}

/**
 * Extracts a clean beneficiary name from an ING Bank transaction description.
 *
 * @param {string} desc Raw transaction description
 * @returns {string} Clean receiving party / merchant name
 */
export function extractIngBeneficiary(desc) {
  if (!desc || typeof desc !== 'string') return 'Unknown';
  const cleanDesc = desc.trim();

  // 1. SEPA Direct Debit: Domiciliëring
  const domMatch = cleanDesc.match(/Domicili.*?ring in euro \(SEPA\)\s+(.*?)(?:\s+Bericht als bijlage|$|\s+Identificatienr:)/i);
  if (domMatch && domMatch[1].trim()) {
    return domMatch[1].trim();
  }

  // 2. SEPA Transfer: Naar: <Beneficiary>
  if (cleanDesc.includes('Naar:')) {
    const naarMatch = cleanDesc.match(/Naar:\s*(.*?)(?:\s*-\s*[A-Z0-9]{10,34}|$|\s*Mededeling:)/i);
    if (naarMatch && naarMatch[1].trim()) {
      return naarMatch[1].trim();
    }
  }

  // 3. SEPA Transfer Incoming: Van: <Sender>
  if (cleanDesc.includes('Van:')) {
    const vanMatch = cleanDesc.match(/Van:\s*(.*?)(?:\s*-\s*[A-Z0-9]{10,34}|$|\s*Mededeling:)/i);
    if (vanMatch && vanMatch[1].trim()) {
      return vanMatch[1].trim();
    }
  }

  // 4. Card Payments & Cash Withdrawals
  if (/^(?:Betaling|Geldopneming|Terugbetaling)/i.test(cleanDesc)) {
    if (cleanDesc.includes('ING : VISA')) {
      return 'ING : VISA';
    }
    if (cleanDesc.includes('Financiering meubilering')) {
      return 'ING Financiering';
    }

    let merchant = cleanDesc;

    // Remove timestamp prefix: e.g. "30/07/26 - 18.26 uur - "
    const timeMatch = merchant.match(/\d{1,2}[.:]\d{2}\s*uur\s*-\s*(.*)/i);
    if (timeMatch) {
      merchant = timeMatch[1];
    } else {
      // If no time stamp, strip prefix: "Betaling Bancontact 15/08/26 - "
      merchant = merchant.replace(/^(?:Betaling|Geldopneming|Terugbetaling)[^-]*-?\s*/i, '');
    }

    // Strip card and reference details
    merchant = merchant.replace(/\s+Kaartnummer.*/i, '');
    merchant = merchant.replace(/\s+Referentie.*/i, '');
    merchant = merchant.replace(/\s+\d+[,.]\d+\s+liter.*/i, '');

    // Strip postal code, city, and country code suffix:
    // e.g. " 3200 - AARSCHOT - BEL" or " 0252 88000 - EPINAL - FRA" or " 1050-012 - Lisboa - PRT"
    merchant = merchant.replace(/\s+[0-9]{2,5}(?:\s+[0-9]{2,5})?\s*-\s*[^-]+-[A-Z\s]+$/i, '');
    merchant = merchant.replace(/\s+[A-Z0-9]{2,8}(?:\s+[A-Z0-9]{2,5})?\s*-\s*[^-]+-[A-Z\s]+$/i, '');
    merchant = merchant.replace(/\s+[0-9]{4}\s+[A-Z]{2}\s*-\s*[^-]+-[A-Z\s]+.*$/i, '');
    merchant = merchant.replace(/\s+[0-9]{4}\s*-\s*[^-]+-[A-Z\s]+$/i, '');

    // Strip foreign currency conversion info if trailing: " 40,00 CHF aan 0,9304"
    merchant = merchant.replace(/\s+\d+[,.]\d+\s+[A-Z]{3}\s+aan\s+.*$/i, '');

    return merchant.trim() || cleanDesc;
  }

  // 5. Bank fee statements
  if (/^Kostenafrekening/i.test(cleanDesc)) {
    return 'ING Bankkosten';
  }

  return cleanDesc;
}

/**
 * Converts European date string DD/MM/YYYY to ISO date YYYY-MM-DD.
 *
 * @param {string} dateStr
 * @returns {string} ISO date YYYY-MM-DD
 */
export function formatToIsoDate(dateStr) {
  if (!dateStr) return '';
  const parts = dateStr.trim().split('/');
  if (parts.length === 3) {
    const day = parts[0].padStart(2, '0');
    const month = parts[1].padStart(2, '0');
    const year = parts[2].length === 2 ? `20${parts[2]}` : parts[2];
    return `${year}-${month}-${day}`;
  }
  return dateStr;
}

/**
 * Parses an amount string like "-3,90", "2.105,68", "41,69" into float and integer cents.
 *
 * @param {string} amountStr
 * @returns {{ amount: number, amountCents: number, isCredit: boolean }}
 */
export function parseEuroAmount(amountStr) {
  if (!amountStr) return { amount: 0, amountCents: 0, isCredit: false };

  let clean = amountStr.toString().trim().replace(/['"€\s]/g, '');
  // Format: in Belgian exports, "." is thousands separator and "," is decimal separator
  clean = clean.replace(/\./g, '').replace(',', '.');
  const num = parseFloat(clean);
  if (isNaN(num)) {
    return { amount: 0, amountCents: 0, isCredit: false };
  }

  const isCredit = num > 0;
  const absNum = Math.abs(num);
  const amountCents = Math.round(absNum * 100);

  return {
    amount: num,
    amountCents,
    isCredit
  };
}

/**
 * Parses ING Bank statement CSV text.
 *
 * @param {string} csvText
 * @param {string} whoPaid Selected household payer
 * @returns {{ transactions: Array, stats: Object }}
 */
export function parseIngCsv(csvText, whoPaid = '') {
  const rows = parseCsvRows(csvText);
  if (rows.length < 2) {
    throw new Error('CSV file is empty or missing data rows.');
  }

  const header = rows[0].map((h) => h.trim().toLowerCase());
  const descIdx = header.findIndex((h) => h.includes('description') || h.includes('omschrijving'));
  const amountIdx = header.findIndex((h) => h.includes('amount') || h.includes('bedrag'));
  const bookingDateIdx = header.findIndex((h) => h.includes('booking date') || h.includes('boekingsdatum'));
  const valueDateIdx = header.findIndex((h) => h.includes('value date') || h.includes('valutadatum'));

  if (amountIdx === -1) {
    throw new Error('Could not find Amount column in CSV.');
  }

  const dateIdx = bookingDateIdx !== -1 ? bookingDateIdx : (valueDateIdx !== -1 ? valueDateIdx : 4);
  const effectiveDescIdx = descIdx !== -1 ? descIdx : 8;

  const transactions = [];
  let totalExpenseCents = 0;
  let totalCreditCents = 0;
  let expensesCount = 0;
  let creditsCount = 0;

  for (let rowIndex = 1; rowIndex < rows.length; rowIndex++) {
    const row = rows[rowIndex];
    if (!row || row.length <= amountIdx) continue;

    const rawDate = row[dateIdx] || '';
    const rawAmount = row[amountIdx] || '';
    const rawDesc = row[effectiveDescIdx] || '';

    const { amount, amountCents, isCredit } = parseEuroAmount(rawAmount);
    if (amountCents === 0 && !rawAmount) continue;

    const isoDate = formatToIsoDate(rawDate);
    const beneficiary = extractIngBeneficiary(rawDesc);

    if (isCredit) {
      creditsCount++;
      totalCreditCents += amountCents;
    } else {
      expensesCount++;
      totalExpenseCents += amountCents;
    }

    transactions.push({
      id: `ing-${rowIndex}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      rowIndex,
      rawDescription: rawDesc,
      date: isoDate,
      displayDate: rawDate,
      amount,
      amountCents,
      isCredit,
      beneficiary,
      whoPaid,
      category: '', // Every transaction requires an explicit category; no defaulting
      tagId: null,
      // By default: include all valid transactions
      include: true,
      groupToOther: false
    });
  }

  return {
    transactions,
    stats: {
      total: transactions.length,
      expensesCount,
      creditsCount,
      totalExpenseCents,
      totalCreditCents
    }
  };
}

/**
 * Main bank statement CSV parser dispatcher.
 *
 * @param {string} csvText
 * @param {'ING' | 'KBC'} bank
 * @param {string} whoPaid
 */
export function parseBankCsv(csvText, bank = 'ING', whoPaid = '') {
  if (bank === 'ING') {
    return parseIngCsv(csvText, whoPaid);
  }
  if (bank === 'KBC') {
    throw new Error('KBC Bank export format is not yet supported. The parser will be added in a future update.');
  }
  throw new Error(`Unsupported bank selection: ${bank}`);
}

/**
 * Cascades a category selection to all transactions sharing the same beneficiary name and type.
 *
 * @param {Array} transactions
 * @param {string} targetBeneficiary
 * @param {string} newCategory
 * @param {boolean} [isCredit] Optional filter to only cascade within same type (income or expense)
 * @returns {{ updatedTransactions: Array, count: number }}
 */
export function cascadeCategory(transactions, targetBeneficiary, newCategory, isCredit = undefined) {
  if (!targetBeneficiary) return { updatedTransactions: transactions, count: 0 };
  let count = 0;
  const updatedTransactions = transactions.map((tx) => {
    const matchBeneficiary = tx.beneficiary === targetBeneficiary;
    const matchType = isCredit === undefined || tx.isCredit === isCredit;
    if (matchBeneficiary && matchType) {
      count++;
      return { ...tx, category: newCategory };
    }
    return tx;
  });
  return { updatedTransactions, count };
}

/**
 * Separates and prepares transactions for storage:
 * - Negative amounts (debits) → expenses (with groupToOther consolidated)
 * - Positive amounts (credits) → incomes
 *
 * Enforces that every to-import transaction has an explicit category assigned.
 * Defaulting to 'other' or a silent fallback is strictly prohibited.
 *
 * @param {Array} transactions All parsed transactions
 * @param {string} fallbackPayer Default payer if not set on transaction
 * @returns {{ expenses: Array<Object>, incomes: Array<Object> }}
 */
export function prepareTransactionsForImport(
  transactions,
  fallbackPayer
) {
  const activeItems = transactions.filter((t) => t.include && t.amountCents > 0);

  const individualExpenses = [];
  const otherExpenses = [];
  const incomes = [];

  for (const item of activeItems) {
    if (item.isCredit) {
      // Income entry
      if (!item.category || !item.category.trim()) {
        throw new Error(`Category is required for income "${item.beneficiary || 'Income'}" (${item.date || 'unknown date'}).`);
      }
      incomes.push({
        name: item.beneficiary || 'Income',
        amount_cents: item.amountCents,
        income_date: item.date,
        who: item.whoPaid || fallbackPayer,
        category: item.category.trim(),
        is_joint: false
      });
    } else {
      // Expense entry
      if (item.groupToOther) {
        otherExpenses.push(item);
      } else {
        if (!item.category || !item.category.trim()) {
          throw new Error(`Category is required for expense "${item.beneficiary || 'Expense'}" (${item.date || 'unknown date'}).`);
        }
        individualExpenses.push({
          name: item.beneficiary || 'Expense',
          cost_cents: item.amountCents,
          expense_date: item.date,
          who_paid: item.whoPaid || fallbackPayer,
          category: item.category.trim(),
          tag_id: item.tagId || null,
          is_joint: false,
          joint_account_id: null,
          overrides: []
        });
      }
    }
  }

  // Bundle grouped expense items if any
  if (otherExpenses.length > 0) {
    const totalOtherCents = otherExpenses.reduce((sum, item) => sum + item.amountCents, 0);
    const sortedDates = otherExpenses.map((i) => i.date).filter(Boolean).sort();
    const bundleDate = sortedDates.length > 0 ? sortedDates[sortedDates.length - 1] : new Date().toISOString().slice(0, 10);
    const bundlePayer = otherExpenses[0].whoPaid || fallbackPayer;

    individualExpenses.push({
      name: `Other expenses (${otherExpenses.length} items bundled)`,
      cost_cents: totalOtherCents,
      expense_date: bundleDate,
      who_paid: bundlePayer,
      category: 'OTHER',
      tag_id: null,
      is_joint: false,
      joint_account_id: null,
      overrides: []
    });
  }

  return {
    expenses: individualExpenses,
    incomes
  };
}

/**
 * Backward-compatible helper for preparing only expenses.
 */
export function prepareExpensesForImport(transactions, fallbackPayer) {
  return prepareTransactionsForImport(transactions, fallbackPayer).expenses;
}
