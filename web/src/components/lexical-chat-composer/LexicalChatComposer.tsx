import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  type ReactElement,
  type Ref,
} from 'react';
import { LexicalComposer } from '@lexical/react/LexicalComposer';
import { useLexicalComposerContext } from '@lexical/react/LexicalComposerContext';
import { ContentEditable } from '@lexical/react/LexicalContentEditable';
import { LexicalErrorBoundary } from '@lexical/react/LexicalErrorBoundary';
import { HistoryPlugin } from '@lexical/react/LexicalHistoryPlugin';
import { PlainTextPlugin } from '@lexical/react/LexicalPlainTextPlugin';
import {
  $createTextNode,
  $getRoot,
  $getSelection,
  $isRangeSelection,
  COMMAND_PRIORITY_HIGH,
  KEY_ENTER_COMMAND,
} from 'lexical';
import { Box, IconButton } from '@mui/material';
import { Send as SendIcon } from '@mui/icons-material';
import type { EmoteItem } from '../../api-client';
import { $createEmoteNode, EmoteNode } from './nodes/EmoteNode';
import { EmoteAutocompletePlugin } from './plugins/EmoteAutocompletePlugin';
import { MaxLengthPlugin, SingleLinePlugin } from './plugins/SingleLinePlugin';

interface LexicalChatComposerProps {
  availableEmotes: EmoteItem[];
  disabled?: boolean;
  onSubmit: (text: string) => void;
}

export interface LexicalChatComposerHandle {
  insertEmote: (code: string) => void;
}

const initialConfig = {
  namespace: 'ChatComposer',
  onError: (error: Error) => {
    console.error('Lexical error:', error);
  },
  nodes: [EmoteNode],
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
    if (disabled) return;

    editor.getEditorState().read(() => {
      const root = $getRoot();
      const text = root.getTextContent().trim();

      if (text.length === 0) return;

      onSubmit(text);

      // Clear editor after submit
      editor.update(() => {
        $getRoot().clear();
      });
    });
  }, [disabled, editor, onSubmit]);

  const insertEmote = useCallback(
    (code: string): void => {
      if (disabled) return;

      const emote = availableEmotes.find((item) => item.code === code);
      if (emote === undefined) return;

      editor.focus(() => {
        editor.update(() => {
          const selection = $getSelection();
          if (!$isRangeSelection(selection)) return;

          selection.insertNodes([$createEmoteNode(emote.code, emote.image_url), $createTextNode(' ')]);
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
    <Box
      sx={{
        position: 'relative',
        width: '100%',
        display: 'grid',
        gridTemplateColumns: '1fr auto',
        alignItems: 'center',
        gap: 0.5,
        border: 1,
        borderColor: 'divider',
        borderRadius: 1.5,
        bgcolor: 'background.paper',
        opacity: disabled ? 0.65 : 1,
      }}
    >
      <PlainTextPlugin
        contentEditable={
          <ContentEditable
            aria-label="Send a message"
            aria-disabled={disabled}
            className="chat-composer-input"
            style={{
              outline: 'none',
              padding: '8px 14px',
              minHeight: '40px',
              caretColor: 'currentColor',
              whiteSpace: 'nowrap',
              overflow: 'hidden',
            }}
          />
        }
        placeholder={<div style={{ padding: '8px 14px', opacity: 0.5 }}>Send a message</div>}
        ErrorBoundary={LexicalErrorBoundary}
      />
      <IconButton disabled={disabled} onClick={handleSubmit} size="small" sx={{ mr: 0.5 }}>
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
    <Box sx={{ position: 'relative', width: '100%' }}>
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
