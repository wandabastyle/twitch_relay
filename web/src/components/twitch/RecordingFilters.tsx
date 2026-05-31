import { ChevronDown } from 'lucide-react';
import type { ReactElement } from 'react';

interface RecordingFiltersProps {
  readonly channelOptions: string[];
  readonly recordingsChannelFilter: string;
  readonly onUpdateFilter: (value: string) => void;
}

export function RecordingFilters({
  channelOptions,
  recordingsChannelFilter,
  onUpdateFilter,
}: RecordingFiltersProps): ReactElement {
  return (
    <div className="recordings-filter-row">
      <label className="recordings-filter-label" htmlFor="recordings-filter">
        Filter by channel
      </label>
      <div className="select-wrapper">
        <select
          id="recordings-filter"
          className="recordings-filter-select"
          value={recordingsChannelFilter}
          onChange={(e) => onUpdateFilter(e.currentTarget.value)}
        >
          <option value="all">All channels</option>
          {channelOptions.map((channelLogin) => (
            <option key={channelLogin} value={channelLogin}>
              {channelLogin}
            </option>
          ))}
        </select>
        <span className="select-chevron" aria-hidden="true">
          <ChevronDown size={14} />
        </span>
      </div>
      <p className="recordings-filter-hint">All channels shows latest 3 per section.</p>
    </div>
  );
}
