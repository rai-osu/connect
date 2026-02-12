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
    connect,
    disconnect,
  } from "$lib/stores/app.svelte";
  import StatusIndicator from "$lib/components/StatusIndicator.svelte";
  import StatsCard from "$lib/components/StatsCard.svelte";
  import Button from "$lib/components/Button.svelte";
  import Settings from "$lib/components/Settings.svelte";

  let showSettings = $state(false);

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

  function handleConnect() {
    if (isConnected()) {
      disconnect();
    } else {
      connect();
    }
  }
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
    <button
      onclick={() => (showSettings = !showSettings)}
      class="p-2 rounded-lg hover:bg-[--color-rai-card] transition-colors"
      aria-label="Settings"
    >
      <svg
        class="w-6 h-6 text-[--color-rai-text-muted]"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
        />
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
        />
      </svg>
    </button>
  </header>

  {#if showSettings}
    <!-- Settings Panel -->
    <div class="mb-8 p-6 bg-[--color-rai-card] rounded-xl border border-[--color-rai-border]">
      <h2 class="text-lg font-semibold text-[--color-rai-text] mb-4">Settings</h2>
      <Settings />
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

        <Button
          variant={connected ? "danger" : "primary"}
          onclick={handleConnect}
          disabled={!connectable && !connected}
          loading={connecting || store.isLoading}
        >
          {#snippet children()}
            {#if connected}
              Disconnect
            {:else if connecting}
              Connecting...
            {:else}
              Connect & Launch osu!
            {/if}
          {/snippet}
        </Button>
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

      <!-- Info Section -->
      <div class="mt-auto">
        <div class="p-4 bg-[--color-rai-card]/50 rounded-lg border border-[--color-rai-border]/50">
          <h3 class="text-sm font-medium text-[--color-rai-text] mb-2">How it works</h3>
          <ul class="text-xs text-[--color-rai-text-muted] space-y-1">
            <li>• Redirects osu!direct requests to rai.moe mirror</li>
            <li>• All other traffic goes to official osu! servers</li>
            <li>• Enables osu!direct features for all users</li>
            <li>• Your gameplay and scores are unaffected</li>
          </ul>
        </div>
      </div>
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
