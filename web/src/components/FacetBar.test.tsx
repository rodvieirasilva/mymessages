import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { FacetBar } from './FacetBar';

describe('FacetBar', () => {
  it('renders Account, Source (including Telegram & SMS), Media, and Dir facets and handles clicks', () => {
    const onChange = vi.fn();
    render(
      <FacetBar
        filters={{ query: '' }}
        accounts={[{ id: 'phone_a', name: 'Phone A', message_count: 10, chat_count: 2, sources: ['whatsapp', 'telegram'] }]}
        onChange={onChange}
      />,
    );

    fireEvent.click(screen.getByText('Phone A'));
    expect(onChange).toHaveBeenCalledWith({ account: 'phone_a', chat_id: undefined });

    fireEvent.click(screen.getByText('Telegram'));
    expect(onChange).toHaveBeenCalledWith({ source: 'telegram', chat_id: undefined });

    fireEvent.click(screen.getByText('Image'));
    expect(onChange).toHaveBeenCalledWith({ media_type: 'image', has_media: true });

    fireEvent.click(screen.getByText('Sent'));
    expect(onChange).toHaveBeenCalledWith({ is_from_me: true });
  });
});
