import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
  type ReactElement,
} from 'react';
import {
  getChannels,
  getRecordingRules,
  upsertRecordingRule,
  type RecordingRule,
} from '../api-client';
import { TwitchPanel } from '../components/twitch/TwitchPanel';
import { LoadedFade } from '../components/ui/LoadedFade';
import { useRouter } from '../hooks/useRouter';
import { navigate } from '../router/routes';

const DEFAULT_QUALITY = '720p60';
const FAILED_TO_LOAD = 'failed to load channel settings';
const FAILED_TO_SAVE = 'failed to save settings';
const MIN_VALUE_ERROR = 'must be at least 1';
const NOT_WHOLE_NUMBER_ERROR = 'must be a whole number';
const SUCCESS_DISMISS_MS = 3500;
const MIN_MESSAGE_LENGTH = 0;
const MIN_VALUE = 1;

const QUALITY_OPTIONS = [
  '1080p',
  '1080p60',
  '160p',
  '360p',
  '480p',
  '720p',
  '720p60',
  'best',
  'source',
];

export const TwitchChannelPage = (): ReactElement => {
  const { page } = useRouter();

  // Get login from router params
  const channelLogin = useMemo(
    () => (page.params?.login ?? '').trim().toLowerCase(),
    [page.params?.login],
  );

  // State
  const [channelExists, setChannelExists] = useState(true);
  const [channelDisplayName, setChannelDisplayName] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [enabled, setEnabled] = useState(false);
  const [quality, setQuality] = useState(DEFAULT_QUALITY);
  const [stopWhenOffline, setStopWhenOffline] = useState(true);
  const [maxDurationMinutesInput, setMaxDurationMinutesInput] = useState('');
  const [keepLastVideosInput, setKeepLastVideosInput] = useState('');

  // Auto-dismiss success message timer
  const successDismissTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const resetFormState = useCallback((): void => {
    setEnabled(false);
    setQuality(DEFAULT_QUALITY);
    setStopWhenOffline(true);
    setMaxDurationMinutesInput('');
    setKeepLastVideosInput('');
  }, []);

  const applyRuleValues = useCallback((rule: RecordingRule): void => {
    setEnabled(rule.enabled);
    setQuality(rule.quality || DEFAULT_QUALITY);
    setStopWhenOffline(rule.stop_when_offline);
    setMaxDurationMinutesInput(
      rule.max_duration_minutes === undefined || rule.max_duration_minutes === null
        ? ''
        : String(rule.max_duration_minutes),
    );
    setKeepLastVideosInput(
      rule.keep_last_videos === undefined || rule.keep_last_videos === null
        ? ''
        : String(rule.keep_last_videos),
    );
  }, []);

  const applyRule = useCallback(
    (rule: RecordingRule | undefined): void => {
      if (!rule) {
        resetFormState();
        return;
      }
      applyRuleValues(rule);
    },
    [applyRuleValues, resetFormState],
  );

  const readMessage = useCallback((error: unknown, fallback: string): string => {
    if (error instanceof Error && error.message.trim().length > MIN_MESSAGE_LENGTH) {
      return error.message;
    }
    return fallback;
  }, []);

  const normalizeValue = useCallback((value: string | number | undefined): string => {
    if (value === undefined) {
      return '';
    }
    if (typeof value === 'number') {
      return String(value);
    }
    return value;
  }, []);

  const parseOptionalPositiveInt = useCallback(
    (value: string | number | undefined, label: string): number | undefined => {
      const normalized = normalizeValue(value);
      const trimmed = normalized.trim();

      if (!trimmed) {
        return undefined;
      }

      if (!/^\d+$/.test(trimmed)) {
        throw new Error(`${label} ${NOT_WHOLE_NUMBER_ERROR}`);
      }

      const parsed = Number(trimmed);
      if (!Number.isSafeInteger(parsed) || parsed < MIN_VALUE) {
        throw new Error(`${label} ${MIN_VALUE_ERROR}`);
      }

      return parsed;
    },
    [normalizeValue],
  );

  const clearMessages = useCallback((): void => {
    setErrorMessage(null);
    setSuccessMessage(null);
  }, []);

  const handleLoadError = useCallback(
    (error: unknown): void => {
      setErrorMessage(readMessage(error, FAILED_TO_LOAD));
    },
    [readMessage],
  );

  const loadChannels = useCallback(async (): Promise<void> => {
    const [channels, rules] = await Promise.all([getChannels(), getRecordingRules()]);
    const channel = channels.find((entry) => entry.login === channelLogin);
    setChannelExists(Boolean(channel));
    setChannelDisplayName(channel?.display_name || channel?.login || channelLogin);

    const rule = rules.find((entry) => entry.channel_login === channelLogin);
    applyRule(rule ?? undefined);
  }, [channelLogin, applyRule]);

  const loadPageState = useCallback(async (): Promise<void> => {
    setIsLoading(true);
    clearMessages();

    try {
      await loadChannels();
    } catch (error) {
      handleLoadError(error);
    } finally {
      setIsLoading(false);
    }
  }, [loadChannels, handleLoadError, clearMessages]);

  const scheduleSuccessDismiss = useCallback((): void => {
    if (successDismissTimerRef.current) {
      clearTimeout(successDismissTimerRef.current);
    }
    successDismissTimerRef.current = setTimeout(() => {
      setSuccessMessage(null);
    }, SUCCESS_DISMISS_MS);
  }, []);

  const handleSaveError = useCallback(
    (error: unknown): void => {
      setErrorMessage(readMessage(error, FAILED_TO_SAVE));
    },
    [readMessage],
  );

  const parseMaxDuration = useCallback(
    (): number | undefined =>
      parseOptionalPositiveInt(maxDurationMinutesInput, 'Max duration minutes'),
    [maxDurationMinutesInput, parseOptionalPositiveInt],
  );

  const parseKeepVideos = useCallback(
    (): number | undefined => parseOptionalPositiveInt(keepLastVideosInput, 'Keep last videos'),
    [keepLastVideosInput, parseOptionalPositiveInt],
  );

  const buildSavePayload = useCallback((): {
    channel_login: string;
    enabled: boolean;
    keep_last_videos: number | undefined;
    max_duration_minutes: number | undefined;
    quality: string;
    stop_when_offline: boolean;
  } => {
    const keepVideos = parseKeepVideos();
    const maxDuration = parseMaxDuration();
    return {
      channel_login: channelLogin,
      enabled,
      keep_last_videos: keepVideos,
      max_duration_minutes: maxDuration,
      quality,
      stop_when_offline: stopWhenOffline,
    };
  }, [channelLogin, enabled, quality, stopWhenOffline, parseKeepVideos, parseMaxDuration]);

  const saveSettings = useCallback(
    async (event: FormEvent): Promise<void> => {
      event.preventDefault();
      setIsSaving(true);
      clearMessages();

      try {
        const saved = await upsertRecordingRule(buildSavePayload());

        applyRule(saved);
        setSuccessMessage('Saved');
        scheduleSuccessDismiss();
      } catch (error) {
        handleSaveError(error);
      } finally {
        setIsSaving(false);
      }
    },
    [buildSavePayload, applyRule, scheduleSuccessDismiss, handleSaveError, clearMessages],
  );

  const goBack = useCallback((): void => {
    navigate('/twitch');
  }, []);

  // Initialize on mount
  useEffect(() => {
    void loadPageState();
  }, [loadPageState]);

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (successDismissTimerRef.current) {
        clearTimeout(successDismissTimerRef.current);
      }
    };
  }, []);

  return (
    <TwitchPanel>
      <div className="channel-settings-page">
        <header className="ui-page-header">
          <div>
            <p className="ui-page-eyebrow">Channel Settings</p>
            <h1 className="ui-page-title">{channelDisplayName}</h1>
            <p className="ui-page-subtle">
              Configure recording behavior for <strong>{channelLogin}</strong>
            </p>
          </div>
          <button type="button" className="ui-nav-chip" onClick={goBack}>
            Back to channels
          </button>
        </header>

        {errorMessage && (
          <p className="ui-error" role="alert">
            {errorMessage}
          </p>
        )}
        {successMessage && (
          <p className="ui-alert-success" role="status">
            {successMessage}
          </p>
        )}

        {!isLoading && !channelExists && (
          <p className="ui-muted">
            This channel is not in your list. Add it on the front page first.
          </p>
        )}

        {!isLoading && channelExists && (
          <LoadedFade loaded={true}>
            <form className="channel-settings-form" onSubmit={saveSettings}>
              <label className="toggle-row">
                <input
                  type="checkbox"
                  checked={enabled}
                  onChange={(e) => setEnabled(e.target.checked)}
                />
                <span>Enable auto-record</span>
              </label>

              <label>
                Quality
                <select value={quality} onChange={(e) => setQuality(e.target.value)}>
                  {QUALITY_OPTIONS.map((option) => (
                    <option key={option} value={option}>
                      {option}
                    </option>
                  ))}
                </select>
              </label>

              <label className="toggle-row">
                <input
                  type="checkbox"
                  checked={stopWhenOffline}
                  onChange={(e) => setStopWhenOffline(e.target.checked)}
                />
                <span>Stop when channel goes offline</span>
              </label>

              <label>
                Max duration minutes
                <input
                  type="number"
                  min="1"
                  step="1"
                  value={maxDurationMinutesInput}
                  onChange={(e) => setMaxDurationMinutesInput(e.target.value)}
                  placeholder="Leave empty for no limit"
                  inputMode="numeric"
                />
              </label>

              <label>
                Keep last videos
                <input
                  type="number"
                  min="1"
                  step="1"
                  value={keepLastVideosInput}
                  onChange={(e) => setKeepLastVideosInput(e.target.value)}
                  placeholder="Leave empty for no limit"
                  inputMode="numeric"
                />
              </label>
              <p className="hint">
                Applies to completed recordings only. Older completed files are deleted automatically.
              </p>

              <div className="ui-action-row">
                <button className="ui-button-primary" type="submit" disabled={isSaving}>
                  {isSaving ? 'Saving...' : 'Save settings'}
                </button>
              </div>
            </form>
          </LoadedFade>
        )}
      </div>
    </TwitchPanel>
  );
}
