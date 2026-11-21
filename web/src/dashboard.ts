import type { VaultStats } from './api';

export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const val = bytes / Math.pow(1024, i);
  return `${val.toFixed(i === 0 ? 0 : 2)} ${units[i]}`;
}

export function renderDashboard(container: HTMLElement, stats: VaultStats): void {
  const sourceRows =
    stats.sources.length === 0
      ? '<tr><td colspan="3" class="dashboard-empty">No messages indexed yet.</td></tr>'
      : stats.sources
          .map(
            (s) => `
          <tr>
            <td><span class="source-pill">${s.source}</span></td>
            <td>${s.message_count.toLocaleString()}</td>
            <td>${s.chat_count.toLocaleString()}</td>
          </tr>`
          )
          .join('');

  container.innerHTML = `
    <div class="dashboard-inner">
      <div class="dashboard-header">
        <h2>Vault Dashboard</h2>
        <span class="dashboard-version">v${stats.version} · ${stats.commit} (${stats.build_time})</span>
      </div>
      <div class="dashboard-cards">
        <div class="dashboard-card">
          <div class="dashboard-card-label">Database Size</div>
          <div class="dashboard-card-value" data-testid="stat-db-size">${formatBytes(stats.db_size_bytes)}</div>
          <div class="dashboard-card-sub">${stats.db_path}</div>
        </div>
        <div class="dashboard-card">
          <div class="dashboard-card-label">Total Messages</div>
          <div class="dashboard-card-value" data-testid="stat-total-messages">${stats.total_messages.toLocaleString()}</div>
        </div>
        <div class="dashboard-card">
          <div class="dashboard-card-label">Total Chats</div>
          <div class="dashboard-card-value" data-testid="stat-total-chats">${stats.total_chats.toLocaleString()}</div>
        </div>
      </div>
      <div class="dashboard-section">
        <h3>Messages by Source</h3>
        <table class="dashboard-table">
          <thead>
            <tr>
              <th>Source</th>
              <th>Messages</th>
              <th>Chats</th>
            </tr>
          </thead>
          <tbody>
            ${sourceRows}
          </tbody>
        </table>
      </div>
    </div>
  `;
}
