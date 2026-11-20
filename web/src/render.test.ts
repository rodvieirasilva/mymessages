import { describe, expect, it, vi } from 'vitest';
import { escapeHtml, formatDate, renderChatList, renderMedia, renderMessage } from './render';
import type { AttachmentInfo, ChatSummary, SearchResult } from './types';

describe('render helpers', () => {
  it('formats timestamp correctly', () => {
    expect(formatDate(0)).toBe('');
    const formatted = formatDate(1700000000);
    expect(formatted.length).toBeGreaterThan(0);
  });

  it('escapes html entities', () => {
    expect(escapeHtml('<script>alert("xss")</script>')).toBe('&lt;script&gt;alert("xss")&lt;/script&gt;');
  });

  it('renders image attachment correctly', () => {
    const att: AttachmentInfo = {
      id: 'att_1',
      filename: 'photo.jpg',
      mimetype: 'image/jpeg',
      file_size: 100,
      media_type: 'image',
    };
    const el = renderMedia(att);
    expect(el.dataset.testid).toBe('attachment-item');
    const img = el.querySelector('img');
    expect(img).not.toBeNull();
    expect(img?.src).toContain('/v1/attachments/att_1');
  });

  it('renders audio attachment correctly', () => {
    const att: AttachmentInfo = {
      id: 'att_2',
      filename: 'audio.ogg',
      mimetype: 'audio/ogg',
      file_size: 200,
      media_type: 'audio',
    };
    const el = renderMedia(att);
    const audio = el.querySelector('audio');
    expect(audio).not.toBeNull();
    expect(audio?.src).toContain('/v1/attachments/att_2');
  });

  it('renders video attachment correctly', () => {
    const att: AttachmentInfo = {
      id: 'att_3',
      filename: 'video.mp4',
      mimetype: 'video/mp4',
      file_size: 300,
      media_type: 'video',
    };
    const el = renderMedia(att);
    const video = el.querySelector('video');
    expect(video).not.toBeNull();
  });

  it('renders document attachment correctly', () => {
    const att: AttachmentInfo = {
      id: 'att_4',
      filename: 'doc.pdf',
      mimetype: 'application/pdf',
      file_size: 400,
      media_type: 'document',
    };
    const el = renderMedia(att);
    const link = el.querySelector('a');
    expect(link).not.toBeNull();
    expect(link?.textContent).toContain('doc.pdf');
  });

  it('renders incoming and outgoing messages with account badge', () => {
    const msg: SearchResult = {
      id: 'm1',
      source: 'whatsapp',
      account: 'phone_a',
      conversation_id: 'c1',
      conversation_name: 'Work',
      sender_name: 'Bob',
      timestamp: 1700000000,
      body: 'Hello team',
      eml_path: 'path.eml',
      has_attachments: false,
      is_from_me: false,
      snippet: null,
      attachments: [],
    };

    const el = renderMessage(msg);
    expect(el.dataset.testid).toBe('message-card');
    expect(el.querySelector('[data-testid="message-sender"]')?.textContent).toBe('Bob');
    expect(el.querySelector('[data-testid="account-badge"]')?.textContent).toBe('phone_a');
    expect(el.classList.contains('incoming')).toBe(true);
    expect(el.textContent).toContain('Bob');
    expect(el.textContent).toContain('Hello team');

    const outgoingMsg: SearchResult = {
      ...msg,
      account: 'default',
      is_from_me: true,
      snippet: '<b>Matched</b> snippet',
    };
    const outEl = renderMessage(outgoingMsg);
    expect(outEl.dataset.testid).toBe('message-card');
    expect(outEl.querySelector('[data-testid="account-badge"]')).toBeNull();
    expect(outEl.classList.contains('outgoing')).toBe(true);
    expect(outEl.innerHTML).toContain('<b>Matched</b> snippet');
  });

  it('escapes malicious HTML in snippet while preserving bold highlight (XSS prevention)', () => {
    const maliciousMsg: SearchResult = {
      id: 'xss_msg',
      source: 'whatsapp',
      account: 'default',
      conversation_id: 'c1',
      conversation_name: null,
      sender_name: 'Attacker',
      timestamp: 1700000000,
      body: '<script>alert(1)</script> <img src=x onerror=alert(2)> contract',
      eml_path: 'path',
      has_attachments: false,
      is_from_me: false,
      snippet: '<script>alert(1)</script> <img src=x onerror=alert(2)> <b>contract</b>',
      attachments: [],
    };
    const el = renderMessage(maliciousMsg);
    const bodyEl = el.querySelector('.message-body') as HTMLElement;
    expect(bodyEl.querySelector('script')).toBeNull();
    expect(bodyEl.querySelector('img')).toBeNull();
    expect(bodyEl.innerHTML).toContain('&lt;script&gt;alert(1)&lt;/script&gt;');
    expect(bodyEl.innerHTML).toContain('<b>contract</b>');
  });

  it('renders chat list correctly', () => {
    const onSelect = vi.fn();
    const emptyEl = renderChatList([], null, onSelect);
    expect(emptyEl.textContent).toContain('No conversations found');

    const chats: ChatSummary[] = [
      {
        id: 'c1',
        source_id: 'whatsapp',
        account_id: 'phone_a',
        remote_id: '123@s.whatsapp.net',
        name: 'Alice',
        is_group: false,
        message_count: 5,
        last_message: 'Hi',
        last_timestamp: 1700000000,
      },
    ];

    const listEl = renderChatList(chats, 'c1', onSelect);
    expect(listEl.dataset.testid).toBe('chat-list');
    const item = listEl.querySelector('[data-testid="chat-item"]') as HTMLElement;
    expect(item).not.toBeNull();
    expect(item.classList.contains('active')).toBe(true);
    expect(item.querySelector('[data-testid="chat-item-name"]')?.textContent).toBe('Alice');
    expect(item.querySelector('[data-testid="chat-account-badge"]')?.textContent).toBe('phone_a');
    item.click();
    expect(onSelect).toHaveBeenCalledWith(chats[0]);
  });
});
