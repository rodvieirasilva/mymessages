import type { ChatSummary, SearchFilters, SearchResult } from './types';

const BASE_URL = '/v1';

export function getAttachmentUrl(id: string): string {
  return `${BASE_URL}/attachments/${encodeURIComponent(id)}`;
}

export async function searchMessages(filters: SearchFilters): Promise<SearchResult[]> {
  const params = new URLSearchParams();
  if (filters.query) params.set('query', filters.query);
  if (filters.source) params.set('source', filters.source);
  if (filters.chat_id) params.set('chat_id', filters.chat_id);
  if (filters.has_media !== undefined) params.set('has_media', String(filters.has_media));
  if (filters.media_type) params.set('media_type', filters.media_type);
  if (filters.is_from_me !== undefined) params.set('is_from_me', String(filters.is_from_me));

  const res = await fetch(`${BASE_URL}/search?${params.toString()}`);
  if (!res.ok) {
    throw new Error(`Search request failed with status ${res.status}`);
  }
  return res.json();
}

export async function fetchChats(source?: string, query?: string): Promise<ChatSummary[]> {
  const params = new URLSearchParams();
  if (source) params.set('source', source);
  if (query) params.set('query', query);
  const qs = params.toString();
  const url = qs ? `${BASE_URL}/chats?${qs}` : `${BASE_URL}/chats`;
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`Failed to fetch chats with status ${res.status}`);
  }
  return res.json();
}

export async function fetchChatMessages(chatId: string, limit = 50): Promise<SearchResult[]> {
  const res = await fetch(`${BASE_URL}/chats/${encodeURIComponent(chatId)}/messages?limit=${limit}`);
  if (!res.ok) {
    throw new Error(`Failed to fetch chat messages with status ${res.status}`);
  }
  return res.json();
}
