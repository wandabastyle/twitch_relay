import { type Dispatch, type SetStateAction, useCallback, useEffect, useState } from 'react';

interface ChatOnlyMode {
  chatOnly: boolean;
  toggleChatCollapse: () => void;
  toggleChatOnly: () => void;
}

export const useChatOnlyMode = (
  ticket: string,
  setIsChatCollapsed: Dispatch<SetStateAction<boolean>>,
): ChatOnlyMode => {
  const [chatOnly, setChatOnly] = useState(false);

  const toggleChatOnly = useCallback((): void => {
    setIsChatCollapsed(false);
    setChatOnly((previous) => !previous);
  }, [setIsChatCollapsed]);

  const toggleChatCollapse = useCallback((): void => {
    setChatOnly(false);
    setIsChatCollapsed((previous) => !previous);
  }, [setIsChatCollapsed]);

  useEffect(() => {
    setChatOnly(false);
  }, [ticket]);

  return { chatOnly, toggleChatCollapse, toggleChatOnly };
};
