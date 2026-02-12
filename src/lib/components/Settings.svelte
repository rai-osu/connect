<script lang="ts">
  import { store, updateConfig, detectOsuPath, validateOsuPath } from "$lib/stores/app.svelte";
  import Button from "./Button.svelte";

  let isDetecting = $state(false);
  let pathInput = $state(store.config.osu_path ?? "");

  async function handleDetect() {
    isDetecting = true;
    try {
      const detected = await detectOsuPath();
      if (detected) {
        pathInput = detected;
        await updateConfig("osu_path", detected);
      }
    } finally {
      isDetecting = false;
    }
  }

  async function handlePathChange() {
    if (!pathInput) return;
    const isValid = await validateOsuPath(pathInput);
    if (isValid) {
      await updateConfig("osu_path", pathInput);
    }
  }

  async function handleToggle(key: "start_at_boot" | "minimize_to_tray" | "start_minimized") {
    await updateConfig(key, !store.config[key]);
  }
</script>

<div class="space-y-6">
  <!-- osu! Path -->
  <div class="space-y-2">
    <label for="osu-path" class="block text-sm font-medium text-[--color-rai-text]">
      osu! Installation Path
    </label>
    <div class="flex gap-2">
      <input
        id="osu-path"
        type="text"
        bind:value={pathInput}
        onblur={handlePathChange}
        placeholder="C:\osu!"
        class="flex-1 px-3 py-2 bg-[--color-rai-card] border border-[--color-rai-border] rounded-lg text-[--color-rai-text] placeholder-[--color-rai-text-muted] focus:outline-none focus:border-[--color-rai-pink] transition-colors"
      />
      <Button variant="secondary" onclick={handleDetect} loading={isDetecting}>
        {#snippet children()}
          Detect
        {/snippet}
      </Button>
    </div>
    {#if store.config.osu_path}
      <p class="text-xs text-green-500">✓ Valid osu! installation found</p>
    {:else if pathInput}
      <p class="text-xs text-red-500">✗ osu! not found at this path</p>
    {/if}
  </div>

  <!-- Toggle Options -->
  <div class="space-y-3">
    <label class="flex items-center gap-3 cursor-pointer group">
      <input
        type="checkbox"
        checked={store.config.start_at_boot}
        onchange={() => handleToggle("start_at_boot")}
        class="w-5 h-5 rounded bg-[--color-rai-card] border-[--color-rai-border] text-[--color-rai-pink] focus:ring-[--color-rai-pink] focus:ring-offset-0"
      />
      <span class="text-sm text-[--color-rai-text] group-hover:text-[--color-rai-pink] transition-colors">
        Start at system boot
      </span>
    </label>

    <label class="flex items-center gap-3 cursor-pointer group">
      <input
        type="checkbox"
        checked={store.config.minimize_to_tray}
        onchange={() => handleToggle("minimize_to_tray")}
        class="w-5 h-5 rounded bg-[--color-rai-card] border-[--color-rai-border] text-[--color-rai-pink] focus:ring-[--color-rai-pink] focus:ring-offset-0"
      />
      <span class="text-sm text-[--color-rai-text] group-hover:text-[--color-rai-pink] transition-colors">
        Minimize to tray on close
      </span>
    </label>

    <label class="flex items-center gap-3 cursor-pointer group">
      <input
        type="checkbox"
        checked={store.config.start_minimized}
        onchange={() => handleToggle("start_minimized")}
        class="w-5 h-5 rounded bg-[--color-rai-card] border-[--color-rai-border] text-[--color-rai-pink] focus:ring-[--color-rai-pink] focus:ring-offset-0"
      />
      <span class="text-sm text-[--color-rai-text] group-hover:text-[--color-rai-pink] transition-colors">
        Start minimized
      </span>
    </label>
  </div>
</div>
