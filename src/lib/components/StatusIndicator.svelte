<script lang="ts">
  import type { ConnectionStatus } from "$lib/types";

  interface Props {
    status: ConnectionStatus;
  }

  let { status }: Props = $props();

  const statusConfig = $derived({
    disconnected: { color: "bg-gray-500", text: "Disconnected", pulse: false },
    connecting: { color: "bg-yellow-500", text: "Connecting...", pulse: true },
    connected: { color: "bg-green-500", text: "Connected", pulse: false },
    error: { color: "bg-red-500", text: "Error", pulse: false },
  }[status]);
</script>

<div class="flex items-center gap-2">
  <div class="relative">
    <div
      class="w-3 h-3 rounded-full {statusConfig.color}"
      class:animate-pulse={statusConfig.pulse}
    ></div>
    {#if statusConfig.pulse}
      <div
        class="absolute inset-0 w-3 h-3 rounded-full {statusConfig.color} animate-ping opacity-75"
      ></div>
    {/if}
  </div>
  <span class="text-sm text-[--color-rai-text-muted]">{statusConfig.text}</span>
</div>
