<script lang="ts">
  import type { LogEntry } from "../lib/types";

  interface Props {
    logs: LogEntry[];
    onClear: () => void;
    onRefresh: () => void;
  }

  let { logs, onClear, onRefresh }: Props = $props();

  function levelClass(level: string): string {
    if (level === "error") return "err";
    if (level === "warn") return "warn";
    if (level === "debug") return "info";
    return "ok";
  }

  function fmtTime(ts: string): string {
    try {
      return new Date(ts).toLocaleTimeString();
    } catch {
      return ts;
    }
  }
</script>

<section class="card log-card">
  <div class="head">
    <div>
      <h2>Logging</h2>
      <p class="muted">In-app event stream for install steps and device scans.</p>
    </div>
    <div class="actions">
      <button class="ghost" onclick={onRefresh}>Refresh</button>
      <button class="ghost" onclick={onClear}>Clear</button>
    </div>
  </div>

  <div class="log-view mono">
    {#if logs.length === 0}
      <div class="muted">No log entries yet.</div>
    {:else}
      {#each [...logs].reverse() as entry (entry.id)}
        <div class="line">
          <span class="time">{fmtTime(entry.timestamp)}</span>
          <span class="badge {levelClass(entry.level)}">{entry.level}</span>
          <span class="src">[{entry.source}]</span>
          <span class="msg">{entry.message}</span>
        </div>
      {/each}
    {/if}
  </div>
</section>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.75rem;
  }
  .actions {
    display: flex;
    gap: 0.4rem;
  }
  .log-view {
    max-height: 420px;
    overflow: auto;
    background: var(--bg-muted);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 0.65rem 0.75rem;
    user-select: text;
  }
  .line {
    display: grid;
    grid-template-columns: 4.5rem auto auto 1fr;
    gap: 0.45rem;
    align-items: baseline;
    padding: 0.2rem 0;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
  }
  .time {
    color: var(--text-muted);
  }
  .src {
    color: var(--accent);
  }
  .msg {
    word-break: break-word;
  }
</style>
