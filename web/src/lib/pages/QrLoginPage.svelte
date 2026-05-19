<script lang="ts">
  import { login } from '$lib/api-client';

  interface Props {
    token: string;
  }

  let { token }: Props = $props();

  let accessCode = $state('');
  let isBusy = $state(false);
  let errorMessage = $state<string | null>(null);
  let success = $state(false);

  async function submitLogin(event: SubmitEvent): Promise<void> {
    event.preventDefault();

    const normalized = accessCode.trim();
    if (!normalized) {
      errorMessage = 'access code is required';
      return;
    }

    isBusy = true;
    errorMessage = null;

    try {
      await login(normalized, token);
      success = true;
    } catch (err) {
      errorMessage = readMessage(err, 'login failed');
    } finally {
      isBusy = false;
    }
  }

  function readMessage(error: unknown, fallback: string): string {
    if (error instanceof Error && error.message.trim().length > 0) {
      return error.message;
    }
    return fallback;
  }
</script>

<main class="ui-page-shell ui-page-shell--centered">
  <section class="ui-page-panel ui-page-panel--narrow">
    <header class="ui-panel-header--centered">
      <p class="ui-page-eyebrow">QR Login</p>
      <h1 class="ui-page-title">Twitch Relay</h1>
    </header>

    {#if errorMessage}
      <p class="ui-error" role="alert">{errorMessage}</p>
    {/if}

    {#if success}
      <div class="ui-success-message">
        <p class="ui-success-text">Console logged in successfully!</p>
        <p class="ui-success-subtext">You can close this window.</p>
      </div>
    {:else}
      <form class="ui-form" onsubmit={submitLogin}>
        <label class="ui-label" for="access-code">Access code</label>
        <input
          id="access-code"
          type="password"
          bind:value={accessCode}
          placeholder="Enter shared access code"
          autocomplete="current-password"
          disabled={isBusy}
        />
        <button class="ui-button-primary" type="submit" disabled={isBusy}>
          {isBusy ? 'Signing in...' : 'Sign in'}
        </button>
      </form>
    {/if}
  </section>
</main>

<style>
  /* QR Login specific input styling - extends shared .ui-input pattern */
  input {
    border: 1px solid rgba(160, 181, 216, 0.35);
    background: rgba(8, 12, 19, 0.9);
    color: var(--fg);
    border-radius: 0.6rem;
    padding: 0.7rem 0.8rem;
    font: inherit;
  }

  input:disabled {
    opacity: 0.6;
  }
</style>
