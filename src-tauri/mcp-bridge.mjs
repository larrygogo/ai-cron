#!/usr/bin/env node
// AI Cron MCP Stdio Bridge
// Bridges MCP stdio transport ↔ Streamable HTTP transport
// Node.js http module does NOT use HTTP_PROXY, so this bypasses system proxy.

import { createInterface } from 'node:readline';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';

// Resolve port: try port file first, then env var, then default
function resolvePort() {
  // Determine app data dir based on platform
  let appDataDir;
  if (process.platform === 'win32') {
    appDataDir = path.join(process.env.APPDATA || '', 'com.ai-cron.app');
  } else if (process.platform === 'darwin') {
    appDataDir = path.join(os.homedir(), 'Library', 'Application Support', 'com.ai-cron.app');
  } else {
    appDataDir = path.join(
      process.env.XDG_DATA_HOME || path.join(os.homedir(), '.local', 'share'),
      'com.ai-cron.app'
    );
  }

  const portFile = path.join(appDataDir, 'mcp-port');
  try {
    return parseInt(fs.readFileSync(portFile, 'utf8').trim(), 10);
  } catch {
    // Fallback to env var or default
    return parseInt(process.env.AI_CRON_MCP_PORT, 10) || 23987;
  }
}

const PORT = resolvePort();
const HOST = '127.0.0.1';
const PATH = '/mcp';
let sessionId = null;

function postToMcp(body) {
  return new Promise((resolve, reject) => {
    const data = JSON.stringify(body);
    const headers = {
      'Content-Type': 'application/json',
      'Accept': 'application/json, text/event-stream',
      'Content-Length': Buffer.byteLength(data),
    };
    if (sessionId) {
      headers['Mcp-Session-Id'] = sessionId;
    }

    const req = http.request(
      { hostname: HOST, port: PORT, path: PATH, method: 'POST', headers },
      (res) => {
        const sid = res.headers['mcp-session-id'];
        if (sid) sessionId = sid;

        const chunks = [];
        res.on('data', (chunk) => chunks.push(chunk));
        res.on('end', () => resolve(Buffer.concat(chunks).toString()));
        res.on('error', reject);
      },
    );

    req.on('error', reject);
    req.end(data);
  });
}

function extractSseData(text) {
  const results = [];
  for (const line of text.split('\n')) {
    if (line.startsWith('data: ')) {
      const payload = line.slice(6).trim();
      if (payload) results.push(payload);
    }
  }
  return results;
}

const rl = createInterface({ input: process.stdin, crlfDelay: Infinity });

rl.on('line', async (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;

  let parsed;
  try {
    parsed = JSON.parse(trimmed);
  } catch {
    return;
  }

  try {
    const response = await postToMcp(parsed);
    for (const msg of extractSseData(response)) {
      process.stdout.write(msg + '\n');
    }
  } catch (err) {
    if (parsed.id != null) {
      process.stdout.write(
        JSON.stringify({
          jsonrpc: '2.0',
          id: parsed.id,
          error: {
            code: -32603,
            message: `Bridge error: ${err.message || 'Cannot connect to ai-cron'}`,
          },
        }) + '\n',
      );
    }
  }
});

rl.on('close', () => process.exit(0));
