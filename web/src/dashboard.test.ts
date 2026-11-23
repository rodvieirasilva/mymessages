import { describe, expect, it } from 'vitest';
import { formatBytes, renderDashboard } from './dashboard';

describe('dashboard module', () => {
  it('formats byte sizes cleanly', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1024)).toBe('1.00 KB');
    expect(formatBytes(1048576)).toBe('1.00 MB');
  });

  it('renders vault stats and source breakdown', () => {
    const container = document.createElement('div');
    renderDashboard(container, {
      version: '0.2.0',
      build_time: '2026-11-22',
      commit: 'abc1234',
      db_path: '/data/messages.sqlite3',
      db_size_bytes: 2097152,
      total_messages: 150,
      total_chats: 12,
      sources: [
        { source: 'whatsapp', message_count: 100, chat_count: 8 },
        { source: 'sms', message_count: 50, chat_count: 4 },
      ],
    });

    expect(container.querySelector('[data-testid="stat-db-size"]')?.textContent).toBe('2.00 MB');
    expect(container.querySelector('[data-testid="stat-total-messages"]')?.textContent).toBe('150');
    expect(container.querySelector('[data-testid="stat-total-chats"]')?.textContent).toBe('12');
    expect(container.textContent).toContain('whatsapp');
    expect(container.textContent).toContain('sms');
  });

  it('renders ingestion progress banner when active', () => {
    const container = document.createElement('div');
    renderDashboard(container, {
      version: '0.2.0',
      build_time: '2026-11-22',
      commit: 'abc1234',
      db_path: '/data/messages.sqlite3',
      db_size_bytes: 1024,
      total_messages: 50,
      total_chats: 2,
      sources: [],
      ingestion: {
        active: true,
        phase: 'indexing',
        scanned_total: 200,
        processed: 100,
        newly_indexed: 50,
        last_completed_at: null,
      },
    });

    const banner = container.querySelector('[data-testid="ingestion-progress"]');
    expect(banner).not.toBeNull();
    expect(banner?.textContent).toContain('50%');
    expect(banner?.textContent).toContain('indexing');
  });
});
