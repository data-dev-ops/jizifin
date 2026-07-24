import '@testing-library/jest-dom/vitest';
import { vi, beforeEach } from 'vitest';
import { webcrypto } from 'node:crypto';

// Polyfill/bind WebCrypto for jsdom environment if crypto.subtle is missing
if (!globalThis.crypto || !globalThis.crypto.subtle) {
  Object.defineProperty(globalThis, 'crypto', {
    value: webcrypto,
    writable: true,
  });
}
if (!window.crypto || !window.crypto.subtle) {
  Object.defineProperty(window, 'crypto', {
    value: webcrypto,
    writable: true,
  });
}

function mockFetchRouter(input, init) {
  const urlStr = typeof input === 'string' ? input : (input?.url || String(input));

  if (urlStr.includes('/auth/salt')) {
    return Promise.resolve(new Response(JSON.stringify({ value: 'mocked-salt' }), { status: 200 }));
  }

  if (urlStr.includes('/salaries')) {
    return Promise.resolve(new Response(JSON.stringify([
      { who: 'Jim', amount_cents: 300000 },
      { who: 'Zina', amount_cents: 200000 },
    ]), { status: 200, headers: { 'Content-Type': 'application/json' } }));
  }

  if (urlStr.includes('/analytics/budget')) {
    return Promise.resolve(new Response(JSON.stringify([
      { category: 'GROCERIES', limit_cents: 30000, actual_cents: 15000, pct_used: 50.0 },
    ]), { status: 200, headers: { 'Content-Type': 'application/json' } }));
  }

  if (urlStr.includes('/paybacks')) {
    return Promise.resolve(new Response(JSON.stringify({ rows: [], debts: [], month: '' }), { status: 200 }));
  }

  if (urlStr.includes('/income/categories')) {
    return Promise.resolve(new Response(JSON.stringify([{ category: 'SALARY' }]), { status: 200 }));
  }

  if (urlStr.includes('/query')) {
    return Promise.resolve(new Response(JSON.stringify({ columns: [], rows: [], row_count: 0, truncated: false }), { status: 200 }));
  }

  // Joint account routes
  if (urlStr.includes('/joint-account/categories') && (!init || init.method === 'GET')) {
    return Promise.resolve(new Response(JSON.stringify([]), { status: 200, headers: { 'Content-Type': 'application/json' } }));
  }
  if (urlStr.includes('/joint-account/deposits') && (!init || init.method === 'GET')) {
    return Promise.resolve(new Response(JSON.stringify([]), { status: 200, headers: { 'Content-Type': 'application/json' } }));
  }
  if (urlStr.includes('/joint-account/expected-costs') && (!init || init.method === 'GET')) {
    return Promise.resolve(new Response(JSON.stringify([]), { status: 200, headers: { 'Content-Type': 'application/json' } }));
  }
  if (urlStr.includes('/joint-account/corrections') && (!init || init.method === 'GET')) {
    return Promise.resolve(new Response(JSON.stringify([]), { status: 200, headers: { 'Content-Type': 'application/json' } }));
  }
  if (urlStr.includes('/joint-account/dashboard')) {
    return Promise.resolve(new Response(null, { status: 404 }));
  }
  if (urlStr.includes('/joint-account')) {
    return Promise.resolve(new Response(JSON.stringify(null), { status: 200, headers: { 'Content-Type': 'application/json' } }));
  }

  return Promise.resolve(new Response(JSON.stringify([]), { status: 200, headers: { 'Content-Type': 'application/json' } }));
}

// Mock Canvas 2D context for Chart.js
if (typeof HTMLCanvasElement !== 'undefined') {
  HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
    fillRect: vi.fn(),
    clearRect: vi.fn(),
    getImageData: vi.fn(() => ({ data: new Array(4) })),
    putImageData: vi.fn(),
    createImageData: vi.fn(() => []),
    setTransform: vi.fn(),
    drawImage: vi.fn(),
    save: vi.fn(),
    fillText: vi.fn(),
    restore: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    closePath: vi.fn(),
    stroke: vi.fn(),
    translate: vi.fn(),
    scale: vi.fn(),
    rotate: vi.fn(),
    arc: vi.fn(),
    fill: vi.fn(),
    measureText: vi.fn(() => ({ width: 0 })),
    transform: vi.fn(),
    rect: vi.fn(),
    clip: vi.fn(),
  }));
}

// Mock ResizeObserver
if (typeof window !== 'undefined' && !window.ResizeObserver) {
  window.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

// Mock matchMedia
if (typeof window !== 'undefined' && !window.matchMedia) {
  window.matchMedia = vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
}

// Setup fetch router on both globalThis and window, and clear localStorage before each test
beforeEach(() => {
  localStorage.clear();
  globalThis.fetch = mockFetchRouter;
  if (typeof window !== 'undefined') {
    window.fetch = mockFetchRouter;
  }
});
