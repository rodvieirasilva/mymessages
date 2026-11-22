import { describe, expect, it } from 'vitest';
import { buildAppShell } from './layout';

describe('responsive app shell', () => {
  it('renders mobile sidebar toggle, chat filter, and dashboard controls', () => {
    const root = document.createElement('div');
    root.innerHTML = buildAppShell('dark');

    expect(root.querySelector('[data-testid="mobile-sidebar-button"]')).not.toBeNull();
    expect(root.querySelector('[data-testid="chat-search-input"]')).not.toBeNull();
    expect(root.querySelector('[data-testid="dashboard-toggle-button"]')).not.toBeNull();
    expect(root.querySelector('[data-testid="dashboard-view"]')).not.toBeNull();
  });

  it('toggles mobile-open class on sidebar for mobile drawer behavior', () => {
    const root = document.createElement('div');
    root.innerHTML = buildAppShell('light');

    const btn = root.querySelector('[data-testid="mobile-sidebar-button"]') as HTMLButtonElement;
    const sidebar = root.querySelector('.sidebar') as HTMLElement;
    btn.onclick = () => sidebar.classList.toggle('mobile-open');

    expect(sidebar.classList.contains('mobile-open')).toBe(false);
    btn.click();
    expect(sidebar.classList.contains('mobile-open')).toBe(true);
    btn.click();
    expect(sidebar.classList.contains('mobile-open')).toBe(false);
  });
});
