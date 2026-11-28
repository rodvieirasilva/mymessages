import { Link } from '@tanstack/react-router';
import { RefreshCw, Search, X } from 'lucide-react';
import type { ChatSummary } from '../types';

export interface ChatSidebarProps {
  chats: ChatSummary[];
  isLoading: boolean;
  isError: boolean;
  onRetry: () => void;
  activeChatId?: string;
  chatFilter: string;
  onChatFilterChange: (val: string) => void;
  mobileOpen: boolean;
  onCloseMobile: () => void;
}

export function ChatSidebar({
  chats,
  isLoading,
  isError,
  onRetry,
  activeChatId,
  chatFilter,
  onChatFilterChange,
  mobileOpen,
  onCloseMobile,
}: ChatSidebarProps) {
  return (
    <>
      {mobileOpen && (
        <div
          onClick={onCloseMobile}
          className="fixed inset-0 z-30 bg-black/50 lg:hidden"
          aria-hidden="true"
        />
      )}
      <aside
        data-testid="chat-sidebar"
        className={`fixed inset-y-0 left-0 z-40 w-[85vw] max-w-xs lg:static lg:z-auto lg:w-80 shrink-0 flex flex-col min-h-0 border-r border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 transition-transform duration-200 ${
          mobileOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0'
        }`}
      >
        <div className="shrink-0 flex items-center justify-between border-b border-slate-200 dark:border-slate-800 px-4 py-3">
          <h2 data-testid="sidebar-header" className="text-sm font-semibold text-slate-900 dark:text-slate-100">
            Conversations {chats.length > 0 ? `(${chats.length})` : ''}
          </h2>
          <button
            type="button"
            onClick={onCloseMobile}
            aria-label="Close conversations"
            className="lg:hidden rounded-md p-1 text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="shrink-0 border-b border-slate-200 dark:border-slate-800 p-2.5">
          <div className="relative">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-slate-400" />
            <input
              type="text"
              value={chatFilter}
              onChange={(e) => onChatFilterChange(e.target.value)}
              data-testid="chat-search-input"
              placeholder="Filter chats (e.g. alice*)..."
              className="w-full rounded-lg border border-slate-300 dark:border-slate-700 bg-slate-50 dark:bg-slate-950 pl-8 pr-7 py-1.5 text-xs text-slate-900 dark:text-slate-100 placeholder:text-slate-400 focus:border-blue-500 focus:outline-none"
            />
            {chatFilter && (
              <button
                type="button"
                onClick={() => onChatFilterChange('')}
                data-testid="clear-chat-filter-button"
                aria-label="Clear chat filter"
                className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
        </div>

        <div
          data-testid="chat-list"
          className="flex-1 min-h-0 overflow-y-auto custom-scroll divide-y divide-slate-100 dark:divide-slate-800/80"
        >
          {isLoading ? (
            <div className="p-4 space-y-3" data-testid="chat-list-loading">
              {[1, 2, 3, 4, 5].map((i) => (
                <div key={i} className="animate-pulse space-y-1.5">
                  <div className="h-4 w-2/3 rounded bg-slate-200 dark:bg-slate-800" />
                  <div className="h-3 w-1/2 rounded bg-slate-200/70 dark:bg-slate-800/70" />
                </div>
              ))}
            </div>
          ) : isError ? (
            <div className="p-4 text-center space-y-2" data-testid="chat-list-error">
              <p className="text-xs text-red-500">Failed to load conversations.</p>
              <button
                type="button"
                onClick={onRetry}
                className="inline-flex items-center gap-1 rounded-lg bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-500"
              >
                <RefreshCw className="h-3.5 w-3.5" /> Retry
              </button>
            </div>
          ) : chats.length === 0 ? (
            <div className="p-6 text-center text-xs text-slate-500 dark:text-slate-400" data-testid="chat-list-empty">
              No conversations found.
            </div>
          ) : (
            chats.map((chat) => {
              const isActive = chat.id === activeChatId;
              const badge =
                chat.source_id === 'sms' ? 'SMS' : chat.source_id === 'telegram' ? 'TG' : null;
              return (
                <Link
                  key={chat.id}
                  to="/chats/$chatId"
                  params={{ chatId: chat.id }}
                  onClick={onCloseMobile}
                  data-testid="chat-item"
                  className={`block px-4 py-3 transition-colors ${
                    isActive
                      ? 'bg-blue-50 dark:bg-blue-950/60 border-l-2 border-l-blue-500'
                      : 'hover:bg-slate-50 dark:hover:bg-slate-800/60'
                  }`}
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="truncate text-sm font-medium text-slate-900 dark:text-slate-100">
                      {chat.name || chat.remote_id}
                    </span>
                    {badge && (
                      <span
                        data-testid="chat-source-badge"
                        className="shrink-0 rounded bg-blue-500/15 px-1.5 py-0.5 text-[10px] font-bold uppercase text-blue-600 dark:text-blue-300"
                      >
                        {badge}
                      </span>
                    )}
                  </div>
                  <div className="mt-1 flex items-center justify-between gap-2 text-xs text-slate-500 dark:text-slate-400">
                    <span className="truncate">{chat.last_message || 'No text preview'}</span>
                    <span className="shrink-0 tabular-nums">{chat.message_count}</span>
                  </div>
                </Link>
              );
            })
          )}
        </div>
      </aside>
    </>
  );
}
