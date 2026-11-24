import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query';
import {
  Outlet,
  RouterProvider,
  createRootRoute,
  createRoute,
  createRouter,
  useNavigate,
  useParams,
} from '@tanstack/react-router';
import { createContext, useContext, useEffect, useState } from 'react';
import { fetchAccounts, fetchChats, fetchStats } from './api';
import { AppHeader } from './components/AppHeader';
import { ChatSidebar } from './components/ChatSidebar';
import { FacetBar } from './components/FacetBar';
import { DashboardPage } from './routes/DashboardPage';
import { MessagesPage } from './routes/MessagesPage';
import { initTheme, toggleTheme } from './theme';
import type { ChatSummary, SearchFilters } from './types';

interface AppContextValue {
  filters: SearchFilters;
  updateFilters: (patch: Partial<SearchFilters>) => void;
  chats: ChatSummary[];
}

const AppContext = createContext<AppContextValue>({
  filters: { query: '' },
  updateFilters: () => {},
  chats: [],
});

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 2,
      staleTime: 15_000,
      refetchOnWindowFocus: false,
    },
  },
});

function readInitialFilters(): SearchFilters {
  if (typeof window === 'undefined') return { query: '' };
  const params = new URLSearchParams(window.location.search);
  const mediaType = params.get('media_type') || undefined;
  const hasMediaParam = params.get('has_media');
  const isFromMeParam = params.get('is_from_me');
  return {
    query: params.get('q') || params.get('query') || '',
    source: params.get('source') || undefined,
    account: params.get('account') || undefined,
    media_type: mediaType,
    has_media: hasMediaParam !== null ? hasMediaParam === 'true' : mediaType ? true : undefined,
    is_from_me: isFromMeParam !== null ? isFromMeParam === 'true' : undefined,
  };
}

function RootLayout() {
  const navigate = useNavigate();
  const [theme, setTheme] = useState<'dark' | 'light'>(() => initTheme());
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);
  const [filters, setFilters] = useState<SearchFilters>(() => readInitialFilters());
  const [searchInput, setSearchInput] = useState(() => readInitialFilters().query);
  const [chatFilterInput, setChatFilterInput] = useState('');
  const [debouncedChatFilter, setDebouncedChatFilter] = useState('');

  useEffect(() => {
    const timer = setTimeout(() => setDebouncedChatFilter(chatFilterInput.trim()), 250);
    return () => clearTimeout(timer);
  }, [chatFilterInput]);

  const statsQuery = useQuery({
    queryKey: ['stats'],
    queryFn: () => fetchStats(),
    refetchInterval: 5000,
  });

  const accountsQuery = useQuery({
    queryKey: ['accounts'],
    queryFn: () => fetchAccounts(),
    staleTime: 60_000,
  });

  const effectiveChatQuery = debouncedChatFilter || filters.query.trim() || undefined;
  const chatsQuery = useQuery({
    queryKey: ['chats', filters.source, filters.account, effectiveChatQuery, filters.has_media, filters.media_type, filters.is_from_me],
    queryFn: ({ signal }) =>
      fetchChats(
        filters.source,
        effectiveChatQuery,
        filters.account,
        {
          has_media: filters.has_media,
          media_type: filters.media_type,
          is_from_me: filters.is_from_me,
        },
        signal,
      ),
  });

  const updateFilters = (patch: Partial<SearchFilters>) => {
    setFilters((prev) => ({ ...prev, ...patch }));
  };

  const handleSearchSubmit = () => {
    updateFilters({ query: searchInput.trim(), chat_id: undefined });
    void navigate({ to: '/' });
  };

  return (
    <AppContext.Provider value={{ filters, updateFilters, chats: chatsQuery.data ?? [] }}>
      <div className="h-dvh flex flex-col overflow-hidden bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100">
        <AppHeader
          theme={theme}
          onToggleTheme={() => setTheme(toggleTheme())}
          onToggleSidebar={() => setMobileSidebarOpen((o) => !o)}
          searchValue={searchInput}
          onSearchChange={setSearchInput}
          onSearchSubmit={handleSearchSubmit}
          ingestion={statsQuery.data?.ingestion}
        />
        <div className="flex-1 flex min-h-0 overflow-hidden">
          <ChatSidebar
            chats={chatsQuery.data ?? []}
            isLoading={chatsQuery.isLoading}
            isError={chatsQuery.isError}
            onRetry={() => void chatsQuery.refetch()}
            chatFilter={chatFilterInput}
            onChatFilterChange={setChatFilterInput}
            mobileOpen={mobileSidebarOpen}
            onCloseMobile={() => setMobileSidebarOpen(false)}
          />
          <main className="flex-1 flex flex-col min-w-0 min-h-0 overflow-hidden">
            <FacetBar
              filters={filters}
              accounts={accountsQuery.data ?? []}
              onChange={(patch) => {
                updateFilters(patch);
                void navigate({ to: '/' });
              }}
            />
            <Outlet />
          </main>
        </div>
      </div>
    </AppContext.Provider>
  );
}

function IndexRouteComponent() {
  const { filters } = useContext(AppContext);
  return <MessagesPage filters={filters} />;
}

function ChatRouteComponent() {
  const { chatId } = useParams({ from: '/chats/$chatId' });
  const { filters, chats } = useContext(AppContext);
  const activeChat = chats.find((c) => c.id === chatId);
  return <MessagesPage chatId={chatId} activeChat={activeChat} filters={filters} />;
}

const rootRoute = createRootRoute({ component: RootLayout });
const indexRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: IndexRouteComponent });
const chatRoute = createRoute({ getParentRoute: () => rootRoute, path: '/chats/$chatId', component: ChatRouteComponent });
const dashboardRoute = createRoute({ getParentRoute: () => rootRoute, path: '/dashboard', component: DashboardPage });

const routeTree = rootRoute.addChildren([indexRoute, chatRoute, dashboardRoute]);
export const router = createRouter({ routeTree });

export function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  );
}
