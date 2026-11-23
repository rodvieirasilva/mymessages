import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DashboardPage } from './DashboardPage';

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe('DashboardPage', () => {
  it('renders stats and active ingestion progress bar from TanStack Query', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        version: '0.2.0',
        build_time: '2026-11-23',
        commit: 'abc1234',
        db_path: '/data/messages.sqlite3',
        db_size_bytes: 2097152,
        total_messages: 150,
        total_chats: 12,
        sources: [
          { source: 'whatsapp', message_count: 100, chat_count: 8 },
          { source: 'telegram', message_count: 50, chat_count: 4 },
        ],
        ingestion: {
          active: true,
          phase: 'indexing',
          scanned_total: 200,
          processed: 100,
          newly_indexed: 50,
          last_completed_at: null,
        },
      }),
    } as Response);

    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={qc}>
        <DashboardPage />
      </QueryClientProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('stat-db-size').textContent).toBe('2.00 MB');
    });
    expect(screen.getByTestId('stat-total-messages').textContent).toBe('150');
    expect(screen.getByTestId('stat-total-chats').textContent).toBe('12');
    expect(screen.getByTestId('ingestion-progress').textContent).toContain('50%');
    expect(screen.getByText('telegram')).toBeTruthy();
  });

  it('renders error state with retry button on fetch failure', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
      ok: false,
      status: 500,
    } as Response);

    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={qc}>
        <DashboardPage />
      </QueryClientProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('dashboard-error')).toBeTruthy();
    });
  });
});
