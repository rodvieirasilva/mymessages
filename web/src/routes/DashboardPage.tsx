import { useQuery } from '@tanstack/react-query';
import { Database, HardDrive, MessageSquare, RefreshCw, Users } from 'lucide-react';
import { fetchStats } from '../api';
import { formatBytes } from '../lib/format';

export function DashboardPage() {
  const { data: stats, isLoading, isError, refetch } = useQuery({
    queryKey: ['stats'],
    queryFn: () => fetchStats(),
    refetchInterval: 5000,
  });

  if (isLoading) {
    return (
      <div className="flex-1 min-h-0 overflow-y-auto custom-scroll p-4 sm:p-6" data-testid="dashboard-loading">
        <div className="mx-auto max-w-4xl animate-pulse space-y-4">
          <div className="h-8 w-48 rounded bg-slate-200 dark:bg-slate-800" />
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-28 rounded-xl bg-slate-200 dark:bg-slate-800" />
            ))}
          </div>
        </div>
      </div>
    );
  }

  if (isError || !stats) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-3 p-6 text-center" data-testid="dashboard-error">
        <p className="text-sm text-red-500">Failed to load vault statistics.</p>
        <button
          type="button"
          onClick={() => void refetch()}
          className="inline-flex items-center gap-1.5 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-500"
        >
          <RefreshCw className="h-4 w-4" /> Retry
        </button>
      </div>
    );
  }

  const ing = stats.ingestion;
  const pct =
    ing && ing.scanned_total > 0
      ? Math.min(100, Math.round((ing.processed / ing.scanned_total) * 100))
      : 0;

  return (
    <div
      data-testid="dashboard-view"
      className="flex-1 min-h-0 overflow-y-auto custom-scroll bg-slate-50 dark:bg-slate-950 p-4 sm:p-6"
    >
      <div className="mx-auto max-w-4xl space-y-6">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <h1 className="text-xl font-bold text-slate-900 dark:text-white">Vault Dashboard</h1>
          <span className="text-xs text-slate-500 dark:text-slate-400">
            v{stats.version} · {stats.commit} ({stats.build_time})
          </span>
        </div>

        {ing?.active && (
          <div
            data-testid="ingestion-progress"
            className="rounded-xl border border-blue-500/50 bg-blue-500/10 p-4 space-y-2.5"
          >
            <div className="flex flex-wrap items-center justify-between gap-2 text-sm">
              <strong className="text-blue-700 dark:text-blue-300">
                ⏳ Processing backups ({ing.phase})...
              </strong>
              <span className="text-xs font-medium text-slate-600 dark:text-slate-300 tabular-nums">
                {ing.scanned_total > 0
                  ? `${ing.processed.toLocaleString()} / ${ing.scanned_total.toLocaleString()} (${pct}%)`
                  : 'Scanning files...'}
                {' · '}+{ing.newly_indexed.toLocaleString()} new
              </span>
            </div>
            <div className="h-2 w-full overflow-hidden rounded-full bg-slate-200 dark:bg-slate-800">
              <div
                className="h-full rounded-full bg-blue-600 transition-all duration-300"
                style={{ width: `${pct}%` }}
              />
            </div>
          </div>
        )}

        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div className="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4 shadow-sm">
            <div className="flex items-center gap-2 text-xs font-medium text-slate-500 dark:text-slate-400">
              <HardDrive className="h-4 w-4 text-blue-500" /> Database Size
            </div>
            <div data-testid="stat-db-size" className="mt-2 text-2xl font-bold text-slate-900 dark:text-white">
              {formatBytes(stats.db_size_bytes)}
            </div>
            <div className="mt-1 truncate text-[11px] text-slate-400" title={stats.db_path}>
              {stats.db_path}
            </div>
          </div>

          <div className="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4 shadow-sm">
            <div className="flex items-center gap-2 text-xs font-medium text-slate-500 dark:text-slate-400">
              <MessageSquare className="h-4 w-4 text-blue-500" /> Total Messages
            </div>
            <div data-testid="stat-total-messages" className="mt-2 text-2xl font-bold text-slate-900 dark:text-white tabular-nums">
              {stats.total_messages.toLocaleString()}
            </div>
          </div>

          <div className="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4 shadow-sm">
            <div className="flex items-center gap-2 text-xs font-medium text-slate-500 dark:text-slate-400">
              <Users className="h-4 w-4 text-blue-500" /> Total Chats
            </div>
            <div data-testid="stat-total-chats" className="mt-2 text-2xl font-bold text-slate-900 dark:text-white tabular-nums">
              {stats.total_chats.toLocaleString()}
            </div>
          </div>
        </div>

        <div className="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 overflow-hidden shadow-sm">
          <div className="flex items-center gap-2 border-b border-slate-200 dark:border-slate-800 px-4 py-3">
            <Database className="h-4 w-4 text-blue-500" />
            <h2 className="text-sm font-semibold text-slate-900 dark:text-white">Messages by Source</h2>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead className="bg-slate-50 dark:bg-slate-950/60 text-xs uppercase text-slate-500 dark:text-slate-400">
                <tr>
                  <th className="px-4 py-2.5">Source</th>
                  <th className="px-4 py-2.5">Messages</th>
                  <th className="px-4 py-2.5">Chats</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100 dark:divide-slate-800">
                {stats.sources.length === 0 ? (
                  <tr>
                    <td colSpan={3} className="px-4 py-6 text-center text-xs text-slate-500">
                      No messages indexed yet.
                    </td>
                  </tr>
                ) : (
                  stats.sources.map((s) => (
                    <tr key={s.source} className="hover:bg-slate-50 dark:hover:bg-slate-800/40">
                      <td className="px-4 py-2.5 font-medium capitalize text-slate-900 dark:text-slate-100">
                        {s.source}
                      </td>
                      <td className="px-4 py-2.5 tabular-nums text-slate-700 dark:text-slate-300">
                        {s.message_count.toLocaleString()}
                      </td>
                      <td className="px-4 py-2.5 tabular-nums text-slate-700 dark:text-slate-300">
                        {s.chat_count.toLocaleString()}
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}
