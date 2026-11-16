import { beforeEach, describe, expect, it } from 'vitest';
import { applyTheme, getPreferredTheme, initTheme, THEME_STORAGE_KEY, toggleTheme } from './theme';

describe('theme system', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
  });

  it('defaults to dark theme when no storage preference exists', () => {
    const theme = getPreferredTheme();
    expect(theme).toBe('dark');
  });

  it('reads theme from localStorage if valid', () => {
    localStorage.setItem(THEME_STORAGE_KEY, 'light');
    expect(getPreferredTheme()).toBe('light');

    localStorage.setItem(THEME_STORAGE_KEY, 'dark');
    expect(getPreferredTheme()).toBe('dark');
  });

  it('applies theme to document element and persists', () => {
    applyTheme('light');
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('light');

    applyTheme('dark');
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('dark');
  });

  it('toggles between dark and light themes', () => {
    applyTheme('dark');
    const next = toggleTheme();
    expect(next).toBe('light');
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');

    const next2 = toggleTheme();
    expect(next2).toBe('dark');
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  it('initializes theme on startup', () => {
    localStorage.setItem(THEME_STORAGE_KEY, 'light');
    const initialized = initTheme();
    expect(initialized).toBe('light');
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });
});
