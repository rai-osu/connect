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
  import { checkForUpdates } from "$lib/stores/updater.svelte";
  import StatusIndicator from "$lib/components/StatusIndicator.svelte";
  import StatsCard from "$lib/components/StatsCard.svelte";
  import Button from "$lib/components/Button.svelte";
  import Settings from "$lib/components/Settings.svelte";
  import LogViewer from "$lib/components/LogViewer.svelte";
  import UpdateNotification from "$lib/components/UpdateNotification.svelte";
  import Onboarding from "$lib/components/Onboarding.svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { FileText, Settings as SettingsIcon } from "lucide-svelte";

  let showSettings = $state(false);
  let showLogs = $state(false);
  let showOnboarding = $state(false);

  const connected = $derived(isConnected());
  const connecting = $derived(isConnecting());
  const error = $derived(hasError());
  const connectable = $derived(canConnect());

  onMount(() => {
    // Check if onboarding has been completed
    const onboardingCompleted = localStorage.getItem("raiconnect_onboarding_completed");
    if (!onboardingCompleted) {
      showOnboarding = true;
    }

    (async () => {
      await loadConfig();

      // Only auto-detect if onboarding was already completed
      if (!store.config.osu_path && !showOnboarding) {
        const detected = await detectOsuPath();
        if (detected) {
          await updateConfig("osu_path", detected);
        }
      }

      // Check for updates after app initialization
      checkForUpdates();
    })();

    const interval = setInterval(async () => {
      if (isConnected()) {
        await refreshStatus();
      }
    }, 2000);

    return () => clearInterval(interval);
  });

  function handleOnboardingComplete() {
    showOnboarding = false;
  }
</script>

<main class="min-h-screen bg-background p-6 flex flex-col font-sans">
  <header class="flex items-center justify-between mb-8">
    <div class="flex items-center gap-3">
      <img src="/favicon.png" alt="rai!connect" class="w-10 h-10" />
      <div>
        <h1 class="text-xl font-bold text-foreground">rai!connect</h1>
        <p class="text-xs text-muted-foreground">osu!direct mirror proxy</p>
      </div>
    </div>
    <div class="flex items-center gap-2">
      {#if store.config.debug_logging}
        <Tooltip text="View debug logs and proxy activity" position="bottom">
          {#snippet children()}
            <Button
              variant="ghost"
              size="icon"
              onclick={() => { showLogs = !showLogs; showSettings = false; }}
              class={showLogs ? "bg-accent text-accent-foreground" : ""}
              aria-label="Logs"
            >
              {#snippet children()}
                <FileText class="w-5 h-5" />
              {/snippet}
            </Button>
          {/snippet}
        </Tooltip>
      {/if}
      <Tooltip text="Configure osu! path and proxy settings" position="bottom">
        {#snippet children()}
          <Button
            variant="ghost"
            size="icon"
            onclick={() => { showSettings = !showSettings; showLogs = false; }}
            class={showSettings ? "bg-accent text-accent-foreground" : ""}
            aria-label="Settings"
          >
            {#snippet children()}
              <SettingsIcon class="w-5 h-5" />
            {/snippet}
          </Button>
        {/snippet}
      </Tooltip>
    </div>
  </header>

  <UpdateNotification />

  {#if showSettings}
    <div class="mb-8 p-6 bg-card rounded-xl border border-border">
      <h2 class="text-lg font-semibold text-foreground mb-4">Settings</h2>
      <Settings />
    </div>
  {:else if showLogs}
    <div class="flex-1 mb-8 p-6 bg-card rounded-xl border border-border flex flex-col min-h-[400px]">
      <LogViewer />
    </div>
  {:else}
    <div class="flex-1 flex flex-col">
      <div class="mb-8 p-6 bg-card rounded-xl border border-border">
        <div class="flex items-center justify-between mb-4">
          <StatusIndicator status={store.appState.status} />
          {#if store.appState.osu_running}
            <span class="text-xs text-success flex items-center gap-1 font-medium">
              <span class="w-2 h-2 bg-success rounded-full animate-pulse"></span>
              osu! running
            </span>
          {/if}
        </div>

        {#if error && store.appState.last_error}
          <div class="mb-4 p-3 bg-destructive/10 border border-destructive/20 rounded-lg">
            <p class="text-sm text-destructive">{store.appState.last_error}</p>
          </div>
        {/if}

        {#if !store.config.osu_path}
          <div class="mb-4 p-3 bg-warning/10 border border-warning/20 rounded-lg">
            <p class="text-sm text-warning">
              osu! installation not found. Please configure the path in settings.
            </p>
          </div>
        {/if}

        <div class="flex gap-3">
          {#if connected}
            <Tooltip text="Stop the proxy server and restore hosts file" position="top">
              {#snippet children()}
                <Button
                  variant="destructive"
                  onclick={() => disconnect()}
                  loading={store.isLoading}
                >
                  {#snippet children()}
                    Disconnect
                  {/snippet}
                </Button>
              {/snippet}
            </Tooltip>
          {:else}
            <Tooltip text="Starts the proxy server, adds hosts file entries, and launches osu! with the devserver flag" position="top">
              {#snippet children()}
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
              {/snippet}
            </Tooltip>
            <Tooltip text="Only starts the proxy without launching osu!. Use this if osu! is already running or you prefer manual setup" position="top">
              {#snippet children()}
                <Button
                  variant="outline"
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
              {/snippet}
            </Tooltip>
          {/if}
        </div>
      </div>

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

  <footer class="mt-6 text-center">
    <a
      href="https://rai.moe"
      target="_blank"
      rel="noopener noreferrer"
      class="text-xs text-muted-foreground hover:text-primary transition-colors"
    >
      rai.moe
    </a>
  </footer>
</main>

{#if showOnboarding}
  <Onboarding onComplete={handleOnboardingComplete} />
{/if}
