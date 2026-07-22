<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "./lib/api";
  import type {
    AppSnapshot,
    DeviceInfo,
    DriverStatus,
    GestureMap,
    LogEntry,
    NavId,
    PlatformInfo,
    ThemeMode,
    TrackpadSettings,
  } from "./lib/types";
  import StatusCard from "./components/StatusCard.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import GestureMapPanel from "./components/GestureMap.svelte";
  import DriverPanel from "./components/DriverPanel.svelte";
  import LogPanel from "./components/LogPanel.svelte";
  import Troubleshooting from "./components/Troubleshooting.svelte";

  let nav: NavId = $state("status");
  let theme: ThemeMode = $state("system");
  let version = $state("0.1.0");
  let loading = $state(true);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let driverMessage = $state<string | null>(null);

  let platform = $state<PlatformInfo | null>(null);
  let devices = $state<DeviceInfo[]>([]);
  let settings = $state<TrackpadSettings>({
    pointerSpeed: 0.5,
    acceleration: 0,
    tapToClick: true,
    naturalScroll: true,
    pinchSensitivity: 0.5,
    dragLock: false,
    forceThreshold: 0.5,
    rightClickTwoFinger: true,
    horizontalScroll: true,
  });
  let gestures = $state<GestureMap>({ bindings: [], backend: "default" });
  let driver = $state<DriverStatus | null>(null);
  let logs = $state<LogEntry[]>([]);

  const navItems: { id: NavId; label: string }[] = [
    { id: "status", label: "Status" },
    { id: "settings", label: "Settings" },
    { id: "gestures", label: "Gestures" },
    { id: "driver", label: "Driver" },
    { id: "logs", label: "Logs" },
    { id: "help", label: "Help" },
  ];

  function applyTheme(mode: ThemeMode) {
    theme = mode;
    localStorage.setItem("magicpad-theme", mode);
    const dark =
      mode === "dark" ||
      (mode === "system" &&
        window.matchMedia("(prefers-color-scheme: dark)").matches);
    document.documentElement.dataset.theme = dark ? "dark" : "light";
  }

  function cycleTheme() {
    const order: ThemeMode[] = ["system", "light", "dark"];
    const next = order[(order.indexOf(theme) + 1) % order.length];
    applyTheme(next);
  }

  async function loadAll() {
    loading = true;
    error = null;
    try {
      const snap: AppSnapshot = await api.snapshot();
      platform = snap.platform;
      devices = snap.devices;
      settings = snap.settings;
      gestures = snap.gestures;
      driver = snap.driver;
      version = await api.version();
      logs = await api.getLogs();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function refreshDevices() {
    loading = true;
    try {
      devices = await api.refreshDevices();
      logs = await api.getLogs();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function applySettings() {
    busy = true;
    error = null;
    try {
      await api.setSettings(settings);
      logs = await api.getLogs();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function resetSettings() {
    busy = true;
    try {
      settings = await api.resetSettings();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function saveGestures() {
    busy = true;
    try {
      await api.setGestures(gestures);
      logs = await api.getLogs();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function installDriver() {
    busy = true;
    driverMessage = null;
    try {
      const r = await api.installDriver();
      driverMessage = r.message + (r.logLines.length ? "\n" + r.logLines.join("\n") : "");
      driver = await api.driverStatus();
      logs = await api.getLogs();
    } catch (e) {
      driverMessage = String(e);
    } finally {
      busy = false;
    }
  }

  async function uninstallDriver() {
    busy = true;
    try {
      const r = await api.uninstallDriver();
      driverMessage = r.message + (r.logLines.length ? "\n" + r.logLines.join("\n") : "");
      driver = await api.driverStatus();
      logs = await api.getLogs();
    } catch (e) {
      driverMessage = String(e);
    } finally {
      busy = false;
    }
  }

  async function refreshDriver() {
    try {
      driver = await api.driverStatus();
    } catch (e) {
      error = String(e);
    }
  }

  async function openDriverSource() {
    const url = driver?.recommendedSource;
    if (!url) return;
    // Local paths: show in message; remote: open
    if (url.startsWith("http")) {
      try {
        await openUrl(url);
      } catch {
        driverMessage = `Open in browser: ${url}`;
      }
    } else {
      driverMessage = `Resource path: ${url}`;
    }
  }

  async function refreshLogs() {
    logs = await api.getLogs();
  }

  async function clearLogs() {
    await api.clearLogs();
    logs = await api.getLogs();
  }

  onMount(() => {
    const saved = (localStorage.getItem("magicpad-theme") as ThemeMode) || "system";
    applyTheme(saved);
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => {
      if (theme === "system") applyTheme("system");
    };
    mq.addEventListener("change", onChange);
    loadAll();
    const timer = setInterval(() => {
      if (nav === "status") {
        api.listDevices().then((d) => (devices = d)).catch(() => {});
      }
    }, 8000);
    return () => {
      mq.removeEventListener("change", onChange);
      clearInterval(timer);
    };
  });
</script>

<div class="shell">
  <aside class="sidebar">
    <div class="brand">
      <div class="logo" aria-hidden="true"></div>
      <div>
        <div class="title">MagicPad</div>
        <div class="sub">Companion</div>
      </div>
    </div>

    <nav>
      {#each navItems as item}
        <button
          class="nav-btn"
          class:active={nav === item.id}
          onclick={() => (nav = item.id)}
        >
          {item.label}
        </button>
      {/each}
    </nav>

    <div class="sidebar-foot">
      <button class="ghost theme-btn" onclick={cycleTheme}>
        Theme: {theme}
      </button>
      <div class="muted mono">v{version}</div>
    </div>
  </aside>

  <main>
    <header class="top">
      <h1>
        {navItems.find((n) => n.id === nav)?.label ?? "MagicPad"}
      </h1>
      {#if platform}
        <span class="badge info">{platform.kind}</span>
      {/if}
    </header>

    {#if error}
      <div class="error-banner">{error}</div>
    {/if}

    <div class="content">
      {#if nav === "status"}
        <StatusCard
          {devices}
          {platform}
          {loading}
          onRefresh={refreshDevices}
        />
      {:else if nav === "settings"}
        <SettingsPanel
          {settings}
          {busy}
          onChange={(s) => (settings = s)}
          onApply={applySettings}
          onReset={resetSettings}
        />
      {:else if nav === "gestures"}
        <GestureMapPanel
          {gestures}
          {busy}
          onChange={(g) => (gestures = g)}
          onSave={saveGestures}
        />
      {:else if nav === "driver"}
        <DriverPanel
          {driver}
          platformKind={platform?.kind ?? null}
          {busy}
          lastMessage={driverMessage}
          onInstall={installDriver}
          onUninstall={uninstallDriver}
          onRefresh={refreshDriver}
          onOpenSource={openDriverSource}
        />
      {:else if nav === "logs"}
        <LogPanel {logs} onClear={clearLogs} onRefresh={refreshLogs} />
      {:else if nav === "help"}
        <Troubleshooting platformKind={platform?.kind ?? null} />
      {/if}
    </div>
  </main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 220px 1fr;
    height: 100%;
    min-height: 100vh;
  }
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1.1rem 0.85rem;
    background: var(--bg-elevated);
    border-right: 1px solid var(--border);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.35rem 0.45rem 0.75rem;
  }
  .logo {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    background: linear-gradient(145deg, #0f172a, #164e63);
    border: 2px solid var(--accent);
    box-shadow: inset 0 0 0 4px rgba(34, 211, 238, 0.15);
  }
  .title {
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1.1;
  }
  .sub {
    font-size: 0.78rem;
    color: var(--text-muted);
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    flex: 1;
  }
  .nav-btn {
    text-align: left;
    border: none;
    background: transparent;
    padding: 0.55rem 0.7rem;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-weight: 550;
  }
  .nav-btn:hover:not(:disabled) {
    background: var(--bg-muted);
    border-color: transparent;
    color: var(--text);
  }
  .nav-btn.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .sidebar-foot {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    padding: 0.35rem;
  }
  .theme-btn {
    width: 100%;
    text-align: left;
    font-size: 0.85rem;
  }
  main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: auto;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1.15rem 1.35rem 0.35rem;
  }
  .content {
    padding: 0.75rem 1.35rem 1.5rem;
    max-width: 920px;
  }
  .error-banner {
    margin: 0.5rem 1.35rem 0;
    padding: 0.65rem 0.85rem;
    border-radius: var(--radius-sm);
    background: var(--danger-soft);
    color: var(--danger);
    font-size: 0.9rem;
    user-select: text;
  }
  @media (max-width: 720px) {
    .shell {
      grid-template-columns: 1fr;
    }
    .sidebar {
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    nav {
      flex-direction: row;
      flex-wrap: wrap;
    }
  }
</style>
