import type { AccountSummary, SearchFilters } from './types';

export interface FacetOption {
  label: string;
  value: string | undefined;
}

export function renderFacetPills(
  groupName: string,
  options: FacetOption[],
  currentValue: string | undefined,
  onChange: (value: string | undefined) => void,
): HTMLElement {
  const container = document.createElement('div');
  container.className = 'facet-group';
  container.dataset.testid = 'facet-group';
  container.setAttribute('data-facet-group', groupName.toLowerCase());

  const label = document.createElement('span');
  label.className = 'facet-group-label';
  label.textContent = `${groupName}:`;
  container.appendChild(label);

  const pillsWrap = document.createElement('div');
  pillsWrap.className = 'facet-pills';

  for (const opt of options) {
    const pill = document.createElement('button');
    const isSelected = opt.value === currentValue;
    pill.className = `facet-pill ${isSelected ? 'active' : ''}`;
    pill.dataset.testid = 'facet-pill';
    pill.setAttribute('data-facet-value', opt.value || 'all');
    pill.textContent = opt.label;
    pill.type = 'button';
    pill.onclick = () => onChange(opt.value);
    pillsWrap.appendChild(pill);
  }

  container.appendChild(pillsWrap);
  return container;
}

export function renderFacetBar(
  filters: SearchFilters,
  onChange: (updated: Partial<SearchFilters>) => void,
  accounts: AccountSummary[] = [],
): HTMLElement {
  const bar = document.createElement('div');
  bar.className = 'search-facets-bar';
  bar.dataset.testid = 'search-facets-bar';

  // 1. Account facet (enables cross-account or single-account search)
  const accountOptions: FacetOption[] = [{ label: 'All Accounts', value: undefined }];
  const seenAccounts = new Set<string>();
  for (const acct of accounts) {
    seenAccounts.add(acct.id);
    accountOptions.push({ label: acct.name, value: acct.id });
  }
  if (filters.account && !seenAccounts.has(filters.account)) {
    accountOptions.push({ label: filters.account, value: filters.account });
  }
  bar.appendChild(
    renderFacetPills('Account', accountOptions, filters.account, (val) => {
      onChange({ account: val, chat_id: undefined });
    }),
  );

  // 2. Source facet
  const sourceOptions: FacetOption[] = [
    { label: 'All Sources', value: undefined },
    { label: 'WhatsApp', value: 'whatsapp' },
    { label: 'Telegram', value: 'telegram' },
    { label: 'Folder', value: 'folder' },
  ];
  const hasWaBusiness = accounts.some((a) => a.sources.includes('whatsapp_business'))
    || filters.source === 'whatsapp_business';
  if (hasWaBusiness) {
    sourceOptions.splice(2, 0, { label: 'WhatsApp Business', value: 'whatsapp_business' });
  }
  const hasSms = accounts.some((a) => a.sources.includes('sms'))
    || filters.source === 'sms';
  if (hasSms) {
    sourceOptions.push({ label: 'SMS', value: 'sms' });
  }
  bar.appendChild(
    renderFacetPills('Source', sourceOptions, filters.source, (val) => {
      onChange({ source: val });
    }),
  );

  // 3. Media Type facet
  const currentMedia = filters.media_type ?? (filters.has_media === false ? 'text' : undefined);
  const mediaOptions: FacetOption[] = [
    { label: 'All Types', value: undefined },
    { label: '📝 Text Only', value: 'text' },
    { label: '🖼️ Images', value: 'image' },
    { label: '🎙️ Audio', value: 'audio' },
    { label: '📄 Docs', value: 'document' },
    { label: '🎬 Videos', value: 'video' },
  ];
  bar.appendChild(
    renderFacetPills('Media', mediaOptions, currentMedia, (val) => {
      if (val === 'text') {
        onChange({ media_type: undefined, has_media: false });
      } else {
        onChange({ media_type: val, has_media: val ? true : undefined });
      }
    }),
  );

  // 4. Direction facet
  const currentDirection = filters.is_from_me === undefined
    ? undefined
    : filters.is_from_me ? 'outgoing' : 'incoming';

  const directionOptions: FacetOption[] = [
    { label: 'All Msgs', value: undefined },
    { label: '⬇️ Incoming', value: 'incoming' },
    { label: '⬆️ Outgoing', value: 'outgoing' },
  ];
  bar.appendChild(
    renderFacetPills('Direction', directionOptions, currentDirection, (val) => {
      const isFromMe = val === undefined ? undefined : val === 'outgoing';
      onChange({ is_from_me: isFromMe });
    }),
  );

  const hasActiveFilters = Boolean(
    filters.query.trim()
      || filters.account
      || filters.source
      || filters.chat_id
      || filters.media_type
      || filters.has_media !== undefined
      || filters.is_from_me !== undefined,
  );

  if (hasActiveFilters) {
    const clearBtn = document.createElement('button');
    clearBtn.type = 'button';
    clearBtn.className = 'facet-pill clear-filters-btn';
    clearBtn.dataset.testid = 'clear-filters-button';
    clearBtn.textContent = '✕ Clear Filters';
    clearBtn.onclick = () => {
      onChange({
        query: '',
        account: undefined,
        source: undefined,
        chat_id: undefined,
        media_type: undefined,
        has_media: undefined,
        is_from_me: undefined,
      });
    };
    bar.appendChild(clearBtn);
  }

  return bar;
}
