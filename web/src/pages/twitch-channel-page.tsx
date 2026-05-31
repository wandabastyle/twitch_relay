import { useCallback, useEffect, useMemo, type ReactElement } from 'react';
import { TwitchPanel } from '../components/twitch/twitch-panel';
import { LoadedFade } from '../components/ui/loaded-fade';
import { useRouter } from '../hooks/use-router';
import { navigate } from '../router/routes';
import { useChannelPageState } from './twitch-channel/page-state';

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
    () => page.params.login.trim().toLowerCase(),
    [page.params.login],
  );

  // Page state hook
  const pageState = useChannelPageState({ channelLogin });
  const { channelDisplayName, channelExists, form, isLoading, isSaving, loadPageState, saveSettings } = pageState;

  const goBack = useCallback((): void => {
    navigate('/twitch');
  }, []);

  // Initialize on mount
  useEffect(() => {
    void loadPageState();
  }, [loadPageState]);

  // Cleanup timer on unmount
  useEffect(() => form.cleanupTimer, [form.cleanupTimer]);

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

        {form.errorMessage !== null && form.errorMessage !== '' && (
          <p className="ui-error" role="alert">
            {form.errorMessage}
          </p>
        )}
        {form.successMessage !== null && form.successMessage !== '' && (
          <p className="ui-alert-success" role="status">
            {form.successMessage}
          </p>
        )}

        {!isLoading && !channelExists && (
          <p className="ui-muted">
            This channel is not in your list. Add it on the front page first.
          </p>
        )}

        {!isLoading && channelExists && (
          <LoadedFade loaded={true}>
            <form
              className="channel-settings-form"
              onSubmit={(submitEvent): void => {
                saveSettings(submitEvent);
              }}
            >
              <label className="toggle-row">
                <input
                  type="checkbox"
                  checked={form.enabled}
                  onChange={(event) => { form.setEnabled(event.target.checked); }}
                />
                <span>Enable auto-record</span>
              </label>

              <label>
                Quality
                <span className="channel-quality-select-wrap">
                  <select value={form.quality} onChange={(event) => { form.setQuality(event.target.value); }}>
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
                  checked={form.stopWhenOffline}
                  onChange={(event) => { form.setStopWhenOffline(event.target.checked); }}
                />
                <span>Stop when channel goes offline</span>
              </label>

              <label>
                Max duration minutes
                <input
                  type="number"
                  min="1"
                  step="1"
                  value={form.maxDurationMinutesInput}
                  onChange={(event) => { form.setMaxDurationMinutesInput(event.target.value); }}
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
                  value={form.keepLastVideosInput}
                  onChange={(event) => { form.setKeepLastVideosInput(event.target.value); }}
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
