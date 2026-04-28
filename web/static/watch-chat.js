  async function chatRequest(path, init) {
    const response = await fetch(path, Object.assign({ credentials: 'same-origin' }, init || {}));
    if (!response.ok) {
      let message = 'chat request failed';
      try {
        const payload = await response.json();
        if (payload && typeof payload.error === 'string') {
          message = payload.error;
        }
      } catch (_) {}
      throw new Error(message);
    }
  }

  function appendChatEvent(event) {
    const row = document.createElement('div');
    row.className = 'chat-message' + (event.kind === 'notice' ? ' notice' : '');

    const who = document.createElement('span');
    who.className = 'who';
    who.textContent = event.sender_display_name || event.sender_login || 'system';
    if (event.kind === 'message' && typeof event.sender_color === 'string' && event.sender_color.trim().length > 0) {
      who.style.color = event.sender_color;
    }

    row.appendChild(who);

    const body = document.createElement('span');
    const parts = Array.isArray(event.parts) ? event.parts : [];

    if (parts.length > 0) {
      for (const part of parts) {
        if (part && part.kind === 'emote' && typeof part.id === 'string') {
          const img = document.createElement('img');
          img.className = 'chat-emote';
          img.src = typeof part.image_url === 'string' && part.image_url.trim().length > 0
            ? part.image_url
            : emoteUrl(part.id);
          img.alt = typeof part.code === 'string' ? part.code : '';
          img.title = typeof part.code === 'string' ? part.code : '';
          img.loading = 'lazy';
          img.decoding = 'async';
          body.appendChild(img);
          continue;
        }

        if (part && part.kind === 'text' && typeof part.text === 'string') {
          body.appendChild(document.createTextNode(part.text));
        }
      }
    } else {
      body.textContent = event.text || '';
    }

    row.appendChild(body);
    chatMessages.appendChild(row);
    chatMessages.scrollTop = chatMessages.scrollHeight;
  }

  function setChatAvailability(connected) {
    if (connected) {
      chatPanel.classList.remove('hidden');
      connectTwitchBtn.classList.add('hidden');
      chatComposer.contentEditable = 'true';
      chatSendBtn.disabled = false;
      chatEmoteBtn.disabled = false;
      syncPlayerLayout();
      return;
    }

    chatPanel.classList.add('hidden');
    connectTwitchBtn.classList.remove('hidden');
    closeEmotePicker();
    closeEmoteSuggestions();
    chatComposer.contentEditable = 'false';
    chatSendBtn.disabled = true;
    chatEmoteBtn.disabled = true;
    syncPlayerLayout();
  }

  async function checkTwitchAndInitChat() {
    try {
      const response = await fetch('/api/twitch/status', { credentials: 'same-origin' });
      if (!response.ok) {
        setChatAvailability(false);
        return;
      }

      const payload = await response.json();
      if (!payload || payload.connected !== true) {
        setChatAvailability(false);
        return;
      }

      setChatAvailability(true);
      await initChat();
    } catch (_) {
      setChatAvailability(false);
    }
  }

  async function initChat() {
    try {
      await chatRequest('/api/chat/subscribe', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ channel_login: chatChannel })
      });

      chatStatus.textContent = 'Connected to #' + chatChannel;
      ensureEmotesLoaded().catch(function() {
        // Emote picker can still retry on demand.
      });

      chatEvents = new EventSource('/api/chat/events/' + encodeURIComponent(chatChannel));
      chatEvents.addEventListener('chat', function(raw) {
        try {
          const event = JSON.parse(raw.data);
          appendChatEvent(event);
        } catch (_) {}
      });
      chatEvents.onerror = function() {
        chatStatus.textContent = 'Chat reconnecting...';
      };
      chatEvents.onopen = function() {
        chatStatus.textContent = 'Connected to #' + chatChannel;
      };
    } catch (error) {
      chatStatus.textContent = error && error.message ? error.message : 'Chat unavailable';
      chatComposer.contentEditable = 'false';
      chatSendBtn.disabled = true;
    }
  }
