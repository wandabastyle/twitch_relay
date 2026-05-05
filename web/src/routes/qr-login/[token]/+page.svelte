<script lang="ts">
  import { page } from '$app/stores';
  import { login } from '$lib/api';

  let accessCode = $state('');
  let isBusy = $state(false);
  let errorMessage = $state<string | null>(null);
  let success = $state(false);

  // Get token from URL
  const token = $derived($page.params.token);

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

<svelte:head>
  <title>QR Login - Twitch Relay</title>
</svelte:head>

<main class="shell">
  <section class="panel">
    <header class="panel-header">
      <div class="panel-title">
        <p class="eyebrow">QR Login</p>
        <h1>Twitch Relay</h1>
      </div>
    </header>

    {#if errorMessage}
      <p class="error" role="alert">{errorMessage}</p>
    {/if}

    {#if success}
      <div class="success-message">
        <p class="success-text">Console logged in successfully!</p>
        <p class="success-subtext">You can close this window.</p>
      </div>
    {:else}
      <form class="login-form" onsubmit={submitLogin}>
        <label for="access-code">Access code</label>
        <input
          id="access-code"
          type="password"
          bind:value={accessCode}
          placeholder="Enter shared access code"
          autocomplete="current-password"
          disabled={isBusy}
        />
        <button type="submit" disabled={isBusy}>
          {isBusy ? 'Signing in...' : 'Sign in'}
        </button>
      </form>
    {/if}
  </section>
</main>

<style>
  /* Tokyo Night Moon theme tokens */
  :global(body) {
    --bg: #1e2030;
    --bg-soft: #222436;
    --surface: #2f334d;
    --surface-2: #3b4261;
    --fg: #c8d3f5;
    --muted: #a9b8e8;
    --accent: #82aaff;
    --accent-2: #c099ff;
    --success: #c3e88d;
    --danger: #ff757f;
    --border: #444a73;
    margin: 0;
    min-height: 100vh;
    background: radial-gradient(circle at 20% -10%, #3b4261 0%, #222436 45%, #1e2030 100%);
    color: var(--fg);
    font-family: 'Space Grotesk', 'IBM Plex Sans', 'Noto Sans', sans-serif;
  }

  .shell {
    min-height: 100dvh;
    box-sizing: border-box;
    display: grid;
    justify-items: center;
    align-content: center;
    padding: 1rem;
  }

  .panel {
    width: min(24rem, 100%);
    background: linear-gradient(160deg, rgba(47, 51, 77, 0.95), rgba(34, 36, 54, 0.95));
    border: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    border-radius: 1rem;
    padding: 1.5rem;
    box-shadow: 0 1rem 2.5rem rgba(3, 8, 16, 0.45);
  }

  .panel-header {
    margin-bottom: 1.25rem;
  }

  .panel-title {
    text-align: center;
  }

  h1 {
    margin: 0.2rem 0 0;
    font-size: 1.5rem;
    line-height: 1.1;
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.68rem;
    color: var(--muted);
  }

  .error {
    margin: 0 0 1rem;
    padding: 0.7rem 0.8rem;
    background: rgba(194, 67, 89, 0.18);
    border: 1px solid rgba(246, 135, 154, 0.45);
    border-radius: 0.6rem;
    color: color-mix(in srgb, var(--danger) 72%, white);
  }

  .success-message {
    text-align: center;
    padding: 1rem 0;
  }

  .success-text {
    margin: 0;
    color: var(--success);
    font-size: 1.1rem;
    font-weight: 600;
  }

  .success-subtext {
    margin: 0.5rem 0 0;
    color: var(--muted);
    font-size: 0.9rem;
  }

  .login-form {
    display: grid;
    gap: 0.75rem;
  }

  .login-form label {
    font-weight: 600;
    color: var(--fg);
  }

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

  button {
    border: 0;
    border-radius: 0.6rem;
    padding: 0.7rem 1rem;
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
</style>
