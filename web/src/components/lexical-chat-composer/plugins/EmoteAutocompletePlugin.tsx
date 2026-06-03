import { useCallback, useEffect, useRef, useState, type ReactElement } from 'react';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import {
  $createTextNode,
  $getSelection,
  $isRangeSelection,
  $isTextNode,
  COMMAND_PRIORITY_NORMAL,
  KEY_DOWN_COMMAND,
  KEY_ESCAPE_COMMAND,
  KEY_ENTER_COMMAND,
  KEY_TAB_COMMAND,
} from 'lexical';
import type { EmoteItem } from '../../../api-client';
import { $createEmoteNode } from '../nodes/EmoteNode';
import { Paper, Stack, Typography } from '@mui/material';

interface EmoteSuggestion {
  code: string;
  imageUrl: string;
}

interface EmoteAutocompletePluginProps {
  availableEmotes: EmoteItem[];
}

const MAX_SUGGESTIONS = 10;

// Simple ranking function
function rankEmotes(emotes: EmoteItem[], query: string): EmoteItem[] {
  const lowerQuery = query.toLowerCase();
  return emotes
    .filter((e) => e.code.toLowerCase().includes(lowerQuery))
    .sort((a, b) => {
      const aStarts = a.code.toLowerCase().startsWith(lowerQuery);
      const bStarts = b.code.toLowerCase().startsWith(lowerQuery);
      if (aStarts && !bStarts) return -1;
      if (!aStarts && bStarts) return 1;
      return a.code.length - b.code.length;
    })
    .slice(0, MAX_SUGGESTIONS);
}

export function EmoteAutocompletePlugin({
  availableEmotes,
}: EmoteAutocompletePluginProps): ReactElement | null {
  const [editor] = useLexicalComposerContext();
  const [suggestions, setSuggestions] = useState<EmoteSuggestion[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState('');
  const containerRef = useRef<HTMLDivElement | null>(null);

  // Get suggestions for a query
  const getSuggestions = useCallback(
    (searchQuery: string): EmoteSuggestion[] => {
      if (searchQuery.length === 0) {
        return availableEmotes.slice(0, MAX_SUGGESTIONS).map((e) => ({
          code: e.code,
          imageUrl: e.image_url,
        }));
      }
      return rankEmotes(availableEmotes, searchQuery).map((e) => ({
        code: e.code,
        imageUrl: e.image_url,
      }));
    },
    [availableEmotes],
  );

  // Insert emote at current selection
  const insertEmote = useCallback(
    (code: string, imageUrl: string) => {
      editor.update(() => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) {
          return;
        }

        // Get text node and find colon
        const anchorNode = selection.anchor.getNode();
        if (!$isTextNode(anchorNode)) {
          return;
        }

        const anchorOffset = selection.anchor.offset;
        const textContent = anchorNode.getTextContent();
        const beforeCursor = textContent.slice(0, anchorOffset);
        const lastColonIndex = beforeCursor.lastIndexOf(':');

        if (lastColonIndex === -1) {
          return;
        }

        // Select from colon to cursor and replace
        anchorNode.select(lastColonIndex, anchorOffset);
        const emoteNode = $createEmoteNode(code, imageUrl);
        const spaceNode = $createTextNode(' ');
        selection.insertNodes([emoteNode, spaceNode]);
      });

      setIsOpen(false);
      setSuggestions([]);
      setQuery('');
    },
    [editor],
  );

  // Check for autocomplete trigger
  useEffect(() => {
    return editor.registerUpdateListener(({ editorState }) => {
      editorState.read(() => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection)) {
          setIsOpen(false);
          return;
        }

        const anchorNode = selection.anchor.getNode();
        if (!$isTextNode(anchorNode)) {
          setIsOpen(false);
          return;
        }

        const anchorOffset = selection.anchor.offset;
        const textContent = anchorNode.getTextContent();
        const beforeCursor = textContent.slice(0, anchorOffset);

        // Match colon followed by word characters
        const match = beforeCursor.match(/:([a-zA-Z0-9]*)$/);
        if (match === null) {
          setIsOpen(false);
          setSuggestions([]);
          setQuery('');
          return;
        }

        const searchQuery = match[1];
        setQuery(searchQuery);
        const newSuggestions = getSuggestions(searchQuery);
        setSuggestions(newSuggestions);
        setSelectedIndex(0);
        setIsOpen(newSuggestions.length > 0);
      });
    });
  }, [editor, getSuggestions]);

  // Keyboard navigation
  useEffect(() => {
    const arrowUp = editor.registerCommand(
      KEY_DOWN_COMMAND,
      (event: KeyboardEvent) => {
        if (!isOpen || event.key !== 'ArrowUp') return false;
        event.preventDefault();
        setSelectedIndex((prev) => (prev > 0 ? prev - 1 : suggestions.length - 1));
        return true;
      },
      COMMAND_PRIORITY_NORMAL,
    );

    const arrowDown = editor.registerCommand(
      KEY_DOWN_COMMAND,
      (event: KeyboardEvent) => {
        if (!isOpen || event.key !== 'ArrowDown') return false;
        event.preventDefault();
        setSelectedIndex((prev) => (prev < suggestions.length - 1 ? prev + 1 : 0));
        return true;
      },
      COMMAND_PRIORITY_NORMAL,
    );

    const enterSelect = editor.registerCommand(
      KEY_ENTER_COMMAND,
      () => {
        if (!isOpen || suggestions.length === 0) return false;
        const suggestion = suggestions[selectedIndex];
        if (suggestion) {
          insertEmote(suggestion.code, suggestion.imageUrl);
        }
        return true;
      },
      COMMAND_PRIORITY_NORMAL,
    );

    const tabSelect = editor.registerCommand(
      KEY_TAB_COMMAND,
      (event: KeyboardEvent) => {
        if (!isOpen || suggestions.length === 0) return false;
        event.preventDefault();
        const suggestion = suggestions[selectedIndex];
        if (suggestion) {
          insertEmote(suggestion.code, suggestion.imageUrl);
        }
        return true;
      },
      COMMAND_PRIORITY_NORMAL,
    );

    const escapeClose = editor.registerCommand(
      KEY_ESCAPE_COMMAND,
      () => {
        if (!isOpen) return false;
        setIsOpen(false);
        return true;
      },
      COMMAND_PRIORITY_NORMAL,
    );

    return () => {
      arrowUp();
      arrowDown();
      enterSelect();
      tabSelect();
      escapeClose();
    };
  }, [editor, isOpen, suggestions, selectedIndex, insertEmote]);

  // Click outside to close
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Node;
      if (containerRef.current && !containerRef.current.contains(target)) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isOpen]);

  if (!isOpen || suggestions.length === 0) {
    return null;
  }

  return (
    <Paper
      ref={containerRef}
      elevation={3}
      sx={{
        position: 'absolute',
        bottom: '100%',
        left: 0,
        right: 0,
        mb: 1,
        maxHeight: 200,
        overflow: 'auto',
        zIndex: 1000,
      }}
    >
      <Stack spacing={0}>
        {suggestions.map((suggestion, index) => (
          <Typography
            key={suggestion.code}
            component="div"
            onClick={() => insertEmote(suggestion.code, suggestion.imageUrl)}
            sx={{
              px: 2,
              py: 1,
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: 1,
              bgcolor: index === selectedIndex ? 'action.selected' : 'inherit',
              '&:hover': { bgcolor: 'action.hover' },
            }}
          >
            <img
              src={suggestion.imageUrl}
              alt={suggestion.code}
              style={{ width: 24, height: 24, objectFit: 'contain' }}
            />
            {suggestion.code}
          </Typography>
        ))}
      </Stack>
    </Paper>
  );
}
