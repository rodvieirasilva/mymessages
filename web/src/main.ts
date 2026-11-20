import { fetchAccounts, fetchChatMessages, fetchChats, searchMessages } from './api';
import { renderFacetBar } from './facets';
import { buildAppShell } from './layout';
import { renderChatList, renderMessage } from './render';
import { initTheme, toggleTheme } from './theme';
import './style.css';
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
  const themeToggleBtn = document.getElementById('themeToggleBtn') as HTMLButtonElement;

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
    const q = currentFilters.query.trim();
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

  try {
    const messages = await fetchChatMessages(chat.id);
    container.innerHTML = '';
    if (messages.length === 0) {
      container.innerHTML = '<div class="empty-state">No messages in this chat.</div>';
      return;
    }
    for (const msg of messages) {
      container.appendChild(renderMessage(msg));
    }
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

  try {
    const results = await searchMessages(currentFilters);
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
