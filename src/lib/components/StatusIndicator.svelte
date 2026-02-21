<script lang="ts">
  import type { ConnectionStatus } from "$lib/types";
  import Tooltip from "./Tooltip.svelte";

  interface Props {
    status: ConnectionStatus;
  }

  let { status }: Props = $props();

  const statusConfig = $derived({
    disconnected: {
      color: "bg-muted-foreground",
      text: "Disconnected",
      pulse: false,
      tooltip: "Proxy is not running. Click Connect to start.",
    },
    connecting: {
      color: "bg-warning",
      text: "Connecting...",
      pulse: true,
      tooltip: "Setting up the proxy server and modifying hosts file.",
    },
    connected: {
      color: "bg-success",
      text: "Connected",
      pulse: false,
      tooltip: "Proxy is active. osu! traffic is being redirected to rai.moe.",
    },
    error: {
      color: "bg-destructive",
      text: "Error",
      pulse: false,
      tooltip: "Something went wrong. Check the error message below.",
    },
  }[status]);
</script>

<Tooltip text={statusConfig.tooltip} position="bottom">
  {#snippet children()}
    <div class="flex items-center gap-2 cursor-help">
      <div class="relative flex items-center justify-center">
        <div
          class="w-2.5 h-2.5 rounded-full {statusConfig.color}"
          class:animate-pulse={statusConfig.pulse}
        ></div>
        {#if statusConfig.pulse || status === 'connected'}
          <div
            class="absolute inset-0 w-2.5 h-2.5 rounded-full {statusConfig.color} animate-ping opacity-75"
          ></div>
        {/if}
      </div>
      <span class="text-sm font-medium text-foreground">{statusConfig.text}</span>
    </div>
  {/snippet}
</Tooltip>
