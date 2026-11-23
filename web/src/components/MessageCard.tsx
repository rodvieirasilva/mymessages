import { Paperclip } from 'lucide-react';
import { getAttachmentUrl } from '../api';
import { formatDate } from '../lib/format';
import type { SearchResult } from '../types';

export interface MessageCardProps {
  msg: SearchResult;
}

export function MessageCard({ msg }: MessageCardProps) {
  const senderLabel = msg.is_from_me
    ? 'Me'
    : msg.sender_name || msg.conversation_name || 'Unknown';
  const badge =
    msg.source === 'sms' ? 'SMS' : msg.source === 'telegram' ? 'TG' : null;

  return (
    <div
      data-testid="message-card"
      className={`flex flex-col max-w-[92%] sm:max-w-[78%] rounded-xl px-3.5 py-2.5 shadow-sm border ${
        msg.is_from_me
          ? 'self-end bg-blue-100 dark:bg-blue-950/80 border-blue-200 dark:border-blue-800/70'
          : 'self-start bg-white dark:bg-slate-800/90 border-slate-200 dark:border-slate-700/70'
      }`}
    >
      <div className="flex flex-wrap items-center justify-between gap-2 mb-1 text-[11px] text-slate-500 dark:text-slate-400">
        <div className="flex items-center gap-1.5 font-semibold text-blue-600 dark:text-blue-400">
          <span>{senderLabel}</span>
          {badge && (
            <span
              data-testid="source-badge"
              className="rounded bg-blue-500/15 px-1.5 py-0.2 text-[10px] font-bold uppercase text-blue-600 dark:text-blue-300"
            >
              {badge}
            </span>
          )}
        </div>
        <span data-testid="message-timestamp" className="tabular-nums">
          {formatDate(msg.timestamp)}
        </span>
      </div>

      {msg.body && (
        <div className="text-sm text-slate-900 dark:text-slate-100 whitespace-pre-wrap break-words leading-relaxed">
          {msg.body}
        </div>
      )}

      {msg.attachments && msg.attachments.length > 0 && (
        <div className="mt-2 flex flex-col gap-2">
          {msg.attachments.map((att) => {
            const url = getAttachmentUrl(att.id);
            if (att.media_type === 'image') {
              return (
                <a key={att.id} href={url} target="_blank" rel="noopener noreferrer">
                  <img
                    src={url}
                    alt={att.filename || 'image'}
                    loading="lazy"
                    className="max-h-72 max-w-full rounded-lg border border-slate-200 dark:border-slate-700 object-contain"
                  />
                </a>
              );
            }
            if (att.media_type === 'audio') {
              return (
                <audio key={att.id} controls src={url} className="max-w-full h-9" />
              );
            }
            if (att.media_type === 'video') {
              return (
                <video
                  key={att.id}
                  controls
                  src={url}
                  className="max-h-72 max-w-full rounded-lg"
                />
              );
            }
            return (
              <a
                key={att.id}
                href={url}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-1.5 rounded-lg bg-slate-100 dark:bg-slate-900/70 px-2.5 py-1.5 text-xs font-medium text-blue-600 dark:text-blue-400 hover:underline"
              >
                <Paperclip className="h-3.5 w-3.5 shrink-0" />
                <span className="truncate">{att.filename || att.mimetype}</span>
              </a>
            );
          })}
        </div>
      )}
    </div>
  );
}
