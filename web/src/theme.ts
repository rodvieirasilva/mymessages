export type ThemeMode = 'dark' | 'light';

export const THEME_STORAGE_KEY = 'mymessages_theme';

export function getPreferredTheme(): ThemeMode {
  if (typeof window !== 'undefined' && window.location && window.location.search) {
    const params = new URLSearchParams(window.location.search);
    const themeParam = params.get('theme');
    if (themeParam === 'dark' || themeParam === 'light') {
      return themeParam;
    }
  }

  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    if (stored === 'dark' || stored === 'light') {
      return stored;
    }
  } catch {
    // LocalStorage might be inaccessible in some environments
  }

  if (typeof window !== 'undefined' && window.matchMedia && window.matchMedia('(prefers-color-scheme: light)').matches) {
    return 'light';
  }
  return 'dark';
}

export function applyTheme(theme: ThemeMode): void {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', theme);
  }
  try {
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // Ignore storage write errors
  }
}

export function toggleTheme(): ThemeMode {
  const current = (typeof document !== 'undefined'
    ? (document.documentElement.getAttribute('data-theme') as ThemeMode)
    : null) || getPreferredTheme();
  const next: ThemeMode = current === 'dark' ? 'light' : 'dark';
  applyTheme(next);
  return next;
}

export function initTheme(): ThemeMode {
  const theme = getPreferredTheme();
  applyTheme(theme);
  return theme;
}
