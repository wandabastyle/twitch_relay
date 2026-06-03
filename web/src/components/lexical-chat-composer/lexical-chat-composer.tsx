import { LexicalComposer } from '@lexical/react/LexicalComposer';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import { ContentEditable } from '@lexical/react/LexicalContentEditable';
import { LexicalErrorBoundary } from '@lexical/react/LexicalErrorBoundary';
import { HistoryPlugin } from '@lexical/react/LexicalHistoryPlugin';
import { PlainTextPlugin } from '@lexical/react/LexicalPlainTextPlugin';
import { Send as SendIcon } from '@mui/icons-material';
import { Box, IconButton } from '@mui/material';
import {
  $createTextNode,
  $getRoot,
  $getSelection,
  $isRangeSelection,
  COMMAND_PRIORITY_HIGH,
  KEY_ENTER_COMMAND,
} from 'lexical';
import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  type ReactElement,
  type Ref,
} from 'react';
import type { EmoteItem } from '../../api-client';
import { $createEmoteNode, EmoteNode } from './nodes/emote-node';
import { EmoteAutocompletePlugin } from './plugins/emote-autocomplete-plugin';
import { MaxLengthPlugin, SingleLinePlugin } from './plugins/single-line-plugin';

interface LexicalChatComposerProps {
  availableEmotes: EmoteItem[];
  disabled?: boolean;
  onSubmit: (text: string) => void;
}

export interface LexicalChatComposerHandle {
  insertEmote: (code: string) => void;
}

const DISABLED_OPACITY = 0.65;
const EMPTY_TEXT_LENGTH = 0;
const ENABLED_OPACITY = 1;
const ICON_BUTTON_MARGIN_RIGHT = 0.5;

const composerRootSx = {
  alignItems: 'center',
  bgcolor: 'background.paper',
  border: 1,
  borderColor: 'divider',
  borderRadius: 1.5,
  display: 'grid',
  gap: 0.5,
  gridTemplateColumns: '1fr auto',
  position: 'relative',
  width: '100%',
};

const outerComposerSx = {
  position: 'relative',
  width: '100%',
};

const composerInputStyle = {
  caretColor: 'currentColor',
  minHeight: '40px',
  outline: 'none',
  overflow: 'hidden',
  padding: '8px 14px',
  whiteSpace: 'nowrap',
};

const placeholderStyle = {
  opacity: 0.5,
  padding: '8px 14px',
};

const handleLexicalError = (error: Error): void => {
  queueMicrotask(() => {
    throw error;
  });
};

const initialConfig = {
  namespace: 'ChatComposer',
  nodes: [EmoteNode],
  onError: handleLexicalError,
  theme: {
    paragraph: 'chat-paragraph',
    text: {
      bold: 'chat-text-bold',
      italic: 'chat-text-italic',
      strikethrough: 'chat-text-strikethrough',
      underline: 'chat-text-underline',
    },
  },
};

interface ComposerBodyProps extends LexicalChatComposerProps {
  disabled: boolean;
  refHandle: Ref<LexicalChatComposerHandle>;
}

const ComposerBody = ({
  availableEmotes,
  disabled,
  onSubmit,
  refHandle,
}: ComposerBodyProps): ReactElement => {
  const [editor] = useLexicalComposerContext();

  const handleSubmit = useCallback(() => {
    if (disabled) {
      return;
    }

    editor.getEditorState().read(() => {
      const root = $getRoot();
      const text = root.getTextContent().trim();

      if (text.length === EMPTY_TEXT_LENGTH) {
        return;
      }

      onSubmit(text);

      // Clear editor after submit
      editor.update(() => {
        $getRoot().clear();
      });
    });
  }, [disabled, editor, onSubmit]);

  const insertEmote = useCallback(
    (code: string): void => {
      if (disabled) {
        return;
      }

      const emote = availableEmotes.find((item) => item.code === code);
      if (emote === undefined) {
        return;
      }

      editor.focus(() => {
        editor.update(() => {
          const selection = $getSelection();
          if (!$isRangeSelection(selection)) {
            return;
          }

          selection.insertNodes([
            $createEmoteNode(emote.code, emote.image_url),
            $createTextNode(' '),
          ]);
        });
      });
    },
    [availableEmotes, disabled, editor],
  );

  useImperativeHandle(refHandle, () => ({ insertEmote }), [insertEmote]);

  useEffect(() => {
    editor.setEditable(!disabled);
  }, [disabled, editor]);

  useEffect(
    () =>
      editor.registerCommand(
        KEY_ENTER_COMMAND,
        (event) => {
          event?.preventDefault();
          handleSubmit();
          return true;
        },
        COMMAND_PRIORITY_HIGH,
      ),
    [editor, handleSubmit],
  );

  return (
    <Box sx={{ ...composerRootSx, opacity: disabled ? DISABLED_OPACITY : ENABLED_OPACITY }}>
      <PlainTextPlugin
        contentEditable={
          <ContentEditable
            aria-label="Send a message"
            aria-disabled={disabled}
            className="chat-composer-input"
            style={composerInputStyle}
          />
        }
        placeholder={<div style={placeholderStyle}>Send a message</div>}
        ErrorBoundary={LexicalErrorBoundary}
      />
      <IconButton
        disabled={disabled}
        onClick={handleSubmit}
        size="small"
        sx={{ mr: ICON_BUTTON_MARGIN_RIGHT }}
      >
        <SendIcon />
      </IconButton>
      <HistoryPlugin />
      <SingleLinePlugin />
      <MaxLengthPlugin />
      <EmoteAutocompletePlugin availableEmotes={availableEmotes} />
    </Box>
  );
};

export const LexicalChatComposer = forwardRef<LexicalChatComposerHandle, LexicalChatComposerProps>(
  ({ availableEmotes, disabled = false, onSubmit }, ref): ReactElement => (
    <Box sx={outerComposerSx}>
      <LexicalComposer initialConfig={initialConfig}>
        <ComposerBody
          availableEmotes={availableEmotes}
          disabled={disabled}
          onSubmit={onSubmit}
          refHandle={ref}
        />
      </LexicalComposer>
    </Box>
  ),
);

LexicalChatComposer.displayName = 'LexicalChatComposer';
