import { describe, expect, it, vi } from 'vitest';
import { renderFacetBar, renderFacetPills } from './facets';

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

    const buttons = el.querySelectorAll('button');
    expect(buttons.length).toBe(2);
    expect(buttons[0].classList.contains('active')).toBe(false);
    expect(buttons[1].classList.contains('active')).toBe(true);

    buttons[0].click();
    expect(onChange).toHaveBeenCalledWith(undefined);
  });

  it('renders facet bar and handles filter changes', () => {
    const onChange = vi.fn();
    const bar = renderFacetBar(
      {
        query: 'test',
        source: 'whatsapp',
        media_type: 'image',
        is_from_me: true,
      },
      onChange,
    );

    const groups = bar.querySelectorAll('.facet-group');
    expect(groups.length).toBe(3);

    // Click on another pill (e.g. Folder in source)
    const folderPill = Array.from(bar.querySelectorAll('button')).find((b) => b.textContent === 'Folder')!;
    folderPill.click();
    expect(onChange).toHaveBeenCalledWith({ source: 'folder' });

    // Click on incoming pill
    const incomingPill = Array.from(bar.querySelectorAll('button')).find((b) => b.textContent?.includes('Incoming'))!;
    incomingPill.click();
    expect(onChange).toHaveBeenCalledWith({ is_from_me: false });
  });
});
