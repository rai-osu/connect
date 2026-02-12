<script lang="ts">
  import { onMount } from "svelte";
  import { store, getLogs, clearLogs } from "$lib/stores/app.svelte";
  import Button from "./Button.svelte";

  let autoRefresh = $state(true);
  let refreshInterval: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    getLogs();

    // Auto-refresh logs every 500ms when enabled
    refreshInterval = setInterval(() => {
      if (autoRefresh) {
        getLogs();
      }
    }, 500);

    return () => {
      if (refreshInterval) {
        clearInterval(refreshInterval);
      }
    };
  });

  function getLevelColor(level: string): string {
    switch (level) {
      case "ERROR":
        return "text-red-400";
      case "WARN":
        return "text-yellow-400";
      case "INFO":
        return "text-blue-400";
      case "DEBUG":
        return "text-gray-400";
      case "TRACE":
        return "text-gray-500";
      default:
        return "text-gray-400";
    }
  }

  function formatTarget(target: string): string {
    // Shorten long module paths
    const parts = target.split("::");
    if (parts.length > 2) {
      return parts.slice(-2).join("::");
    }
    return target;
  }
</script>

<div class="flex flex-col h-full">
  <!-- Header -->
  <div class="flex items-center justify-between mb-3">
    <div class="flex items-center gap-3">
      <h3 class="text-sm font-medium text-[--color-rai-text]">Debug Logs</h3>
      <span class="text-xs text-[--color-rai-text-muted]">
        {store.logs.length} entries
      </span>
    </div>
    <div class="flex items-center gap-2">
      <label class="flex items-center gap-2 text-xs text-[--color-rai-text-muted] cursor-pointer">
        <input
          type="checkbox"
          bind:checked={autoRefresh}
          class="w-4 h-4 rounded bg-[--color-rai-card] border-[--color-rai-border] text-[--color-rai-pink] focus:ring-[--color-rai-pink] focus:ring-offset-0"
        />
        Auto-refresh
      </label>
      <Button variant="secondary" onclick={() => getLogs()}>
        {#snippet children()}
          Refresh
        {/snippet}
      </Button>
      <Button variant="danger" onclick={() => clearLogs()}>
        {#snippet children()}
          Clear
        {/snippet}
      </Button>
    </div>
  </div>

  <!-- Log entries -->
  <div
    class="flex-1 overflow-auto bg-[--color-rai-bg] border border-[--color-rai-border] rounded-lg p-3 font-mono text-xs"
  >
    {#if store.logs.length === 0}
      <div class="text-[--color-rai-text-muted] text-center py-8">
        No log entries yet. Logs will appear here when the proxy is active.
      </div>
    {:else}
      <div class="space-y-1">
        {#each store.logs as log}
          <div class="flex gap-2 hover:bg-[--color-rai-card]/50 px-1 rounded">
            <span class="text-[--color-rai-text-muted] shrink-0">{log.timestamp}</span>
            <span class={`shrink-0 w-12 ${getLevelColor(log.level)}`}>{log.level}</span>
            <span class="text-[--color-rai-pink] shrink-0">{formatTarget(log.target)}</span>
            <span class="text-[--color-rai-text] break-all">{log.message}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
