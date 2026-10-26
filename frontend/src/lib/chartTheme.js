/**
 * chartTheme.js — Chart.js colours for the current light/dark theme.
 */

export function isDark() {
  return typeof document === 'undefined' || document.documentElement.classList.contains('dark');
}

export function chartColors() {
  const dark = isDark();
  return {
    grid: dark ? 'rgba(255, 255, 255, 0.05)' : 'rgba(0, 0, 0, 0.05)',
    tick: dark ? '#9ca3af' : '#64748b',
    label: dark ? '#e5e7eb' : '#1e293b',
    tooltip: {
      backgroundColor: dark ? 'rgba(15, 23, 42, 0.95)' : 'rgba(255, 255, 255, 0.95)',
      titleColor: dark ? '#f1f5f9' : '#0f172a',
      bodyColor: dark ? '#cbd5e1' : '#475569',
      borderColor: dark ? 'rgba(99, 102, 241, 0.4)' : 'rgba(99, 102, 241, 0.25)',
      borderWidth: 1,
    },
  };
}

/** `#rrggbb` → `rgba(r,g,b,alpha)`. */
export function hexToRgba(hex, alpha) {
  const h = (hex || '#6366f1').replace('#', '');
  const [r, g, b] = [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16));
  return `rgba(${r || 99},${g || 102},${b || 241},${alpha})`;
}

export const PALETTE = [
  'rgba(99, 102, 241, 0.85)', 'rgba(139, 92, 246, 0.85)', 'rgba(14, 165, 233, 0.85)', 'rgba(236, 72, 153, 0.85)',
  'rgba(16, 185, 129, 0.85)', 'rgba(245, 158, 11, 0.85)', 'rgba(244, 63, 94, 0.85)', 'rgba(20, 184, 166, 0.85)',
];
