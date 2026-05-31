import { useCallback, useEffect, useRef, useState, type ReactElement } from 'react';
import type { EmoteItem } from '../../api-client';

interface EmotePickerProps {
  availableEmotes: EmoteItem[];
  onSelect: (code: string) => void;
}

interface GroupedEmotes {
  items: EmoteItem[];
  key: string;
  title: string;
}

const EMPTY_LENGTH = 0;
const FOCUS_DELAY_MS = 0;
const MIN_GROUP_NAME_LENGTH = 0;

const filterEmotes = (emotes: readonly EmoteItem[], term: string): readonly EmoteItem[] =>
  term ? emotes.filter((item) => item.code.toLowerCase().includes(term)) : emotes;

const createGroup = (key: string, title: string): GroupedEmotes => ({ items: [], key, title });

const buildGroups = (filtered: readonly EmoteItem[]): readonly GroupedEmotes[] => {
  const groupedMap = new Map<string, GroupedEmotes>();

  for (const item of filtered) {
    const { group_key: key, group_name: groupName } = item;
    const title = groupName.trim().length > MIN_GROUP_NAME_LENGTH ? groupName : 'Global';
    if (!groupedMap.has(key)) {
      groupedMap.set(key, createGroup(key, title));
    }
    groupedMap.get(key)?.items.push(item);
  }

  return [...groupedMap.values()];
};

export const EmotePicker = ({ availableEmotes, onSelect }: EmotePickerProps): ReactElement => {
  const [pickerOpen, setPickerOpen] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');
  const searchElRef = useRef<HTMLInputElement>(null);

  const groupedEmotes = buildGroups(filterEmotes(availableEmotes, searchTerm.trim().toLowerCase()));

  const openAndFocus = useCallback((): void => {
    setSearchTerm('');
    // Use setTimeout to wait for the input to render
    setTimeout(() => {
      searchElRef.current?.focus();
    }, FOCUS_DELAY_MS);
  }, []);

  const togglePicker = useCallback((): void => {
    setPickerOpen((prev) => {
      const next = !prev;
      if (next) {
        openAndFocus();
      }
      return next;
    });
  }, [openAndFocus]);

  const closePicker = useCallback((): void => {
    setPickerOpen(false);
  }, []);

  const handleSelect = useCallback(
    (code: string): void => {
      onSelect(code);
    },
    [onSelect],
  );

  // Close picker when clicking outside
  useEffect(() => {
    const handleDocumentClick = (event: MouseEvent): void => {
      const { target } = event;
      if (!(target instanceof HTMLElement)) {
        return;
      }

      const clickedInsidePopup = target.closest('.emote-popup') !== null;
      const clickedToggle = target.closest('.emote-toggle') !== null;

      if (!clickedInsidePopup && !clickedToggle) {
        closePicker();
      }
    };

    document.addEventListener('click', handleDocumentClick);
    return (): void => {
      document.removeEventListener('click', handleDocumentClick);
    };
  }, [closePicker]);

  return (
    <>
      <button
        type="button"
        className="emote-toggle"
        title="Open emote picker"
        aria-label="Open emote picker"
        onClick={togglePicker}
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <circle cx="12" cy="12" r="10" />
          <path d="M8 14s1.5 2 4 2 4-2 4-2" />
          <line x1="9" y1="9" x2="9.01" y2="9" />
          <line x1="15" y1="9" x2="15.01" y2="9" />
        </svg>
      </button>

      {pickerOpen && (
        <div className="emote-popup">
          <input
            ref={searchElRef}
            className="emote-search"
            type="text"
            placeholder="Search emotes"
            autoComplete="off"
            value={searchTerm}
            onChange={(event) => {
              setSearchTerm(event.target.value);
            }}
          />

          <div className="emote-groups ui-hide-scrollbar">
            {groupedEmotes.map((group) => (
              <div key={group.key}>
                <p className="emote-group-title">{group.title}</p>
                <div className="emote-grid">
                  {group.items.map((item) => (
                    <button
                      key={item.id}
                      type="button"
                      className="emote-item"
                      title={item.code}
                      aria-label={item.code}
                      onClick={() => {
                        handleSelect(item.code);
                      }}
                    >
                      <img src={item.image_url} alt={item.code} loading="lazy" decoding="async" />
                    </button>
                  ))}
                </div>
              </div>
            ))}

            {groupedEmotes.length === EMPTY_LENGTH && (
              <div className="emote-empty">
                {searchTerm ? 'No emotes match your search.' : 'No emotes available.'}
              </div>
            )}
          </div>
        </div>
      )}
    </>
  );
}
