(function() {
	//#region src/lib/watch/entry.js
	var watchConfig = window.__WATCH_CONFIG__ || {};
	var chatChannel = typeof watchConfig.channel === "string" ? watchConfig.channel : "";
	var manifestUrl = typeof watchConfig.manifestUrl === "string" ? watchConfig.manifestUrl : "";
	var video = document.getElementById("player");
	var videoContainer = document.getElementById("videoContainer");
	var controlsBar = document.getElementById("controlsBar");
	var playBtn = document.getElementById("playBtn");
	var playIcon = playBtn.querySelector(".play-icon");
	var pauseIcon = playBtn.querySelector(".pause-icon");
	var volumeBtn = document.getElementById("volumeBtn");
	var volumeHigh = volumeBtn.querySelector(".volume-high");
	var volumeMute = volumeBtn.querySelector(".volume-mute");
	var volumeSlider = document.getElementById("volumeSlider");
	var currentTimeEl = document.getElementById("currentTime");
	var durationEl = document.getElementById("duration");
	var progressBar = document.getElementById("progressBar");
	var progressBuffered = document.getElementById("progressBuffered");
	var progressPlayed = document.getElementById("progressPlayed");
	var goLiveBtn = document.getElementById("goLiveBtn");
	var qualityBtn = document.getElementById("qualityBtn");
	var qualityMenu = document.getElementById("qualityMenu");
	var chatStatus = document.getElementById("chatStatus");
	var chatMessages = document.getElementById("chatMessages");
	var chatForm = document.getElementById("chatForm");
	var chatComposer = document.getElementById("chatComposer");
	var chatSendBtn = document.getElementById("chatSendBtn");
	var chatEmoteBtn = document.getElementById("chatEmoteBtn");
	var emotePopup = document.getElementById("emotePopup");
	var emoteSearch = document.getElementById("emoteSearch");
	var emoteGroups = document.getElementById("emoteGroups");
	var emoteSuggestions = document.getElementById("emoteSuggestions");
	var chatPanel = document.querySelector(".chat-panel");
	var connectTwitchBtn = document.getElementById("connectTwitchBtn");
	var watchShell = document.querySelector(".watch-shell");
	var chatEvents = null;
	var fullscreenBtn = document.getElementById("fullscreenBtn");
	var MOBILE_LAYOUT_QUERY = window.matchMedia("(max-width: 700px)");
	var LIVE_STATUS_CACHE_KEY = "twitchRelay.liveStatus";
	var LIVE_STATUS_REFRESH_MS = 45e3;
	var hlsInstance = null;
	var debugVisible = false;
	var controlsTimeout = null;
	var liveStatusRefreshTimer = null;
	var CONTROLS_HIDE_DELAY_MS = 2e3;
	var LIVE_BUTTON_ENTER_LIVE_SECS = 5.5;
	var LIVE_BUTTON_EXIT_LIVE_SECS = 7.5;
	var currentPlayingLevelIdx = -1;
	var userSelectedAuto = true;
	var attemptedRelayFallback = watchConfig.relay === true || new URLSearchParams(window.location.search).get("relay") === "1";
	var availableEmotes = [];
	var emotePickerLoaded = false;
	var emotePickerOpen = false;
	var emoteSearchTerm = "";
	var emoteSuggestionsOpen = false;
	var emoteSuggestionIndex = 0;
	var emoteSuggestionItems = [];
	var liveButtonIsLive = true;
	var debugOverlay = document.createElement("div");
	debugOverlay.style.cssText = "position:fixed;top:50px;left:10px;background:rgba(0,0,0,0.9);color:#0f0;padding:10px;font-family:monospace;font-size:11px;z-index:99999;display:none;max-width:350px;border-radius:4px;";
	document.body.appendChild(debugOverlay);
	function readNumericStyle(element, propertyName) {
		const value = getComputedStyle(element).getPropertyValue(propertyName);
		const parsed = parseFloat(value);
		return Number.isFinite(parsed) ? parsed : 0;
	}
	function currentAspectRatio() {
		if (video.videoWidth > 0 && video.videoHeight > 0) return video.videoWidth / video.videoHeight;
		const ratioText = (videoContainer.style.aspectRatio || getComputedStyle(videoContainer).aspectRatio || "16 / 9").trim();
		if (ratioText.includes("/")) {
			const parts = ratioText.split("/");
			const w = parseFloat(parts[0]);
			const h = parseFloat(parts[1]);
			if (Number.isFinite(w) && Number.isFinite(h) && w > 0 && h > 0) return w / h;
		}
		const numeric = parseFloat(ratioText);
		if (Number.isFinite(numeric) && numeric > 0) return numeric;
		return 16 / 9;
	}
	function syncPlayerLayout() {
		if (MOBILE_LAYOUT_QUERY.matches || document.fullscreenElement === videoContainer) {
			videoContainer.style.removeProperty("width");
			videoContainer.style.removeProperty("height");
			videoContainer.style.removeProperty("max-height");
			chatPanel.style.removeProperty("height");
			return;
		}
		const shellRect = watchShell.getBoundingClientRect();
		const shellPadX = readNumericStyle(watchShell, "padding-left") + readNumericStyle(watchShell, "padding-right");
		const shellPadY = readNumericStyle(watchShell, "padding-top") + readNumericStyle(watchShell, "padding-bottom");
		const availableWidth = Math.max(280, shellRect.width - shellPadX);
		const availableHeight = Math.max(220, shellRect.height - shellPadY);
		const chatWidth = chatPanel.getBoundingClientRect().width || 320;
		const gap = readNumericStyle(watchShell, "column-gap") || 12;
		const ratio = currentAspectRatio();
		const widthByHeight = availableHeight * ratio;
		const widthBySpace = Math.max(280, availableWidth - chatWidth - gap);
		const videoWidth = Math.max(280, Math.min(widthByHeight, widthBySpace));
		const videoHeight = Math.max(160, videoWidth / ratio);
		videoContainer.style.width = Math.round(videoWidth) + "px";
		videoContainer.style.height = Math.round(videoHeight) + "px";
		videoContainer.style.maxHeight = Math.round(availableHeight) + "px";
		chatPanel.style.height = Math.round(videoHeight) + "px";
	}
	function applyVideoAspectRatio() {
		if (video.videoWidth > 0 && video.videoHeight > 0) videoContainer.style.aspectRatio = video.videoWidth + " / " + video.videoHeight;
		syncPlayerLayout();
	}
	function showControls() {
		videoContainer.classList.add("controls-visible");
		controlsBar.classList.add("visible");
		clearTimeout(controlsTimeout);
		if (!video.paused) controlsTimeout = setTimeout(hideControls, CONTROLS_HIDE_DELAY_MS);
	}
	function hideControls() {
		if (!video.paused) {
			videoContainer.classList.remove("controls-visible");
			controlsBar.classList.remove("visible");
		}
	}
	function formatTime(seconds) {
		if (!isFinite(seconds)) return "0:00";
		var h = Math.floor(seconds / 3600);
		var m = Math.floor(seconds % 3600 / 60);
		var s = Math.floor(seconds % 60);
		if (h > 0) return h + ":" + (m < 10 ? "0" : "") + m + ":" + (s < 10 ? "0" : "") + s;
		return m + ":" + (s < 10 ? "0" : "") + s;
	}
	function formatBitrate(bitrate) {
		if (!bitrate) return "";
		return (bitrate / 1e6).toFixed(1) + " Mbps";
	}
	function clamp(value, min, max) {
		return Math.min(max, Math.max(min, value));
	}
	function getTimelineModel() {
		var duration = video.duration;
		if (isFinite(duration) && duration > 0) return {
			mode: "vod",
			start: 0,
			end: duration,
			length: duration,
			seekable: true
		};
		if (video.seekable.length > 0) {
			var idx = video.seekable.length - 1;
			var start = video.seekable.start(idx);
			var end = video.seekable.end(idx);
			var length = end - start;
			if (isFinite(start) && isFinite(end) && length > 0) return {
				mode: "live-dvr",
				start,
				end,
				length,
				seekable: true
			};
		}
		return {
			mode: "live-not-seekable",
			start: 0,
			end: 0,
			length: 0,
			seekable: false
		};
	}
	function getTimelinePercent(time, timeline) {
		if (!timeline || timeline.length <= 0) return 0;
		return clamp((time - timeline.start) / timeline.length, 0, 1) * 100;
	}
	function updateTimelineInteractivity(timeline) {
		var canSeek = !!(timeline && timeline.seekable);
		progressBar.classList.toggle("disabled", !canSeek);
		progressBar.setAttribute("aria-disabled", canSeek ? "false" : "true");
		if (canSeek) progressBar.removeAttribute("title");
		else progressBar.setAttribute("title", "Live stream is not seekable");
	}
	function updateTime() {
		var timeline = getTimelineModel();
		updateTimelineInteractivity(timeline);
		updateGoLiveButton(timeline);
		currentTimeEl.textContent = formatTime(video.currentTime);
		if (timeline.mode === "vod") durationEl.textContent = formatTime(timeline.end);
		else if (timeline.mode === "live-dvr") durationEl.textContent = formatTime(timeline.length);
		else durationEl.textContent = "LIVE";
		progressPlayed.style.width = getTimelinePercent(video.currentTime, timeline) + "%";
	}
	function updateBuffer() {
		var timeline = getTimelineModel();
		var bufferedPercent = 0;
		if (video.buffered.length > 0) {
			var bufferedEnd = video.buffered.end(video.buffered.length - 1);
			if (timeline.mode === "vod" && timeline.length > 0) bufferedPercent = clamp(bufferedEnd / timeline.length, 0, 1) * 100;
			else if (timeline.mode === "live-dvr") bufferedPercent = getTimelinePercent(bufferedEnd, timeline);
		}
		progressBuffered.style.width = bufferedPercent + "%";
	}
	function updatePlayButton() {
		if (video.paused) {
			playIcon.style.display = "block";
			pauseIcon.style.display = "none";
		} else {
			playIcon.style.display = "none";
			pauseIcon.style.display = "block";
		}
	}
	function updateVolumeButton() {
		if (video.muted || video.volume === 0) {
			volumeHigh.style.display = "none";
			volumeMute.style.display = "block";
		} else {
			volumeHigh.style.display = "block";
			volumeMute.style.display = "none";
		}
	}
	function togglePlay() {
		if (video.paused) video.play();
		else video.pause();
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
		if (timeline.mode === "vod") video.currentTime = percent * timeline.length;
		else video.currentTime = timeline.start + percent * timeline.length;
	}
	function toggleFullscreen() {
		if (document.fullscreenElement) document.exitFullscreen();
		else videoContainer.requestFullscreen();
	}
	function updateGoLiveButton(timeline) {
		if (!timeline.seekable) {
			liveButtonIsLive = true;
			goLiveBtn.textContent = "Live";
			goLiveBtn.classList.add("live");
			goLiveBtn.disabled = true;
			return;
		}
		var lag = Math.max(0, timeline.end - video.currentTime);
		if (liveButtonIsLive) {
			if (lag > LIVE_BUTTON_EXIT_LIVE_SECS) liveButtonIsLive = false;
		} else if (lag < LIVE_BUTTON_ENTER_LIVE_SECS) liveButtonIsLive = true;
		goLiveBtn.textContent = liveButtonIsLive ? "Live" : "Go Live";
		goLiveBtn.classList.toggle("live", liveButtonIsLive);
		goLiveBtn.disabled = liveButtonIsLive;
	}
	function goLive() {
		var liveSyncPosition = null;
		if (hlsInstance && Number.isFinite(hlsInstance.liveSyncPosition)) liveSyncPosition = hlsInstance.liveSyncPosition;
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
		for (var i = 0; i < video.buffered.length; i++) bufferedRanges.push({
			start: video.buffered.start(i).toFixed(1),
			end: video.buffered.end(i).toFixed(1)
		});
		var currentQuality = "Auto";
		if (hlsInstance && hlsInstance.currentLevel >= 0 && hlsInstance.levels && hlsInstance.levels[hlsInstance.currentLevel]) currentQuality = hlsInstance.levels[hlsInstance.currentLevel].height + "p";
		debugOverlay.innerHTML = "<div style=\"margin-bottom:8px;font-weight:bold;\">Debug (Shift+D)</div><div>Quality: " + currentQuality + "</div><div>Levels: " + (hlsInstance ? hlsInstance.levels.length : 0) + "</div><div>currentTime: " + video.currentTime.toFixed(1) + "</div><div>paused: " + video.paused + "</div><div>buffered: " + JSON.stringify(bufferedRanges) + "</div>";
	}
	function isObject(value) {
		return value !== null && typeof value === "object" && !Array.isArray(value);
	}
	async function refreshLiveStatusCache() {
		try {
			const response = await fetch("/api/live-status", { credentials: "same-origin" });
			if (!response.ok) return;
			const payload = await response.json();
			if (!isObject(payload) || !isObject(payload.channels)) return;
			window.sessionStorage.setItem(LIVE_STATUS_CACHE_KEY, JSON.stringify({
				timestamp: Date.now(),
				data: { channels: payload.channels }
			}));
		} catch {}
	}
	function handleVisibilityChange() {
		if (document.visibilityState === "visible") refreshLiveStatusCache();
	}
	function startLiveStatusRefreshLoop() {
		refreshLiveStatusCache();
		if (liveStatusRefreshTimer) clearInterval(liveStatusRefreshTimer);
		liveStatusRefreshTimer = setInterval(function() {
			if (document.visibilityState !== "visible") return;
			refreshLiveStatusCache();
		}, LIVE_STATUS_REFRESH_MS);
	}
	document.addEventListener("keydown", function(e) {
		if (e.shiftKey && (e.key === "D" || e.key === "d")) {
			debugVisible = !debugVisible;
			debugOverlay.style.display = debugVisible ? "block" : "none";
			if (debugVisible) updateDebug();
		}
	});
	playBtn.addEventListener("click", togglePlay);
	video.addEventListener("click", function(e) {
		if (e.target === video) togglePlay();
	});
	volumeBtn.addEventListener("click", toggleMute);
	chatEmoteBtn.addEventListener("click", function() {
		if (emotePickerOpen) {
			closeEmotePicker();
			placeComposerCaretAtEnd();
		} else openEmotePicker();
	});
	emoteSearch.addEventListener("input", function() {
		emoteSearchTerm = emoteSearch.value || "";
		renderEmotePicker();
	});
	volumeSlider.addEventListener("input", function() {
		video.volume = this.value;
		video.muted = false;
		updateVolumeButton();
	});
	progressBar.addEventListener("click", seek);
	goLiveBtn.addEventListener("click", goLive);
	fullscreenBtn.addEventListener("click", toggleFullscreen);
	video.addEventListener("play", function() {
		updatePlayButton();
		showControls();
	});
	video.addEventListener("pause", function() {
		updatePlayButton();
		showControls();
	});
	video.addEventListener("timeupdate", function() {
		updateTime();
		updateBuffer();
		updateDebug();
	});
	video.addEventListener("progress", updateBuffer);
	video.addEventListener("durationchange", function() {
		updateTime();
		updateBuffer();
	});
	video.addEventListener("loadedmetadata", function() {
		updateTime();
		updateBuffer();
		updatePlayButton();
		updateVolumeButton();
		applyVideoAspectRatio();
	});
	video.addEventListener("volumechange", updateVolumeButton);
	video.addEventListener("waiting", function() {
		video.style.opacity = "0.7";
	});
	video.addEventListener("playing", function() {
		video.style.opacity = "1";
	});
	videoContainer.addEventListener("mouseenter", showControls);
	videoContainer.addEventListener("mousemove", showControls);
	videoContainer.addEventListener("mouseleave", function() {
		if (!video.paused) hideControls();
	});
	chatComposer.addEventListener("input", function() {
		normalizeComposerInput();
		refreshEmoteSuggestions();
	});
	chatComposer.addEventListener("click", function() {
		placeComposerCaretAtEnd();
		refreshEmoteSuggestions();
	});
	chatComposer.addEventListener("paste", function(e) {
		e.preventDefault();
		const text = (e.clipboardData && e.clipboardData.getData("text/plain") || "").replace(/[\r\n]+/g, " ");
		if (!text) return;
		applyPlainTextToComposer((getComposerPlainText() + text).slice(0, 500));
		refreshEmoteSuggestions();
	});
	chatComposer.addEventListener("keydown", function(e) {
		if (e.key === "Enter" && !(emoteSuggestionsOpen && emoteSuggestionItems.length)) {
			e.preventDefault();
			chatForm.requestSubmit();
			return;
		}
		if (!emoteSuggestionsOpen || !emoteSuggestionItems.length) {
			if (e.key === "Escape") closeEmotePicker();
			return;
		}
		if (e.key === "ArrowDown") {
			e.preventDefault();
			emoteSuggestionIndex = (emoteSuggestionIndex + 1) % emoteSuggestionItems.length;
			renderEmoteSuggestions();
			return;
		}
		if (e.key === "ArrowUp") {
			e.preventDefault();
			emoteSuggestionIndex = (emoteSuggestionIndex - 1 + emoteSuggestionItems.length) % emoteSuggestionItems.length;
			renderEmoteSuggestions();
			return;
		}
		if (e.key === "Tab" || e.key === "Enter") {
			e.preventDefault();
			const selected = emoteSuggestionItems[emoteSuggestionIndex];
			const range = findActiveEmoteQuery();
			if (selected && range) applyEmoteCode(selected.code, range);
			closeEmoteSuggestions();
			return;
		}
		if (e.key === "Escape") {
			e.preventDefault();
			closeEmoteSuggestions();
			return;
		}
	});
	window.addEventListener("resize", syncPlayerLayout);
	document.addEventListener("fullscreenchange", syncPlayerLayout);
	document.addEventListener("click", function(e) {
		if (!chatForm.contains(e.target)) {
			closeEmotePicker();
			closeEmoteSuggestions();
			return;
		}
		if (e.target === chatComposer) placeComposerCaretAtEnd();
	});
	if (typeof MOBILE_LAYOUT_QUERY.addEventListener === "function") MOBILE_LAYOUT_QUERY.addEventListener("change", syncPlayerLayout);
	function buildQualityMenu(levels, currentLevelIdx) {
		qualityMenu.innerHTML = "";
		var autoItem = document.createElement("div");
		autoItem.className = "quality-menu-item" + (currentLevelIdx === -1 ? " active" : "");
		autoItem.innerHTML = "<span>Auto</span>";
		autoItem.onclick = function() {
			setLevel(-1);
		};
		qualityMenu.appendChild(autoItem);
		for (var i = 0; i < levels.length; i++) {
			var level = levels[i];
			var item = document.createElement("div");
			item.className = "quality-menu-item" + (currentLevelIdx === i ? " active" : "");
			item.innerHTML = "<span>" + level.height + "p</span><span class=\"bitrate\">" + formatBitrate(level.bitrate) + "</span>";
			(function(idx) {
				item.onclick = function() {
					setLevel(idx);
				};
			})(i);
			qualityMenu.appendChild(item);
		}
	}
	function setLevel(levelIdx) {
		if (!hlsInstance) return;
		hlsInstance.currentLevel = levelIdx;
		userSelectedAuto = levelIdx === -1;
		qualityMenu.classList.remove("open");
		if (levelIdx === -1) {
			var level = hlsInstance.levels && hlsInstance.levels[currentPlayingLevelIdx];
			if (level) qualityBtn.textContent = "Auto (" + level.height + "p)";
			else qualityBtn.textContent = "Auto";
		} else if (hlsInstance.levels && hlsInstance.levels[levelIdx]) qualityBtn.textContent = hlsInstance.levels[levelIdx].height + "p";
		buildQualityMenu(hlsInstance.levels || [], levelIdx);
	}
	qualityBtn.addEventListener("click", function(e) {
		e.stopPropagation();
		qualityMenu.classList.toggle("open");
	});
	document.addEventListener("click", function(e) {
		if (!qualityMenu.contains(e.target) && e.target !== qualityBtn) qualityMenu.classList.remove("open");
	});
	if (Hls.isSupported()) {
		hlsInstance = new Hls({
			startPosition: -6,
			lowLatencyMode: true,
			liveSyncDuration: 6,
			liveMaxLatencyDuration: 14,
			maxLiveSyncPlaybackRate: 1.1,
			maxBufferLength: 20,
			maxMaxBufferLength: 45,
			backBufferLength: 15,
			manifestLoadingTimeOut: 15e3,
			levelLoadingTimeOut: 15e3,
			fragLoadingTimeOut: 2e4,
			manifestLoadingMaxRetry: 3,
			levelLoadingMaxRetry: 3,
			fragLoadingMaxRetry: 5,
			manifestLoadingRetryDelay: 750,
			levelLoadingRetryDelay: 750,
			fragLoadingRetryDelay: 750
		});
		hlsInstance.currentLevel = -1;
		hlsInstance.on(Hls.Events.MANIFEST_PARSED, function(e, data) {
			console.log("[HLS] " + data.levels.length + " quality levels loaded");
			qualityBtn.textContent = "Auto";
			buildQualityMenu(data.levels, hlsInstance.currentLevel);
		});
		hlsInstance.on(Hls.Events.LEVEL_SWITCHED, function(e, data) {
			currentPlayingLevelIdx = data.level;
			buildQualityMenu(hlsInstance.levels, data.level);
			var level = hlsInstance.levels && hlsInstance.levels[data.level];
			if (level) if (userSelectedAuto) {
				hlsInstance.currentLevel = -1;
				qualityBtn.textContent = "Auto (" + level.height + "p)";
			} else qualityBtn.textContent = level.height + "p";
		});
		hlsInstance.on(Hls.Events.ERROR, function(e, data) {
			console.error("[HLS] ERROR:", data.details, data.fatal ? "(fatal)" : "");
			if (data.fatal) {
				if (!attemptedRelayFallback) {
					attemptedRelayFallback = true;
					var fallbackUrl = new URL(window.location.href);
					fallbackUrl.searchParams.set("relay", "1");
					window.location.assign(fallbackUrl.toString());
					return;
				}
				video.dispatchEvent(new CustomEvent("stream-error", { detail: data }));
			}
		});
		hlsInstance.loadSource(manifestUrl);
		hlsInstance.attachMedia(video);
	} else if (video.canPlayType("application/vnd.apple.mpegurl")) video.src = manifestUrl;
	else video.dispatchEvent(new CustomEvent("stream-error", { detail: { type: "not-supported" } }));
	video.addEventListener("stream-error", function() {
		document.body.innerHTML = "<div class=\"error-screen\"><div class=\"error-box\"><p>Stream unavailable. The channel may be offline or not accessible.</p></div></div>";
	});
	syncPlayerLayout();
	async function chatRequest(path, init) {
		const response = await fetch(path, Object.assign({ credentials: "same-origin" }, init || {}));
		if (!response.ok) {
			let message = "chat request failed";
			try {
				const payload = await response.json();
				if (payload && typeof payload.error === "string") message = payload.error;
			} catch {}
			throw new Error(message);
		}
	}
	function emoteUrl(emoteId) {
		return "https://static-cdn.jtvnw.net/emoticons/v2/" + encodeURIComponent(emoteId) + "/default/dark/2.0";
	}
	function normalizeEmoteCode(code) {
		if (typeof code !== "string") return "";
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
		if (!node) return "";
		if (node.nodeType === Node.TEXT_NODE) return node.textContent || "";
		if (node.nodeType !== Node.ELEMENT_NODE) return "";
		const element = node;
		if (element.tagName === "IMG") return element.dataset.code || "";
		if (element.tagName === "BR") return "\n";
		let out = "";
		for (const child of Array.from(element.childNodes)) out += composerTextFromNode(child);
		return out;
	}
	function getComposerPlainText() {
		let out = "";
		for (const child of Array.from(chatComposer.childNodes)) out += composerTextFromNode(child);
		return out;
	}
	function buildEmoteMapByCode() {
		const emotesByCode = /* @__PURE__ */ new Map();
		for (const item of availableEmotes) if (typeof item.code === "string" && typeof item.image_url === "string" && typeof item.id === "string") emotesByCode.set(item.code, item);
		return emotesByCode;
	}
	function renderComposerFromPlainText(text) {
		const emotesByCode = buildEmoteMapByCode();
		chatComposer.innerHTML = "";
		if (!text) return;
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
			const img = document.createElement("img");
			img.className = "composer-emote";
			img.src = match.image_url;
			img.alt = match.code;
			img.title = match.code;
			img.dataset.code = match.code;
			img.dataset.id = match.id;
			img.loading = "lazy";
			img.decoding = "async";
			img.contentEditable = "false";
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
		return {
			query,
			start: full.length - query.length - 1,
			end: full.length
		};
	}
	function applyEmoteCode(code, queryRange) {
		const safeCode = normalizeEmoteCode(code);
		if (!safeCode) return;
		const full = getComposerPlainText();
		if (queryRange) {
			const before = full.slice(0, queryRange.start);
			const after = full.slice(queryRange.end);
			applyPlainTextToComposer(before + safeCode + " " + after);
			return;
		}
		applyPlainTextToComposer(full + safeCode + " ");
	}
	function splitMessageSegments(input) {
		const out = [];
		let current = "";
		let currentWhitespace = null;
		for (const ch of input) {
			const isWhitespace = /\s/.test(ch);
			if (currentWhitespace === null || currentWhitespace === isWhitespace) {
				current += ch;
				currentWhitespace = isWhitespace;
			} else {
				out.push({
					text: current,
					whitespace: currentWhitespace
				});
				current = ch;
				currentWhitespace = isWhitespace;
			}
		}
		if (current.length > 0) out.push({
			text: current,
			whitespace: currentWhitespace
		});
		return out;
	}
	function normalizeComposerInput() {
		let plain = getComposerPlainText();
		plain = plain.replace(/[\r\n]+/g, " ");
		if (plain.length > 500) plain = plain.slice(0, 500);
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
		const groupedMap = /* @__PURE__ */ new Map();
		for (const item of filtered) {
			const key = typeof item.group_key === "string" ? item.group_key : "global";
			const title = typeof item.group_name === "string" && item.group_name.trim().length > 0 ? item.group_name.trim() : "Global";
			if (!groupedMap.has(key)) groupedMap.set(key, {
				key,
				title,
				items: []
			});
			groupedMap.get(key).items.push(item);
		}
		return Array.from(groupedMap.values());
	}
	function renderEmotePicker() {
		emoteGroups.innerHTML = "";
		const grouped = groupedPickerEmotes();
		function renderGroup(group) {
			if (!group.items.length) return;
			const heading = document.createElement("p");
			heading.className = "emote-group-title";
			heading.textContent = group.title;
			emoteGroups.appendChild(heading);
			const grid = document.createElement("div");
			grid.className = "emote-grid";
			for (const item of group.items) {
				const button = document.createElement("button");
				button.type = "button";
				button.className = "emote-item";
				button.title = item.code;
				button.setAttribute("aria-label", item.code);
				button.addEventListener("click", function() {
					applyEmoteCode(item.code, null);
					placeComposerCaretAtEnd();
				});
				const img = document.createElement("img");
				img.src = item.image_url;
				img.alt = item.code;
				img.loading = "lazy";
				img.decoding = "async";
				button.appendChild(img);
				grid.appendChild(button);
			}
			emoteGroups.appendChild(grid);
		}
		for (const group of grouped) renderGroup(group);
		if (!grouped.length) {
			const empty = document.createElement("div");
			empty.className = "emote-empty";
			empty.textContent = emoteSearchTerm ? "No emotes match your search." : "No emotes available.";
			emoteGroups.appendChild(empty);
		}
	}
	function renderEmoteSuggestions() {
		emoteSuggestions.innerHTML = "";
		if (!emoteSuggestionsOpen || !emoteSuggestionItems.length) {
			emoteSuggestions.classList.remove("open");
			return;
		}
		emoteSuggestions.classList.add("open");
		for (let i = 0; i < emoteSuggestionItems.length; i++) {
			const item = emoteSuggestionItems[i];
			const row = document.createElement("div");
			row.className = "emote-suggestion" + (i === emoteSuggestionIndex ? " active" : "");
			row.addEventListener("mousedown", function(e) {
				e.preventDefault();
				const range = findActiveEmoteQuery();
				applyEmoteCode(item.code, range);
				closeEmoteSuggestions();
			});
			const img = document.createElement("img");
			img.src = item.image_url;
			img.alt = item.code;
			img.loading = "lazy";
			img.decoding = "async";
			row.appendChild(img);
			const label = document.createElement("span");
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
			ensureEmotesLoaded().then(function() {
				refreshEmoteSuggestions();
			}).catch(function(error) {
				chatStatus.textContent = error && error.message ? error.message : "Failed to load emotes";
			});
			return;
		}
		const q = active.query.toLowerCase();
		const ranked = availableEmotes.map(function(item) {
			return {
				item,
				score: scoreEmote(item.code, q)
			};
		}).filter(function(entry) {
			return entry.score < 99;
		}).sort(function(a, b) {
			if (a.score !== b.score) return a.score - b.score;
			return a.item.code.toLowerCase().localeCompare(b.item.code.toLowerCase());
		}).slice(0, 10).map(function(entry) {
			return entry.item;
		});
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
		const response = await fetch("/api/chat/emotes?channel_login=" + encodeURIComponent(chatChannel), { credentials: "same-origin" });
		if (!response.ok) {
			let message = "failed to load emotes";
			try {
				const payload = await response.json();
				if (payload && typeof payload.error === "string") message = payload.error;
			} catch {}
			throw new Error(message);
		}
		const payload = await response.json();
		availableEmotes = (Array.isArray(payload && payload.emotes) ? payload.emotes : []).filter(function(item) {
			return item && typeof item.id === "string" && typeof item.code === "string" && typeof item.image_url === "string";
		}).map(function(item) {
			return {
				id: item.id,
				code: normalizeEmoteCode(item.code),
				image_url: item.image_url,
				group_key: typeof item.group_key === "string" ? item.group_key : "global",
				group_name: typeof item.group_name === "string" ? item.group_name : "Global"
			};
		}).filter(function(item) {
			return item.code.length > 0;
		});
		emotePickerLoaded = true;
		renderEmotePicker();
		normalizeComposerInput();
	}
	async function openEmotePicker() {
		closeEmoteSuggestions();
		emoteSearchTerm = "";
		emoteSearch.value = "";
		try {
			await ensureEmotesLoaded();
			renderEmotePicker();
			emotePopup.classList.add("open");
			emotePickerOpen = true;
			emoteSearch.focus();
		} catch (error) {
			chatStatus.textContent = error && error.message ? error.message : "Failed to load emotes";
		}
	}
	function closeEmotePicker() {
		emotePopup.classList.remove("open");
		emotePickerOpen = false;
	}
	function appendChatEvent(event) {
		const row = document.createElement("div");
		row.className = "chat-message" + (event.kind === "notice" ? " notice" : "");
		const who = document.createElement("span");
		who.className = "who";
		who.textContent = event.sender_display_name || event.sender_login || "system";
		if (event.kind === "message" && typeof event.sender_color === "string" && event.sender_color.trim().length > 0) who.style.color = event.sender_color;
		row.appendChild(who);
		const body = document.createElement("span");
		const parts = Array.isArray(event.parts) ? event.parts : [];
		if (parts.length > 0) for (const part of parts) {
			if (part && part.kind === "emote" && typeof part.id === "string") {
				const img = document.createElement("img");
				img.className = "chat-emote";
				img.src = typeof part.image_url === "string" && part.image_url.trim().length > 0 ? part.image_url : emoteUrl(part.id);
				img.alt = typeof part.code === "string" ? part.code : "";
				img.title = typeof part.code === "string" ? part.code : "";
				img.loading = "lazy";
				img.decoding = "async";
				body.appendChild(img);
				continue;
			}
			if (part && part.kind === "text" && typeof part.text === "string") body.appendChild(document.createTextNode(part.text));
		}
		else body.textContent = event.text || "";
		row.appendChild(body);
		chatMessages.appendChild(row);
		chatMessages.scrollTop = chatMessages.scrollHeight;
	}
	function setChatAvailability(connected) {
		if (connected) {
			chatPanel.classList.remove("hidden");
			connectTwitchBtn.classList.add("hidden");
			chatComposer.contentEditable = "true";
			chatSendBtn.disabled = false;
			chatEmoteBtn.disabled = false;
			syncPlayerLayout();
			return;
		}
		chatPanel.classList.add("hidden");
		connectTwitchBtn.classList.remove("hidden");
		closeEmotePicker();
		closeEmoteSuggestions();
		chatComposer.contentEditable = "false";
		chatSendBtn.disabled = true;
		chatEmoteBtn.disabled = true;
		syncPlayerLayout();
	}
	async function checkTwitchAndInitChat() {
		try {
			const response = await fetch("/api/twitch/status", { credentials: "same-origin" });
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
			await chatRequest("/api/chat/subscribe", {
				method: "POST",
				headers: { "content-type": "application/json" },
				body: JSON.stringify({ channel_login: chatChannel })
			});
			chatStatus.textContent = "Connected to #" + chatChannel;
			ensureEmotesLoaded().catch(function() {});
			chatEvents = new EventSource("/api/chat/events/" + encodeURIComponent(chatChannel));
			chatEvents.addEventListener("chat", function(raw) {
				try {
					appendChatEvent(JSON.parse(raw.data));
				} catch {}
			});
			chatEvents.onerror = function() {
				chatStatus.textContent = "Chat reconnecting...";
			};
			chatEvents.onopen = function() {
				chatStatus.textContent = "Connected to #" + chatChannel;
			};
		} catch (error) {
			chatStatus.textContent = error && error.message ? error.message : "Chat unavailable";
			chatComposer.contentEditable = "false";
			chatSendBtn.disabled = true;
		}
	}
	chatForm.addEventListener("submit", async function(e) {
		e.preventDefault();
		closeEmotePicker();
		closeEmoteSuggestions();
		const text = getComposerPlainText().trim();
		if (!text) return;
		chatSendBtn.disabled = true;
		try {
			await chatRequest("/api/chat/send", {
				method: "POST",
				headers: { "content-type": "application/json" },
				body: JSON.stringify({
					channel_login: chatChannel,
					message: text
				})
			});
			chatComposer.innerHTML = "";
			chatStatus.textContent = "Connected to #" + chatChannel;
			placeComposerCaretAtEnd();
		} catch (error) {
			chatStatus.textContent = error && error.message ? error.message : "Failed to send message";
		} finally {
			chatSendBtn.disabled = false;
		}
	});
	window.addEventListener("beforeunload", function() {
		fetch("/api/chat/subscribe/" + encodeURIComponent(chatChannel), {
			method: "DELETE",
			credentials: "same-origin",
			keepalive: true
		});
		if (chatEvents) chatEvents.close();
		if (typeof MOBILE_LAYOUT_QUERY.removeEventListener === "function") MOBILE_LAYOUT_QUERY.removeEventListener("change", syncPlayerLayout);
		document.removeEventListener("visibilitychange", handleVisibilityChange);
		if (liveStatusRefreshTimer) {
			clearInterval(liveStatusRefreshTimer);
			liveStatusRefreshTimer = null;
		}
	});
	document.addEventListener("visibilitychange", handleVisibilityChange);
	startLiveStatusRefreshLoop();
	checkTwitchAndInitChat();
	//#endregion
})();
