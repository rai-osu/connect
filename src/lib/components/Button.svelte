<script lang="ts">
  import type { Snippet } from "svelte";
  import { Loader2 } from "lucide-svelte";

  interface Props {
    variant?: "primary" | "secondary" | "danger";
    disabled?: boolean;
    loading?: boolean;
    onclick?: () => void;
    children: Snippet;
  }

  let {
    variant = "primary",
    disabled = false,
    loading = false,
    onclick,
    children,
  }: Props = $props();

  const baseClasses =
    "px-6 py-3 rounded-lg font-medium transition-all duration-200 flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed";

  const variantClasses = $derived({
    primary:
      "bg-[--color-rai-pink] hover:bg-[--color-rai-pink-dark] text-white shadow-lg shadow-[--color-rai-pink]/20",
    secondary:
      "bg-[--color-rai-card] hover:bg-[--color-rai-border] text-[--color-rai-text] border border-[--color-rai-border]",
    danger:
      "bg-red-600 hover:bg-red-700 text-white",
  }[variant]);
</script>

<button
  class="{baseClasses} {variantClasses}"
  disabled={disabled || loading}
  onclick={onclick}
>
  {#if loading}
    <Loader2 class="animate-spin h-5 w-5" />
  {/if}
  {@render children()}
</button>
