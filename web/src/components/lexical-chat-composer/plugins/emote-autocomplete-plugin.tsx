import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import { Paper, Stack, Typography } from '@mui/material';
import {
  $createTextNode,
  $getSelection,
  $isRangeSelection,
  $isTextNode,
  COMMAND_PRIORITY_NORMAL,
  KEY_DOWN_COMMAND,
  KEY_ENTER_COMMAND,
  KEY_ESCAPE_COMMAND,
  KEY_TAB_COMMAND,
} from 'lexical';
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactElement,
} from 'react';
import type { EmoteItem } from '../../../api-client';
import { $createEmoteNode } from '../nodes/emote-node';

interface EmoteSuggestion {
  code: string;
  imageUrl: string;
}

interface EmoteAutocompletePluginProps {
  availableEmotes: EmoteItem[];
}

interface SuggestionListProps {
  containerRef: React.RefObject<HTMLDivElement | null>;
  insertEmote: (code: string, imageUrl: string) => void;
  selectedIndex: number;
  suggestions: EmoteSuggestion[];
}

interface KeyboardNavigationOptions {
  closeSuggestions: () => void;
  insertEmote: (code: string, imageUrl: string) => void;
  selectedIndex: number;
  setSelectedIndex: React.Dispatch<React.SetStateAction<number>>;
  suggestions: EmoteSuggestion[];
}

const AUTOCOMPLETE_HEIGHT = 200;
const AUTOCOMPLETE_Z_INDEX = 1000;
const EMPTY_INDEX = 0;
const EMPTY_LENGTH = 0;
const EMOTE_IMAGE_SIZE = 24;
const INDEX_STEP = 1;
const MAX_SUGGESTIONS = 10;
const NO_MATCH_INDEX = -1;
const SORT_AFTER = 1;
const SORT_BEFORE = -1;

const panelSx = {
  bottom: '100%',
  left: 0,
  maxHeight: AUTOCOMPLETE_HEIGHT,
  mb: 1,
  overflow: 'auto',
  position: 'absolute',
  right: 0,
  zIndex: AUTOCOMPLETE_Z_INDEX,
};

const suggestionImageStyle: CSSProperties = {
  height: EMOTE_IMAGE_SIZE,
  objectFit: 'contain',
  width: EMOTE_IMAGE_SIZE,
};

const suggestionSx = {
  '&:hover': { bgcolor: 'action.hover' },
  alignItems: 'center',
  cursor: 'pointer',
  display: 'flex',
  gap: 1,
  px: 2,
  py: 1,
};

const triggerPattern = /:([a-zA-Z0-9]*)$/;

const rankEmotes = (emotes: EmoteItem[], query: string): EmoteItem[] => {
  const lowerQuery = query.toLowerCase();
  return emotes
    .filter((emoteItem) => emoteItem.code.toLowerCase().includes(lowerQuery))
    .sort((firstEmote, secondEmote) => {
      const firstStarts = firstEmote.code.toLowerCase().startsWith(lowerQuery);
      const secondStarts = secondEmote.code.toLowerCase().startsWith(lowerQuery);
      if (!firstStarts && secondStarts) {
        return SORT_AFTER;
      }
      if (firstStarts && !secondStarts) {
        return SORT_BEFORE;
      }
      return firstEmote.code.length - secondEmote.code.length;
    })
    .slice(EMPTY_INDEX, MAX_SUGGESTIONS);
};

const toSuggestion = (emote: EmoteItem): EmoteSuggestion => ({
  code: emote.code,
  imageUrl: emote.image_url,
});

const getSuggestions = (availableEmotes: EmoteItem[], searchQuery: string): EmoteSuggestion[] => {
  if (searchQuery.length === EMPTY_LENGTH) {
    return availableEmotes
      .slice(EMPTY_INDEX, MAX_SUGGESTIONS)
      .map((emoteItem) => toSuggestion(emoteItem));
  }
  return rankEmotes(availableEmotes, searchQuery).map((emoteItem) => toSuggestion(emoteItem));
};

const useInsertEmote = (
  closeSuggestions: () => void,
): ((code: string, imageUrl: string) => void) => {
  const [editor] = useLexicalComposerContext();

  return useCallback(
    (code: string, imageUrl: string): void => {
      editor.update(() => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) {
          return;
        }

        const anchorNode = selection.anchor.getNode();
        if (!$isTextNode(anchorNode)) {
          return;
        }

        const anchorOffset = selection.anchor.offset;
        const beforeCursor = anchorNode.getTextContent().slice(EMPTY_INDEX, anchorOffset);
        const lastColonIndex = beforeCursor.lastIndexOf(':');
        if (lastColonIndex === NO_MATCH_INDEX) {
          return;
        }

        anchorNode.select(lastColonIndex, anchorOffset);
        selection.insertNodes([$createEmoteNode(code, imageUrl), $createTextNode(' ')]);
      });
      closeSuggestions();
    },
    [closeSuggestions, editor],
  );
};

const useAutocompleteListener = (
  availableEmotes: EmoteItem[],
  closeSuggestions: () => void,
  setSelectedIndex: (index: number) => void,
  setSuggestions: (suggestions: EmoteSuggestion[]) => void,
): void => {
  const [editor] = useLexicalComposerContext();

  useEffect(
    () =>
      editor.registerUpdateListener(({ editorState }) => {
        editorState.read(() => {
          const selection = $getSelection();
          if (!$isRangeSelection(selection)) {
            closeSuggestions();
            return;
          }

          const anchorNode = selection.anchor.getNode();
          if (!$isTextNode(anchorNode)) {
            closeSuggestions();
            return;
          }

          const beforeCursor = anchorNode
            .getTextContent()
            .slice(EMPTY_INDEX, selection.anchor.offset);
          const match = triggerPattern.exec(beforeCursor);
          if (match === null) {
            closeSuggestions();
            return;
          }

          const [, searchQuery] = match;
          setSuggestions(getSuggestions(availableEmotes, searchQuery));
          setSelectedIndex(EMPTY_INDEX);
        });
      }),
    [availableEmotes, closeSuggestions, editor, setSelectedIndex, setSuggestions],
  );
};

const useKeyboardNavigation = ({
  closeSuggestions,
  insertEmote,
  selectedIndex,
  setSelectedIndex,
  suggestions,
}: KeyboardNavigationOptions): void => {
  const [editor] = useLexicalComposerContext();
  const suggestionsOpen = suggestions.length > EMPTY_LENGTH;

  useEffect(() => {
    const arrowUp = editor.registerCommand(
      KEY_DOWN_COMMAND,
      (event: KeyboardEvent): boolean => {
        if (!suggestionsOpen || event.key !== 'ArrowUp') {
          return false;
        }
        event.preventDefault();
        setSelectedIndex((previousIndex) =>
          previousIndex > EMPTY_INDEX
            ? previousIndex - INDEX_STEP
            : suggestions.length - INDEX_STEP,
        );
        return true;
      },
      COMMAND_PRIORITY_NORMAL,
    );

    const arrowDown = editor.registerCommand(
      KEY_DOWN_COMMAND,
      (event: KeyboardEvent): boolean => {
        if (!suggestionsOpen || event.key !== 'ArrowDown') {
          return false;
        }
        event.preventDefault();
        setSelectedIndex((previousIndex) =>
          previousIndex < suggestions.length - INDEX_STEP
            ? previousIndex + INDEX_STEP
            : EMPTY_INDEX,
        );
        return true;
      },
      COMMAND_PRIORITY_NORMAL,
    );

    const enterSelect = editor.registerCommand(
      KEY_ENTER_COMMAND,
      (): boolean => {
        if (!suggestionsOpen) {
          return false;
        }
        const suggestion = suggestions[selectedIndex];
        insertEmote(suggestion.code, suggestion.imageUrl);
        return true;
      },
      COMMAND_PRIORITY_NORMAL,
    );

    const tabSelect = editor.registerCommand(
      KEY_TAB_COMMAND,
      (event: KeyboardEvent): boolean => {
        if (!suggestionsOpen) {
          return false;
        }
        event.preventDefault();
        const suggestion = suggestions[selectedIndex];
        insertEmote(suggestion.code, suggestion.imageUrl);
        return true;
      },
      COMMAND_PRIORITY_NORMAL,
    );

    const escapeClose = editor.registerCommand(
      KEY_ESCAPE_COMMAND,
      (): boolean => {
        if (!suggestionsOpen) {
          return false;
        }
        closeSuggestions();
        return true;
      },
      COMMAND_PRIORITY_NORMAL,
    );

    return (): void => {
      arrowDown();
      arrowUp();
      enterSelect();
      escapeClose();
      tabSelect();
    };
  }, [
    closeSuggestions,
    editor,
    insertEmote,
    selectedIndex,
    setSelectedIndex,
    suggestions,
    suggestionsOpen,
  ]);
};

const useOutsideClickDismiss = (
  closeSuggestions: () => void,
  containerRef: React.RefObject<HTMLDivElement | null>,
  suggestionsOpen: boolean,
): void => {
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent): void => {
      if (!(event.target instanceof Node)) {
        return;
      }
      if (containerRef.current !== null && !containerRef.current.contains(event.target)) {
        closeSuggestions();
      }
    };

    if (suggestionsOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return (): void => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [closeSuggestions, containerRef, suggestionsOpen]);
};

const SuggestionList = ({
  containerRef,
  insertEmote,
  selectedIndex,
  suggestions,
}: SuggestionListProps): ReactElement => (
  <Paper ref={containerRef} elevation={3} sx={panelSx}>
    <Stack spacing={EMPTY_INDEX}>
      {suggestions.map((suggestion, suggestionIndex) => (
        <Typography
          key={suggestion.code}
          component="div"
          onClick={() => {
            insertEmote(suggestion.code, suggestion.imageUrl);
          }}
          sx={{
            ...suggestionSx,
            bgcolor: suggestionIndex === selectedIndex ? 'action.selected' : 'inherit',
          }}
        >
          <img src={suggestion.imageUrl} alt={suggestion.code} style={suggestionImageStyle} />
          {suggestion.code}
        </Typography>
      ))}
    </Stack>
  </Paper>
);

export const EmoteAutocompletePlugin = ({
  availableEmotes,
}: EmoteAutocompletePluginProps): ReactElement | null => {
  const [suggestions, setSuggestions] = useState<EmoteSuggestion[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(EMPTY_INDEX);
  const containerRef = useRef<HTMLDivElement | null>(null);

  const closeSuggestions = useCallback((): void => {
    setSelectedIndex(EMPTY_INDEX);
    setSuggestions([]);
  }, []);

  const insertEmote = useInsertEmote(closeSuggestions);
  const suggestionsOpen = suggestions.length > EMPTY_LENGTH;

  useAutocompleteListener(availableEmotes, closeSuggestions, setSelectedIndex, setSuggestions);
  useKeyboardNavigation({
    closeSuggestions,
    insertEmote,
    selectedIndex,
    setSelectedIndex,
    suggestions,
  });
  useOutsideClickDismiss(closeSuggestions, containerRef, suggestionsOpen);

  if (!suggestionsOpen) {
    return null;
  }

  return (
    <SuggestionList
      containerRef={containerRef}
      insertEmote={insertEmote}
      selectedIndex={selectedIndex}
      suggestions={suggestions}
    />
  );
};
