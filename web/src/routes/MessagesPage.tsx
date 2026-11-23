import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
import { ArrowUp, Loader2, RefreshCw } from 'lucide-react';
import { useEffect, useRef } from 'react';
import { fetchChatMessages, searchMessages } from '../api';
import { MessageCard } from '../components/MessageCard';
import type { ChatSummary, SearchFilters } from '../types';

export interface MessagesPageProps {
  chatId?: string;
  activeChat?: ChatSummary;
  filters: SearchFilters;
}

export function MessagesPage({ chatId, activeChat, filters }: MessagesPageProps) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const prevChatIdRef = useRef<string | undefined>(undefined);

  const hasSearch =
    Boolean(filters.query.trim()) ||
    Boolean(filters.source) ||
    Boolean(filters.account) ||
    Boolean(filters.media_type) ||
    filters.has_media !== undefined ||
    filters.is_from_me !== undefined;

  const chatQuery = useInfiniteQuery({
    queryKey: ['chat-messages', chatId],
    queryFn: ({ pageParam }) => fetchChatMessages(chatId!, 50, pageParam),
    initialPageParam: undefined as number | undefined,
    getNextPageParam: (lastPage) => {
      if (lastPage.length < 50) return undefined;
      const oldest = lastPage[lastPage.length - 1];
      return oldest ? oldest.timestamp : undefined;
    },
    enabled: Boolean(chatId),
    retry: 2,
  });

  const searchQuery = useQuery({
    queryKey: ['search', filters],
    queryFn: ({ signal }) => searchMessages(filters, signal),
    enabled: !chatId && hasSearch,
    retry: 2,
  });

  const chatMessages = chatQuery.data
    ? [...chatQuery.data.pages].reverse().flatMap((page) => [...page].reverse())
    : [];

  useEffect(() => {
    if (chatId && chatId !== prevChatIdRef.current && chatMessages.length > 0) {
      prevChatIdRef.current = chatId;
      requestAnimationFrame(() => {
        if (scrollRef.current) {
          scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
      });
    }
  }, [chatId, chatMessages.length]);

  const handleScroll = () => {
    const el = scrollRef.current;
    if (!el || !chatId || !chatQuery.hasNextPage || chatQuery.isFetchingNextPage) return;
    if (el.scrollTop < 120) {
      const prevHeight = el.scrollHeight;
      void chatQuery.fetchNextPage().then(() => {
        requestAnimationFrame(() => {
          if (scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight - prevHeight;
          }
        });
      });
    }
  };

  if (chatId) {
    return (
      <div className="flex-1 flex flex-col min-h-0 overflow-hidden bg-slate-100 dark:bg-slate-950">
        <div
          data-testid="content-header"
          className="shrink-0 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 px-4 py-2.5 text-sm font-semibold text-slate-900 dark:text-white"
        >
          {activeChat ? activeChat.name || activeChat.remote_id : `Chat ${chatId}`}
        </div>

        <div
          ref={scrollRef}
          onScroll={handleScroll}
          data-testid="messages-container"
          className="flex-1 min-h-0 overflow-y-auto custom-scroll p-3 sm:p-5 flex flex-col gap-2.5"
        >
          {chatQuery.hasNextPage && (
            <button
              type="button"
              onClick={() => void chatQuery.fetchNextPage()}
              disabled={chatQuery.isFetchingNextPage}
              className="self-center inline-flex items-center gap-1.5 rounded-full border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-1 text-xs font-medium text-slate-700 dark:text-slate-300 hover:border-blue-500"
            >
              {chatQuery.isFetchingNextPage ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <ArrowUp className="h-3.5 w-3.5" />
              )}
              Load older messages
            </button>
          )}

          {chatQuery.isLoading ? (
            <div className="flex-1 flex items-center justify-center text-sm text-slate-500">
              <Loader2 className="h-5 w-5 animate-spin mr-2 text-blue-500" /> Loading messages...
            </div>
          ) : chatQuery.isError ? (
            <div className="flex-1 flex flex-col items-center justify-center gap-2 text-center">
              <p className="text-sm text-red-500">Failed to load messages.</p>
              <button
                type="button"
                onClick={() => void chatQuery.refetch()}
                className="inline-flex items-center gap-1.5 rounded-lg bg-blue-600 px-3 py-1.5 text-xs font-medium text-white"
              >
                <RefreshCw className="h-3.5 w-3.5" /> Retry
              </button>
            </div>
          ) : chatMessages.length === 0 ? (
            <div className="flex-1 flex items-center justify-center text-sm text-slate-500" data-testid="empty-state">
              No messages in this conversation.
            </div>
          ) : (
            chatMessages.map((msg) => <MessageCard key={msg.id} msg={msg} />)
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col min-h-0 overflow-hidden bg-slate-100 dark:bg-slate-950">
      <div
        data-testid="content-header"
        className="shrink-0 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 px-4 py-2.5 text-sm font-semibold text-slate-900 dark:text-white"
      >
        {hasSearch ? 'Search Results' : 'All Messages'}
      </div>

      <div
        data-testid="messages-container"
        className="flex-1 min-h-0 overflow-y-auto custom-scroll p-3 sm:p-5 flex flex-col gap-2.5"
      >
        {!hasSearch ? (
          <div className="flex-1 flex items-center justify-center text-center text-sm text-slate-500 dark:text-slate-400 px-4" data-testid="empty-state">
            Use search, select a facet, or pick a conversation to view history.
          </div>
        ) : searchQuery.isLoading ? (
          <div className="flex-1 flex items-center justify-center text-sm text-slate-500">
            <Loader2 className="h-5 w-5 animate-spin mr-2 text-blue-500" /> Searching...
          </div>
        ) : searchQuery.isError ? (
          <div className="flex-1 flex flex-col items-center justify-center gap-2 text-center">
            <p className="text-sm text-red-500">Search failed.</p>
            <button
              type="button"
              onClick={() => void searchQuery.refetch()}
              className="inline-flex items-center gap-1.5 rounded-lg bg-blue-600 px-3 py-1.5 text-xs font-medium text-white"
            >
              <RefreshCw className="h-3.5 w-3.5" /> Retry
            </button>
          </div>
        ) : (searchQuery.data?.length ?? 0) === 0 ? (
          <div className="flex-1 flex items-center justify-center text-sm text-slate-500" data-testid="empty-state">
            No matching messages found.
          </div>
        ) : (
          searchQuery.data!.map((msg) => <MessageCard key={msg.id} msg={msg} />)
        )}
      </div>
    </div>
  );
}
