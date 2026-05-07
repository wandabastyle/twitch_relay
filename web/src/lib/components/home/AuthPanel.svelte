<script lang="ts">
  import type { AuthPanelProps } from './types';

  let {
    loginMode,
    accessCode,
    qrDataUrl,
    isBusy,
    onSubmitLogin,
    onSwitchToQr,
    onSwitchToCode,
    onUpdateAccessCode
  }: AuthPanelProps = $props();
</script>

{#if loginMode === 'code'}
  <form class="login-form" onsubmit={onSubmitLogin}>
    <label for="access-code">Access code</label>
    <input
      id="access-code"
      class="ui-input"
      type="password"
      value={accessCode}
      oninput={(e) => onUpdateAccessCode(e.currentTarget.value)}
      placeholder="Enter shared access code"
      autocomplete="current-password"
    />
    <button type="submit" disabled={isBusy}>{isBusy ? 'Signing in...' : 'Sign in'}</button>
    <button type="button" class="ui-ghost-btn" onclick={onSwitchToQr}>
      Sign in with QR code
    </button>
  </form>
{:else}
  <div class="qr-login">
    {#if qrDataUrl}
      <img src={qrDataUrl} alt="QR Code for login" class="qr-code" />
    {:else}
      <div class="qr-placeholder">Generating QR code...</div>
    {/if}
    <p class="qr-instructions">
      Scan with your phone
      <br />
      <span class="qr-expires">expires in 5 minutes</span>
    </p>
    <button type="button" class="ui-ghost-btn" onclick={onSwitchToCode}>
      Sign in with access code
    </button>
  </div>
{/if}

<style>
  .login-form {
    display: grid;
    gap: 0.75rem;
  }

  .login-form label {
    font-weight: 600;
    color: var(--fg);
  }

  /* input styles now provided by app.css via .ui-input */

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

  /* .ghost button styles now provided by app.css via .ui-ghost-btn */

  .qr-login {
    display: grid;
    gap: 0.75rem;
    place-items: center;
  }

  .qr-code {
    width: 200px;
    height: 200px;
    border-radius: 0.75rem;
    background: var(--surface);
    padding: 0.5rem;
  }

  .qr-placeholder {
    width: 200px;
    height: 200px;
    border-radius: 0.75rem;
    background: var(--surface);
    display: grid;
    place-items: center;
    color: var(--muted);
    font-size: 0.9rem;
  }

  .qr-instructions {
    margin: 0;
    text-align: center;
    color: var(--fg);
    font-size: 0.95rem;
  }

  .qr-expires {
    color: var(--muted);
    font-size: 0.8rem;
  }
</style>
