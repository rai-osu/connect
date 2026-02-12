<script lang="ts">
  import type { ConnectionStatus } from "$lib/types";

  interface Props {
    status: ConnectionStatus;
  }

  let { status }: Props = $props();

  const statusConfig = $derived({
    disconnected: { color: "bg-muted-foreground", text: "Disconnected", pulse: false },
    connecting: { color: "bg-yellow-500", text: "Connecting...", pulse: true },
    connected: { color: "bg-green-500", text: "Connected", pulse: false },
    error: { color: "bg-destructive", text: "Error", pulse: false },
  }[status]);
</script>

<div class="flex items-center gap-2">
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