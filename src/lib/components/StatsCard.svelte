<script lang="ts">
  interface Props {
    label: string;
    value: number | string;
    icon?: string;
  }

  let { label, value, icon }: Props = $props();
</script>

<div class="bg-[--color-rai-card] rounded-lg p-4 border border-[--color-rai-border]">
  <div class="flex items-center gap-3">
    {#if icon}
      <span class="text-2xl">{icon}</span>
    {/if}
    <div>
      <div class="text-2xl font-bold text-[--color-rai-text]">{value}</div>
      <div class="text-xs text-[--color-rai-text-muted]">{label}</div>
    </div>
  </div>
</div>
