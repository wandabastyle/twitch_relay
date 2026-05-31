import { useCallback, useEffect, useMemo, useState, type ReactElement } from 'react';
import {
  getChannels,
  getRecordingRules,
  upsertRecordingRule,
} from '../api-client';
import { TwitchPanel } from '../components/twitch/TwitchPanel';
import { LoadedFade } from '../components/ui/LoadedFade';
import { useRouter } from '../hooks/useRouter';
import { navigate } from '../router/routes';
import { useRecordingRuleForm } from './twitch-channel/recording-rule-form';

const DEFAULT_QUALITY = '720p60';
const FAILED_TO_LOAD = 'failed to load channel settings';
const FAILED_TO_SAVE = 'failed to save settings';
const MIN_MESSAGE_LENGTH = 0;

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
    () => (page.params.login ?? '').trim().toLowerCase(),
    [page.params.login],
  );

  // State
  const [channelExists, setChannelExists] = useState(true);
  const [channelDisplayName, setChannelDisplayName] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);

  // Form state from custom hook
  const {
    enabled,
    quality,
    stopWhenOffline,
    maxDurationMinutesInput,
    keepLastVideosInput,
    errorMessage,
    successMessage,
    setEnabled,
    setQuality,
    setStopWhenOffline,
    setMaxDurationMinutesInput,
    setKeepLastVideosInput,
    setErrorMessage,
    setSuccessMessage,
    applyRule,
    clearMessages,
    scheduleSuccessDismiss,
    buildSavePayload,
    cleanupTimer,
  } = useRecordingRuleForm();

  const readMessage = useCallback((error: unknown, fallback: string): string => {
    if (error instanceof Error && error.message.trim().length > MIN_MESSAGE_LENGTH) {
      return error.message;
    }
    return fallback;
  }, []);

  const handleLoadError = useCallback(
    (error: unknown): void => {
      setErrorMessage(readMessage(error, FAILED_TO_LOAD));
    },
    [readMessage, setErrorMessage],
  );

  const loadChannels = useCallback(async (): Promise<void> => {
    const [channels, rules] = await Promise.all([getChannels(), getRecordingRules()]);
    const channel = channels.find((entry) => entry.login === channelLogin);
    setChannelExists(Boolean(channel));
    setChannelDisplayName(channel?.display_name ?? channel?.login ?? channelLogin);

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

  const handleSaveError = useCallback(
    (error: unknown): void => {
      setErrorMessage(readMessage(error, FAILED_TO_SAVE));
    },
    [readMessage, setErrorMessage],
  );

  const saveSettings = useCallback(
    (event: React.FormEvent): void => {
      event.preventDefault();
      setIsSaving(true);
      clearMessages();

      void (async (): Promise<void> => {
        try {
          const saved = await upsertRecordingRule(buildSavePayload(channelLogin));

          applyRule(saved);
          setSuccessMessage('Saved');
          scheduleSuccessDismiss();
        } catch (error) {
          handleSaveError(error);
        } finally {
          setIsSaving(false);
        }
      })();
    },
    [buildSavePayload, channelLogin, applyRule, scheduleSuccessDismiss, handleSaveError, clearMessages],
  );

  const goBack = useCallback((): void => {
    navigate('/twitch');
  }, []);

  // Initialize on mount
  useEffect(() => {
    void loadPageState();
  }, [loadPageState]);

  // Cleanup timer on unmount
  useEffect(() => cleanupTimer, [cleanupTimer]);

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

        {errorMessage !== null && errorMessage !== '' && (
          <p className="ui-error" role="alert">
            {errorMessage}
          </p>
        )}
        {successMessage !== null && successMessage !== '' && (
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
            <form className="channel-settings-form" onSubmit={(event) => { void saveSettings(event); }}>
              <label className="toggle-row">
                <input
                  type="checkbox"
                  checked={enabled}
                  onChange={(event) => { setEnabled(event.target.checked); }}
                />
                <span>Enable auto-record</span>
              </label>

              <label>
                Quality
                <span className="channel-quality-select-wrap">
                  <select value={quality} onChange={(event) => { setQuality(event.target.value); }}>
                    {QUALITY_OPTIONS.map((option) => (
                      <option key={option} value={option}>
                        {option}
                      </option>
                    ))}
                  </select>
                </span>
              </label>

              <label className="toggle-row">
                <input
                  type="checkbox"
                  checked={stopWhenOffline}
                  onChange={(event) => { setStopWhenOffline(event.target.checked); }}
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
                  onChange={(event) => { setMaxDurationMinutesInput(event.target.value); }}
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
                  onChange={(event) => { setKeepLastVideosInput(event.target.value); }}
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
};
