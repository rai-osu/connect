<script lang="ts">
  import type { Snippet } from "svelte";
  import { Loader2 } from "lucide-svelte";

  interface Props {
    variant?: "primary" | "secondary" | "outline" | "ghost" | "destructive";
    size?: "sm" | "default" | "lg" | "icon";
    disabled?: boolean;
    loading?: boolean;
    onclick?: () => void;
    children: Snippet;
    class?: string;
  }

  let {
    variant = "primary",
    size = "default",
    disabled = false,
    loading = false,
    onclick,
    children,
    class: className = "",
  }: Props = $props();

  const baseClasses =
    "inline-flex items-center justify-center rounded-md font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50";

  const variantClasses = $derived({
    primary:
      "bg-primary text-primary-foreground shadow hover:bg-primary/90",
    secondary:
      "bg-secondary text-secondary-foreground shadow-sm hover:bg-secondary/80",
    outline:
      "border border-input bg-background shadow-sm hover:bg-accent hover:text-accent-foreground",
    ghost:
      "hover:bg-accent hover:text-accent-foreground",
    destructive:
      "bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90",
  }[variant]);

  const sizeClasses = $derived({
    default: "h-9 px-4 py-2",
    sm: "h-8 rounded-md px-3 text-xs",
    lg: "h-10 rounded-md px-8",
    icon: "h-9 w-9",
  }[size]);
</script>

<button
  class="{baseClasses} {variantClasses} {sizeClasses} {className}"
  disabled={disabled || loading}
  onclick={onclick}
>
  {#if loading}
    <Loader2 class="mr-2 h-4 w-4 animate-spin" />
  {/if}
  {@render children()}
</button>