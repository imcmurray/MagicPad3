<script lang="ts">
  import type { DriverStatus, PlatformKind } from "../lib/types";
  import { driverStateLabel } from "../lib/labels";

  interface Props {
    driver: DriverStatus | null;
    platformKind: PlatformKind | null;
    busy: boolean;
    lastMessage: string | null;
    onInstall: () => void;
    onUninstall: () => void;
    onRefresh: () => void;
    onOpenSource: () => void;
  }

  let {
    driver,
    platformKind,
    busy,
    lastMessage,
    onInstall,
    onUninstall,
    onRefresh,
    onOpenSource,
  }: Props = $props();

  function stateClass(s: string): string {
    if (s === "installed") return "ok";
    if (s === "conflict" || s === "outdated") return "warn";
    if (s === "not_installed") return "err";
    return "info";
  }
</script>

<section class="card">
  <div class="head">
    <div>
      <h2>Driver & helpers</h2>
      <p class="muted">
        {#if platformKind === "windows"}
          Windows Precision Touchpad package (vitoplantamura / imbushuo lineage)
          for USB-C + Bluetooth gestures.
        {:else if platformKind === "linux"}
          udev rules, input-remapper profiles, and optional systemd user unit for
          EndeavourOS / Arch.
        {:else}
          Platform-specific helper management.
        {/if}
      </p>
    </div>
    <button class="ghost" onclick={onRefresh} disabled={busy}>Refresh</button>
  </div>

  {#if driver}
    <div class="status-block">
      <div class="title-row">
        <strong>{driver.name}</strong>
        <span class="badge {stateClass(driver.state)}">
          {driverStateLabel(driver.state)}
        </span>
      </div>
      {#if driver.version}
        <div class="muted mono">Version: {driver.version}</div>
      {/if}
      <p class="detail">{driver.detail}</p>
      {#if driver.conflicts.length}
        <p class="warn-text">
          Conflicts: {driver.conflicts.join(", ")}
        </p>
      {/if}
    </div>

    <div class="actions">
      <button
        class="primary"
        disabled={busy || !driver.canInstall}
        onclick={onInstall}
      >
        {platformKind === "linux" ? "Install helpers" : "Install driver"}
      </button>
      <button
        class="danger"
        disabled={busy || !driver.canUninstall}
        onclick={onUninstall}
      >
        Uninstall
      </button>
      {#if driver.recommendedSource}
        <button class="ghost" onclick={onOpenSource}>Open source / docs</button>
      {/if}
    </div>
  {:else}
    <p class="muted">Loading driver status…</p>
  {/if}

  {#if lastMessage}
    <div class="msg mono">{lastMessage}</div>
  {/if}

  {#if platformKind === "windows"}
    <ol class="steps muted">
      <li>Uninstall Magic Utilities / Trackpad++ if present.</li>
      <li>
        Download AMD64 or ARM64 package from MagicTrackpad2ForWindows releases
        into <code>%LOCALAPPDATA%\MagicPadCompanion\drivers\</code>
      </li>
      <li>Click Install driver (run as Administrator if prompted).</li>
      <li>Replug USB-C trackpad or re-pair Bluetooth.</li>
    </ol>
  {:else if platformKind === "linux"}
    <ol class="steps muted">
      <li>Click Install helpers (polkit/pkexec will prompt for root).</li>
      <li>Optional: <code>sudo pacman -S input-remapper</code></li>
      <li>Replug the trackpad; check Status for VID/PID and battery.</li>
    </ol>
  {/if}
</section>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }
  .status-block {
    padding: 0.85rem 1rem;
    border-radius: var(--radius-sm);
    background: var(--bg-muted);
    border: 1px solid var(--border);
    margin-bottom: 1rem;
  }
  .title-row {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    align-items: center;
    margin-bottom: 0.35rem;
  }
  .detail {
    margin: 0.5rem 0 0;
    font-size: 0.92rem;
  }
  .warn-text {
    color: var(--warn);
    font-size: 0.9rem;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }
  .msg {
    padding: 0.65rem 0.75rem;
    background: var(--accent-soft);
    border-radius: var(--radius-sm);
    margin-bottom: 0.75rem;
    white-space: pre-wrap;
  }
  .steps {
    margin: 0.5rem 0 0;
    padding-left: 1.2rem;
    font-size: 0.85rem;
    line-height: 1.55;
  }
  code {
    font-family: var(--mono);
    font-size: 0.8rem;
  }
</style>
