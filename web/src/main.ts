import { fetchAccounts, fetchChatMessages, fetchChats, fetchStats, searchMessages } from './api';
import { renderDashboard } from './dashboard';
import { renderFacetBar } from './facets';
import { buildAppShell } from './layout';
import { renderChatList, renderMessage } from './render';
import { initTheme, toggleTheme } from './theme';
import './style.css';

let searchAbortController: AbortController | null = null;
import type { AccountSummary, ChatSummary, SearchFilters } from './types';

let currentAccounts: AccountSummary[] = [];
let currentChats: ChatSummary[] = [];
let selectedChatId: string | null = null;
const currentFilters: SearchFilters = { query: '' };

async function init() {
  const initialTheme = initTheme();
  const app = document.querySelector<HTMLDivElement>('#app')!;
  app.innerHTML = buildAppShell(initialTheme);

  const searchInput = document.getElementById('searchInput') as HTMLInputElement;
  const searchBtn = document.getElementById('searchBtn') as HTMLButtonElement;
  const mobileSidebarBtn = document.getElementById('mobileSidebarBtn') as HTMLButtonElement | null;
  const sidebarEl = document.querySelector('.sidebar') as HTMLElement | null;
  if (mobileSidebarBtn && sidebarEl) {
    mobileSidebarBtn.onclick = () => {
      sidebarEl.classList.toggle('mobile-open');
    };
  }

  const chatSearchInput = document.getElementById('chatSearchInput') as HTMLInputElement | null;
  if (chatSearchInput) {
    let chatSearchTimer: ReturnType<typeof setTimeout> | null = null;
    chatSearchInput.oninput = () => {
      if (chatSearchTimer) clearTimeout(chatSearchTimer);
      chatSearchTimer = setTimeout(() => {
        void loadChats();
      }, 250);
    };
  }

  const themeToggleBtn = document.getElementById('themeToggleBtn') as HTMLButtonElement;
  const syncIndicatorBtn = document.getElementById('syncIndicatorBtn') as HTMLButtonElement | null;
  const dashboardToggleBtn = document.getElementById('dashboardToggleBtn') as HTMLButtonElement;
  const appMain = document.querySelector('.app-main') as HTMLElement | null;
  const dashboardView = document.getElementById('dashboardView') as HTMLElement | null;

  if (dashboardToggleBtn && appMain && dashboardView) {
    let showingDashboard = false;
    const openDashboard = async () => {
      showingDashboard = true;
      appMain.style.display = 'none';
      dashboardView.style.display = 'block';
      dashboardToggleBtn.textContent = '💬 Messages';
      dashboardView.innerHTML = '<div class="empty-state">Loading vault stats...</div>';
      try {
        const stats = await fetchStats();
        renderDashboard(dashboardView, stats);
      } catch (err) {
        dashboardView.innerHTML = `<div class="empty-state">Failed to load stats: ${String(err)}</div>`;
      }
    };

    dashboardToggleBtn.onclick = async () => {
      if (!showingDashboard) {
        await openDashboard();
      } else {
        showingDashboard = false;
        dashboardView.style.display = 'none';
        appMain.style.display = 'flex';
        dashboardToggleBtn.textContent = '📊 Dashboard';
      }
    };

    if (syncIndicatorBtn) {
      syncIndicatorBtn.onclick = () => {
        if (!showingDashboard) {
          void openDashboard();
        }
      };
    }

    const pollSyncStatus = async () => {
      try {
        const stats = await fetchStats();
        const ing = stats.ingestion;
        if (syncIndicatorBtn) {
          if (ing?.active) {
            const pct = ing.scanned_total > 0 ? Math.min(100, Math.round((ing.processed / ing.scanned_total) * 100)) : 0;
            syncIndicatorBtn.textContent = ing.scanned_total > 0 ? `⏳ Syncing ${pct}%` : '⏳ Syncing...';
            syncIndicatorBtn.style.display = 'inline-flex';
          } else {
            syncIndicatorBtn.style.display = 'none';
          }
        }
        if (showingDashboard) {
          renderDashboard(dashboardView, stats);
        }
      } catch {
        // Ignore transient poll errors
      }
    };
    void pollSyncStatus();
    setInterval(() => void pollSyncStatus(), 5000);
  }

  if (themeToggleBtn) {
    themeToggleBtn.onclick = () => {
      const next = toggleTheme();
      themeToggleBtn.textContent = next === 'dark' ? '🌙' : '☀️';
    };
  }

  searchBtn.onclick = () => {
    currentFilters.query = searchInput.value;
    currentFilters.chat_id = undefined;
    selectedChatId = null;
    renderChats();
    runSearch();
  };

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  searchInput.oninput = () => {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      currentFilters.query = searchInput.value;
      currentFilters.chat_id = undefined;
      selectedChatId = null;
      renderChats();
      runSearch();
    }, 200);
  };

  searchInput.onkeydown = (e) => {
    if (e.key === 'Enter') {
      if (debounceTimer) clearTimeout(debounceTimer);
      currentFilters.query = searchInput.value;
      currentFilters.chat_id = undefined;
      selectedChatId = null;
      renderChats();
      runSearch();
    }
  };

  const urlParams = new URLSearchParams(window.location.search);
  const qParam = urlParams.get('q') || urlParams.get('query');
  if (qParam) {
    searchInput.value = qParam;
    currentFilters.query = qParam;
  }
  const mediaParam = urlParams.get('media_type');
  if (mediaParam) {
    currentFilters.media_type = mediaParam;
    currentFilters.has_media = true;
  }
  const accountParam = urlParams.get('account');
  if (accountParam) {
    currentFilters.account = accountParam;
  }

  await loadAccounts();
  renderFacets();
  await loadChats();
  await runSearch();
}

async function loadAccounts() {
  try {
    currentAccounts = await fetchAccounts();
  } catch {
    currentAccounts = [];
  }
}

function renderFacets() {
  const container = document.getElementById('facetBarContainer')!;
  container.innerHTML = '';
  container.appendChild(
    renderFacetBar(
      currentFilters,
      (updated) => {
        Object.assign(currentFilters, updated);
        if (updated.query === '') {
          const searchInput = document.getElementById('searchInput') as HTMLInputElement | null;
          if (searchInput) searchInput.value = '';
        }
        if (updated.chat_id === undefined && ('account' in updated || 'query' in updated)) {
          selectedChatId = null;
        }
        renderFacets();
        loadChats();
        runSearch();
      },
      currentAccounts,
    ),
  );
}

async function loadChats() {
  const container = document.getElementById('chatListContainer')!;
  try {
    const chatSearchInput = document.getElementById('chatSearchInput') as HTMLInputElement | null;
    const q = chatSearchInput?.value.trim() || currentFilters.query.trim();
    currentChats = await fetchChats(currentFilters.source, q || undefined, currentFilters.account, {
      has_media: currentFilters.has_media,
      media_type: currentFilters.media_type,
      is_from_me: currentFilters.is_from_me,
    });
    renderChats();
  } catch {
    container.innerHTML = `<div class="error-state">Failed to load chats</div>`;
  }
}

function renderChats() {
  const sidebarHeader = document.getElementById('sidebarHeader');
  if (sidebarHeader) {
    sidebarHeader.textContent = currentChats.length > 0 ? `Conversations (${currentChats.length})` : 'Conversations';
  }
  const container = document.getElementById('chatListContainer')!;
  container.innerHTML = '';
  container.appendChild(
    renderChatList(currentChats, selectedChatId, async (chat) => {
      selectedChatId = chat.id;
      currentFilters.chat_id = chat.id;
      renderChats();
      const hasMsgFilters = Boolean(
        currentFilters.query.trim()
          || currentFilters.media_type
          || currentFilters.has_media !== undefined
          || currentFilters.is_from_me !== undefined,
      );
      if (hasMsgFilters) {
        await runSearch();
      } else {
        await loadChatTimeline(chat);
      }
    }),
  );
}

async function loadChatTimeline(chat: ChatSummary) {
  const header = document.getElementById('contentHeader')!;
  const container = document.getElementById('messagesContainer')!;
  header.textContent = chat.name || chat.remote_id;
  container.innerHTML = '<div class="empty-state">Loading timeline...</div>';
  container.onscroll = null;

  try {
    const messages = await fetchChatMessages(chat.id, 50);
    container.innerHTML = '';
    if (messages.length === 0) {
      container.innerHTML = '<div class="empty-state">No messages in this chat.</div>';
      return;
    }

    const chronological = [...messages].reverse();
    let oldestTimestamp = chronological[0].timestamp;
    let hasMoreOlder = messages.length === 50;
    let loadingOlder = false;

    for (const msg of chronological) {
      container.appendChild(renderMessage(msg));
    }
    container.scrollTop = container.scrollHeight;

    container.onscroll = async () => {
      if (selectedChatId !== chat.id || !hasMoreOlder || loadingOlder || container.scrollTop > 80) {
        return;
      }
      loadingOlder = true;
      try {
        const older = await fetchChatMessages(chat.id, 50, oldestTimestamp);
        if (older.length < 50) {
          hasMoreOlder = false;
        }
        if (older.length > 0 && selectedChatId === chat.id) {
          const olderChrono = [...older].reverse();
          oldestTimestamp = olderChrono[0].timestamp;
          const prevHeight = container.scrollHeight;
          const frag = document.createDocumentFragment();
          for (const msg of olderChrono) {
            frag.appendChild(renderMessage(msg));
          }
          container.insertBefore(frag, container.firstChild);
          container.scrollTop += container.scrollHeight - prevHeight;
        }
      } finally {
        loadingOlder = false;
      }
    };
  } catch {
    container.innerHTML = `<div class="error-state">Failed to load messages</div>`;
  }
}

async function runSearch() {
  if (!currentFilters.chat_id) {
    loadChats();
  }
  const header = document.getElementById('contentHeader')!;
  const container = document.getElementById('messagesContainer')!;
  const q = currentFilters.query.trim();
  header.textContent = q ? `Search results for "${q}"` : 'Recent Messages';
  container.innerHTML = '<div class="empty-state">Searching...</div>';

  if (searchAbortController) {
    searchAbortController.abort();
  }
  searchAbortController = new AbortController();
  const { signal } = searchAbortController;

  try {
    const results = await searchMessages(currentFilters, signal);
    container.innerHTML = '';
    if (results.length === 0) {
      container.innerHTML = '<div class="empty-state">No messages match your search and facet filters.</div>';
      return;
    }
    for (const msg of results) {
      container.appendChild(renderMessage(msg));
    }
  } catch {
    container.innerHTML = `<div class="error-state">Search error</div>`;
  }
}

init();
