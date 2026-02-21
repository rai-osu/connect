<script lang="ts">
  interface Props {
    checked?: boolean;
    size?: "sm" | "default";
    onchange?: () => void;
    onclick?: (e: MouseEvent) => void;
    class?: string;
  }

  let {
    checked = $bindable(false),
    size = "default",
    onchange,
    onclick,
    class: className = "",
  }: Props = $props();

  const sizeClasses = $derived(size === "sm" ? "w-3 h-3" : "w-4 h-4");
</script>

<input
  type="checkbox"
  {checked}
  {onchange}
  {onclick}
  class="{sizeClasses} rounded border-input bg-input text-primary accent-primary focus:ring-primary focus:ring-offset-background {className}"
/>
