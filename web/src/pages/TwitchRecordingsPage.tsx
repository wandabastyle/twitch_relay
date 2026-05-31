import { useCallback, useEffect, useState, type ReactElement } from 'react';
import type { RecordingFileEntry } from '../api-client/types';
import { RelayHeader } from '../components/shared/RelayHeader';
import { RecordingsOverview } from '../components/twitch/RecordingsOverview';
import { TwitchPanel } from '../components/twitch/TwitchPanel';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import { ErrorState } from '../components/ui/ErrorState';
import { SkeletonRecordingList } from '../components/ui/SkeletonRecordingList';
import { useRecordingsController } from '../hooks';
import { navigate } from '../router/routes';

const DEFAULT_FILTER = 'all';
const FAILED_TO_LOAD = 'Failed to load recordings';
const INITIAL_SKELETON_SECTIONS = 3;
const INITIAL_SKELETON_ITEMS = 3;

export function TwitchRecordingsPage(): ReactElement {
  const [recordingsChannelFilter, setRecordingsChannelFilter] = useState(DEFAULT_FILTER);
  const [loadError, setLoadError] = useState<string | undefined>();
  const [isLoadingRecordings, setIsLoadingRecordings] = useState(true);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const setError = useCallback((msg: string | null): void => {
    setErrorMessage(msg);
    if (msg) {
      setLoadError(msg);
    }
  }, []);

  const recordingsController = useRecordingsController({ setError });

  const loadRecordings = useCallback(async (): Promise<void> => {
    setIsLoadingRecordings(true);
    setLoadError(undefined);
    try {
      await recordingsController.loadRecordingState();
    } catch (error_) {
      const errorMessage = error_ instanceof Error ? error_.message : FAILED_TO_LOAD;
      setLoadError(errorMessage);
    } finally {
      setIsLoadingRecordings(false);
    }
  }, [recordingsController]);

  useEffect(() => {
    void loadRecordings();
  }, [loadRecordings]);

  const backToChannels = useCallback((): void => {
    navigate('/twitch');
  }, []);

  const openRecordingPlayer = useCallback((file: RecordingFileEntry): void => {
    const query = new URLSearchParams({
      channel_login: file.channel_login,
      filename: file.filename,
    });
    navigate(`/twitch/recordings/play?${query.toString()}`);
  }, []);

  const onUpdateFilter = useCallback((value: string): void => {
    setRecordingsChannelFilter(value);
  }, []);

  return (
    <TwitchPanel>
      <RelayHeader
        eyebrow="Private Deck"
        title="Twitch Relay"
        subtitleText="Recording activity and files"
        onToggle={() => navigate('/youtube')}
        toggleLabel="Switch to YouTube Relay"
      />

      {errorMessage && (
        <p className="ui-error" role="alert">
          {errorMessage}
        </p>
      )}

      {isLoadingRecordings ? (
        <SkeletonRecordingList
          sections={INITIAL_SKELETON_SECTIONS}
          itemsPerSection={INITIAL_SKELETON_ITEMS}
        />
      ) : loadError ? (
        <ErrorState message={loadError} onRetry={loadRecordings} isRetrying={isLoadingRecordings} />
      ) : (
        <RecordingsOverview
          activeRecordings={recordingsController.activeRecordings}
          completedRecordings={recordingsController.completedRecordings}
          incompleteRecordings={recordingsController.incompleteRecordings}
          recordingsChannelFilter={recordingsChannelFilter}
          deletingRecordingKey={recordingsController.deletingRecordingKey}
          pinningRecordingKey={recordingsController.pinningRecordingKey}
          repairingRecordingKey={recordingsController.repairingRecordingKey}
          mergingRecordingKey={recordingsController.mergingRecordingKey}
          selectedIncompleteFilenames={recordingsController.selectedIncompleteFilenames}
          pendingJob={recordingsController.pendingJob}
          pendingDelete={recordingsController.pendingDelete}
          pendingMerge={recordingsController.pendingMerge}
          onBackToChannels={backToChannels}
          onUpdateFilter={onUpdateFilter}
          onOpenRecordingPlayer={openRecordingPlayer}
          onRequestDeleteRecordingFile={recordingsController.requestDeleteRecordingFile}
          onConfirmDeleteRecordingFile={recordingsController.confirmDeleteRecordingFile}
          onCancelDeleteRecordingFile={recordingsController.cancelDeleteRecordingFile}
          onToggleRecordingPin={recordingsController.toggleRecordingPin}
          onRepairRecording={recordingsController.repairRecording}
          onToggleIncompleteMergeSelection={recordingsController.toggleIncompleteMergeSelection}
          onRequestProcessIncompleteFiles={recordingsController.requestProcessIncompleteFiles}
          onConfirmProcessIncompleteFiles={recordingsController.confirmProcessIncompleteFiles}
          onCancelProcessIncompleteFiles={recordingsController.cancelProcessIncompleteFiles}
        />
      )}

      <ConfirmDialog
        isOpen={recordingsController.pendingDelete !== undefined}
        isBusy={recordingsController.deletingRecordingKey !== undefined}
        onConfirm={recordingsController.confirmDeleteRecordingFile}
        onCancel={recordingsController.cancelDeleteRecordingFile}
        confirmText={
          recordingsController.deletingRecordingKey !== undefined ? 'Deleting...' : 'Delete'
        }
        confirmVariant="danger"
      >
        <p>
          Delete{' '}
          <strong className="danger-text">
            {recordingsController.pendingDelete?.file.filename}
          </strong>
          ?
        </p>
        <p className="subtle">This action cannot be undone.</p>
      </ConfirmDialog>

      <ConfirmDialog
        isOpen={recordingsController.pendingMerge !== undefined}
        isBusy={recordingsController.mergingRecordingKey !== undefined}
        onConfirm={recordingsController.confirmProcessIncompleteFiles}
        onCancel={recordingsController.cancelProcessIncompleteFiles}
        confirmText={
          recordingsController.mergingRecordingKey !== undefined
            ? 'Processing...'
            : recordingsController.pendingMerge?.action === 'finalize'
              ? 'Finalize'
              : 'Merge'
        }
      >
        <p>
          {recordingsController.pendingMerge?.action === 'finalize' ? 'Finalize' : 'Merge'}{' '}
          <strong>{recordingsController.pendingMerge?.filenames.length}</strong> incomplete
          recording(s) for <strong>{recordingsController.pendingMerge?.channelLogin}</strong>?
        </p>
        <p className="subtle">This action cannot be undone.</p>
      </ConfirmDialog>
    </TwitchPanel>
  );
}
