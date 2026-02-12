<script lang="ts">
  import { store, updateConfig, detectOsuPath, validateOsuPath, isConnected } from "$lib/stores/app.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { Info } from "lucide-svelte";
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

  async function handleToggle(key: "start_at_boot" | "minimize_to_tray" | "start_minimized" | "debug_logging") {
    await updateConfig(key, !store.config[key]);
  }

  let showSupporterConfirm = $state(false);

  async function confirmSupporter() {
    const newProxy = { ...store.config.proxy, inject_supporter: true };
    await updateConfig("proxy", newProxy);
    showSupporterConfirm = false;
  }

  async function cancelSupporter() {
    await openUrl("https://osu.ppy.sh/home/support");
    showSupporterConfirm = false;
  }
</script>

{#if showSupporterConfirm}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
    <div class="bg-[--color-rai-card] border border-[--color-rai-border] rounded-xl p-6 max-w-md shadow-2xl">
      <h3 class="text-xl font-bold text-[--color-rai-text] mb-4">Wait a moment!</h3>
      <p class="text-[--color-rai-text] mb-4 leading-relaxed">
        While we provide this feature for convenience, <strong>osu! relies on supporter tags to survive</strong> and pay for servers.
      </p>
      <p class="text-[--color-rai-text] mb-6 leading-relaxed">
        If you have the means, please consider buying a supporter tag to help keep the game alive.
      </p>
      <div class="flex gap-3 justify-end">
        <Button variant="secondary" onclick={cancelSupporter}>
          I'll Support osu!
        </Button>
        <Button variant="danger" onclick={confirmSupporter}>
          Enable Anyway
        </Button>
      </div>
    </div>
  </div>
{/if}

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

    <label class="flex items-center gap-3 cursor-pointer group">
      <input
        type="checkbox"
        checked={store.config.debug_logging}
        onchange={() => handleToggle("debug_logging")}
        class="w-5 h-5 rounded bg-[--color-rai-card] border-[--color-rai-border] text-[--color-rai-pink] focus:ring-[--color-rai-pink] focus:ring-offset-0"
      />
      <div class="flex flex-col">
        <span class="text-sm text-[--color-rai-text] group-hover:text-[--color-rai-pink] transition-colors">
          Show debug logs
        </span>
        <span class="text-xs text-[--color-rai-text-muted]">
          View internal logs for troubleshooting
        </span>
      </div>
    </label>

    <!-- Inject Supporter -->
    <div class="pt-4 border-t border-[--color-rai-border]">
      <label class="flex items-center gap-3 cursor-pointer group">
        <input
          type="checkbox"
          checked={store.config.proxy.inject_supporter}
          onchange={(e) => {
            const checked = e.currentTarget.checked;
            if (checked) {
              e.currentTarget.checked = false; // Revert visual immediately
              showSupporterConfirm = true;
            } else {
              const newProxy = { ...store.config.proxy, inject_supporter: false };
              updateConfig("proxy", newProxy);
            }
          }}
          class="w-5 h-5 rounded bg-[--color-rai-card] border-[--color-rai-border] text-[--color-rai-pink] focus:ring-[--color-rai-pink] focus:ring-offset-0"
        />
        <div class="flex flex-col">
          <span class="text-sm text-[--color-rai-text] group-hover:text-[--color-rai-pink] transition-colors">
            Inject Supporter Tag
          </span>
          <span class="text-xs text-[--color-rai-text-muted]">
            Unlocks osu!direct in-game.
          </span>
        </div>
      </label>
    </div>

    {#if isConnected()}
      <div class="p-3 bg-yellow-500/10 border border-yellow-500/20 rounded-lg flex items-center gap-2">
        <Info class="w-4 h-4 text-yellow-500" />
        <p class="text-xs text-yellow-200">
          Reconnect for changes to take effect.
        </p>
      </div>
    {/if}
  </div>
</div>
