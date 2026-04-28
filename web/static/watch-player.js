  function syncPlayerLayout() {
    if (MOBILE_LAYOUT_QUERY.matches || document.fullscreenElement === videoContainer) {
      videoContainer.style.removeProperty('width');
      videoContainer.style.removeProperty('height');
      videoContainer.style.removeProperty('max-height');
      chatPanel.style.removeProperty('height');
      return;
    }

    const shellRect = watchShell.getBoundingClientRect();
    const shellPadX = readNumericStyle(watchShell, 'padding-left') + readNumericStyle(watchShell, 'padding-right');
    const shellPadY = readNumericStyle(watchShell, 'padding-top') + readNumericStyle(watchShell, 'padding-bottom');
    const availableWidth = Math.max(280, shellRect.width - shellPadX);
    const availableHeight = Math.max(220, shellRect.height - shellPadY);
    const chatWidth = chatPanel.getBoundingClientRect().width || 320;
    const gap = readNumericStyle(watchShell, 'column-gap') || 12;
    const ratio = currentAspectRatio();

    const widthByHeight = availableHeight * ratio;
    const widthBySpace = Math.max(280, availableWidth - chatWidth - gap);
    const videoWidth = Math.max(280, Math.min(widthByHeight, widthBySpace));
    const videoHeight = Math.max(160, videoWidth / ratio);

    videoContainer.style.width = Math.round(videoWidth) + 'px';
    videoContainer.style.height = Math.round(videoHeight) + 'px';
    videoContainer.style.maxHeight = Math.round(availableHeight) + 'px';
    chatPanel.style.height = Math.round(videoHeight) + 'px';
  }

  function applyVideoAspectRatio() {
    if (video.videoWidth > 0 && video.videoHeight > 0) {
      videoContainer.style.aspectRatio = video.videoWidth + ' / ' + video.videoHeight;
    }
    syncPlayerLayout();
  }

  function showControls() {
    videoContainer.classList.add('controls-visible');
    controlsBar.classList.add('visible');
    clearTimeout(controlsTimeout);
    if (!video.paused) {
      controlsTimeout = setTimeout(hideControls, CONTROLS_HIDE_DELAY_MS);
    }
  }

  function hideControls() {
    if (!video.paused) {
      videoContainer.classList.remove('controls-visible');
      controlsBar.classList.remove('visible');
    }
  }

  function getTimelineModel() {
    var duration = video.duration;
    if (isFinite(duration) && duration > 0) {
      return { mode: 'vod', start: 0, end: duration, length: duration, seekable: true };
    }

    if (video.seekable.length > 0) {
      var idx = video.seekable.length - 1;
      var start = video.seekable.start(idx);
      var end = video.seekable.end(idx);
      var length = end - start;
      if (isFinite(start) && isFinite(end) && length > 0) {
        return { mode: 'live-dvr', start: start, end: end, length: length, seekable: true };
      }
    }

    return { mode: 'live-not-seekable', start: 0, end: 0, length: 0, seekable: false };
  }

  function getTimelinePercent(time, timeline) {
    if (!timeline || timeline.length <= 0) return 0;
    return clamp((time - timeline.start) / timeline.length, 0, 1) * 100;
  }

  function updateTimelineInteractivity(timeline) {
    var canSeek = !!(timeline && timeline.seekable);
    progressBar.classList.toggle('disabled', !canSeek);
    progressBar.setAttribute('aria-disabled', canSeek ? 'false' : 'true');
    if (canSeek) {
      progressBar.removeAttribute('title');
    } else {
      progressBar.setAttribute('title', 'Live stream is not seekable');
    }
  }

  function updateTime() {
    var timeline = getTimelineModel();
    updateTimelineInteractivity(timeline);
    updateGoLiveButton(timeline);
    currentTimeEl.textContent = formatTime(video.currentTime);
    if (timeline.mode === 'vod') {
      durationEl.textContent = formatTime(timeline.end);
    } else if (timeline.mode === 'live-dvr') {
      durationEl.textContent = formatTime(timeline.length);
    } else {
      durationEl.textContent = 'LIVE';
    }
    progressPlayed.style.width = getTimelinePercent(video.currentTime, timeline) + '%';
  }

  function updateBuffer() {
    var timeline = getTimelineModel();
    var bufferedPercent = 0;
    if (video.buffered.length > 0) {
      var bufferedEnd = video.buffered.end(video.buffered.length - 1);
      if (timeline.mode === 'vod' && timeline.length > 0) {
        bufferedPercent = clamp(bufferedEnd / timeline.length, 0, 1) * 100;
      } else if (timeline.mode === 'live-dvr') {
        bufferedPercent = getTimelinePercent(bufferedEnd, timeline);
      }
    }
    progressBuffered.style.width = bufferedPercent + '%';
  }

  function updatePlayButton() {
    if (video.paused) {
      playIcon.style.display = 'block';
      pauseIcon.style.display = 'none';
    } else {
      playIcon.style.display = 'none';
      pauseIcon.style.display = 'block';
    }
  }

  function updateVolumeButton() {
    if (video.muted || video.volume === 0) {
      volumeHigh.style.display = 'none';
      volumeMute.style.display = 'block';
    } else {
      volumeHigh.style.display = 'block';
      volumeMute.style.display = 'none';
    }
  }

  function togglePlay() {
    if (video.paused) {
      video.play();
    } else {
      video.pause();
    }
  }

  function toggleMute() {
    video.muted = !video.muted;
    updateVolumeButton();
  }

  function seek(e) {
    var timeline = getTimelineModel();
    if (!timeline.seekable || timeline.length <= 0) return;
    var rect = progressBar.getBoundingClientRect();
    if (rect.width <= 0) return;
    var percent = clamp((e.clientX - rect.left) / rect.width, 0, 1);
    if (timeline.mode === 'vod') {
      video.currentTime = percent * timeline.length;
    } else {
      video.currentTime = timeline.start + (percent * timeline.length);
    }
  }

  function toggleFullscreen() {
    if (document.fullscreenElement) {
      document.exitFullscreen();
    } else {
      videoContainer.requestFullscreen();
    }
  }

  function updateGoLiveButton(timeline) {
    if (!timeline.seekable) {
      liveButtonIsLive = true;
      goLiveBtn.textContent = 'Live';
      goLiveBtn.classList.add('live');
      goLiveBtn.disabled = true;
      return;
    }

    var lag = Math.max(0, timeline.end - video.currentTime);
    if (liveButtonIsLive) {
      if (lag > LIVE_BUTTON_EXIT_LIVE_SECS) {
        liveButtonIsLive = false;
      }
    } else if (lag < LIVE_BUTTON_ENTER_LIVE_SECS) {
      liveButtonIsLive = true;
    }

    goLiveBtn.textContent = liveButtonIsLive ? 'Live' : 'Go Live';
    goLiveBtn.classList.toggle('live', liveButtonIsLive);
    goLiveBtn.disabled = liveButtonIsLive;
  }

  function goLive() {
    var liveSyncPosition = null;
    if (hlsInstance && Number.isFinite(hlsInstance.liveSyncPosition)) {
      liveSyncPosition = hlsInstance.liveSyncPosition;
    }

    if (liveSyncPosition !== null) {
      video.currentTime = liveSyncPosition;
      showControls();
      return;
    }

    var timeline = getTimelineModel();
    if (!timeline.seekable || timeline.length <= 0) return;
    video.currentTime = timeline.end;
    showControls();
  }

  function updateDebug() {
    if (!debugVisible) return;
    var bufferedRanges = [];
    for (var i = 0; i < video.buffered.length; i++) {
      bufferedRanges.push({ start: video.buffered.start(i).toFixed(1), end: video.buffered.end(i).toFixed(1) });
    }
    var currentQuality = 'Auto';
    if (hlsInstance && hlsInstance.currentLevel >= 0 && hlsInstance.levels && hlsInstance.levels[hlsInstance.currentLevel]) {
      currentQuality = hlsInstance.levels[hlsInstance.currentLevel].height + 'p';
    }
    debugOverlay.innerHTML = '' +
      '<div style="margin-bottom:8px;font-weight:bold;">Debug (Shift+D)</div>' +
      '<div>Quality: ' + currentQuality + '</div>' +
      '<div>Levels: ' + (hlsInstance ? hlsInstance.levels.length : 0) + '</div>' +
      '<div>currentTime: ' + video.currentTime.toFixed(1) + '</div>' +
      '<div>paused: ' + video.paused + '</div>' +
      '<div>buffered: ' + JSON.stringify(bufferedRanges) + '</div>';
  }


  async function refreshLiveStatusCache() {
    try {
      const response = await fetch('/api/live-status', { credentials: 'same-origin' });
      if (!response.ok) {
        return;
      }

      const payload = await response.json();
      if (!isObject(payload) || !isObject(payload.channels)) {
        return;
      }

      window.sessionStorage.setItem(
        LIVE_STATUS_CACHE_KEY,
        JSON.stringify({
          timestamp: Date.now(),
          data: {
            channels: payload.channels
          }
        })
      );
    } catch (_) {}
  }

  function handleVisibilityChange() {
    if (document.visibilityState === 'visible') {
      void refreshLiveStatusCache();
    }
  }

  function startLiveStatusRefreshLoop() {
    void refreshLiveStatusCache();

    if (liveStatusRefreshTimer) {
      clearInterval(liveStatusRefreshTimer);
    }

    liveStatusRefreshTimer = setInterval(function() {
      if (document.visibilityState !== 'visible') {
        return;
      }
      void refreshLiveStatusCache();
    }, LIVE_STATUS_REFRESH_MS);
  }

  function buildQualityMenu(levels, currentLevelIdx) {
    qualityMenu.innerHTML = '';
    var autoItem = document.createElement('div');
    autoItem.className = 'quality-menu-item' + (currentLevelIdx === -1 ? ' active' : '');
    autoItem.innerHTML = '<span>Auto</span>';
    autoItem.onclick = function() { setLevel(-1); };
    qualityMenu.appendChild(autoItem);
    for (var i = 0; i < levels.length; i++) {
      var level = levels[i];
      var item = document.createElement('div');
      item.className = 'quality-menu-item' + (currentLevelIdx === i ? ' active' : '');
      item.innerHTML = '<span>' + level.height + 'p</span><span class="bitrate">' + formatBitrate(level.bitrate) + '</span>';
      (function(idx) { item.onclick = function() { setLevel(idx); }; })(i);
      qualityMenu.appendChild(item);
    }
  }

  function setLevel(levelIdx) {
    if (!hlsInstance) return;
    hlsInstance.currentLevel = levelIdx;
    userSelectedAuto = (levelIdx === -1);
    qualityMenu.classList.remove('open');
    if (levelIdx === -1) {
      var level = hlsInstance.levels && hlsInstance.levels[currentPlayingLevelIdx];
      if (level) {
        qualityBtn.textContent = 'Auto (' + level.height + 'p)';
      } else {
        qualityBtn.textContent = 'Auto';
      }
    } else if (hlsInstance.levels && hlsInstance.levels[levelIdx]) {
      qualityBtn.textContent = hlsInstance.levels[levelIdx].height + 'p';
    }
    buildQualityMenu(hlsInstance.levels || [], levelIdx);
  }

  if (Hls.isSupported()) {
    // Twitch-like low-latency HLS profile:
    // hls.js is the single source of truth for live timing.
    // Target roughly 5-7s behind live with enough buffer/retry tolerance
    // to avoid stalls on imperfect networks.
    hlsInstance = new Hls({
      startPosition: -6,
      lowLatencyMode: true,
      liveSyncDuration: 6,
      liveMaxLatencyDuration: 14,
      maxLiveSyncPlaybackRate: 1.1,
      maxBufferLength: 20,
      maxMaxBufferLength: 45,
      backBufferLength: 15,
      manifestLoadingTimeOut: 15000,
      levelLoadingTimeOut: 15000,
      fragLoadingTimeOut: 20000,
      manifestLoadingMaxRetry: 3,
      levelLoadingMaxRetry: 3,
      fragLoadingMaxRetry: 5,
      manifestLoadingRetryDelay: 750,
      levelLoadingRetryDelay: 750,
      fragLoadingRetryDelay: 750
    });
    hlsInstance.currentLevel = -1;
    hlsInstance.on(Hls.Events.MANIFEST_PARSED, function(e, data) {
      console.log('[HLS] ' + data.levels.length + ' quality levels loaded');
      qualityBtn.textContent = 'Auto';
      buildQualityMenu(data.levels, hlsInstance.currentLevel);
    });
    hlsInstance.on(Hls.Events.LEVEL_SWITCHED, function(e, data) {
      currentPlayingLevelIdx = data.level;
      buildQualityMenu(hlsInstance.levels, data.level);
      var level = hlsInstance.levels && hlsInstance.levels[data.level];
      if (level) {
        if (userSelectedAuto) {
          hlsInstance.currentLevel = -1;
          qualityBtn.textContent = 'Auto (' + level.height + 'p)';
        } else {
          qualityBtn.textContent = level.height + 'p';
        }
      }
    });
    hlsInstance.on(Hls.Events.ERROR, function(e, data) {
      console.error('[HLS] ERROR:', data.details, data.fatal ? '(fatal)' : '');
      if (data.fatal) {
        if (!attemptedRelayFallback) {
          attemptedRelayFallback = true;
          var fallbackUrl = new URL(window.location.href);
          fallbackUrl.searchParams.set('relay', '1');
          window.location.assign(fallbackUrl.toString());
          return;
        }
        video.dispatchEvent(new CustomEvent('stream-error', { detail: data }));
      }
    });
    hlsInstance.loadSource(manifestUrl);
    hlsInstance.attachMedia(video);
  } else if (video.canPlayType('application/vnd.apple.mpegurl')) {
    video.src = manifestUrl;
  } else {
    video.dispatchEvent(new CustomEvent('stream-error', { detail: { type: 'not-supported' } }));
  }

  video.addEventListener('stream-error', function() {
    document.body.innerHTML = '<div class="error-screen"><div class="error-box"><p>Stream unavailable. The channel may be offline or not accessible.</p></div></div>';
  });
