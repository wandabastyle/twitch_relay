  const CHAT_EMOTE_SCALE = '2.0';
  const watchConfig = window.__WATCH_CONFIG__ || {};
  const chatChannel = typeof watchConfig.channel === 'string' ? watchConfig.channel : '';
  const manifestUrl = typeof watchConfig.manifestUrl === 'string' ? watchConfig.manifestUrl : '';
  const video = document.getElementById('player');
  const videoContainer = document.getElementById('videoContainer');
  const controlsBar = document.getElementById('controlsBar');
  const playBtn = document.getElementById('playBtn');
  const playIcon = playBtn.querySelector('.play-icon');
  const pauseIcon = playBtn.querySelector('.pause-icon');
  const volumeBtn = document.getElementById('volumeBtn');
  const volumeHigh = volumeBtn.querySelector('.volume-high');
  const volumeMute = volumeBtn.querySelector('.volume-mute');
  const volumeSlider = document.getElementById('volumeSlider');
  const currentTimeEl = document.getElementById('currentTime');
  const durationEl = document.getElementById('duration');
  const progressBar = document.getElementById('progressBar');
  const progressBuffered = document.getElementById('progressBuffered');
  const progressPlayed = document.getElementById('progressPlayed');
  const goLiveBtn = document.getElementById('goLiveBtn');
  const qualityBtn = document.getElementById('qualityBtn');
  const qualityMenu = document.getElementById('qualityMenu');
  const chatStatus = document.getElementById('chatStatus');
  const chatMessages = document.getElementById('chatMessages');
  const chatForm = document.getElementById('chatForm');
  const chatComposer = document.getElementById('chatComposer');
  const chatSendBtn = document.getElementById('chatSendBtn');
  const chatEmoteBtn = document.getElementById('chatEmoteBtn');
  const emotePopup = document.getElementById('emotePopup');
  const emoteSearch = document.getElementById('emoteSearch');
  const emoteGroups = document.getElementById('emoteGroups');
  const emoteSuggestions = document.getElementById('emoteSuggestions');
  const chatPanel = document.querySelector('.chat-panel');
  const connectTwitchBtn = document.getElementById('connectTwitchBtn');
  const watchShell = document.querySelector('.watch-shell');
  let chatEvents = null;
  const fullscreenBtn = document.getElementById('fullscreenBtn');
  const MOBILE_LAYOUT_QUERY = window.matchMedia('(max-width: 700px)');
  const LIVE_STATUS_CACHE_KEY = 'twitchRelay.liveStatus';
  const LIVE_STATUS_REFRESH_MS = 45000;

  let hlsInstance = null;
  let debugVisible = false;
  let controlsTimeout = null;
  let liveStatusRefreshTimer = null;
  const CONTROLS_HIDE_DELAY_MS = 2000;
  const LIVE_BUTTON_ENTER_LIVE_SECS = 5.5;
  const LIVE_BUTTON_EXIT_LIVE_SECS = 7.5;
  let currentPlayingLevelIdx = -1;
  let userSelectedAuto = true;
  let attemptedRelayFallback =
    watchConfig.relay === true || new URLSearchParams(window.location.search).get('relay') === '1';
  let availableEmotes = [];
  let emotePickerLoaded = false;
  let emotePickerOpen = false;
  let emoteSearchTerm = '';
  let emoteSuggestionsOpen = false;
  let emoteSuggestionIndex = 0;
  let emoteSuggestionItems = [];
  let liveButtonIsLive = true;
  
  const debugOverlay = document.createElement('div');
  debugOverlay.style.cssText = 'position:fixed;top:50px;left:10px;background:rgba(0,0,0,0.9);color:#0f0;padding:10px;font-family:monospace;font-size:11px;z-index:99999;display:none;max-width:350px;border-radius:4px;';
  document.body.appendChild(debugOverlay);

  function readNumericStyle(element, propertyName) {
    const value = getComputedStyle(element).getPropertyValue(propertyName);
    const parsed = parseFloat(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }

  function currentAspectRatio() {
    if (video.videoWidth > 0 && video.videoHeight > 0) {
      return video.videoWidth / video.videoHeight;
    }

    const ratioText = (videoContainer.style.aspectRatio || getComputedStyle(videoContainer).aspectRatio || '16 / 9').trim();
    if (ratioText.includes('/')) {
      const parts = ratioText.split('/');
      const w = parseFloat(parts[0]);
      const h = parseFloat(parts[1]);
      if (Number.isFinite(w) && Number.isFinite(h) && w > 0 && h > 0) {
        return w / h;
      }
    }

    const numeric = parseFloat(ratioText);
    if (Number.isFinite(numeric) && numeric > 0) {
      return numeric;
    }

    return 16 / 9;
  }


  function formatTime(seconds) {
    if (!isFinite(seconds)) return '0:00';
    var h = Math.floor(seconds / 3600);
    var m = Math.floor((seconds % 3600) / 60);
    var s = Math.floor(seconds % 60);
    if (h > 0) {
      return h + ':' + (m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s;
    }
    return m + ':' + (s < 10 ? '0' : '') + s;
  }

  function formatBitrate(bitrate) {
    if (!bitrate) return '';
    var mbps = (bitrate / 1000000).toFixed(1);
    return mbps + ' Mbps';
  }

  function clamp(value, min, max) {
    return Math.min(max, Math.max(min, value));
  }


  function isObject(value) {
    return value !== null && typeof value === 'object' && !Array.isArray(value);
  }

  function emoteUrl(emoteId) {
    return 'https://static-cdn.jtvnw.net/emoticons/v2/' + encodeURIComponent(emoteId) + '/default/dark/' + CHAT_EMOTE_SCALE;
  }

  function normalizeEmoteCode(code) {
    if (typeof code !== 'string') return '';
    return code.trim();
  }

  function scoreEmote(code, query) {
    const c = code.toLowerCase();
    const q = query.toLowerCase();
    if (c === q) return 0;
    if (c.startsWith(q)) return 1;
    if (c.includes(q)) return 2;
    return 99;
  }

  function splitMessageSegments(input) {
    const out = [];
    let current = '';
    let currentWhitespace = null;

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

    if (current.length > 0) {
      out.push({ text: current, whitespace: currentWhitespace });
    }

    return out;
  }
