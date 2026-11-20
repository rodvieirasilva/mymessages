#!/usr/bin/env node
import { spawn, execSync } from 'node:child_process';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(__dirname, '..');
const WEB_DIST = path.join(ROOT_DIR, 'web', 'dist');
const OUTPUT_DIR = path.join(ROOT_DIR, 'docs', 'assets');
const FRAMES_DIR = path.join(ROOT_DIR, 'target', 'gif_frames');
const PORT = 8798;

if (!fs.existsSync(OUTPUT_DIR)) fs.mkdirSync(OUTPUT_DIR, { recursive: true });
if (!fs.existsSync(FRAMES_DIR)) fs.mkdirSync(FRAMES_DIR, { recursive: true });

const SAMPLE_ACCOUNTS = [
  { id: 'personal', name: 'personal', sources: ['whatsapp'], message_count: 198, chat_count: 2 },
  { id: 'work', name: 'work', sources: ['whatsapp', 'folder'], message_count: 18, chat_count: 1 },
];

const SAMPLE_CHATS = [
  { id: 'chat_1', source_id: 'whatsapp', account_id: 'personal', remote_id: '1234567890@s.whatsapp.net', name: 'Alice Smith', is_group: false, message_count: 42, last_message: 'Here is the signed agreement.', last_timestamp: 1710002400 },
  { id: 'chat_2', source_id: 'whatsapp', account_id: 'personal', remote_id: 'group_987@g.us', name: 'Engineering Core Team', is_group: true, message_count: 156, last_message: 'Release v0.2.0 is ready!', last_timestamp: 1710001800 },
  { id: 'chat_3', source_id: 'folder', account_id: 'work', remote_id: 'accounting_export', name: 'Finance & Invoices', is_group: false, message_count: 18, last_message: 'Q1 summary invoice attached.', last_timestamp: 1709998000 },
];

const ALL_MESSAGES = [
  { id: 'msg_1', source: 'whatsapp', account: 'personal', conversation_id: 'chat_1', conversation_name: 'Alice Smith', sender_name: 'Alice Smith', timestamp: 1710001000, is_from_me: false, body: 'Hi! Could you send me the latest contract for the messaging vault?', snippet: 'Hi! Could you send me the latest contract for the messaging vault?', attachments: [] },
  { id: 'msg_2', source: 'whatsapp', account: 'personal', conversation_id: 'chat_1', conversation_name: 'Alice Smith', sender_name: null, timestamp: 1710001500, is_from_me: true, body: 'Sure Alice, here is the proposal with full Maildir and SQLite FTS5 specs.', snippet: 'Sure Alice, here is the proposal with full Maildir and SQLite FTS5 specs.', attachments: [] },
  { id: 'msg_3', source: 'whatsapp', account: 'work', conversation_id: 'chat_3', conversation_name: 'Finance & Invoices', sender_name: 'Alice Smith', timestamp: 1710002400, is_from_me: false, body: 'Thank you! I have reviewed and signed the contract.', snippet: 'Thank you! I have reviewed and signed the <b>contract</b>.', attachments: [{ id: 'att_1', filename: 'vault_contract_2026.pdf', mimetype: 'application/pdf', file_size: 245120, media_type: 'document' }] },
  { id: 'msg_4', source: 'whatsapp', account: 'personal', conversation_id: 'chat_2', conversation_name: 'Engineering Core Team', sender_name: 'Bob Johnson', timestamp: 1710001800, is_from_me: false, body: 'Release v0.2.0 is ready for deployment.', snippet: 'Release v0.2.0 is ready for deployment.', attachments: [] },
];

const MIME_MAP = {
  '.html': 'text/html',
  '.js': 'application/javascript',
  '.css': 'text/css',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.ico': 'image/x-icon',
  '.json': 'application/json',
};

const server = http.createServer((req, res) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);
  const pathname = url.pathname;

  if (pathname === '/v1/accounts') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify(SAMPLE_ACCOUNTS));
  }

  if (pathname === '/v1/chats') {
    const rawQ = (url.searchParams.get('query') || '').toLowerCase().trim();
    const source = url.searchParams.get('source');
    const account = url.searchParams.get('account');
    let chats = SAMPLE_CHATS;
    if (source) chats = chats.filter((c) => c.source_id === source);
    if (account) chats = chats.filter((c) => c.account_id === account);
    if (rawQ) {
      const terms = rawQ.split(/\s+/).filter((t) => !t.includes(':') && t.length > 0);
      const searchTerm = terms[0] || rawQ;
      const matchingChatIds = new Set(
        ALL_MESSAGES.filter((m) => m.body.toLowerCase().includes(searchTerm)).map((m) => m.conversation_id),
      );
      chats = chats.filter(
        (c) =>
          c.name.toLowerCase().includes(searchTerm) ||
          c.remote_id.toLowerCase().includes(searchTerm) ||
          matchingChatIds.has(c.id),
      );
    }
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify(chats));
  }

  if (pathname === '/v1/search' || pathname.startsWith('/v1/chats/')) {
    const q = (url.searchParams.get('query') || '').toLowerCase();
    const hasMedia = url.searchParams.get('has_media');
    const account = url.searchParams.get('account');
    let result = ALL_MESSAGES;
    if (account) result = result.filter((m) => m.account === account);
    if (q.includes('contract') || hasMedia === 'true') {
      result = result.filter((m) => m.body.toLowerCase().includes('contract') && (hasMedia !== 'true' || m.attachments.length > 0));
    }
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify(result));
  }

  if (pathname.startsWith('/v1/attachments/')) {
    res.writeHead(200, { 'Content-Type': 'application/pdf' });
    return res.end(Buffer.from('%PDF-1.4 dummy'));
  }

  let filePath = path.join(WEB_DIST, pathname === '/' ? 'index.html' : pathname);
  if (!fs.existsSync(filePath)) filePath = path.join(WEB_DIST, 'index.html');

  const ext = path.extname(filePath);
  const contentType = MIME_MAP[ext] || 'application/octet-stream';

  fs.readFile(filePath, (err, data) => {
    if (err) {
      res.writeHead(404);
      return res.end('Not Found');
    }
    res.writeHead(200, { 'Content-Type': contentType });
    res.end(data);
  });
});

async function captureFrame(params, outFile) {
  const url = `http://localhost:${PORT}/?${params}`;
  const chromiumBin = process.env.CHROMIUM_BIN || (fs.existsSync('/usr/bin/chromium') ? '/usr/bin/chromium' : 'chromium');
  const args = ['--headless', '--no-sandbox', '--disable-gpu', '--hide-scrollbars', '--virtual-time-budget=2500', '--window-size=1100,680', `--screenshot=${outFile}`, url];

  return new Promise((resolve, reject) => {
    const child = spawn(chromiumBin, args);
    child.on('close', (code) => {
      if (code === 0) resolve();
      else reject(new Error(`Chromium frame failed with code ${code}`));
    });
  });
}

server.listen(PORT, async () => {
  console.log(`GIF capture server listening on port ${PORT}...`);
  try {
    const steps = [
      { name: '01', params: 'theme=dark' },
      { name: '02', params: 'theme=dark&q=con' },
      { name: '03', params: 'theme=dark&q=contract' },
      { name: '04', params: 'theme=dark&q=contract+from:Alice&account=work&media_type=document' },
      { name: '05', params: 'theme=light&q=contract+from:Alice&account=work&media_type=document' },
      { name: '06', params: 'theme=light' },
    ];

    for (const step of steps) {
      const framePath = path.join(FRAMES_DIR, `frame_${step.name}.png`);
      console.log(`Capturing frame ${step.name}...`);
      await captureFrame(step.params, framePath);
    }

    const gifOutput = path.join(OUTPUT_DIR, 'realtime-filter.gif');
    const demoGifOutput = path.join(OUTPUT_DIR, 'demo.gif');
    const palettePath = path.join(FRAMES_DIR, 'palette.png');
    const inputPattern = path.join(FRAMES_DIR, 'frame_%02d.png');

    execSync(
      `ffmpeg -y -framerate 0.8 -i "${inputPattern}" -vf "fps=10,scale=1000:-1:flags=lanczos,palettegen" "${palettePath}"`,
      { stdio: 'inherit' },
    );
    execSync(
      `ffmpeg -y -framerate 0.8 -i "${inputPattern}" -i "${palettePath}" -lavfi "fps=10,scale=1000:-1:flags=lanczos [x]; [x][1:v] paletteuse=dither=bayer:bayer_scale=3" "${gifOutput}"`,
      { stdio: 'inherit' },
    );

    fs.copyFileSync(gifOutput, demoGifOutput);
    console.log(`Saved animated GIF to ${gifOutput} & ${demoGifOutput}`);
  } catch (err) {
    console.error('Failed to generate demo GIF:', err);
  } finally {
    server.close();
  }
});
