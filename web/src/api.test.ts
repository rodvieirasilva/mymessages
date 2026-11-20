import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fetchAccounts, fetchChatMessages, fetchChats, getAttachmentUrl, searchMessages } from './api';

describe('api module', () => {
  const originalFetch = globalThis.fetch;

  beforeEach(() => {
    globalThis.fetch = vi.fn();
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('builds attachment url', () => {
    expect(getAttachmentUrl('att_123/xyz')).toBe('/v1/attachments/att_123%2Fxyz');
  });

  it('fetches accounts list', async () => {
    (globalThis.fetch as any).mockResolvedValueOnce({
      ok: true,
      json: async () => [{ id: 'phone_a', name: 'phone_a', sources: ['whatsapp'], message_count: 3, chat_count: 1 }],
    });

    const accounts = await fetchAccounts();
    expect(globalThis.fetch).toHaveBeenCalledWith('/v1/accounts');
    expect(accounts.length).toBe(1);
    expect(accounts[0].id).toBe('phone_a');

    (globalThis.fetch as any).mockResolvedValueOnce({ ok: false, status: 500 });
    await expect(fetchAccounts()).rejects.toThrow('Failed to fetch accounts with status 500');
  });

  it('calls search endpoint with parameters including account', async () => {
    (globalThis.fetch as any).mockResolvedValueOnce({
      ok: true,
      json: async () => [{ id: 'm1' }],
    });

    const res = await searchMessages({
      query: 'hello',
      has_media: true,
      media_type: 'image',
      is_from_me: true,
      chat_id: 'chat_1',
      source: 'whatsapp',
      account: 'phone_a',
    });
    expect(globalThis.fetch).toHaveBeenCalledWith(
      '/v1/search?query=hello&source=whatsapp&account=phone_a&chat_id=chat_1&has_media=true&media_type=image&is_from_me=true',
    );
    expect(res).toEqual([{ id: 'm1' }]);
  });

  it('handles search failure', async () => {
    (globalThis.fetch as any).mockResolvedValueOnce({
      ok: false,
      status: 500,
    });

    await expect(searchMessages({ query: 'fail' })).rejects.toThrow('Search request failed with status 500');
  });

  it('fetches chats with source, account, and query filters', async () => {
    (globalThis.fetch as any).mockResolvedValueOnce({
      ok: true,
      json: async () => [{ id: 'c1' }],
    });

    const chatsAll = await fetchChats();
    expect(globalThis.fetch).toHaveBeenCalledWith('/v1/chats');
    expect(chatsAll).toEqual([{ id: 'c1' }]);

    (globalThis.fetch as any).mockResolvedValueOnce({
      ok: true,
      json: async () => [],
    });

    await fetchChats('whatsapp');
    expect(globalThis.fetch).toHaveBeenCalledWith('/v1/chats?source=whatsapp');

    (globalThis.fetch as any).mockResolvedValueOnce({
      ok: true,
      json: async () => [{ id: 'c1' }],
    });

    await fetchChats('whatsapp', 'Alice', 'phone_a');
    expect(globalThis.fetch).toHaveBeenCalledWith('/v1/chats?source=whatsapp&account=phone_a&query=Alice');
  });

  it('fetches chat timeline messages', async () => {
    (globalThis.fetch as any).mockResolvedValueOnce({
      ok: true,
      json: async () => [{ id: 'm1' }],
    });

    const msgs = await fetchChatMessages('c1', 20);
    expect(globalThis.fetch).toHaveBeenCalledWith('/v1/chats/c1/messages?limit=20');
    expect(msgs).toEqual([{ id: 'm1' }]);
  });
});
