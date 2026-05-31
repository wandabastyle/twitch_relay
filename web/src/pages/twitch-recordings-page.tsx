import { useCallback, useEffect, useState, type ReactElement } from 'react';
import type { RecordingFileEntry } from '../api-client/types';
import { RelayHeader } from '../components/shared/relay-header';
import { RecordingsOverview } from '../components/twitch/recordings-overview';
import { TwitchPanel } from '../components/twitch/twitch-panel';
import { ConfirmDialog } from '../components/ui/confirm-dialog';
import { ErrorState } from '../components/ui/error-state';
import { SkeletonRecordingList } from '../components/ui/skeleton-recording-list';
import { useRecordingsController } from '../hooks';
import { navigate } from '../router/routes';

const DEFAULT_FILTER = 'all';
const FAILED_TO_LOAD = 'Failed to load recordings';
const INITIAL_SKELETON_SECTIONS = 3;
const INITIAL_SKELETON_ITEMS = 3;

export const TwitchRecordingsPage = (): ReactElement => {
  const [recordingsChannelFilter, setRecordingsChannelFilter] = useState(DEFAULT_FILTER);
  const [loadError, setLoadError] = useState<string | undefined>();
  const [isLoadingRecordings, setIsLoadingRecordings] = useState(true);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const recordingsController = useRecordingsController({
    setError: useCallback((msg: string | null): void => {
      setErrorMessage(msg);
      if (msg !== null && msg !== '') {
        setLoadError(msg);
      }
    }, [setLoadError]),
  });
  const { loadRecordingState } = recordingsController;

  const loadRecordings = useCallback(async (): Promise<void> => {
    setIsLoadingRecordings(true);
    setLoadError(undefined);
    try {
      await loadRecordingState();
    } catch (error_) {
      const errorMsg = error_ instanceof Error ? error_.message : FAILED_TO_LOAD;
      setLoadError(errorMsg);
    } finally {
      setIsLoadingRecordings(false);
    }
  }, [loadRecordingState]);

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
        onToggle={() => { navigate('/youtube'); }}
        toggleLabel="Switch to YouTube Relay"
      />

      {errorMessage !== null && errorMessage !== '' && (
        <p className="ui-error" role="alert">
          {errorMessage}
        </p>
      )}

      {isLoadingRecordings ? (
        <SkeletonRecordingList
          sections={INITIAL_SKELETON_SECTIONS}
          itemsPerSection={INITIAL_SKELETON_ITEMS}
        />
      ) : ((loadError !== undefined && loadError !== '') ? (
        <ErrorState message={loadError} onRetry={() => { void loadRecordings(); }} isRetrying={isLoadingRecordings} />
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
          onToggleRecordingPin={(recordingKey) => {
            void recordingsController.toggleRecordingPin(recordingKey);
          }}
          onRepairRecording={(file) => {
            void recordingsController.repairRecording(file);
          }}
          onToggleIncompleteMergeSelection={recordingsController.toggleIncompleteMergeSelection}
          onRequestProcessIncompleteFiles={recordingsController.requestProcessIncompleteFiles}
          onConfirmProcessIncompleteFiles={async (): Promise<void> => {
            await recordingsController.confirmProcessIncompleteFiles();
          }}
          onCancelProcessIncompleteFiles={recordingsController.cancelProcessIncompleteFiles}
        />
      ))}

      <ConfirmDialog
        isOpen={recordingsController.pendingDelete !== undefined}
        isBusy={recordingsController.deletingRecordingKey !== undefined}
        onConfirm={() => { void recordingsController.confirmDeleteRecordingFile(); }}
        onCancel={recordingsController.cancelDeleteRecordingFile}
        confirmText={
          recordingsController.deletingRecordingKey === undefined
            ? 'Delete'
            : 'Deleting...'
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
        onConfirm={() => { void recordingsController.confirmProcessIncompleteFiles(); }}
        onCancel={recordingsController.cancelProcessIncompleteFiles}
        confirmText={
          recordingsController.mergingRecordingKey === undefined
            ? (recordingsController.pendingMerge?.action === 'finalize'
              ? 'Finalize'
              : 'Merge')
            : 'Processing...'
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
