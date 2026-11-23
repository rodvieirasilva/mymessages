import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { MessageCard } from './MessageCard';

afterEach(cleanup);

describe('MessageCard', () => {
  it('renders incoming SMS message with badge and timestamp', () => {
    render(
      <MessageCard
        msg={{
          id: 'm1',
          source: 'sms',
          account: 'phone_a',
          conversation_id: 'c1',
          conversation_name: 'Alice',
          sender_name: 'Alice',
          timestamp: 1700000000,
          body: 'Hello via SMS',
          is_from_me: false,
          has_attachments: false,
          eml_path: '',
          snippet: '',
          attachments: [],
        }}
      />,
    );
    expect(screen.getByText('Hello via SMS')).toBeTruthy();
    expect(screen.getByTestId('source-badge').textContent).toBe('SMS');
    expect(screen.getByTestId('message-timestamp').textContent).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
  });

  it('renders outgoing Telegram message with image, audio, video, and document attachments', () => {
    render(
      <MessageCard
        msg={{
          id: 'm2',
          source: 'telegram',
          account: 'phone_a',
          conversation_id: 'c2',
          conversation_name: 'Bob',
          sender_name: 'Me',
          timestamp: 1700000100,
          body: 'Check these files',
          is_from_me: true,
          has_attachments: true,
          eml_path: '',
          snippet: '',
          attachments: [
            { id: 'a1', filename: 'pic.jpg', mimetype: 'image/jpeg', media_type: 'image', file_size: 100 },
            { id: 'a2', filename: 'voice.ogg', mimetype: 'audio/ogg', media_type: 'audio', file_size: 200 },
            { id: 'a3', filename: 'clip.mp4', mimetype: 'video/mp4', media_type: 'video', file_size: 300 },
            { id: 'a4', filename: 'doc.pdf', mimetype: 'application/pdf', media_type: 'document', file_size: 400 },
          ],
        }}
      />,
    );
    expect(screen.getByTestId('source-badge').textContent).toBe('TG');
    expect(screen.getByAltText('pic.jpg')).toBeTruthy();
    expect(screen.getByText('doc.pdf')).toBeTruthy();
  });
});
