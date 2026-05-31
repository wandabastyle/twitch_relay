import type { ReactElement } from 'react';
import type { RecordingFileEntry } from '../../api-client/types';
import { RecordingActions } from './RecordingActions';
import { RecordingBadges } from './RecordingBadges';

interface CompletedRecordingRowProps {
  readonly file: RecordingFileEntry;
  readonly deletingRecordingKey: string | undefined;
  readonly pinningRecordingKey: string | undefined;
  readonly repairingRecordingKey: string | undefined;
  readonly onToggleRecordingPin: (file: RecordingFileEntry) => void;
  readonly onOpenRecordingPlayer: (file: RecordingFileEntry) => void;
  readonly onRepairRecording: (file: RecordingFileEntry) => void;
  readonly onRequestDeleteRecordingFile: (
    bucket: 'completed' | 'incomplete',
    file: RecordingFileEntry,
  ) => void;
}

export function CompletedRecordingRow({
  file,
  deletingRecordingKey,
  pinningRecordingKey,
  repairingRecordingKey,
  onToggleRecordingPin,
  onOpenRecordingPlayer,
  onRepairRecording,
  onRequestDeleteRecordingFile,
}: CompletedRecordingRowProps): ReactElement {
  return (
    <li className="recordings-item-with-action">
      <div className="recording-entry">
        <div className="recording-title-row">
          <span className="entry-main" title={file.filename}>
            {file.filename}
          </span>
          <RecordingBadges file={file} />
        </div>
        <span className="entry-meta" title={file.path_display}>
          {file.path_display}
        </span>
      </div>
      <RecordingActions
        file={file}
        deletingRecordingKey={deletingRecordingKey}
        pinningRecordingKey={pinningRecordingKey}
        repairingRecordingKey={repairingRecordingKey}
        onToggleRecordingPin={onToggleRecordingPin}
        onOpenRecordingPlayer={onOpenRecordingPlayer}
        onRepairRecording={onRepairRecording}
        onRequestDeleteRecordingFile={onRequestDeleteRecordingFile}
      />
    </li>
  );
}
