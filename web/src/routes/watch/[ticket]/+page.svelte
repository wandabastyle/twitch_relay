<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { page } from '$app/stores';
  import { getTwitchConnectUrl, getTwitchStatus, getWatchSession } from '$lib/api';

  type HlsLevel = { height: number; bitrate: number };

  type WatchChatPart =
    | {
        kind: 'text';
        text: string;
      }
    | {
        kind: 'emote';
        id: string;
        code: string;
        image_url?: string;
      };

  type WatchChatEvent = {
    id: string;
    kind: 'message' | 'notice';
    sender_display_name: string;
    sender_color: string | null;
    text: string;
    parts: WatchChatPart[];
  };

  type EmoteItem = {
    id: string;
    code: string;
    image_url: string;
    group_key: string;
    group_name: string;
  };

  type ActiveEmoteQuery = {
    query: string;
    start: number;
    end: number;
  };

  const RESUME_ENTER_LIVE_SECS = 5.5;
  const RESUME_EXIT_LIVE_SECS = 7.5;
  const AUTO_SCROLL_THRESHOLD_PX = 32;

  const ticket = $derived($page.params.ticket ?? '');

  let channelLogin = $state('');
  let appVersion = $state('');
  let manifestUrl = $state('');
  let watchLoading = $state(true);
  let watchError = $state<string | null>(null);

  let playerEl = $state<HTMLVideoElement | null>(null);
  let hlsInstance = $state<Hls | null>(null);
  let hlsLevels = $state<HlsLevel[]>([]);
  let qualityLevel = $state(-1);
  let currentPlayingLevel = $state(-1);
  let userSelectedAuto = $state(true);
  let attemptedRelayFallback = $state(false);
  let liveButtonIsLive = $state(true);
  let playbackError = $state<string | null>(null);

  let chatEvents = $state<EventSource | null>(null);
  let chatConnected = $state(false);
  let chatAvailable = $state(false);
  let chatStatus = $state('Checking Twitch chat...');
  let chatMessages = $state<WatchChatEvent[]>([]);
  let unreadChatCount = $state(0);
  let chatSending = $state(false);

  let chatMessagesEl = $state<HTMLDivElement | null>(null);
  let chatComposerEl = $state<HTMLDivElement | null>(null);
  let emoteSearchEl = $state<HTMLInputElement | null>(null);

  let emotePickerLoaded = $state(false);
  let emotePickerOpen = $state(false);
  let emoteSearchTerm = $state('');
  let availableEmotes = $state<EmoteItem[]>([]);

  let emoteSuggestionsOpen = $state(false);
  let emoteSuggestionIndex = $state(0);
  let emoteSuggestionItems = $state<EmoteItem[]>([]);
  const groupedEmotes = $derived(groupedPickerEmotes());

  const connectTwitchUrl = getTwitchConnectUrl();

  onMount(() => {
    if (!ticket) {
      watchError = 'Missing watch ticket.';
      watchLoading = false;
      return;
    }

    document.addEventListener('click', handleDocumentClick);
    void initializeWatchPage();
  });

  onDestroy(() => {
    document.removeEventListener('click', handleDocumentClick);
    cleanupPlayer();
    void cleanupChat();
  });

  async function initializeWatchPage(): Promise<void> {
    watchError = null;
    watchLoading = true;
    playbackError = null;

    const forceRelay =
      typeof window !== 'undefined' && new URLSearchParams(window.location.search).get('relay') === '1';
    attemptedRelayFallback = forceRelay;

    try {
      const session = await getWatchSession(ticket, forceRelay);
      channelLogin = session.channel;
      appVersion = session.app_version;
      manifestUrl = session.manifest_url;
      attemptedRelayFallback = session.relay;

      watchLoading = false;
      await tick();
      await setupPlayer();
      await setupChat();
    } catch (error) {
      watchError = readMessage(error, 'Failed to initialize watch session.');
      watchLoading = false;
    }
  }

  async function setupPlayer(): Promise<void> {
    if (!playerEl) return;

    const hlsLoaded = await ensureHlsLoaded('/hls.js');
    if (!hlsLoaded) {
      playbackError = 'Failed to load HLS player.';
      return;
    }

    if ('Hls' in window && Hls.isSupported()) {
      const HlsClass = window.Hls;
      hlsInstance = new HlsClass({
        startLevel: -1,
        startPosition: -1,
        capLevelToPlayerSize: true,
        lowLatencyMode: true,
        liveSyncDuration: 10,
        liveMaxLatencyDuration: 24,
        maxLiveSyncPlaybackRate: 1.05,
        maxBufferLength: 30,
        maxMaxBufferLength: 60,
        backBufferLength: 30,
        abrEwmaFastLive: 3.0,
        abrEwmaSlowLive: 9.0,
        manifestLoadingTimeOut: 15_000,
        levelLoadingTimeOut: 15_000,
        fragLoadingTimeOut: 20_000,
        manifestLoadingMaxRetry: 3,
        levelLoadingMaxRetry: 3,
        fragLoadingMaxRetry: 5,
        manifestLoadingRetryDelay: 750,
        levelLoadingRetryDelay: 750,
        fragLoadingRetryDelay: 750,
      });

      hlsInstance.currentLevel = -1;
      qualityLevel = -1;
      currentPlayingLevel = -1;
      userSelectedAuto = true;

      hlsInstance.on(HlsClass.Events.MANIFEST_PARSED, (_event: string, data: unknown) => {
        const parsed = toObject(data);
        const levels = Array.isArray(parsed?.levels)
          ? parsed.levels.filter(
              (item): item is HlsLevel =>
                toObject(item) !== null &&
                typeof toObject(item)?.height === 'number' &&
                typeof toObject(item)?.bitrate === 'number'
            )
          : [];
        hlsLevels = levels;
      });

      hlsInstance.on(HlsClass.Events.LEVEL_SWITCHED, (_event: string, data: unknown) => {
        const parsed = toObject(data);
        const level = typeof parsed?.level === 'number' ? parsed.level : -1;
        currentPlayingLevel = level;
        if (userSelectedAuto) qualityLevel = -1;
      });

      hlsInstance.on(HlsClass.Events.ERROR, (_event: string, data: unknown) => {
        const parsed = toObject(data);
        if (parsed?.fatal === true) {
          handleFatalPlaybackError();
        }
      });

      hlsInstance.loadSource(manifestUrl);
      hlsInstance.attachMedia(playerEl);
    } else if (playerEl.canPlayType('application/vnd.apple.mpegurl')) {
      playerEl.src = manifestUrl;
    } else {
      playbackError = 'Your browser does not support HLS playback.';
      return;
    }

    playerEl.addEventListener('timeupdate', updateGoLiveState);
    playerEl.addEventListener('loadedmetadata', updateGoLiveState);
    playerEl.addEventListener('durationchange', updateGoLiveState);
    playerEl.addEventListener('error', () => {
      playbackError = 'Stream unavailable. The channel may be offline or not accessible.';
    });
  }

  function cleanupPlayer(): void {
    if (playerEl) {
      playerEl.removeEventListener('timeupdate', updateGoLiveState);
      playerEl.removeEventListener('loadedmetadata', updateGoLiveState);
      playerEl.removeEventListener('durationchange', updateGoLiveState);
    }

    if (hlsInstance) {
      hlsInstance.destroy();
      hlsInstance = null;
    }
  }

  function updateGoLiveState(): void {
    if (!playerEl) {
      liveButtonIsLive = true;
      return;
    }

    if (playerEl.seekable.length <= 0) {
      liveButtonIsLive = true;
      return;
    }

    const end = playerEl.seekable.end(playerEl.seekable.length - 1);
    const lag = Math.max(0, end - playerEl.currentTime);
    if (liveButtonIsLive) {
      if (lag > RESUME_EXIT_LIVE_SECS) {
        liveButtonIsLive = false;
      }
      return;
    }
    if (lag < RESUME_ENTER_LIVE_SECS) {
      liveButtonIsLive = true;
    }
  }

  function goLive(): void {
    if (!playerEl || liveButtonIsLive) return;

    if (hlsInstance && Number.isFinite(hlsInstance.liveSyncPosition)) {
      playerEl.currentTime = hlsInstance.liveSyncPosition as number;
      updateGoLiveState();
      return;
    }

    if (playerEl.seekable.length > 0) {
      playerEl.currentTime = playerEl.seekable.end(playerEl.seekable.length - 1);
      updateGoLiveState();
    }
  }

  function qualityLabel(level: HlsLevel, idx: number): string {
    if (idx === 0) {
      const bitrate = level.bitrate > 0 ? ` (${(level.bitrate / 1_000_000).toFixed(1)} Mbps)` : '';
      return `Source${bitrate}`;
    }
    const bitrate = level.bitrate > 0 ? ` (${(level.bitrate / 1_000_000).toFixed(1)} Mbps)` : '';
    return `${level.height}p${bitrate}`;
  }

  function selectedQualityLabel(): string {
    if (qualityLevel === -1) {
      if (currentPlayingLevel >= 0 && hlsLevels[currentPlayingLevel]) {
        return `Auto (${hlsLevels[currentPlayingLevel].height}p)`;
      }
      return 'Auto';
    }

    if (hlsLevels[qualityLevel]) {
      if (qualityLevel === 0) {
        return 'Source';
      }
      return `${hlsLevels[qualityLevel].height}p`;
    }

    return 'Manual';
  }

  function setQuality(level: number): void {
    if (!hlsInstance) return;
    hlsInstance.currentLevel = level;
    qualityLevel = level;
    userSelectedAuto = level === -1;
  }

  function handleQualityChange(event: Event): void {
    const target = event.currentTarget as HTMLSelectElement;
    const next = Number.parseInt(target.value, 10);
    if (!Number.isFinite(next)) return;
    setQuality(next);
  }

  function handleFatalPlaybackError(): void {
    if (typeof window === 'undefined') {
      playbackError = 'Stream unavailable. The channel may be offline or not accessible.';
      return;
    }

    if (!attemptedRelayFallback) {
      attemptedRelayFallback = true;
      const nextUrl = new URL(window.location.href);
      nextUrl.searchParams.set('relay', '1');
      window.location.assign(nextUrl.toString());
      return;
    }

    playbackError = 'Stream unavailable. The channel may be offline or not accessible.';
  }

  async function setupChat(): Promise<void> {
    chatStatus = 'Checking Twitch chat...';
    chatConnected = false;

    try {
      const twitchStatus = await getTwitchStatus();
      if (!twitchStatus.connected) {
        chatAvailable = false;
        chatStatus = 'Connect Twitch to use chat.';
        return;
      }

      chatAvailable = true;
      await subscribeChat();
      openChatEvents();
      void ensureEmotesLoaded();
    } catch (error) {
      chatAvailable = false;
      chatStatus = readMessage(error, 'Chat unavailable');
    }
  }

  async function cleanupChat(): Promise<void> {
    if (chatEvents) {
      chatEvents.close();
      chatEvents = null;
    }

    if (!channelLogin) return;
    try {
      await fetch(`/api/chat/subscribe/${encodeURIComponent(channelLogin)}`, {
        method: 'DELETE',
        credentials: 'same-origin',
        keepalive: true,
      });
    } catch {
      // no-op
    }
  }

  async function subscribeChat(): Promise<void> {
    await chatRequest('/api/chat/subscribe', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ channel_login: channelLogin }),
    });
    chatStatus = `Connected to #${channelLogin}`;
  }

  function openChatEvents(): void {
    if (chatEvents) {
      chatEvents.close();
      chatEvents = null;
    }

    chatEvents = new EventSource(`/api/chat/events/${encodeURIComponent(channelLogin)}`);
    chatEvents.onopen = () => {
      chatConnected = true;
      chatStatus = `Connected to #${channelLogin}`;
    };
    chatEvents.onerror = () => {
      chatConnected = false;
      chatStatus = 'Chat reconnecting...';
    };
    chatEvents.addEventListener('chat', (event) => {
      const message = parseChatEvent((event as MessageEvent<string>).data);
      if (!message) return;
      void appendChatEvent(message);
    });
  }

  async function appendChatEvent(message: WatchChatEvent): Promise<void> {
    const shouldStickToBottom = isNearBottom(chatMessagesEl);
    chatMessages = [...chatMessages, message];
    await tick();

    if (shouldStickToBottom && chatMessagesEl) {
      chatMessagesEl.scrollTop = chatMessagesEl.scrollHeight;
      clearUnreadChat();
      return;
    }

    unreadChatCount += 1;
  }

  function clearUnreadChat(): void {
    unreadChatCount = 0;
  }

  function chatUnreadLabel(count: number): string {
    if (count <= 1) return '1 new message';
    if (count > 99) return '99+ new messages';
    return `${count} new messages`;
  }

  function handleChatScroll(): void {
    if (isNearBottom(chatMessagesEl)) {
      clearUnreadChat();
    }
  }

  function jumpToLatestChat(): void {
    if (!chatMessagesEl) return;
    chatMessagesEl.scrollTop = chatMessagesEl.scrollHeight;
    clearUnreadChat();
  }

  async function chatRequest(path: string, init?: RequestInit): Promise<void> {
    const response = await fetch(path, {
      credentials: 'same-origin',
      ...init,
    });

    if (!response.ok) {
      throw new Error(await readApiError(response));
    }
  }

  async function sendChatMessage(): Promise<void> {
    closeEmotePicker();
    closeEmoteSuggestions();

    const text = getComposerPlainText().trim();
    if (!text) return;

    chatSending = true;
    try {
      await chatRequest('/api/chat/send', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          channel_login: channelLogin,
          message: text,
        }),
      });
      if (chatComposerEl) {
        chatComposerEl.innerHTML = '';
      }
      chatStatus = `Connected to #${channelLogin}`;
      closeEmoteSuggestions();
      placeComposerCaretAtEnd();
    } catch (error) {
      chatStatus = readMessage(error, 'Failed to send message');
    } finally {
      chatSending = false;
    }
  }

  async function handleChatSend(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    await sendChatMessage();
  }

  function handleComposerInput(): void {
    normalizeComposerInput();
    refreshEmoteSuggestions();
  }

  function handleComposerClick(): void {
    placeComposerCaretAtEnd();
    refreshEmoteSuggestions();
  }

  function handleComposerPaste(event: ClipboardEvent): void {
    event.preventDefault();
    const text = ((event.clipboardData && event.clipboardData.getData('text/plain')) || '').replace(
      /[\r\n]+/g,
      ' '
    );
    if (!text) return;
    applyPlainTextToComposer((getComposerPlainText() + text).slice(0, 500));
    refreshEmoteSuggestions();
  }

  function handleComposerKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' && !(emoteSuggestionsOpen && emoteSuggestionItems.length > 0)) {
      event.preventDefault();
      if (chatSending) return;
      void sendChatMessage();
      return;
    }

    if (!emoteSuggestionsOpen || emoteSuggestionItems.length === 0) {
      if (event.key === 'Escape') {
        closeEmotePicker();
      }
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      emoteSuggestionIndex = (emoteSuggestionIndex + 1) % emoteSuggestionItems.length;
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      emoteSuggestionIndex =
        (emoteSuggestionIndex - 1 + emoteSuggestionItems.length) % emoteSuggestionItems.length;
      return;
    }

    if (event.key === 'Tab' || event.key === 'Enter') {
      event.preventDefault();
      const selected = emoteSuggestionItems[emoteSuggestionIndex];
      const range = findActiveEmoteQuery();
      if (selected && range) {
        applyEmoteCode(selected.code, range);
      }
      closeEmoteSuggestions();
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      closeEmoteSuggestions();
    }
  }

  async function ensureEmotesLoaded(): Promise<void> {
    if (emotePickerLoaded || !channelLogin) return;

    const response = await fetch(`/api/chat/emotes?channel_login=${encodeURIComponent(channelLogin)}`, {
      credentials: 'same-origin',
    });
    if (!response.ok) {
      throw new Error(await readApiError(response));
    }

    const payload = await response.json();
    const parsed = toObject(payload);
    availableEmotes = Array.isArray(parsed?.emotes)
      ? parsed.emotes
          .filter(
            (item): item is EmoteItem =>
              toObject(item) !== null &&
              typeof toObject(item)?.id === 'string' &&
              typeof toObject(item)?.code === 'string' &&
              typeof toObject(item)?.image_url === 'string' &&
              typeof toObject(item)?.group_key === 'string' &&
              typeof toObject(item)?.group_name === 'string'
          )
          .map((item) => ({
            id: item.id,
            code: normalizeEmoteCode(item.code),
            image_url: item.image_url,
            group_key: item.group_key,
            group_name: item.group_name,
          }))
          .filter((item) => item.code.length > 0)
      : [];

    emotePickerLoaded = true;
    normalizeComposerInput();
  }

  async function toggleEmotePicker(): Promise<void> {
    if (emotePickerOpen) {
      closeEmotePicker();
      placeComposerCaretAtEnd();
      return;
    }

    closeEmoteSuggestions();
    emoteSearchTerm = '';
    if (emoteSearchEl) {
      emoteSearchEl.value = '';
    }

    try {
      await ensureEmotesLoaded();
      emotePickerOpen = true;
      await tick();
      emoteSearchEl?.focus();
    } catch (error) {
      chatStatus = readMessage(error, 'Failed to load emotes');
    }
  }

  function closeEmotePicker(): void {
    emotePickerOpen = false;
  }

  function closeEmoteSuggestions(): void {
    emoteSuggestionsOpen = false;
    emoteSuggestionItems = [];
    emoteSuggestionIndex = 0;
  }

  function refreshEmoteSuggestions(): void {
    const active = findActiveEmoteQuery();
    if (!active) {
      closeEmoteSuggestions();
      return;
    }

    if (!emotePickerLoaded) {
      void ensureEmotesLoaded()
        .then(() => refreshEmoteSuggestions())
        .catch((error) => {
          chatStatus = readMessage(error, 'Failed to load emotes');
        });
      return;
    }

    const query = active.query.toLowerCase();
    const ranked = availableEmotes
      .map((item) => ({ item, score: scoreEmote(item.code, query) }))
      .filter((entry) => entry.score < 99)
      .sort((left, right) => {
        if (left.score !== right.score) return left.score - right.score;
        return left.item.code.toLowerCase().localeCompare(right.item.code.toLowerCase());
      })
      .slice(0, 10)
      .map((entry) => entry.item);

    if (ranked.length === 0) {
      closeEmoteSuggestions();
      return;
    }

    emoteSuggestionsOpen = true;
    emoteSuggestionItems = ranked;
    emoteSuggestionIndex = Math.min(emoteSuggestionIndex, ranked.length - 1);
  }

  function filteredPickerEmotes(): EmoteItem[] {
    const term = emoteSearchTerm.trim().toLowerCase();
    if (!term) return availableEmotes;
    return availableEmotes.filter((item) => item.code.toLowerCase().includes(term));
  }

  function groupedPickerEmotes(): Array<{ key: string; title: string; items: EmoteItem[] }> {
    const groupedMap = new Map<string, { key: string; title: string; items: EmoteItem[] }>();

    for (const item of filteredPickerEmotes()) {
      const key = item.group_key || 'global';
      const title = item.group_name.trim().length > 0 ? item.group_name : 'Global';
      if (!groupedMap.has(key)) {
        groupedMap.set(key, { key, title, items: [] });
      }
      groupedMap.get(key)?.items.push(item);
    }

    return Array.from(groupedMap.values());
  }

  function applyEmoteCode(code: string, queryRange: ActiveEmoteQuery | null): void {
    const safeCode = normalizeEmoteCode(code);
    if (!safeCode) return;

    const full = getComposerPlainText();
    if (queryRange) {
      const before = full.slice(0, queryRange.start);
      const after = full.slice(queryRange.end);
      applyPlainTextToComposer(`${before}${safeCode} ${after}`);
      return;
    }

    applyPlainTextToComposer(`${full}${safeCode} `);
  }

  function composerTextFromNode(node: Node | null): string {
    if (!node) return '';
    if (node.nodeType === Node.TEXT_NODE) return node.textContent || '';
    if (node.nodeType !== Node.ELEMENT_NODE) return '';

    const element = node as HTMLElement;
    if (element.tagName === 'IMG') return element.dataset.code || '';
    if (element.tagName === 'BR') return '\n';

    let out = '';
    for (const child of Array.from(element.childNodes)) {
      out += composerTextFromNode(child);
    }
    return out;
  }

  function getComposerPlainText(): string {
    if (!chatComposerEl) return '';
    let out = '';
    for (const child of Array.from(chatComposerEl.childNodes)) {
      out += composerTextFromNode(child);
    }
    return out;
  }

  function splitMessageSegments(input: string): Array<{ text: string; whitespace: boolean }> {
    const out: Array<{ text: string; whitespace: boolean }> = [];
    let current = '';
    let currentWhitespace: boolean | null = null;

    for (const ch of input) {
      const isWhitespace = /\s/.test(ch);
      if (currentWhitespace === null || currentWhitespace === isWhitespace) {
        current += ch;
        currentWhitespace = isWhitespace;
      } else {
        out.push({ text: current, whitespace: currentWhitespace });
        current = ch;
        currentWhitespace = isWhitespace;
      }
    }

    if (current.length > 0 && currentWhitespace !== null) {
      out.push({ text: current, whitespace: currentWhitespace });
    }

    return out;
  }

  function buildEmoteMapByCode(): Map<string, EmoteItem> {
    const emotesByCode = new Map<string, EmoteItem>();
    for (const item of availableEmotes) {
      emotesByCode.set(item.code, item);
    }
    return emotesByCode;
  }

  function renderComposerFromPlainText(text: string): void {
    if (!chatComposerEl) return;

    const emotesByCode = buildEmoteMapByCode();
    chatComposerEl.innerHTML = '';
    if (!text) return;

    for (const segment of splitMessageSegments(text)) {
      if (segment.whitespace) {
        chatComposerEl.appendChild(document.createTextNode(segment.text));
        continue;
      }

      const match = emotesByCode.get(segment.text);
      if (!match) {
        chatComposerEl.appendChild(document.createTextNode(segment.text));
        continue;
      }

      const img = document.createElement('img');
      img.className = 'watch-composer-emote';
      img.src = match.image_url;
      img.alt = match.code;
      img.title = match.code;
      img.dataset.code = match.code;
      img.dataset.id = match.id;
      img.loading = 'lazy';
      img.decoding = 'async';
      img.contentEditable = 'false';
      chatComposerEl.appendChild(img);
    }
  }

  function applyPlainTextToComposer(text: string): void {
    renderComposerFromPlainText(text);
    placeComposerCaretAtEnd();
  }

  function placeComposerCaretAtEnd(): void {
    if (!chatComposerEl) return;
    chatComposerEl.focus();

    const range = document.createRange();
    range.selectNodeContents(chatComposerEl);
    range.collapse(false);

    const selection = window.getSelection();
    if (!selection) return;
    selection.removeAllRanges();
    selection.addRange(range);
  }

  function normalizeComposerInput(): void {
    let plain = getComposerPlainText();
    plain = plain.replace(/[\r\n]+/g, ' ');
    if (plain.length > 500) plain = plain.slice(0, 500);
    renderComposerFromPlainText(plain);
    placeComposerCaretAtEnd();
  }

  function findActiveEmoteQuery(): ActiveEmoteQuery | null {
    const full = getComposerPlainText();
    const match = full.match(/(^|\s):([A-Za-z0-9_]{2,})$/);
    if (!match) return null;
    const query = match[2];
    return {
      query,
      start: full.length - query.length - 1,
      end: full.length,
    };
  }

  function scoreEmote(code: string, query: string): number {
    const loweredCode = code.toLowerCase();
    const loweredQuery = query.toLowerCase();
    if (loweredCode === loweredQuery) return 0;
    if (loweredCode.startsWith(loweredQuery)) return 1;
    if (loweredCode.includes(loweredQuery)) return 2;
    return 99;
  }

  function normalizeEmoteCode(code: string): string {
    return code.trim();
  }

  function emoteUrl(emoteId: string): string {
    return `https://static-cdn.jtvnw.net/emoticons/v2/${encodeURIComponent(emoteId)}/default/dark/2.0`;
  }

  function parseChatEvent(raw: string): WatchChatEvent | null {
    try {
      const payload = toObject(JSON.parse(raw));
      if (!payload || (payload.kind !== 'message' && payload.kind !== 'notice')) return null;

      const sender_display_name =
        typeof payload.sender_display_name === 'string' && payload.sender_display_name.trim().length > 0
          ? payload.sender_display_name
          : typeof payload.sender_login === 'string'
            ? payload.sender_login
            : 'system';

      const sender_color =
        payload.kind === 'message' && typeof payload.sender_color === 'string' && payload.sender_color.trim().length > 0
          ? payload.sender_color
          : null;

      const parsedParts: WatchChatPart[] = [];
      if (Array.isArray(payload.parts)) {
        for (const part of payload.parts) {
          const parsedPart = toObject(part);
          if (!parsedPart || typeof parsedPart.kind !== 'string') continue;

          if (parsedPart.kind === 'text' && typeof parsedPart.text === 'string') {
            parsedParts.push({ kind: 'text', text: parsedPart.text });
            continue;
          }

          if (parsedPart.kind === 'emote' && typeof parsedPart.id === 'string') {
            parsedParts.push({
              kind: 'emote',
              id: parsedPart.id,
              code: typeof parsedPart.code === 'string' ? parsedPart.code : '',
              image_url: typeof parsedPart.image_url === 'string' ? parsedPart.image_url : undefined,
            });
          }
        }
      }

      return {
        id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
        kind: payload.kind,
        sender_display_name,
        sender_color,
        text: typeof payload.text === 'string' ? payload.text : '',
        parts: parsedParts,
      };
    } catch {
      return null;
    }
  }

  function isNearBottom(element: HTMLElement | null): boolean {
    if (!element) return true;
    const distanceFromBottom = element.scrollHeight - element.clientHeight - element.scrollTop;
    return distanceFromBottom <= AUTO_SCROLL_THRESHOLD_PX;
  }

  async function ensureHlsLoaded(path: string): Promise<boolean> {
    if (typeof window === 'undefined') return false;
    if ('Hls' in window) return true;

    const existing = document.querySelector<HTMLScriptElement>(`script[src="${path}"]`);
    if (existing) {
      await waitForHls();
      return 'Hls' in window;
    }

    const script = document.createElement('script');
    script.src = path;
    script.async = true;

    const loaded = await new Promise<boolean>((resolve) => {
      script.onload = () => resolve(true);
      script.onerror = () => resolve(false);
      document.head.appendChild(script);
    });

    if (!loaded) return false;
    await waitForHls();
    return 'Hls' in window;
  }

  async function waitForHls(): Promise<void> {
    let attempts = 0;
    while (typeof window !== 'undefined' && !('Hls' in window) && attempts < 50) {
      await new Promise((resolve) => setTimeout(resolve, 100));
      attempts += 1;
    }
  }

  async function readApiError(response: Response): Promise<string> {
    try {
      const payload = await response.json();
      const parsed = toObject(payload);
      if (parsed && typeof parsed.error === 'string') {
        return parsed.error;
      }
    } catch {
      // no-op
    }
    return 'request failed';
  }

  function toObject(value: unknown): Record<string, any> | null {
    return typeof value === 'object' && value !== null ? (value as Record<string, any>) : null;
  }

  function readMessage(error: unknown, fallback: string): string {
    if (error instanceof Error && error.message.trim().length > 0) {
      return error.message;
    }
    return fallback;
  }

  function handleDocumentClick(event: MouseEvent): void {
    const target = event.target as Node | null;
    if (!target) return;
    const targetElement = target instanceof Element ? target : target.parentElement;
    if (!targetElement) return;

    const clickedInsideComposer =
      (chatComposerEl && chatComposerEl.contains(target)) ||
      (emoteSearchEl && emoteSearchEl.contains(target));
    const clickedInsidePopup =
      targetElement.closest('.watch-emote-popup') !== null ||
      targetElement.closest('.watch-emote-suggestions') !== null;

    if (clickedInsideComposer || clickedInsidePopup) return;
    closeEmotePicker();
    closeEmoteSuggestions();
  }
</script>

<svelte:head>
  <title>{channelLogin ? `Watch ${channelLogin}` : 'Watch stream'} - Twitch Relay</title>
</svelte:head>

<section class="watch-page">
  <header class="watch-page-header">
    <div class="watch-page-meta">
      <strong>{channelLogin || 'stream'}</strong>
      <span>via Twitch Relay{appVersion ? ` · v${appVersion}` : ''}</span>
    </div>
    <div class="watch-page-actions">
      <a class="ui-nav-chip" href="/?view=channels">Back to channels</a>
      {#if !chatAvailable}
        <a class="ui-nav-chip" href={connectTwitchUrl}>Connect Twitch</a>
      {/if}
    </div>
  </header>

  {#if watchLoading}
    <div class="watch-loading-state">
      <p class="ui-muted">Loading watch session...</p>
    </div>
  {:else if watchError}
    <div class="watch-loading-state">
      <p class="ui-error">{watchError}</p>
    </div>
  {:else}
    <div class="watch-layout">
      <section class="watch-player-panel">
        <div class="watch-video-shell">
          <video bind:this={playerEl} class="watch-video" autoplay controls playsinline>
            Your browser cannot play this stream format.
          </video>
          
          <div class="watch-video-overlay-controls">
            <div class="watch-overlay-left">
              <button 
                type="button" 
                class="watch-overlay-btn go-live-btn" 
                onclick={goLive} 
                disabled={liveButtonIsLive}
                class:live={liveButtonIsLive}
              >
                {liveButtonIsLive ? 'Live' : 'Go Live'}
              </button>
            </div>
            <div class="watch-overlay-right">
              <button 
                type="button" 
                class="watch-overlay-btn quality-btn" 
                onclick={() => {
                  const select = document.querySelector('.watch-overlay-quality-menu');
                  if (select) {
                    select.classList.toggle('open');
                  }
                }}
              >
                {selectedQualityLabel()}
              </button>
              <div class="watch-overlay-quality-menu">
                <button 
                  type="button" 
                  class="watch-overlay-quality-item {qualityLevel === -1 ? 'active' : ''}"
                  onclick={() => setQuality(-1)}
                >
                  Auto
                </button>
                {#each hlsLevels as level, idx (idx)}
                  <button 
                    type="button" 
                    class="watch-overlay-quality-item {qualityLevel === idx ? 'active' : ''}"
                    onclick={() => setQuality(idx)}
                  >
                    {qualityLabel(level, idx)}
                  </button>
                {/each}
              </div>
            </div>
          </div>
        </div>

        {#if playbackError}
          <p class="ui-error">{playbackError}</p>
        {/if}
      </section>

      <aside class="watch-chat-panel">
        <div class="watch-chat-header">
          <strong>Chat</strong>
          <span class:watch-chat-status-live={chatConnected}>{chatStatus}</span>
        </div>

        {#if !chatAvailable}
          <div class="watch-chat-offline">
            <p class="ui-muted">Connect Twitch to read and send messages.</p>
            <a class="ui-nav-chip" href={connectTwitchUrl}>Connect Twitch</a>
          </div>
        {:else}
          <div class="watch-chat-messages" bind:this={chatMessagesEl} onscroll={handleChatScroll}>
            {#if chatMessages.length === 0}
              <p class="watch-chat-empty">Waiting for messages...</p>
            {/if}
            {#each chatMessages as message (message.id)}
              <div class={`watch-chat-message ${message.kind === 'notice' ? 'notice' : ''}`}>
                <span class="who" style:color={message.sender_color || undefined}>{message.sender_display_name}</span>
                <span>
                  {#if message.parts.length > 0}
                    {#each message.parts as part, index (`${message.id}-${index}`)}
                      {#if part.kind === 'emote'}
                        <img
                          class="watch-chat-emote"
                          src={part.image_url || emoteUrl(part.id)}
                          alt={part.code}
                          title={part.code}
                          loading="lazy"
                          decoding="async"
                        />
                      {:else}
                        {part.text}
                      {/if}
                    {/each}
                  {:else}
                    {message.text}
                  {/if}
                </span>
              </div>
            {/each}
          </div>

          {#if unreadChatCount > 0}
            <button type="button" class="watch-chat-unread-pill" onclick={jumpToLatestChat}>
              {chatUnreadLabel(unreadChatCount)}
            </button>
          {/if}

          <form class="watch-chat-form" onsubmit={handleChatSend}>
            <button
              type="button"
              class="watch-chat-emote-toggle"
              title="Open emote picker"
              aria-label="Open emote picker"
              onclick={toggleEmotePicker}
            >
              ☺
            </button>

            <div
              bind:this={chatComposerEl}
              class="watch-chat-input"
              contenteditable="true"
              role="textbox"
              tabindex="0"
              aria-label="Send a message"
              data-placeholder="Send a message"
              oninput={handleComposerInput}
              onclick={handleComposerClick}
              onpaste={handleComposerPaste}
              onkeydown={handleComposerKeydown}
            ></div>

            <button type="submit" class="watch-chat-send" disabled={chatSending}>Send</button>

            <div class={`watch-emote-suggestions ${emoteSuggestionsOpen ? 'open' : ''}`}>
              {#each emoteSuggestionItems as item, index (`${item.id}-${index}`)}
                <button
                  type="button"
                  class={`watch-emote-suggestion ${index === emoteSuggestionIndex ? 'active' : ''}`}
                  onmousedown={(event) => {
                    event.preventDefault();
                    const range = findActiveEmoteQuery();
                    applyEmoteCode(item.code, range);
                    closeEmoteSuggestions();
                  }}
                >
                  <img src={item.image_url} alt={item.code} loading="lazy" decoding="async" />
                  <span>{item.code}</span>
                </button>
              {/each}
            </div>

            <div class={`watch-emote-popup ${emotePickerOpen ? 'open' : ''}`}>
              <input
                bind:this={emoteSearchEl}
                class="watch-emote-search"
                type="text"
                placeholder="Search emotes"
                autocomplete="off"
                oninput={(event) => {
                  emoteSearchTerm = (event.currentTarget as HTMLInputElement).value;
                }}
              />

              <div class="watch-emote-groups">
                {#each groupedEmotes as group (group.key)}
                  <p class="watch-emote-group-title">{group.title}</p>
                  <div class="watch-emote-grid">
                    {#each group.items as item (item.id)}
                      <button
                        type="button"
                        class="watch-emote-item"
                        title={item.code}
                        aria-label={item.code}
                        onclick={() => {
                          applyEmoteCode(item.code, null);
                          placeComposerCaretAtEnd();
                        }}
                      >
                        <img src={item.image_url} alt={item.code} loading="lazy" decoding="async" />
                      </button>
                    {/each}
                  </div>
                {/each}

                {#if groupedEmotes.length === 0}
                  <div class="watch-emote-empty">
                    {emoteSearchTerm ? 'No emotes match your search.' : 'No emotes available.'}
                  </div>
                {/if}
              </div>
            </div>
          </form>
        {/if}
      </aside>
    </div>
  {/if}
</section>
