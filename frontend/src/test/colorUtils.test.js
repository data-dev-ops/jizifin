import { describe, it, expect } from 'vitest';
import { getRandomPastelColor, hslToHex } from '../lib/colorUtils.js';

describe('colorUtils.js — Pastel colour utilities', () => {
  it.each([
    { h: 0, s: 100, l: 50, expected: '#ff0000' },
    { h: 120, s: 100, l: 50, expected: '#00ff00' },
    { h: 240, s: 100, l: 50, expected: '#0000ff' },
  ])('hslToHex converts HSL ($h, $s, $l) to valid hex string ($expected)', ({ h, s, l, expected }) => {
    expect(hslToHex(h, s, l)).toBe(expected);
  });

  it.each([
    { iteration: 1 },
    { iteration: 2 },
  ])('getRandomPastelColor returns a valid 7-character hex string starting with # (iteration $iteration)', () => {
    for (let i = 0; i < 20; i++) {
      const color = getRandomPastelColor();
      expect(color).toMatch(/^#[0-9a-f]{6}$/);
    }
  });

  it.each([
    { sample: 1 },
    { sample: 2 },
  ])('getRandomPastelColor produces pastel colours within light RGB ranges (sample $sample)', () => {
    for (let i = 0; i < 10; i++) {
      const hex = getRandomPastelColor();
      const r = parseInt(hex.slice(1, 3), 16);
      const g = parseInt(hex.slice(3, 5), 16);
      const b = parseInt(hex.slice(5, 7), 16);
      
      const maxChannel = Math.max(r, g, b);
      const minChannel = Math.min(r, g, b);
      expect(maxChannel).toBeGreaterThan(160);
      expect(minChannel).toBeGreaterThan(80);
    }
  });
});
