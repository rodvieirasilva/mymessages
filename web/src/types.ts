export interface AttachmentInfo {
  id: string;
  filename: string | null;
  mimetype: string;
  file_size: number;
  media_type: string;
}

export interface SearchResult {
  id: string;
  source: string;
  conversation_id: string;
  conversation_name: string | null;
  sender_name: string | null;
  timestamp: number;
  body: string | null;
  eml_path: string;
  has_attachments: boolean;
  is_from_me: boolean;
  snippet: string | null;
  attachments: AttachmentInfo[];
}

export interface ChatSummary {
  id: string;
  source_id: string;
  remote_id: string;
  name: string | null;
  is_group: boolean;
  message_count: number;
  last_message: string | null;
  last_timestamp: number | null;
}

export interface SearchFilters {
  query: string;
  source?: string;
  chat_id?: string;
  has_media?: boolean;
  media_type?: string;
  is_from_me?: boolean;
}
