export function formatDate(ts: number): string {
  const date = new Date(ts * 1000);
  const yyyy = date.getFullYear();
  const mm = String(date.getMonth() + 1).padStart(2, '0');
  const dd = String(date.getDate()).padStart(2, '0');
  const hh = String(date.getHours()).padStart(2, '0');
  const min = String(date.getMinutes()).padStart(2, '0');
  const sec = String(date.getSeconds()).padStart(2, '0');
  return `${yyyy}-${mm}-${dd} ${hh}:${min}:${sec}`;
}

export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const val = bytes / Math.pow(1024, i);
  return `${val.toFixed(i === 0 ? 0 : 2)} ${units[i]}`;
}

export function getVersionLabel(): string {
  const ver = typeof __APP_VERSION__ !== 'undefined' ? __APP_VERSION__ : '0.2.0';
  const built = typeof __BUILD_TIME__ !== 'undefined' ? __BUILD_TIME__ : '';
  const sha = typeof __GIT_SHA__ !== 'undefined' ? __GIT_SHA__ : '';
  return built && sha ? `v${ver} (${built} · ${sha})` : `v${ver}`;
}
