import { fetchChatMessages, fetchChats, searchMessages } from './api';
import { renderFacetBar } from './facets';
import { renderChatList, renderMessage } from './render';
import { initTheme, toggleTheme } from './theme';
import './style.css';
import type { ChatSummary, SearchFilters } from './types';

let currentChats: ChatSummary[] = [];
let selectedChatId: string | null = null;
const currentFilters: SearchFilters = { query: '' };

async function init() {
  const initialTheme = initTheme();
  const app = document.querySelector<HTMLDivElement>('#app')!;
  app.innerHTML = `
    <header class="app-header">
      <div class="logo">
        <img src="/logo.svg" alt="mymessages" class="brand-logo" />
        <span>mymessages</span>
        <span class="version-badge">v0.1.0</span>
      </div>
      <div class="search-bar-container">
        <input type="text" id="searchInput" placeholder="Search text or Lucene (e.g. from:Alice after:2024-01-01)..." />
        <button id="searchBtn">Search</button>
      </div>
      <div class="header-actions">
        <button id="themeToggleBtn" class="theme-toggle-btn" title="Toggle dark/light theme" aria-label="Toggle theme">
          ${initialTheme === 'dark' ? '🌙' : '☀️'}
        </button>
        <a href="/swagger-ui/" target="_blank" rel="noopener" class="swagger-link" title="OpenAPI & Swagger Documentation">
          Swagger API &rarr;
        </a>
      </div>
    </header>
    <main class="app-main">
      <aside class="sidebar">
        <div class="sidebar-header">Conversations</div>
        <div id="chatListContainer">Loading chats...</div>
      </aside>
      <section class="content-area">
        <div class="content-header" id="contentHeader">All Messages</div>
        <div id="facetBarContainer"></div>
        <div class="messages-container" id="messagesContainer">
          <div class="empty-state">Use search or select a chat from the left to view history.</div>
        </div>
      </section>
    </main>
  `;

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

  searchInput.onkeydown = (e) => {
    if (e.key === 'Enter') {
      currentFilters.query = searchInput.value;
      currentFilters.chat_id = undefined;
      selectedChatId = null;
      renderChats();
      runSearch();
    }
  };

  renderFacets();
  await loadChats();
  await runSearch();
}

function renderFacets() {
  const container = document.getElementById('facetBarContainer')!;
  container.innerHTML = '';
  container.appendChild(
    renderFacetBar(currentFilters, (updated) => {
      Object.assign(currentFilters, updated);
      renderFacets();
      runSearch();
    }),
  );
}

async function loadChats() {
  const container = document.getElementById('chatListContainer')!;
  try {
    currentChats = await fetchChats();
    renderChats();
  } catch (e) {
    container.innerHTML = `<div class="error-state">Failed to load chats</div>`;
  }
}

function renderChats() {
  const container = document.getElementById('chatListContainer')!;
  container.innerHTML = '';
  container.appendChild(
    renderChatList(currentChats, selectedChatId, async (chat) => {
      selectedChatId = chat.id;
      currentFilters.chat_id = chat.id;
      renderChats();
      await loadChatTimeline(chat);
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
  } catch (e) {
    container.innerHTML = `<div class="error-state">Failed to load messages</div>`;
  }
}

async function runSearch() {
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
  } catch (e) {
    container.innerHTML = `<div class="error-state">Search error</div>`;
  }
}

init();
