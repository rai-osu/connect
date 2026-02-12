<script lang="ts">
  import { onMount } from "svelte";
  import {
    store,
    isConnected,
    isConnecting,
    hasError,
    canConnect,
    loadConfig,
    updateConfig,
    detectOsuPath,
    refreshStatus,
    startProxy,
    connect,
    disconnect,
  } from "$lib/stores/app.svelte";
  import StatusIndicator from "$lib/components/StatusIndicator.svelte";
  import StatsCard from "$lib/components/StatsCard.svelte";
  import Button from "$lib/components/Button.svelte";
  import Settings from "$lib/components/Settings.svelte";
  import LogViewer from "$lib/components/LogViewer.svelte";
  import { FileText, Settings as SettingsIcon } from "lucide-svelte";

  let showSettings = $state(false);
  let showLogs = $state(false);

  // Local derived state from getter functions (allowed in components)
  const connected = $derived(isConnected());
  const connecting = $derived(isConnecting());
  const error = $derived(hasError());
  const connectable = $derived(canConnect());

  onMount(() => {
    // Load saved configuration and auto-detect path
    (async () => {
      await loadConfig();

      if (!store.config.osu_path) {
        const detected = await detectOsuPath();
        if (detected) {
          await updateConfig("osu_path", detected);
        }
      }
    })();

    // Set up status polling when connected
    const interval = setInterval(async () => {
      if (isConnected()) {
        await refreshStatus();
      }
    }, 2000);

    return () => clearInterval(interval);
  });
</script>

<main class="min-h-screen bg-[--color-rai-bg] p-6 flex flex-col">
  <!-- Header -->
  <header class="flex items-center justify-between mb-8">
    <div class="flex items-center gap-3">
      <img src="/favicon.png" alt="rai!connect" class="w-10 h-10" />
      <div>
        <h1 class="text-xl font-bold text-[--color-rai-text]">rai!connect</h1>
        <p class="text-xs text-[--color-rai-text-muted]">osu!direct mirror proxy</p>
      </div>
    </div>
    <div class="flex items-center gap-2">
      {#if store.config.debug_logging}
        <button
          onclick={() => { showLogs = !showLogs; showSettings = false; }}
          class="p-2 rounded-lg hover:bg-[--color-rai-card] transition-colors"
          class:bg-[--color-rai-card]={showLogs}
          aria-label="Logs"
        >
          <FileText class="w-6 h-6 text-[--color-rai-text-muted]" />
        </button>
      {/if}
      <button
        onclick={() => { showSettings = !showSettings; showLogs = false; }}
        class="p-2 rounded-lg hover:bg-[--color-rai-card] transition-colors"
        class:bg-[--color-rai-card]={showSettings}
        aria-label="Settings"
      >
        <SettingsIcon class="w-6 h-6 text-[--color-rai-text-muted]" />
      </button>
    </div>
  </header>

  {#if showSettings}
    <!-- Settings Panel -->
    <div class="mb-8 p-6 bg-[--color-rai-card] rounded-xl border border-[--color-rai-border]">
      <h2 class="text-lg font-semibold text-[--color-rai-text] mb-4">Settings</h2>
      <Settings />
    </div>
  {:else if showLogs}
    <!-- Logs Panel -->
    <div class="flex-1 mb-8 p-6 bg-[--color-rai-card] rounded-xl border border-[--color-rai-border] flex flex-col min-h-[400px]">
      <LogViewer />
    </div>
  {:else}
    <!-- Main Content -->
    <div class="flex-1 flex flex-col">
      <!-- Status Section -->
      <div class="mb-8 p-6 bg-[--color-rai-card] rounded-xl border border-[--color-rai-border]">
        <div class="flex items-center justify-between mb-4">
          <StatusIndicator status={store.appState.status} />
          {#if store.appState.osu_running}
            <span class="text-xs text-green-500 flex items-center gap-1">
              <span class="w-2 h-2 bg-green-500 rounded-full"></span>
              osu! running
            </span>
          {/if}
        </div>

        {#if error && store.appState.last_error}
          <div class="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg">
            <p class="text-sm text-red-400">{store.appState.last_error}</p>
          </div>
        {/if}

        {#if !store.config.osu_path}
          <div class="mb-4 p-3 bg-yellow-500/10 border border-yellow-500/20 rounded-lg">
            <p class="text-sm text-yellow-400">
              osu! installation not found. Please configure the path in settings.
            </p>
          </div>
        {/if}

        <div class="flex gap-3">
          {#if connected}
            <Button
              variant="danger"
              onclick={() => disconnect()}
              loading={store.isLoading}
            >
              {#snippet children()}
                Disconnect
              {/snippet}
            </Button>
          {:else}
            <Button
              variant="primary"
              onclick={() => connect()}
              disabled={!connectable}
              loading={connecting || store.isLoading}
            >
              {#snippet children()}
                {#if connecting}
                  Connecting...
                {:else}
                  Connect & Launch osu!
                {/if}
              {/snippet}
            </Button>
            <Button
              variant="secondary"
              onclick={() => startProxy()}
              disabled={connecting || store.isLoading}
              loading={connecting || store.isLoading}
            >
              {#snippet children()}
                {#if connecting}
                  Starting...
                {:else}
                  Start Proxy Only
                {/if}
              {/snippet}
            </Button>
          {/if}
        </div>
      </div>

      <!-- Stats Section -->
      {#if connected}
        <div class="grid grid-cols-2 gap-4 mb-8">
          <StatsCard
            label="Requests Proxied"
            value={store.appState.requests_proxied}
            icon="📡"
          />
          <StatsCard
            label="Beatmaps Downloaded"
            value={store.appState.beatmaps_downloaded}
            icon="🎵"
          />
        </div>
      {/if}
    </div>
  {/if}

  <!-- Footer -->
  <footer class="mt-6 text-center">
    <a
      href="https://rai.moe"
      target="_blank"
      rel="noopener noreferrer"
      class="text-xs text-[--color-rai-text-muted] hover:text-[--color-rai-pink] transition-colors"
    >
      rai.moe
    </a>
  </footer>
</main>
