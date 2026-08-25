import './app.css';
import App from './App.svelte';
import { theme } from './lib/stores.js';

// ── Theme Manager ────────────────────────────────────────────────────────────
function applyTheme(pref) {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  const prefersDark = window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false;
  const isDark = pref === 'dark' || (pref === 'system' && prefersDark);

  if (isDark) {
    root.classList.add('dark');
    root.style.colorScheme = 'dark';
  } else {
    root.classList.remove('dark');
    root.style.colorScheme = 'light';
  }
}

// Subscribe to store changes
theme.subscribe((val) => {
  applyTheme(val);
});

// Listen to OS system theme changes if set to 'system'
if (typeof window !== 'undefined') {
  window.matchMedia?.('(prefers-color-scheme: dark)')?.addEventListener('change', () => {
    let currentTheme = 'dark';
    const unsub = theme.subscribe((v) => { currentTheme = v; });
    unsub();
    if (currentTheme === 'system') {
      applyTheme('system');
    }
  });
}

const app = new App({
  target: document.getElementById('app'),
});

export default app;

