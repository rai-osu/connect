<script lang="ts">
  import { onMount } from "svelte";
  import { store, getLogs, clearLogs } from "$lib/stores/app.svelte";
  import Button from "./Button.svelte";

  let autoRefresh = $state(true);
  let refreshInterval: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    getLogs();

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
        return "text-error";
      case "WARN":
        return "text-warning";
      case "INFO":
        return "text-secondary";
      case "DEBUG":
        return "text-muted-foreground";
      case "TRACE":
        return "text-muted-foreground";
      default:
        return "text-muted-foreground";
    }
  }

  function formatTarget(target: string): string {
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
      <h3 class="text-sm font-medium text-foreground">Debug Logs</h3>
      <span class="text-xs text-muted-foreground">
        {store.logs.length} entries
      </span>
    </div>
    <div class="flex items-center gap-2">
      <label class="flex items-center gap-2 text-xs text-muted-foreground cursor-pointer hover:text-foreground transition-colors">
        <input
          type="checkbox"
          bind:checked={autoRefresh}
          class="w-3 h-3 rounded border-input bg-input text-primary focus:ring-primary focus:ring-offset-background"
        />
        Auto-refresh
      </label>
      <Button variant="outline" size="sm" onclick={() => getLogs()}>
        {#snippet children()}
          Refresh
        {/snippet}
      </Button>
      <Button variant="destructive" size="sm" onclick={() => clearLogs()}>
        {#snippet children()}
          Clear
        {/snippet}
      </Button>
    </div>
  </div>

  <!-- Log entries -->
  <div
    class="flex-1 overflow-auto bg-background border border-border rounded-lg p-3 font-mono text-xs"
  >
    {#if store.logs.length === 0}
      <div class="text-muted-foreground text-center py-8">
        No log entries yet. Logs will appear here when the proxy is active.
      </div>
    {:else}
      <div class="space-y-1">
        {#each store.logs as log}
          <div class="flex gap-2 hover:bg-white/5 px-1 rounded transition-colors">
            <span class="text-muted-foreground shrink-0">{log.timestamp}</span>
            <span class={`shrink-0 w-12 font-bold ${getLevelColor(log.level)}`}>{log.level}</span>
            <span class="text-primary shrink-0">{formatTarget(log.target)}</span>
            <span class="text-foreground break-all">{log.message}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>