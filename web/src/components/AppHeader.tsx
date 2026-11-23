import { Link } from '@tanstack/react-router';
import { BarChart3, ExternalLink, Menu, MessageSquare, Moon, Search, Sun } from 'lucide-react';
import type { IngestionStatus } from '../api';
import { getVersionLabel } from '../lib/format';

export interface AppHeaderProps {
  theme: 'dark' | 'light';
  onToggleTheme: () => void;
  onToggleSidebar: () => void;
  searchValue: string;
  onSearchChange: (val: string) => void;
  onSearchSubmit: () => void;
  ingestion?: IngestionStatus;
}

export function AppHeader({
  theme,
  onToggleTheme,
  onToggleSidebar,
  searchValue,
  onSearchChange,
  onSearchSubmit,
  ingestion,
}: AppHeaderProps) {
  const versionLabel = getVersionLabel();
  const pct =
    ingestion && ingestion.scanned_total > 0
      ? Math.min(100, Math.round((ingestion.processed / ingestion.scanned_total) * 100))
      : 0;

  return (
    <header className="shrink-0 z-30 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 px-3 py-2.5 sm:px-4">
      <div className="flex flex-wrap items-center gap-2 sm:gap-3">
        <div className="flex items-center gap-2 shrink-0">
          <button
            type="button"
            onClick={onToggleSidebar}
            data-testid="mobile-sidebar-button"
            aria-label="Toggle conversations"
            className="lg:hidden inline-flex items-center justify-center rounded-lg border border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-800 p-2 text-slate-700 dark:text-slate-200 hover:bg-slate-200 dark:hover:bg-slate-700"
          >
            <Menu className="h-4 w-4" />
          </button>
          <Link to="/" className="flex items-center gap-2 font-bold text-slate-900 dark:text-white">
            <img src="/logo.svg" alt="mymessages" className="h-7 w-7 rounded-md" />
            <span className="text-base tracking-tight">mymessages</span>
          </Link>
          <span
            data-testid="version-badge"
            className="hidden xl:inline-flex items-center rounded-full bg-blue-500/10 border border-blue-500/30 px-2 py-0.5 text-[11px] font-medium text-blue-600 dark:text-blue-400"
          >
            {versionLabel}
          </span>
        </div>

        <form
          onSubmit={(e) => {
            e.preventDefault();
            onSearchSubmit();
          }}
          className="order-3 lg:order-2 flex w-full lg:flex-1 lg:max-w-xl items-center gap-1.5"
        >
          <div className="relative flex-1">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-slate-400" />
            <input
              type="text"
              value={searchValue}
              onChange={(e) => onSearchChange(e.target.value)}
              data-testid="search-input"
              placeholder="Search text or Lucene (e.g. from:Alice source:telegram)..."
              className="w-full rounded-lg border border-slate-300 dark:border-slate-700 bg-slate-50 dark:bg-slate-950 pl-8 pr-3 py-1.5 text-sm text-slate-900 dark:text-slate-100 placeholder:text-slate-400 focus:border-blue-500 focus:outline-none"
            />
          </div>
          <button
            type="submit"
            data-testid="search-button"
            className="shrink-0 rounded-lg bg-blue-600 hover:bg-blue-500 px-3 py-1.5 text-sm font-medium text-white transition-colors"
          >
            Search
          </button>
        </form>

        <nav className="order-2 lg:order-3 ml-auto flex items-center gap-1.5 sm:gap-2">
          {ingestion?.active && (
            <Link
              to="/dashboard"
              data-testid="sync-indicator"
              className="inline-flex items-center gap-1 rounded-full border border-blue-500/50 bg-blue-500/15 px-2.5 py-1 text-xs font-semibold text-blue-600 dark:text-blue-300 animate-pulse"
            >
              ⏳ {ingestion.scanned_total > 0 ? `${pct}%` : 'Syncing'}
            </Link>
          )}

          <Link
            to="/"
            activeProps={{ className: 'bg-blue-600 text-white border-blue-600' }}
            inactiveProps={{ className: 'bg-slate-100 dark:bg-slate-800 text-slate-700 dark:text-slate-200 border-slate-300 dark:border-slate-700 hover:bg-slate-200 dark:hover:bg-slate-700' }}
            className="inline-flex items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs sm:text-sm font-medium transition-colors"
          >
            <MessageSquare className="h-4 w-4" />
            <span className="hidden sm:inline">Messages</span>
          </Link>

          <Link
            to="/dashboard"
            data-testid="dashboard-toggle-button"
            activeProps={{ className: 'bg-blue-600 text-white border-blue-600' }}
            inactiveProps={{ className: 'bg-slate-100 dark:bg-slate-800 text-slate-700 dark:text-slate-200 border-slate-300 dark:border-slate-700 hover:bg-slate-200 dark:hover:bg-slate-700' }}
            className="inline-flex items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs sm:text-sm font-medium transition-colors"
          >
            <BarChart3 className="h-4 w-4" />
            <span className="hidden sm:inline">Dashboard</span>
          </Link>

          <button
            type="button"
            onClick={onToggleTheme}
            data-testid="theme-toggle-button"
            aria-label="Toggle theme"
            className="inline-flex items-center justify-center rounded-lg border border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-800 p-1.5 text-slate-700 dark:text-slate-200 hover:bg-slate-200 dark:hover:bg-slate-700"
          >
            {theme === 'dark' ? <Moon className="h-4 w-4" /> : <Sun className="h-4 w-4" />}
          </button>

          <a
            href="/swagger-ui/"
            target="_blank"
            rel="noopener noreferrer"
            data-testid="swagger-link"
            className="hidden md:inline-flex items-center gap-1 rounded-lg border border-slate-300 dark:border-slate-700 px-2.5 py-1.5 text-xs font-medium text-slate-600 dark:text-slate-300 hover:border-blue-500 hover:text-blue-500"
          >
            API <ExternalLink className="h-3.5 w-3.5" />
          </a>
        </nav>
      </div>
    </header>
  );
}
