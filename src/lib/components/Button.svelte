<script lang="ts">
  import type { Snippet } from "svelte";

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
    <svg
      class="animate-spin h-5 w-5"
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
    >
      <circle
        class="opacity-25"
        cx="12"
        cy="12"
        r="10"
        stroke="currentColor"
        stroke-width="4"
      ></circle>
      <path
        class="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
      ></path>
    </svg>
  {/if}
  {@render children()}
</button>
