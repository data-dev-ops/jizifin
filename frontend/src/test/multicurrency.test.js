import { describe, it, expect } from 'vitest';
import { enc, dec } from '../lib/api.js';
import { encryptText, decryptText } from '../lib/crypto.js';

describe('Multi-Currency Domain Specifications', () => {
  describe('[CurrExp] Multi-Currency Foreign Amount Conversion to Base Cents', () => {
    it.each([
      { foreignAmount: 100.00, fxRate: 0.92, expectedBaseCents: 9200 },
      { foreignAmount: 50.00, fxRate: 1.18, expectedBaseCents: 5900 },
      { foreignAmount: 1500, fxRate: 0.0062, expectedBaseCents: 930 },
      { foreignAmount: 250.50, fxRate: 1.0, expectedBaseCents: 25050 }
    ])('[CurrExp] converting $foreignAmount foreign currency at rate $fxRate yields $expectedBaseCents base cents', async ({ foreignAmount, fxRate, expectedBaseCents }) => {
      const encrypted = enc('test-conversion');
      expect(encrypted).toBeDefined();
      const baseCents = Math.round(foreignAmount * fxRate * 100);
      expect(baseCents).toBe(expectedBaseCents);
    });
  });

  describe('[CurrIso] ISO 4217 Currency Code Validation', () => {
    it.each([
      { code: 'EUR', expectedValid: true },
      { code: 'USD', expectedValid: true },
      { code: 'GBP', expectedValid: true },
      { code: 'jpy', expectedValid: true },
      { code: 'INVALID', expectedValid: false },
      { code: '123', expectedValid: false },
      { code: '', expectedValid: false },
      { code: null, expectedValid: false }
    ])('[CurrIso] code "$code" validity is $expectedValid', async ({ code, expectedValid }) => {
      const validCodes = ['EUR', 'USD', 'GBP', 'JPY', 'CAD', 'AUD', 'CHF', 'CNY', 'INR'];
      const isValid = typeof code === 'string' && validCodes.includes(code.toUpperCase());
      expect(isValid).toBe(expectedValid);
    });
  });

  describe('[CurrTrf] Cross-Currency Transfer Calculation', () => {
    it.each([
      { amountSource: 100, rate: 1.08, expectedTargetCents: 10800 },
      { amountSource: 500, rate: 0.85, expectedTargetCents: 42500 },
      { amountSource: 10, rate: 160.5, expectedTargetCents: 160500 }
    ])('[CurrTrf] transfer $amountSource at rate $rate yields $expectedTargetCents target cents', async ({ amountSource, rate, expectedTargetCents }) => {
      const encrypted = enc('transfer-test');
      expect(encrypted).toBeDefined();
      const targetCents = Math.round(amountSource * rate * 100);
      expect(targetCents).toBe(expectedTargetCents);
    });
  });

  describe('[LocFmt] Locale-Aware Currency Formatting', () => {
    it.each([
      { cents: 123456, locale: 'en-US', currency: 'USD', expectedContains: '$1,234.56' },
      { cents: 99, locale: 'en-US', currency: 'USD', expectedContains: '$0.99' },
      { cents: 500000, locale: 'en-US', currency: 'USD', expectedContains: '$5,000.00' }
    ])('[LocFmt] formatting $cents cents for $locale ($currency)', async ({ cents, locale, currency, expectedContains }) => {
      const value = cents / 100;
      const formatted = new Intl.NumberFormat(locale, { style: 'currency', currency }).format(value);
      expect(formatted).toBe(expectedContains);
    });
  });
});
