export function buildAppShell(initialTheme: 'dark' | 'light'): string {
  const ver = typeof __APP_VERSION__ !== 'undefined' ? __APP_VERSION__ : '0.2.0';
  const built = typeof __BUILD_TIME__ !== 'undefined' ? __BUILD_TIME__ : '';
  const sha = typeof __GIT_SHA__ !== 'undefined' ? __GIT_SHA__ : '';
  const versionLabel = built && sha ? `v${ver} (${built} · ${sha})` : `v${ver}`;

  return `
    <header class="app-header">
      <div class="logo">
        <button id="mobileSidebarBtn" class="mobile-sidebar-btn" data-testid="mobile-sidebar-button" title="Toggle conversations" aria-label="Toggle conversations">☰</button>
        <img src="/logo.svg" alt="mymessages" class="brand-logo" />
        <span>mymessages</span>
        <span class="version-badge" data-testid="version-badge">${versionLabel}</span>
      </div>
      <div class="search-bar-container">
        <input type="text" id="searchInput" data-testid="search-input" placeholder="Search text or Lucene (e.g. from:Alice account:phone_a after:2024-01-01)..." />
        <button id="searchBtn" data-testid="search-button">Search</button>
      </div>
      <div class="header-actions">
        <button id="syncIndicatorBtn" class="sync-indicator" data-testid="sync-indicator" style="display: none;" title="Indexing in progress — click for Dashboard">
          ⏳ Syncing...
        </button>
        <button id="dashboardToggleBtn" class="theme-toggle-btn" data-testid="dashboard-toggle-button" title="Toggle Dashboard">
          📊 Dashboard
        </button>
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
        <div class="sidebar-search">
          <input type="text" id="chatSearchInput" data-testid="chat-search-input" placeholder="Filter chats (e.g. alice*)..." />
        </div>
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
    <section id="dashboardView" class="dashboard-view" data-testid="dashboard-view" style="display: none;"></section>
  `;
}
