<script lang="ts">
  import type { GestureMap as GMap } from "../lib/types";
  import { actionLabel, ALL_ACTIONS, triggerLabel } from "../lib/labels";

  interface Props {
    gestures: GMap;
    busy: boolean;
    onChange: (g: GMap) => void;
    onSave: () => void;
  }

  let { gestures, busy, onChange, onSave }: Props = $props();

  function setAction(index: number, action: string) {
    const bindings = gestures.bindings.map((b, i) =>
      i === index ? { ...b, action: action as typeof b.action } : b,
    );
    onChange({ ...gestures, bindings });
  }
</script>

<section class="card">
  <div class="head">
    <div>
      <h2>Gesture customization</h2>
      <p class="muted">
        Backend: <span class="mono">{gestures.backend}</span> — maps 3/4-finger
        gestures to system actions where the platform allows.
      </p>
    </div>
    <button class="primary" onclick={onSave} disabled={busy}>
      {busy ? "Saving…" : "Save gestures"}
    </button>
  </div>

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
</section>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 1rem;
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
