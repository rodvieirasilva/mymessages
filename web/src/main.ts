import { fetchChatMessages, fetchChats, searchMessages } from './api';
import { renderChatList, renderMessage } from './render';
import './style.css';
import type { ChatSummary } from './types';

let currentChats: ChatSummary[] = [];
let selectedChatId: string | null = null;

async function init() {
  const app = document.querySelector<HTMLDivElement>('#app')!;
  app.innerHTML = `
    <header class="app-header">
      <div class="logo">💬 <span>mymessages</span></div>
      <div class="search-bar-container">
        <input type="text" id="searchInput" placeholder="Search all messages, senders, files..." />
        <button id="searchBtn">Search</button>
      </div>
    </header>
    <main class="app-main">
      <aside class="sidebar">
        <div class="sidebar-header">Conversations</div>
        <div id="chatListContainer">Loading chats...</div>
      </aside>
      <section class="content-area">
        <div class="content-header" id="contentHeader">All Messages</div>
        <div class="messages-container" id="messagesContainer">
          <div class="empty-state">Use search or select a chat from the left to view history.</div>
        </div>
      </section>
    </main>
  `;

  const searchInput = document.getElementById('searchInput') as HTMLInputElement;
  const searchBtn = document.getElementById('searchBtn') as HTMLButtonElement;

  searchBtn.onclick = () => doSearch(searchInput.value);
  searchInput.onkeydown = (e) => {
    if (e.key === 'Enter') doSearch(searchInput.value);
  };

  await loadChats();
  await doSearch('');
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

async function doSearch(query: string) {
  const header = document.getElementById('contentHeader')!;
  const container = document.getElementById('messagesContainer')!;
  header.textContent = query.trim() ? `Search results for "${query}"` : 'Recent Messages';
  container.innerHTML = '<div class="empty-state">Searching...</div>';

  try {
    const results = await searchMessages({ query });
    container.innerHTML = '';
    if (results.length === 0) {
      container.innerHTML = '<div class="empty-state">No messages found.</div>';
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
