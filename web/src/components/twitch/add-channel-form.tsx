import type { ReactElement } from 'react';

interface AddChannelFormProps {
  newChannelLogin: string;
  isAdding: boolean;
  onSubmit: (event: React.SyntheticEvent<HTMLFormElement>) => void;
  onCancel: () => void;
  onUpdateValue: (value: string) => void;
}

export const AddChannelForm = ({
  newChannelLogin,
  isAdding,
  onSubmit,
  onCancel,
  onUpdateValue,
}: AddChannelFormProps): ReactElement => (
  <form className="add-form" onSubmit={onSubmit}>
    <input
      className="ui-input"
      type="text"
      value={newChannelLogin}
      onChange={(event) => {
        onUpdateValue(event.currentTarget.value);
      }}
      placeholder="channel_login"
      autoComplete="off"
      spellCheck="false"
    />
    <button type="submit" disabled={isAdding}>
      {isAdding ? 'Adding...' : 'Add'}
    </button>
    <button type="button" className="ui-ghost-btn" onClick={onCancel}>
      Cancel
    </button>
  </form>
);
