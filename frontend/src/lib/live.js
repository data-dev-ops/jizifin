/**
 * live.js — `/ws/finance` subscription. Each `{ event: "<table>.<action>" }`
 * refreshes the matching catalog store and bumps `ledgerVersion`, which every
 * view depends on, so open tabs refetch what the server now says.
 */

import { get } from 'svelte/store';
import { sessionToken, ledgerVersion } from './stores.js';
import {
  memberApi, categoryApi, accountApi, projectApi, tagApi, jobApi, recurringApi,
  budgetApi, offsetRuleApi, audit, loadCatalog,
} from './api.js';

const RECONNECT_MS = 3000;

const refreshers = {
  members: memberApi.list,
  categories: categoryApi.list,
  accounts: accountApi.list,
  projects: projectApi.list,
  tags: tagApi.list,
  jobs: jobApi.list,
  recurring: recurringApi.list,
  budgets: budgetApi.list,
  offset_rules: offsetRuleApi.list,
  settlements: audit.seals,
  backup: loadCatalog,
};

let socket = null;
let retry = null;

function url(token) {
  const scheme = window.location.protocol === 'https:' ? 'wss' : 'ws';
  return `${scheme}://${window.location.host}/ws/finance?token=${encodeURIComponent(token)}`;
}

function onEvent(message) {
  const { event } = JSON.parse(message.data);
  const refresh = refreshers[event.split('.')[0]];
  ledgerVersion.update((v) => v + 1);
  refresh?.().catch(() => null);
}

/** Opens the socket for the current session; reconnects until `disconnect()`. */
export function connect() {
  const token = get(sessionToken);
  if (!token || socket) return;
  socket = new WebSocket(url(token));
  socket.onmessage = onEvent;
  socket.onclose = () => {
    socket = null;
    if (get(sessionToken)) retry = setTimeout(connect, RECONNECT_MS);
  };
}

export function disconnect() {
  clearTimeout(retry);
  socket?.close();
  socket = null;
}
