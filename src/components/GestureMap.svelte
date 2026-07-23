<script lang="ts">
  import { onMount } from "svelte";
  import type { GestureDaemonStatus, GestureMap as GMap } from "../lib/types";
  import { actionLabel, ALL_ACTIONS, triggerLabel } from "../lib/labels";
  import { api } from "../lib/api";

  interface Props {
    gestures: GMap;
    busy: boolean;
    onChange: (g: GMap) => void;
    onSave: () => void;
  }

  let { gestures, busy, onChange, onSave }: Props = $props();

  let daemon = $state<GestureDaemonStatus | null>(null);
  let daemonBusy = $state(false);
  let daemonMsg = $state<string | null>(null);

  async function refreshDaemon() {
    try {
      daemon = await api.gestureDaemonStatus();
    } catch {
      daemon = null;
    }
  }

  async function startDaemon() {
    daemonBusy = true;
    daemonMsg = null;
    try {
      daemonMsg = await api.startGestureDaemon();
      await refreshDaemon();
    } catch (e) {
      daemonMsg = String(e);
      await refreshDaemon();
    } finally {
      daemonBusy = false;
    }
  }

  function setAction(index: number, action: string) {
    const bindings = gestures.bindings.map((b, i) =>
      i === index ? { ...b, action: action as typeof b.action } : b,
    );
    onChange({ ...gestures, bindings });
  }

  onMount(() => {
    refreshDaemon();
    const t = setInterval(refreshDaemon, 5000);
    return () => clearInterval(t);
  });
</script>

<section class="card">
  <div class="head">
    <div>
      <h2>Gesture customization</h2>
      <p class="muted">
        Backend: <span class="mono">{gestures.backend}</span>
        {#if daemon?.available}
          — multi-finger swipes are handled by the MagicPad gesture daemon
          (libinput → system shortcuts).
        {:else}
          — on Windows, the Precision driver / OS owns multi-finger gestures.
        {/if}
      </p>
    </div>
    <button class="primary" onclick={onSave} disabled={busy}>
      {busy ? "Saving…" : "Save gestures"}
    </button>
  </div>

  {#if daemon?.available}
    <div class="daemon" class:ok={daemon.running} class:warn={!daemon.running}>
      <div class="daemon-top">
        <strong>Linux gesture daemon</strong>
        <span class="badge" class:ok={daemon.running} class:warn={!daemon.running}>
          {daemon.running ? "Running" : "Stopped"}
        </span>
      </div>
      <p class="muted small">{daemon.message}</p>
      <ul class="checks">
        <li class:ok={daemon.libinputOk}>
          libinput-tools {daemon.libinputOk ? "✓" : "✗"}
        </li>
        <li class:ok={daemon.wtypeOk}>wtype {daemon.wtypeOk ? "✓" : "✗"}</li>
        <li class:ok={daemon.inputGroup}>
          input group {daemon.inputGroup ? "✓" : "✗ (log out after adding)"}
        </li>
      </ul>
      {#if !daemon.libinputOk || !daemon.wtypeOk}
        <p class="mono small tip">
          sudo pacman -S --needed libinput-tools wtype
        </p>
      {/if}
      {#if !daemon.inputGroup}
        <p class="mono small tip">
          sudo usermod -aG input "$USER" &amp;&amp; # then log out/in
        </p>
      {/if}
      <div class="daemon-actions">
        <button
          class="primary"
          disabled={daemonBusy || busy}
          onclick={startDaemon}
        >
          {daemonBusy ? "Starting…" : daemon.running ? "Restart daemon" : "Start daemon"}
        </button>
        <button class="ghost" disabled={daemonBusy} onclick={refreshDaemon}>
          Refresh status
        </button>
      </div>
      {#if daemonMsg}
        <p class="mono small result">{daemonMsg}</p>
      {/if}
    </div>
  {/if}

  <div class="table">
    {#each gestures.bindings as b, i (b.trigger)}
      <div class="row" class:disabled={!b.available}>
        <div class="trigger">{triggerLabel(b.trigger)}</div>
        <select
          disabled={!b.available || busy}
          value={b.action}
          onchange={(e) =>
            setAction(i, (e.currentTarget as HTMLSelectElement).value)}
        >
          {#each ALL_ACTIONS as a}
            <option value={a}>{actionLabel(a)}</option>
          {/each}
        </select>
        {#if !b.available}
          <span class="badge warn">OS only</span>
        {/if}
      </div>
    {/each}
  </div>

  {#if daemon?.available}
    <p class="muted small foot">
      Defaults map to Budgie/labwc shortcuts: Super+Page Up/Down (workspaces),
      Super+D (desktop), Super+A (Raven), Super+Tab (apps). Pinch out/in sends
      Ctrl+= / Ctrl+- (browser and app zoom). Save starts the user service
      <span class="mono">magicpad-gestures.service</span>.
    </p>
  {/if}
</section>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 1rem;
  }
  .daemon {
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 0.75rem 0.9rem;
    margin-bottom: 1rem;
    background: var(--bg-muted);
  }
  .daemon.ok {
    border-color: color-mix(in srgb, var(--success) 40%, var(--border));
  }
  .daemon.warn {
    border-color: color-mix(in srgb, var(--warn) 40%, var(--border));
  }
  .daemon-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }
  .checks {
    display: flex;
    flex-wrap: wrap;
    gap: 0.65rem 1rem;
    list-style: none;
    margin: 0.4rem 0 0.55rem;
    padding: 0;
    font-size: 0.85rem;
    color: var(--text-muted);
  }
  .checks li.ok {
    color: var(--success);
  }
  .daemon-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  .tip,
  .result,
  .small {
    font-size: 0.82rem;
  }
  .tip {
    margin: 0.25rem 0 0.5rem;
    color: var(--accent);
  }
  .result {
    margin: 0.5rem 0 0;
    white-space: pre-wrap;
    user-select: text;
  }
  .foot {
    margin: 0.85rem 0 0;
  }
  .table {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }
  .row {
    display: grid;
    grid-template-columns: 1fr minmax(180px, 240px) auto;
    gap: 0.75rem;
    align-items: center;
    padding: 0.45rem 0.55rem;
    border-radius: var(--radius-sm);
    background: var(--bg-muted);
  }
  .row.disabled {
    opacity: 0.65;
  }
  .trigger {
    font-size: 0.9rem;
    font-weight: 500;
  }
  select {
    width: 100%;
  }
</style>
