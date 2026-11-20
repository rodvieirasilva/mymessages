export function buildAppShell(initialTheme: 'dark' | 'light'): string {
  return `
    <header class="app-header">
      <div class="logo">
        <img src="/logo.svg" alt="mymessages" class="brand-logo" />
        <span>mymessages</span>
        <span class="version-badge">v0.2.0</span>
      </div>
      <div class="search-bar-container">
        <input type="text" id="searchInput" data-testid="search-input" placeholder="Search text or Lucene (e.g. from:Alice account:phone_a after:2024-01-01)..." />
        <button id="searchBtn" data-testid="search-button">Search</button>
      </div>
      <div class="header-actions">
        <button id="themeToggleBtn" class="theme-toggle-btn" data-testid="theme-toggle-button" title="Toggle dark/light theme" aria-label="Toggle theme">
          ${initialTheme === 'dark' ? '🌙' : '☀️'}
        </button>
        <a href="/swagger-ui/" target="_blank" rel="noopener" class="swagger-link" data-testid="swagger-link" title="OpenAPI & Swagger Documentation">
          Swagger API &rarr;
        </a>
      </div>
    </header>
    <main class="app-main">
      <aside class="sidebar">
        <div class="sidebar-header" id="sidebarHeader" data-testid="sidebar-header">Conversations</div>
        <div id="chatListContainer" data-testid="chat-list-container">Loading chats...</div>
      </aside>
      <section class="content-area">
        <div class="content-header" id="contentHeader" data-testid="content-header">All Messages</div>
        <div id="facetBarContainer" data-testid="facet-bar-container"></div>
        <div class="messages-container" id="messagesContainer" data-testid="messages-container">
          <div class="empty-state" data-testid="empty-state">Use search or select a chat from the left to view history.</div>
        </div>
      </section>
    </main>
  `;
}
