<script lang="ts">
  import type { PlatformKind } from "../lib/types";

  interface Props {
    platformKind: PlatformKind | null;
  }

  let { platformKind }: Props = $props();
</script>

<section class="card">
  <h2>Troubleshooting</h2>
  <p class="muted">Common fixes for USB-C reconnection, gestures, and lag.</p>

  <details open>
    <summary>Trackpad not detected (USB-C)</summary>
    <ul>
      <li>Use a data-capable USB-C cable (charge-only cables will not enumerate HID).</li>
      <li>Try another port / avoid passive hubs when first pairing.</li>
      <li>
        Confirm VID/PID: Magic Trackpad 3 USB-C is typically
        <code>VID_05AC&amp;PID_0324</code>.
      </li>
      {#if platformKind === "windows"}
        <li>Open Device Manager → Human Interface Devices after install.</li>
        <li>Install the Precision driver, then unplug/replug.</li>
      {:else if platformKind === "linux"}
        <li>Check <code>lsusb | grep 05ac</code> and <code>libinput list-devices</code>.</li>
        <li>Install udev helpers from the Driver tab; ensure user is in <code>input</code> group.</li>
      {/if}
    </ul>
  </details>

  <details>
    <summary>Bluetooth connects but gestures missing</summary>
    <ul>
      {#if platformKind === "windows"}
        <li>
          Generic HID is not enough — install MagicTrackpad2ForWindows (signed)
          or imbushuo PTP stack for full Precision gestures.
        </li>
        <li>Remove Magic Utilities if it conflicts with the PTP driver.</li>
      {:else if platformKind === "linux"}
        <li>Ensure <code>hid-magicmouse</code> is loaded: <code>lsmod | grep magic</code>.</li>
        <li>Use compositor multitouch (Mutter / KWin / labwc+libinput) for 3-finger.</li>
        <li>Install input-remapper for advanced maps.</li>
      {:else}
        <li>Use System Settings → Trackpad on macOS.</li>
      {/if}
    </ul>
  </details>

  <details>
    <summary>Gesture lag / stutter</summary>
    <ul>
      <li>Disable battery saver / USB selective suspend for the device.</li>
      <li>Prefer 2.4 GHz-clear Bluetooth or wired USB-C while testing.</li>
      <li>On Linux, avoid nested VMs; check CPU frequency governors.</li>
      <li>Lower pinch sensitivity and acceleration if cursor feels sticky.</li>
    </ul>
  </details>

  <details>
    <summary>Reconnection after sleep</summary>
    <ul>
      <li>Windows: Device Manager → power management → allow wake; update Bluetooth driver.</li>
      <li>Linux: blacklist aggressive USB autosuspend for the Apple HID if needed.</li>
      <li>Keep a short USB-C cable attached for charge + instant wake.</li>
    </ul>
  </details>

  <details>
    <summary>Battery percentage missing</summary>
    <ul>
      <li>Requires a driver/stack that exposes battery (Windows PTP fork, or Linux power_supply).</li>
      <li>Wired USB-C may report charging without a percent on some stacks — that is expected.</li>
    </ul>
  </details>
</section>

<style>
  details {
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 0.55rem 0.85rem;
    margin-top: 0.55rem;
    background: var(--bg-muted);
  }
  summary {
    cursor: pointer;
    font-weight: 600;
    font-size: 0.92rem;
  }
  ul {
    margin: 0.5rem 0 0.25rem;
    padding-left: 1.15rem;
    font-size: 0.88rem;
    color: var(--text-muted);
    line-height: 1.5;
  }
  code {
    font-family: var(--mono);
    font-size: 0.8rem;
    color: var(--text);
  }
</style>
