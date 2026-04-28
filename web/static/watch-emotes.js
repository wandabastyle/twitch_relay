  function placeComposerCaretAtEnd() {
    chatComposer.focus();
    const range = document.createRange();
    range.selectNodeContents(chatComposer);
    range.collapse(false);
    const selection = window.getSelection();
    if (!selection) return;
    selection.removeAllRanges();
    selection.addRange(range);
  }

  function composerTextFromNode(node) {
    if (!node) return '';
    if (node.nodeType === Node.TEXT_NODE) {
      return node.textContent || '';
    }

    if (node.nodeType !== Node.ELEMENT_NODE) {
      return '';
    }

    const element = node;
    if (element.tagName === 'IMG') {
      return element.dataset.code || '';
    }
    if (element.tagName === 'BR') {
      return '\n';
    }

    let out = '';
    for (const child of Array.from(element.childNodes)) {
      out += composerTextFromNode(child);
    }
    return out;
  }

  function getComposerPlainText() {
    let out = '';
    for (const child of Array.from(chatComposer.childNodes)) {
      out += composerTextFromNode(child);
    }
    return out;
  }

  function buildEmoteMapByCode() {
    const emotesByCode = new Map();
    for (const item of availableEmotes) {
      if (typeof item.code === 'string' && typeof item.image_url === 'string' && typeof item.id === 'string') {
        emotesByCode.set(item.code, item);
      }
    }
    return emotesByCode;
  }

  function renderComposerFromPlainText(text) {
    const emotesByCode = buildEmoteMapByCode();
    chatComposer.innerHTML = '';

    if (!text) {
      return;
    }

    for (const segment of splitMessageSegments(text)) {
      if (segment.whitespace) {
        chatComposer.appendChild(document.createTextNode(segment.text));
        continue;
      }

      const match = emotesByCode.get(segment.text);
      if (!match) {
        chatComposer.appendChild(document.createTextNode(segment.text));
        continue;
      }

      const img = document.createElement('img');
      img.className = 'composer-emote';
      img.src = match.image_url;
      img.alt = match.code;
      img.title = match.code;
      img.dataset.code = match.code;
      img.dataset.id = match.id;
      img.loading = 'lazy';
      img.decoding = 'async';
      img.contentEditable = 'false';
      chatComposer.appendChild(img);
    }
  }

  function applyPlainTextToComposer(text) {
    renderComposerFromPlainText(text);
    placeComposerCaretAtEnd();
  }

  function findActiveEmoteQuery() {
    const full = getComposerPlainText();
    const match = full.match(/(^|\s):([A-Za-z0-9_]{2,})$/);
    if (!match) return null;
    const query = match[2];
    const tokenStart = full.length - query.length - 1;
    return { query: query, start: tokenStart, end: full.length };
  }

  function applyEmoteCode(code, queryRange) {
    const safeCode = normalizeEmoteCode(code);
    if (!safeCode) return;

    const full = getComposerPlainText();
    if (queryRange) {
      const before = full.slice(0, queryRange.start);
      const after = full.slice(queryRange.end);
      applyPlainTextToComposer(before + safeCode + ' ' + after);
      return;
    }

    applyPlainTextToComposer(full + safeCode + ' ');
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

  function normalizeComposerInput() {
    let plain = getComposerPlainText();
    plain = plain.replace(/[\r\n]+/g, ' ');
    if (plain.length > 500) {
      plain = plain.slice(0, 500);
    }

    renderComposerFromPlainText(plain);
    placeComposerCaretAtEnd();
  }

  function filteredPickerEmotes() {
    const term = emoteSearchTerm.trim().toLowerCase();
    if (!term) return availableEmotes;
    return availableEmotes.filter(function(item) {
      return item.code.toLowerCase().includes(term);
    });
  }

  function groupedPickerEmotes() {
    const filtered = filteredPickerEmotes();
    const groupedMap = new Map();
    for (const item of filtered) {
      const key = typeof item.group_key === 'string' ? item.group_key : 'global';
      const title = typeof item.group_name === 'string' && item.group_name.trim().length > 0
        ? item.group_name.trim()
        : 'Global';

      if (!groupedMap.has(key)) {
        groupedMap.set(key, { key: key, title: title, items: [] });
      }
      groupedMap.get(key).items.push(item);
    }

    return Array.from(groupedMap.values());
  }

  function renderEmotePicker() {
    emoteGroups.innerHTML = '';
    const grouped = groupedPickerEmotes();

    function renderGroup(group) {
      if (!group.items.length) return;
      const heading = document.createElement('p');
      heading.className = 'emote-group-title';
      heading.textContent = group.title;
      emoteGroups.appendChild(heading);

      const grid = document.createElement('div');
      grid.className = 'emote-grid';
      for (const item of group.items) {
        const button = document.createElement('button');
        button.type = 'button';
        button.className = 'emote-item';
        button.title = item.code;
        button.setAttribute('aria-label', item.code);
        button.addEventListener('click', function() {
          applyEmoteCode(item.code, null);
          placeComposerCaretAtEnd();
        });

        const img = document.createElement('img');
        img.src = item.image_url;
        img.alt = item.code;
        img.loading = 'lazy';
        img.decoding = 'async';
        button.appendChild(img);

        grid.appendChild(button);
      }
      emoteGroups.appendChild(grid);
    }

    for (const group of grouped) {
      renderGroup(group);
    }

    if (!grouped.length) {
      const empty = document.createElement('div');
      empty.className = 'emote-empty';
      empty.textContent = emoteSearchTerm ? 'No emotes match your search.' : 'No emotes available.';
      emoteGroups.appendChild(empty);
    }
  }

  function renderEmoteSuggestions() {
    emoteSuggestions.innerHTML = '';
    if (!emoteSuggestionsOpen || !emoteSuggestionItems.length) {
      emoteSuggestions.classList.remove('open');
      return;
    }

    emoteSuggestions.classList.add('open');
    for (let i = 0; i < emoteSuggestionItems.length; i++) {
      const item = emoteSuggestionItems[i];
      const row = document.createElement('div');
      row.className = 'emote-suggestion' + (i === emoteSuggestionIndex ? ' active' : '');
      row.addEventListener('mousedown', function(e) {
        e.preventDefault();
        const range = findActiveEmoteQuery();
        applyEmoteCode(item.code, range);
        closeEmoteSuggestions();
      });

      const img = document.createElement('img');
      img.src = item.image_url;
      img.alt = item.code;
      img.loading = 'lazy';
      img.decoding = 'async';
      row.appendChild(img);

      const label = document.createElement('span');
      label.textContent = item.code;
      row.appendChild(label);
      emoteSuggestions.appendChild(row);
    }
  }

  function closeEmoteSuggestions() {
    emoteSuggestionsOpen = false;
    emoteSuggestionItems = [];
    emoteSuggestionIndex = 0;
    renderEmoteSuggestions();
  }

  function refreshEmoteSuggestions() {
    const active = findActiveEmoteQuery();
    if (!active) {
      closeEmoteSuggestions();
      return;
    }

    if (!emotePickerLoaded) {
      ensureEmotesLoaded()
        .then(function() {
          refreshEmoteSuggestions();
        })
        .catch(function(error) {
          chatStatus.textContent = error && error.message ? error.message : 'Failed to load emotes';
        });
      return;
    }

    const q = active.query.toLowerCase();
    const ranked = availableEmotes
      .map(function(item) {
        return { item: item, score: scoreEmote(item.code, q) };
      })
      .filter(function(entry) { return entry.score < 99; })
      .sort(function(a, b) {
        if (a.score !== b.score) return a.score - b.score;
        return a.item.code.toLowerCase().localeCompare(b.item.code.toLowerCase());
      })
      .slice(0, 10)
      .map(function(entry) { return entry.item; });

    if (!ranked.length) {
      closeEmoteSuggestions();
      return;
    }

    emoteSuggestionsOpen = true;
    emoteSuggestionItems = ranked;
    emoteSuggestionIndex = Math.min(emoteSuggestionIndex, ranked.length - 1);
    renderEmoteSuggestions();
  }

  async function ensureEmotesLoaded() {
    if (emotePickerLoaded) return;
    const response = await fetch('/api/chat/emotes?channel_login=' + encodeURIComponent(chatChannel), {
      credentials: 'same-origin'
    });

    if (!response.ok) {
      let message = 'failed to load emotes';
      try {
        const payload = await response.json();
        if (payload && typeof payload.error === 'string') message = payload.error;
      } catch (_) {}
      throw new Error(message);
    }

    const payload = await response.json();
    const incoming = Array.isArray(payload && payload.emotes) ? payload.emotes : [];
    availableEmotes = incoming
      .filter(function(item) {
        return item && typeof item.id === 'string' && typeof item.code === 'string' && typeof item.image_url === 'string';
      })
      .map(function(item) {
        return {
          id: item.id,
          code: normalizeEmoteCode(item.code),
          image_url: item.image_url,
          group_key: typeof item.group_key === 'string' ? item.group_key : 'global',
          group_name: typeof item.group_name === 'string' ? item.group_name : 'Global'
        };
      })
      .filter(function(item) { return item.code.length > 0; });

    emotePickerLoaded = true;
    renderEmotePicker();
    normalizeComposerInput();
  }

  async function openEmotePicker() {
    closeEmoteSuggestions();
    emoteSearchTerm = '';
    emoteSearch.value = '';
    try {
      await ensureEmotesLoaded();
      renderEmotePicker();
      emotePopup.classList.add('open');
      emotePickerOpen = true;
      emoteSearch.focus();
    } catch (error) {
      chatStatus.textContent = error && error.message ? error.message : 'Failed to load emotes';
    }
  }

  function closeEmotePicker() {
    emotePopup.classList.remove('open');
    emotePickerOpen = false;
  }
