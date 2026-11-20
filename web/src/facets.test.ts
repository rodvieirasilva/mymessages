import { describe, expect, it, vi } from 'vitest';
import { renderFacetBar, renderFacetPills } from './facets';
import type { AccountSummary } from './types';

describe('facets module', () => {
  it('renders facet group with active pill', () => {
    const onChange = vi.fn();
    const el = renderFacetPills(
      'Type',
      [
        { label: 'All', value: undefined },
        { label: 'Images', value: 'image' },
      ],
      'image',
      onChange,
    );

    expect(el.dataset.testid).toBe('facet-group');
    const buttons = el.querySelectorAll('button');
    expect(buttons.length).toBe(2);
    expect(buttons[0].dataset.testid).toBe('facet-pill');
    expect(buttons[0].classList.contains('active')).toBe(false);
    expect(buttons[1].classList.contains('active')).toBe(true);

    buttons[0].click();
    expect(onChange).toHaveBeenCalledWith(undefined);
  });

  it('renders facet bar with Account facet and handles filter changes', () => {
    const onChange = vi.fn();
    const accounts: AccountSummary[] = [
      { id: 'phone_a', name: 'phone_a', sources: ['whatsapp', 'sms'], message_count: 10, chat_count: 3 },
      { id: 'phone_b', name: 'phone_b', sources: ['whatsapp', 'whatsapp_business'], message_count: 5, chat_count: 2 },
    ];
    const bar = renderFacetBar(
      {
        query: 'test',
        source: 'whatsapp',
        account: 'phone_a',
        media_type: 'image',
        is_from_me: true,
      },
      onChange,
      accounts,
    );
    expect(bar.dataset.testid).toBe('search-facets-bar');

    const groups = bar.querySelectorAll('.facet-group');
    expect(groups.length).toBe(4);

    // Click on Account pill (phone_b)
    const acctBPill = Array.from(bar.querySelectorAll('button')).find((b) => b.textContent === 'phone_b')!;
    acctBPill.click();
    expect(onChange).toHaveBeenCalledWith({ account: 'phone_b', chat_id: undefined });

    // Click on WhatsApp Business source pill
    const waBizPill = Array.from(bar.querySelectorAll('button')).find((b) => b.textContent === 'WhatsApp Business')!;
    waBizPill.click();
    expect(onChange).toHaveBeenCalledWith({ source: 'whatsapp_business' });

    // Click on Folder in source
    const folderPill = Array.from(bar.querySelectorAll('button')).find((b) => b.textContent === 'Folder')!;
    folderPill.click();
    expect(onChange).toHaveBeenCalledWith({ source: 'folder' });

    // Click on incoming pill
    const incomingPill = Array.from(bar.querySelectorAll('button')).find((b) => b.textContent?.includes('Incoming'))!;
    incomingPill.click();
    expect(onChange).toHaveBeenCalledWith({ is_from_me: false });
  });

  it('preserves active account pill when not in accounts list', () => {
    const onChange = vi.fn();
    const bar = renderFacetBar({ query: '', account: 'custom_acct' }, onChange, []);
    const customPill = Array.from(bar.querySelectorAll('button')).find((b) => b.textContent === 'custom_acct');
    expect(customPill).toBeDefined();
    expect(customPill?.classList.contains('active')).toBe(true);
  });
});
