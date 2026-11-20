import { getAttachmentUrl } from './api';
import type { AttachmentInfo, ChatSummary, SearchResult } from './types';

export function formatDate(timestamp: number): string {
  if (!timestamp) return '';
  const d = new Date(timestamp * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function escapeHtml(str: string): string {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

export function renderSafeSnippet(container: HTMLElement, snippet: string): void {
  const escaped = escapeHtml(snippet);
  container.innerHTML = escaped.replace(/&lt;b&gt;/g, '<b>').replace(/&lt;\/b&gt;/g, '</b>');
}

export function renderMedia(att: AttachmentInfo): HTMLElement {
  const wrapper = document.createElement('div');
  wrapper.className = 'attachment-item';
  wrapper.dataset.testid = 'attachment-item';
  const url = getAttachmentUrl(att.id);

  if (att.media_type === 'image') {
    const img = document.createElement('img');
    img.src = url;
    img.alt = att.filename || 'Image';
    img.className = 'media-image';
    img.loading = 'lazy';
    wrapper.appendChild(img);
  } else if (att.media_type === 'audio') {
    const audio = document.createElement('audio');
    audio.src = url;
    audio.controls = true;
    audio.className = 'media-audio';
    wrapper.appendChild(audio);
  } else if (att.media_type === 'video') {
    const video = document.createElement('video');
    video.src = url;
    video.controls = true;
    video.className = 'media-video';
    wrapper.appendChild(video);
  } else {
    const link = document.createElement('a');
    link.href = url;
    link.download = att.filename || 'document';
    link.className = 'media-document';
    link.innerHTML = `📄 <span>${escapeHtml(att.filename || 'Download file')}</span>`;
    wrapper.appendChild(link);
  }

  return wrapper;
}

export function renderMessage(msg: SearchResult): HTMLElement {
  const card = document.createElement('div');
  card.className = `message-card ${msg.is_from_me ? 'outgoing' : 'incoming'}`;
  card.dataset.testid = 'message-card';

  const header = document.createElement('div');
  header.className = 'message-header';

  const sender = document.createElement('span');
  sender.className = 'message-sender';
  sender.dataset.testid = 'message-sender';
  sender.textContent = msg.sender_name || (msg.is_from_me ? 'Me' : 'Unknown');

  const time = document.createElement('span');
  time.className = 'message-time';
  time.textContent = formatDate(msg.timestamp);

  header.appendChild(sender);
  if (msg.account && msg.account !== 'default') {
    const badge = document.createElement('span');
    badge.className = 'account-badge';
    badge.dataset.testid = 'account-badge';
    badge.textContent = msg.account;
    header.appendChild(badge);
  }
  header.appendChild(time);
  card.appendChild(header);

  if (msg.conversation_name) {
    const chatTitle = document.createElement('div');
    chatTitle.className = 'message-chat-title';
    chatTitle.textContent = `Chat: ${msg.conversation_name}`;
    card.appendChild(chatTitle);
  }

  const content = document.createElement('div');
  content.className = 'message-body';
  content.dataset.testid = 'message-body';
  if (msg.snippet) {
    renderSafeSnippet(content, msg.snippet);
  } else if (msg.body) {
    content.textContent = msg.body;
  }
  card.appendChild(content);

  if (msg.attachments && msg.attachments.length > 0) {
    const attachmentsContainer = document.createElement('div');
    attachmentsContainer.className = 'attachments-container';
    for (const att of msg.attachments) {
      attachmentsContainer.appendChild(renderMedia(att));
    }
    card.appendChild(attachmentsContainer);
  }

  return card;
}

export function renderChatList(
  chats: ChatSummary[],
  selectedId: string | null,
  onSelect: (chat: ChatSummary) => void,
): HTMLElement {
  const list = document.createElement('div');
  list.className = 'chat-list';
  list.dataset.testid = 'chat-list';

  if (chats.length === 0) {
    const empty = document.createElement('div');
    empty.className = 'empty-state';
    empty.dataset.testid = 'chat-list-empty';
    empty.textContent = 'No conversations found';
    list.appendChild(empty);
    return list;
  }

  for (const chat of chats) {
    const item = document.createElement('div');
    item.className = `chat-item ${chat.id === selectedId ? 'active' : ''}`;
    item.dataset.testid = 'chat-item';
    item.onclick = () => onSelect(chat);

    const name = document.createElement('div');
    name.className = 'chat-item-name';
    name.dataset.testid = 'chat-item-name';
    name.textContent = chat.name || chat.remote_id;

    const meta = document.createElement('div');
    meta.className = 'chat-item-meta';
    const acctPart = chat.account_id && chat.account_id !== 'default'
      ? `<span class="account-badge" data-testid="chat-account-badge">${escapeHtml(chat.account_id)}</span>`
      : '';
    meta.innerHTML = `<span>${chat.message_count} msgs</span>${acctPart}<span>${formatDate(chat.last_timestamp || 0)}</span>`;

    item.appendChild(name);
    item.appendChild(meta);
    list.appendChild(item);
  }

  return list;
}
