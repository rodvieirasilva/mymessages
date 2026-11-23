import type { AccountSummary, SearchFilters } from '../types';

export interface FacetBarProps {
  filters: SearchFilters;
  accounts: AccountSummary[];
  onChange: (updated: Partial<SearchFilters>) => void;
}

interface PillOption {
  label: string;
  value: string | undefined;
}

function FacetRow({
  label,
  options,
  current,
  onSelect,
}: {
  label: string;
  options: PillOption[];
  current: string | undefined;
  onSelect: (val: string | undefined) => void;
}) {
  return (
    <div className="flex items-center gap-1.5 shrink-0" data-testid="facet-group">
      <span className="text-[11px] font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
        {label}:
      </span>
      <div className="flex items-center gap-1">
        {options.map((opt) => {
          const active = opt.value === current;
          return (
            <button
              key={opt.label}
              type="button"
              data-testid="facet-pill"
              onClick={() => onSelect(opt.value)}
              className={`rounded-full px-2.5 py-0.5 text-xs font-medium transition-colors whitespace-nowrap ${
                active
                  ? 'bg-blue-600 text-white'
                  : 'bg-slate-200/80 dark:bg-slate-800 text-slate-700 dark:text-slate-300 hover:bg-slate-300 dark:hover:bg-slate-700'
              }`}
            >
              {opt.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export function FacetBar({ filters, accounts, onChange }: FacetBarProps) {
  const accountOptions: PillOption[] = [
    { label: 'All', value: undefined },
    ...accounts.map((a) => ({ label: a.name, value: a.id })),
  ];

  const sourceOptions: PillOption[] = [
    { label: 'All', value: undefined },
    { label: 'WhatsApp', value: 'whatsapp' },
    { label: 'Telegram', value: 'telegram' },
    { label: 'SMS', value: 'sms' },
    { label: 'Folder', value: 'folder' },
  ];

  const mediaOptions: PillOption[] = [
    { label: 'Any', value: undefined },
    { label: 'Image', value: 'image' },
    { label: 'Video', value: 'video' },
    { label: 'Audio', value: 'audio' },
    { label: 'Doc', value: 'document' },
  ];

  const dirOptions: PillOption[] = [
    { label: 'All', value: undefined },
    { label: 'Received', value: 'false' },
    { label: 'Sent', value: 'true' },
  ];

  return (
    <div
      data-testid="search-facets-bar"
      className="shrink-0 flex items-center gap-4 overflow-x-auto custom-scroll border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-900/60 px-3 py-2"
    >
      {accounts.length > 0 && (
        <FacetRow
          label="Account"
          options={accountOptions}
          current={filters.account}
          onSelect={(val) => onChange({ account: val, chat_id: undefined })}
        />
      )}
      <FacetRow
        label="Source"
        options={sourceOptions}
        current={filters.source}
        onSelect={(val) => onChange({ source: val, chat_id: undefined })}
      />
      <FacetRow
        label="Media"
        options={mediaOptions}
        current={filters.media_type}
        onSelect={(val) =>
          onChange({ media_type: val, has_media: val ? true : undefined })
        }
      />
      <FacetRow
        label="Dir"
        options={dirOptions}
        current={filters.is_from_me === undefined ? undefined : String(filters.is_from_me)}
        onSelect={(val) =>
          onChange({ is_from_me: val === undefined ? undefined : val === 'true' })
        }
      />
    </div>
  );
}
