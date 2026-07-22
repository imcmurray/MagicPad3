<script lang="ts">
  import type { DeviceInfo, PlatformInfo } from "../lib/types";
  import { connectionLabel, hexId } from "../lib/labels";

  interface Props {
    devices: DeviceInfo[];
    platform: PlatformInfo | null;
    loading: boolean;
    onRefresh: () => void;
  }

  let { devices, platform, loading, onRefresh }: Props = $props();

  function batteryText(d: DeviceInfo): string {
    if (!d.battery?.percent && d.battery?.percent !== 0) return "Battery: —";
    const ch =
      d.battery.charging === true
        ? " (charging)"
        : d.battery.charging === false
          ? ""
          : "";
    return `Battery: ${d.battery.percent}%${ch}`;
  }
</script>

<section class="card status">
  <div class="head">
    <div>
      <h2>Connection status</h2>
      <p class="muted">
        {#if platform}
          {platform.osName}
          {#if platform.desktop}· {platform.desktop}{/if}
          · {platform.arch}
        {:else}
          Detecting platform…
        {/if}
      </p>
    </div>
    <button onclick={onRefresh} disabled={loading}>
      {loading ? "Scanning…" : "Refresh"}
    </button>
  </div>

  {#if devices.length === 0}
    <div class="empty">
      <div class="pad-icon" aria-hidden="true"></div>
      <p><strong>No Magic Trackpad detected</strong></p>
      <p class="muted">
        Connect via USB-C (A3120 / VID_05AC&amp;PID_0324) or pair over Bluetooth,
        then refresh. On Windows, install the Precision driver if the device is
        present but gestures are missing.
      </p>
    </div>
  {:else}
    <ul class="devices">
      {#each devices as d (d.id)}
        <li>
          <div class="dev-top">
            <div>
              <strong>{d.name}</strong>
              <div class="muted">{d.model}</div>
            </div>
            <span class="badge" class:ok={d.connected}>
              {d.connected ? "Connected" : "Offline"}
            </span>
          </div>
          <div class="meta">
            <span class="badge info">{connectionLabel(d.connection)}</span>
            <span class="mono">VID {hexId(d.vid)} · PID {hexId(d.pid)}</span>
            <span>{batteryText(d)}</span>
            {#if d.driverBound != null}
              <span class="badge" class:ok={d.driverBound} class:warn={!d.driverBound}>
                {d.driverBound ? "Driver bound" : "Generic HID"}
              </span>
            {/if}
          </div>
          {#if d.path}
            <div class="mono path">{d.path}</div>
          {/if}
          {#if d.notes.length}
            <ul class="notes">
              {#each d.notes as n}
                <li class="muted">{n}</li>
              {/each}
            </ul>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1rem;
  }
  .empty {
    text-align: center;
    padding: 1.5rem 1rem 0.5rem;
  }
  .pad-icon {
    width: 72px;
    height: 56px;
    margin: 0 auto 0.75rem;
    border-radius: 14px;
    background: linear-gradient(145deg, var(--bg-muted), var(--border));
    border: 2px solid var(--accent);
    box-shadow: inset 0 0 0 6px var(--bg-elevated);
    position: relative;
  }
  .pad-icon::after {
    content: "";
    position: absolute;
    inset: 18px 22px;
    border-radius: 8px;
    background: var(--accent-soft);
  }
  .devices {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .devices > li {
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 0.85rem 1rem;
    background: var(--bg-muted);
  }
  .dev-top {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.55rem;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem 0.75rem;
    align-items: center;
    font-size: 0.85rem;
  }
  .path {
    margin-top: 0.45rem;
    color: var(--text-muted);
    word-break: break-all;
  }
  .notes {
    margin: 0.4rem 0 0;
    padding-left: 1.1rem;
  }
</style>
