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
const PORT = 8799;

if (!fs.existsSync(OUTPUT_DIR)) {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
}

if (!fs.existsSync(path.join(WEB_DIST, 'index.html'))) {
  console.log('Building web bundle first...');
  execSync('npm --prefix web run build', { cwd: ROOT_DIR, stdio: 'inherit' });
}

const SAMPLE_ACCOUNTS = [
  { id: 'personal', name: 'personal', sources: ['whatsapp'], message_count: 198, chat_count: 2 },
  { id: 'work', name: 'work', sources: ['whatsapp', 'folder'], message_count: 18, chat_count: 1 },
];

const SAMPLE_CHATS = [
  { id: 'chat_1', source_id: 'whatsapp', account_id: 'personal', remote_id: '1234567890@s.whatsapp.net', name: 'Alice Smith', is_group: false, message_count: 42, last_message: 'Here is the signed agreement.', last_timestamp: 1710002400 },
  { id: 'chat_2', source_id: 'whatsapp', account_id: 'personal', remote_id: 'group_987@g.us', name: 'Engineering Core Team', is_group: true, message_count: 156, last_message: 'Release v0.2.0 is ready!', last_timestamp: 1710001800 },
  { id: 'chat_3', source_id: 'folder', account_id: 'work', remote_id: 'accounting_export', name: 'Finance & Invoices', is_group: false, message_count: 18, last_message: 'Q1 summary invoice attached.', last_timestamp: 1709998000 },
];

const SAMPLE_MESSAGES = [
  { id: 'msg_1', source: 'whatsapp', account: 'personal', conversation_id: 'chat_1', conversation_name: 'Alice Smith', sender_name: 'Alice Smith', timestamp: 1710001000, is_from_me: false, body: 'Hi! Could you send me the latest contract for the messaging vault?', snippet: 'Hi! Could you send me the latest contract for the messaging vault?', attachments: [] },
  { id: 'msg_2', source: 'whatsapp', account: 'personal', conversation_id: 'chat_1', conversation_name: 'Alice Smith', sender_name: null, timestamp: 1710001500, is_from_me: true, body: 'Sure Alice, here is the proposal with full Maildir and SQLite FTS5 specs.', snippet: 'Sure Alice, here is the proposal with full Maildir and SQLite FTS5 specs.', attachments: [] },
  { id: 'msg_3', source: 'whatsapp', account: 'work', conversation_id: 'chat_3', conversation_name: 'Finance & Invoices', sender_name: 'Alice Smith', timestamp: 1710002400, is_from_me: false, body: 'Thank you! I have reviewed and signed the contract.', snippet: 'Thank you! I have reviewed and signed the <b>contract</b>.', attachments: [{ id: 'att_1', filename: 'vault_contract_2026.pdf', mimetype: 'application/pdf', file_size: 245120, media_type: 'document' }] },
];

const MIME_MAP = {
  '.html': 'text/html',
  '.js': 'application/javascript',
  '.css': 'text/css',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
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
        SAMPLE_MESSAGES.filter((m) => m.body.toLowerCase().includes(searchTerm)).map((m) => m.conversation_id),
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
    const account = url.searchParams.get('account');
    const msgs = account ? SAMPLE_MESSAGES.filter((m) => m.account === account) : SAMPLE_MESSAGES;
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify(msgs));
  }

  if (pathname.startsWith('/v1/attachments/')) {
    res.writeHead(200, { 'Content-Type': 'application/pdf' });
    return res.end(Buffer.from('%PDF-1.4 dummy'));
  }

  let filePath = path.join(WEB_DIST, pathname === '/' ? 'index.html' : pathname);
  if (!fs.existsSync(filePath)) {
    filePath = path.join(WEB_DIST, 'index.html');
  }

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

async function capture(theme, outFile) {
  const url = `http://localhost:${PORT}/?theme=${theme}`;
  const chromiumBin = process.env.CHROMIUM_BIN || (fs.existsSync('/usr/bin/chromium') ? '/usr/bin/chromium' : 'chromium');
  const args = ['--headless', '--no-sandbox', '--disable-gpu', '--hide-scrollbars', '--virtual-time-budget=2500', '--window-size=1280,820', `--screenshot=${outFile}`, url];

  return new Promise((resolve, reject) => {
    const child = spawn(chromiumBin, args);
    child.on('close', (code) => {
      if (code === 0) resolve();
      else reject(new Error(`Chromium screenshot failed with code ${code}`));
    });
  });
}

server.listen(PORT, async () => {
  console.log(`Preview server listening on port ${PORT}...`);
  try {
    const darkPath = path.join(OUTPUT_DIR, 'screenshot-dark.png');
    const lightPath = path.join(OUTPUT_DIR, 'screenshot-light.png');
    const mainPath = path.join(OUTPUT_DIR, 'screenshot.png');

    console.log('Capturing dark theme screenshot...');
    await capture('dark', darkPath);
    fs.copyFileSync(darkPath, mainPath);
    console.log(`Saved: ${darkPath} & ${mainPath}`);

    console.log('Capturing light theme screenshot...');
    await capture('light', lightPath);
    console.log(`Saved: ${lightPath}`);

    console.log('Screenshot capture complete!');
  } catch (err) {
    console.error('Failed to capture screenshot:', err);
  } finally {
    server.close();
  }
});
