<script lang="ts">
  import type { AddChannelFormProps } from './types';

  let {
    newChannelLogin,
    isAdding,
    onSubmit,
    onCancel,
    onUpdateValue
  }: AddChannelFormProps = $props();
</script>

<form class="add-form" onsubmit={onSubmit}>
  <input
    type="text"
    value={newChannelLogin}
    oninput={(e) => onUpdateValue(e.currentTarget.value)}
    placeholder="channel_login"
    autocomplete="off"
    spellcheck="false"
  />
  <button type="submit" disabled={isAdding}>
    {isAdding ? 'Adding...' : 'Add'}
  </button>
  <button type="button" class="ghost" onclick={onCancel}>
    Cancel
  </button>
</form>

<style>
  .add-form {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .add-form input {
    flex: 1;
    text-transform: lowercase;
  }

  input {
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    background: rgba(8, 12, 19, 0.9);
    color: var(--fg);
    border-radius: 0.6rem;
    padding: 0.7rem 0.8rem;
    font: inherit;
  }

  button {
    border: 0;
    border-radius: 0.6rem;
    padding: 0.62rem 0.95rem;
    background: var(--accent);
    color: #1e2030;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .ghost {
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    color: var(--fg);
  }

  @media (max-width: 600px) {
    .add-form {
      flex-wrap: wrap;
    }

    .add-form input {
      width: 100%;
    }
  }
</style>
