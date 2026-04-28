  document.addEventListener('keydown', function(e) {
    if (e.shiftKey && (e.key === 'D' || e.key === 'd')) {
      debugVisible = !debugVisible;
      debugOverlay.style.display = debugVisible ? 'block' : 'none';
      if (debugVisible) updateDebug();
    }
  });

  playBtn.addEventListener('click', togglePlay);
  video.addEventListener('click', function(e) {
    if (e.target === video) togglePlay();
  });
  volumeBtn.addEventListener('click', toggleMute);
  chatEmoteBtn.addEventListener('click', function() {
    if (emotePickerOpen) {
      closeEmotePicker();
      placeComposerCaretAtEnd();
    } else {
      openEmotePicker();
    }
  });
  emoteSearch.addEventListener('input', function() {
    emoteSearchTerm = emoteSearch.value || '';
    renderEmotePicker();
  });
  volumeSlider.addEventListener('input', function() {
    video.volume = this.value;
    video.muted = false;
    updateVolumeButton();
  });
  progressBar.addEventListener('click', seek);
  goLiveBtn.addEventListener('click', goLive);
  fullscreenBtn.addEventListener('click', toggleFullscreen);

  video.addEventListener('play', function() {
    updatePlayButton();
    showControls();
  });
  video.addEventListener('pause', function() {
    updatePlayButton();
    showControls();
  });
  video.addEventListener('timeupdate', function() {
    updateTime();
    updateBuffer();
    updateDebug();
  });
  video.addEventListener('progress', updateBuffer);
  video.addEventListener('durationchange', function() {
    updateTime();
    updateBuffer();
  });
  video.addEventListener('loadedmetadata', function() {
    updateTime();
    updateBuffer();
    updatePlayButton();
    updateVolumeButton();
    applyVideoAspectRatio();
  });
  video.addEventListener('volumechange', updateVolumeButton);
  video.addEventListener('waiting', function() { video.style.opacity = '0.7'; });
  video.addEventListener('playing', function() { video.style.opacity = '1'; });
  videoContainer.addEventListener('mouseenter', showControls);
  videoContainer.addEventListener('mousemove', showControls);
  videoContainer.addEventListener('mouseleave', function() { if (!video.paused) hideControls(); });
  chatComposer.addEventListener('input', function() {
    normalizeComposerInput();
    refreshEmoteSuggestions();
  });
  chatComposer.addEventListener('click', function() {
    placeComposerCaretAtEnd();
    refreshEmoteSuggestions();
  });
  chatComposer.addEventListener('paste', function(e) {
    e.preventDefault();
    const text = ((e.clipboardData && e.clipboardData.getData('text/plain')) || '').replace(/[\r\n]+/g, ' ');
    if (!text) return;

    const plain = getComposerPlainText();
    applyPlainTextToComposer((plain + text).slice(0, 500));
    refreshEmoteSuggestions();
  });
  chatComposer.addEventListener('keydown', function(e) {
    if (e.key === 'Enter' && !(emoteSuggestionsOpen && emoteSuggestionItems.length)) {
      e.preventDefault();
      chatForm.requestSubmit();
      return;
    }

    if (!emoteSuggestionsOpen || !emoteSuggestionItems.length) {
      if (e.key === 'Escape') {
        closeEmotePicker();
      }
      return;
    }

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      emoteSuggestionIndex = (emoteSuggestionIndex + 1) % emoteSuggestionItems.length;
      renderEmoteSuggestions();
      return;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      emoteSuggestionIndex = (emoteSuggestionIndex - 1 + emoteSuggestionItems.length) % emoteSuggestionItems.length;
      renderEmoteSuggestions();
      return;
    }
    if (e.key === 'Tab' || e.key === 'Enter') {
      e.preventDefault();
      const selected = emoteSuggestionItems[emoteSuggestionIndex];
      const range = findActiveEmoteQuery();
      if (selected && range) {
        applyEmoteCode(selected.code, range);
      }
      closeEmoteSuggestions();
      return;
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      closeEmoteSuggestions();
      return;
    }
  });
  window.addEventListener('resize', syncPlayerLayout);
  document.addEventListener('fullscreenchange', syncPlayerLayout);
  document.addEventListener('click', function(e) {
    if (!chatForm.contains(e.target)) {
      closeEmotePicker();
      closeEmoteSuggestions();
      return;
    }

    if (e.target === chatComposer) {
      placeComposerCaretAtEnd();
    }
  });
  if (typeof MOBILE_LAYOUT_QUERY.addEventListener === 'function') {
    MOBILE_LAYOUT_QUERY.addEventListener('change', syncPlayerLayout);
  }


  qualityBtn.addEventListener('click', function(e) {
    e.stopPropagation();
    qualityMenu.classList.toggle('open');
  });
  document.addEventListener('click', function(e) {
    if (!qualityMenu.contains(e.target) && e.target !== qualityBtn) {
      qualityMenu.classList.remove('open');
    }
  });

  chatForm.addEventListener('submit', async function(e) {
    e.preventDefault();
    closeEmotePicker();
    closeEmoteSuggestions();
    const text = getComposerPlainText().trim();
    if (!text) return;

    chatSendBtn.disabled = true;
    try {
      await chatRequest('/api/chat/send', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ channel_login: chatChannel, message: text })
      });
      chatComposer.innerHTML = '';
      chatStatus.textContent = 'Connected to #' + chatChannel;
      placeComposerCaretAtEnd();
    } catch (error) {
      chatStatus.textContent = error && error.message ? error.message : 'Failed to send message';
    } finally {
      chatSendBtn.disabled = false;
    }
  });

  window.addEventListener('beforeunload', function() {
    fetch('/api/chat/subscribe/' + encodeURIComponent(chatChannel), {
      method: 'DELETE',
      credentials: 'same-origin',
      keepalive: true
    });
    if (chatEvents) {
      chatEvents.close();
    }
    if (typeof MOBILE_LAYOUT_QUERY.removeEventListener === 'function') {
      MOBILE_LAYOUT_QUERY.removeEventListener('change', syncPlayerLayout);
    }
    document.removeEventListener('visibilitychange', handleVisibilityChange);
    if (liveStatusRefreshTimer) {
      clearInterval(liveStatusRefreshTimer);
      liveStatusRefreshTimer = null;
    }
  });

  document.addEventListener('visibilitychange', handleVisibilityChange);
  startLiveStatusRefreshLoop();
  checkTwitchAndInitChat();
