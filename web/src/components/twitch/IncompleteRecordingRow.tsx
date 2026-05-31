import { CheckSquare, Square, Trash2 } from 'lucide-react';
import type { ReactElement } from 'react';
import type { RecordingFileEntry } from '../../api-client/types';
import { recordingDeleteKey } from '../../lib/home/recordings';

interface IncompleteRecordingRowProps {
  readonly file: RecordingFileEntry;
  readonly deletingRecordingKey: string | undefined;
  readonly mergingRecordingKey: string | undefined;
  readonly selectedIncompleteFilenames: ReadonlySet<string>;
  readonly recordingsChannelFilter: string;
  readonly onToggleIncompleteMergeSelection: (filename: string) => void;
  readonly onRequestDeleteRecordingFile: (
    bucket: 'completed' | 'incomplete',
    file: RecordingFileEntry,
  ) => void;
}

export function IncompleteRecordingRow({
  file,
  deletingRecordingKey,
  mergingRecordingKey,
  selectedIncompleteFilenames,
  recordingsChannelFilter,
  onToggleIncompleteMergeSelection,
  onRequestDeleteRecordingFile,
}: IncompleteRecordingRowProps): ReactElement {
  const deleteKey = recordingDeleteKey('incomplete', file);
  const isSelected = selectedIncompleteFilenames.has(file.filename);

  return (
    <li className="recordings-item-with-action">
      <div className="recording-entry-incomplete">
        <div className="recording-title-row">
          <span className="entry-main" title={file.filename}>
            {file.filename}
          </span>
        </div>
        <span className="entry-meta" title={file.path_display}>
          {file.path_display}
        </span>
      </div>
      <div className="recording-item-actions">
        {recordingsChannelFilter !== 'all' && (
          <button
            type="button"
            className={`recording-select-btn ${isSelected ? 'selected' : ''}`}
            onClick={() => onToggleIncompleteMergeSelection(file.filename)}
            disabled={mergingRecordingKey === recordingsChannelFilter}
            title={isSelected ? 'Deselect' : 'Select'}
            aria-label={isSelected ? 'Deselect file' : 'Select file'}
            aria-pressed={isSelected}
          >
            {isSelected ? <CheckSquare size={14} /> : <Square size={14} />}
          </button>
        )}
        <button
          type="button"
          className="recording-delete-btn"
          onClick={() => onRequestDeleteRecordingFile('incomplete', file)}
          title="Delete recording"
          aria-label="Delete recording"
          aria-busy={deletingRecordingKey === deleteKey}
          disabled={deletingRecordingKey === deleteKey}
        >
          {deletingRecordingKey === deleteKey ? (
            <span className="delete-spinner" />
          ) : (
            <Trash2 size={14} />
          )}
        </button>
      </div>
    </li>
  );
}
