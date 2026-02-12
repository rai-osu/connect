<script lang="ts">
  interface Props {
    label: string;
    value: number | string;
    icon?: string;
  }

  let { label, value, icon }: Props = $props();
</script>

<div class="bg-card rounded-lg p-4 border border-border hover:border-primary/50 transition-colors">
  <div class="flex items-center gap-3">
    {#if icon}
      <span class="text-2xl grayscale brightness-75">{icon}</span>
    {/if}
    <div>
      <div class="text-2xl font-bold text-foreground font-mono">{value}</div>
      <div class="text-xs text-muted-foreground uppercase tracking-wider font-medium">{label}</div>
    </div>
  </div>
</div>