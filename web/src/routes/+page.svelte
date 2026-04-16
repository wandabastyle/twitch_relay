<script lang="ts">
  import { onMount } from 'svelte';

  import { createWatchTicket, getChannels, getSessionState, login, logout } from '$lib/api';

  type AuthMode = 'checking' | 'authenticated' | 'unauthenticated';

  let authMode = $state<AuthMode>('checking');
  let isBusy = $state(false);
  let errorMessage = $state<string | null>(null);
  let accessCode = $state('');
  let channels = $state<Array<{ login: string }>>([]);
  let watchingChannel = $state<string | null>(null);

  onMount(async () => {
    await initialize();
  });

  async function initialize(): Promise<void> {
    errorMessage = null;
    authMode = 'checking';

    try {
      const authenticated = await getSessionState();
      authMode = authenticated ? 'authenticated' : 'unauthenticated';
      if (authenticated) {
        await loadChannels();
      }
    } catch (err) {
      authMode = 'unauthenticated';
      errorMessage = readMessage(err, 'failed to initialize session');
    }
  }

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
      await login(normalized);
      accessCode = '';
      authMode = 'authenticated';
      await loadChannels();
    } catch (err) {
      errorMessage = readMessage(err, 'login failed');
    } finally {
      isBusy = false;
    }
  }

  async function loadChannels(): Promise<void> {
    try {
      channels = await getChannels();
    } catch (err) {
      errorMessage = readMessage(err, 'failed to load channels');
      channels = [];
    }
  }

  async function startWatching(channelLogin: string): Promise<void> {
    watchingChannel = channelLogin;
    errorMessage = null;

    try {
      const ticket = await createWatchTicket(channelLogin);
      window.location.assign(ticket.watch_url);
    } catch (err) {
      errorMessage = readMessage(err, `failed to open ${channelLogin}`);
    } finally {
      watchingChannel = null;
    }
  }

  async function signOut(): Promise<void> {
    isBusy = true;
    errorMessage = null;

    try {
      await logout();
      authMode = 'unauthenticated';
      channels = [];
    } catch (err) {
      errorMessage = readMessage(err, 'logout failed');
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
  <title>Twitch Relay</title>
</svelte:head>

<main class="shell">
  <section class="panel">
    <header class="panel-header">
      <div>
        <p class="eyebrow">Private Deck</p>
        <h1>Twitch Relay</h1>
      </div>

      {#if authMode === 'authenticated'}
        <button class="ghost" onclick={signOut} disabled={isBusy}>
          Sign out
        </button>
      {/if}
    </header>

    {#if errorMessage}
      <p class="error" role="alert">{errorMessage}</p>
    {/if}

    {#if authMode === 'checking'}
      <p class="muted">Checking session...</p>
    {:else if authMode === 'unauthenticated'}
      <form class="login-form" onsubmit={submitLogin}>
        <label for="access-code">Access code</label>
        <input
          id="access-code"
          type="password"
          bind:value={accessCode}
          placeholder="Enter shared access code"
          autocomplete="current-password"
        />
        <button type="submit" disabled={isBusy}>{isBusy ? 'Signing in...' : 'Sign in'}</button>
      </form>
    {:else}
      <div class="channels">
        {#if channels.length === 0}
          <p class="muted">No channels configured yet.</p>
        {:else}
          {#each channels as channel (channel.login)}
            <article class="channel-card">
              <div>
                <p class="channel-name">{channel.login}</p>
                <p class="channel-subtitle">Allowlisted channel</p>
              </div>
              <button
                type="button"
                onclick={() => startWatching(channel.login)}
                disabled={watchingChannel === channel.login}
              >
                {watchingChannel === channel.login ? 'Opening...' : 'Watch'}
              </button>
            </article>
          {/each}
        {/if}
      </div>
    {/if}
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
    min-height: 100vh;
    background: radial-gradient(circle at 20% -10%, #29324a 0%, #111722 45%, #090d14 100%);
    color: #edf2fb;
    font-family: 'Space Grotesk', 'IBM Plex Sans', 'Noto Sans', sans-serif;
  }

  .shell {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 2rem 1rem;
  }

  .panel {
    width: min(46rem, 100%);
    background: linear-gradient(160deg, rgba(20, 28, 43, 0.95), rgba(13, 18, 28, 0.95));
    border: 1px solid rgba(164, 182, 216, 0.25);
    border-radius: 1rem;
    padding: 1.2rem;
    box-shadow: 0 1rem 2.5rem rgba(3, 8, 16, 0.45);
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  h1 {
    margin: 0.2rem 0 0;
    font-size: clamp(1.5rem, 4vw, 2rem);
    line-height: 1.1;
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.68rem;
    color: #9cb2d7;
  }

  .error {
    margin: 0 0 1rem;
    padding: 0.7rem 0.8rem;
    background: rgba(194, 67, 89, 0.18);
    border: 1px solid rgba(246, 135, 154, 0.45);
    border-radius: 0.6rem;
    color: #ffd9e2;
  }

  .muted {
    margin: 0;
    color: #b6c4de;
  }

  .login-form {
    display: grid;
    gap: 0.75rem;
  }

  .login-form label {
    font-weight: 600;
    color: #d7e2f7;
  }

  input {
    border: 1px solid rgba(160, 181, 216, 0.35);
    background: rgba(8, 12, 19, 0.9);
    color: #f1f5ff;
    border-radius: 0.6rem;
    padding: 0.7rem 0.8rem;
    font: inherit;
  }

  button {
    border: 0;
    border-radius: 0.6rem;
    padding: 0.62rem 0.95rem;
    background: linear-gradient(130deg, #ff6f61, #cf4f50);
    color: #fff6f0;
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
    border: 1px solid rgba(162, 182, 217, 0.35);
    color: #d5e0f7;
  }

  .channels {
    display: grid;
    gap: 0.75rem;
  }

  .channel-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    border: 1px solid rgba(156, 178, 215, 0.22);
    background: rgba(10, 16, 27, 0.78);
    border-radius: 0.75rem;
    padding: 0.8rem;
  }

  .channel-name {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    text-transform: lowercase;
    color: #f2f7ff;
  }

  .channel-subtitle {
    margin: 0.15rem 0 0;
    color: #9eb3d6;
    font-size: 0.87rem;
  }

  @media (max-width: 600px) {
    .panel {
      padding: 1rem;
    }

    .channel-card {
      align-items: flex-start;
      flex-direction: column;
    }

    .channel-card button {
      width: 100%;
    }
  }
</style>
