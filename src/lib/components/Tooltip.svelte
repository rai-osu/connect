<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    text: string;
    position?: "top" | "bottom" | "left" | "right";
    delay?: number;
    children: Snippet;
  }

  let {
    text,
    position = "top",
    delay = 200,
    children,
  }: Props = $props();

  let showTooltip = $state(false);
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  function handleMouseEnter() {
    timeoutId = setTimeout(() => {
      showTooltip = true;
    }, delay);
  }

  function handleMouseLeave() {
    if (timeoutId) {
      clearTimeout(timeoutId);
      timeoutId = null;
    }
    showTooltip = false;
  }

  function handleFocus() {
    showTooltip = true;
  }

  function handleBlur() {
    showTooltip = false;
  }

  const positionClasses = $derived({
    top: "bottom-full left-1/2 -translate-x-1/2 mb-2 w-max",
    bottom: "top-full left-1/2 -translate-x-1/2 mt-2 w-max",
    left: "right-full top-1/2 -translate-y-1/2 mr-2 w-max",
    right: "left-full top-1/2 -translate-y-1/2 ml-2 w-max",
  }[position]);

  const arrowClasses = $derived({
    top: "top-full left-1/2 -translate-x-1/2 border-t-popover border-x-transparent border-b-transparent",
    bottom: "bottom-full left-1/2 -translate-x-1/2 border-b-popover border-x-transparent border-t-transparent",
    left: "left-full top-1/2 -translate-y-1/2 border-l-popover border-y-transparent border-r-transparent",
    right: "right-full top-1/2 -translate-y-1/2 border-r-popover border-y-transparent border-l-transparent",
  }[position]);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="relative inline-flex"
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
  onfocusin={handleFocus}
  onfocusout={handleBlur}
>
  {@render children()}

  {#if showTooltip}
    <div
      class="absolute z-50 {positionClasses} pointer-events-none"
      role="tooltip"
    >
      <div class="bg-popover text-popover-foreground text-xs px-3 py-2 rounded-md shadow-md border border-border max-w-xs whitespace-normal">
        {text}
      </div>
      <div class="absolute {arrowClasses} border-4 w-0 h-0"></div>
    </div>
  {/if}
</div>
