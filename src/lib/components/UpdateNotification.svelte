<script lang="ts">
  import {
    updateStore,
    isUpdateAvailable,
    isDownloading,
    downloadAndInstall,
    dismissUpdate,
  } from "$lib/stores/updater.svelte";
  import Button from "./Button.svelte";
  import { Download, X } from "lucide-svelte";

  const available = $derived(isUpdateAvailable());
  const downloading = $derived(isDownloading());
</script>

{#if available}
  <div class="mb-4 p-4 bg-primary/10 border border-primary/20 rounded-xl">
    <div class="flex items-start justify-between gap-3">
      <div class="flex-1">
        <div class="flex items-center gap-2 mb-1">
          <Download class="w-4 h-4 text-primary" />
          <h3 class="text-sm font-semibold text-foreground">
            Update Available
          </h3>
        </div>
        <p class="text-xs text-muted-foreground">
          Version {updateStore.version} is ready to install.
        </p>
        {#if updateStore.releaseNotes}
          <p class="text-xs text-muted-foreground mt-1 line-clamp-2">
            {updateStore.releaseNotes}
          </p>
        {/if}
      </div>
      {#if !downloading}
        <button
          onclick={() => dismissUpdate()}
          class="text-muted-foreground hover:text-foreground transition-colors"
          aria-label="Dismiss"
        >
          <X class="w-4 h-4" />
        </button>
      {/if}
    </div>

    {#if downloading}
      <div class="mt-3">
        <div class="flex items-center justify-between text-xs text-muted-foreground mb-1">
          <span>Downloading...</span>
          <span>{updateStore.progress}%</span>
        </div>
        <div class="w-full bg-muted rounded-full h-1.5">
          <div
            class="bg-primary h-1.5 rounded-full transition-all duration-300"
            style="width: {updateStore.progress}%"
          ></div>
        </div>
      </div>
    {:else}
      <div class="mt-3 flex gap-2">
        <Button
          variant="primary"
          size="sm"
          onclick={() => downloadAndInstall()}
        >
          {#snippet children()}
            Update Now
          {/snippet}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onclick={() => dismissUpdate()}
        >
          {#snippet children()}
            Later
          {/snippet}
        </Button>
      </div>
    {/if}

    {#if updateStore.error}
      <p class="mt-2 text-xs text-destructive">
        Failed to update: {updateStore.error}
      </p>
    {/if}
  </div>
{/if}
