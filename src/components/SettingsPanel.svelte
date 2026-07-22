<script lang="ts">
  import type { TrackpadSettings } from "../lib/types";

  interface Props {
    settings: TrackpadSettings;
    busy: boolean;
    onChange: (s: TrackpadSettings) => void;
    onApply: () => void;
    onReset: () => void;
  }

  let { settings, busy, onChange, onApply, onReset }: Props = $props();

  function patch(partial: Partial<TrackpadSettings>) {
    onChange({ ...settings, ...partial });
  }
</script>

<section class="card">
  <div class="head">
    <div>
      <h2>Trackpad settings</h2>
      <p class="muted">
        Unified controls mapped to Windows Precision / libinput where supported.
      </p>
    </div>
    <div class="actions">
      <button class="ghost" onclick={onReset} disabled={busy}>Reset</button>
      <button class="primary" onclick={onApply} disabled={busy}>
        {busy ? "Applying…" : "Apply"}
      </button>
    </div>
  </div>

  <div class="grid">
    <div class="field">
      <label for="speed">Cursor speed ({settings.pointerSpeed.toFixed(2)})</label>
      <input
        id="speed"
        type="range"
        min="0"
        max="1"
        step="0.01"
        value={settings.pointerSpeed}
        oninput={(e) =>
          patch({ pointerSpeed: Number((e.currentTarget as HTMLInputElement).value) })}
      />
    </div>

    <div class="field">
      <label for="accel">Acceleration ({settings.acceleration.toFixed(2)})</label>
      <input
        id="accel"
        type="range"
        min="-1"
        max="1"
        step="0.01"
        value={settings.acceleration}
        oninput={(e) =>
          patch({ acceleration: Number((e.currentTarget as HTMLInputElement).value) })}
      />
    </div>

    <div class="field">
      <label for="pinch">Pinch sensitivity ({settings.pinchSensitivity.toFixed(2)})</label>
      <input
        id="pinch"
        type="range"
        min="0"
        max="1"
        step="0.01"
        value={settings.pinchSensitivity}
        oninput={(e) =>
          patch({
            pinchSensitivity: Number((e.currentTarget as HTMLInputElement).value),
          })}
      />
    </div>

    <div class="field">
      <label for="force">Force / click threshold ({settings.forceThreshold.toFixed(2)})</label>
      <input
        id="force"
        type="range"
        min="0"
        max="1"
        step="0.01"
        value={settings.forceThreshold}
        oninput={(e) =>
          patch({
            forceThreshold: Number((e.currentTarget as HTMLInputElement).value),
          })}
      />
    </div>
  </div>

  <div class="toggles">
    <label class="toggle">
      <input
        type="checkbox"
        checked={settings.tapToClick}
        onchange={(e) =>
          patch({ tapToClick: (e.currentTarget as HTMLInputElement).checked })}
      />
      Tap to click
    </label>
    <label class="toggle">
      <input
        type="checkbox"
        checked={settings.naturalScroll}
        onchange={(e) =>
          patch({ naturalScroll: (e.currentTarget as HTMLInputElement).checked })}
      />
      Natural scrolling
    </label>
    <label class="toggle">
      <input
        type="checkbox"
        checked={settings.dragLock}
        onchange={(e) =>
          patch({ dragLock: (e.currentTarget as HTMLInputElement).checked })}
      />
      Drag lock
    </label>
    <label class="toggle">
      <input
        type="checkbox"
        checked={settings.rightClickTwoFinger}
        onchange={(e) =>
          patch({
            rightClickTwoFinger: (e.currentTarget as HTMLInputElement).checked,
          })}
      />
      Two-finger secondary click
    </label>
    <label class="toggle">
      <input
        type="checkbox"
        checked={settings.horizontalScroll}
        onchange={(e) =>
          patch({
            horizontalScroll: (e.currentTarget as HTMLInputElement).checked,
          })}
      />
      Horizontal scroll
    </label>
  </div>
</section>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 1rem;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.25rem 1.25rem;
  }
  @media (max-width: 720px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
  .toggles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.55rem 1rem;
    margin-top: 0.5rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border);
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    font-size: 0.92rem;
  }
</style>
