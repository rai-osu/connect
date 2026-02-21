<script lang="ts">
  import { Play, Zap, FolderSearch, ExternalLink } from "lucide-svelte";
  import Button from "./Button.svelte";
  import { detectOsuPath, updateConfig, store, createDesktopShortcut, checkShortcutExists } from "$lib/stores/app.svelte";

  interface Props {
    onComplete: () => void;
  }

  let { onComplete }: Props = $props();

  let currentStep = $state(0);
  let dontShowAgain = $state(true);
  let isDetecting = $state(false);
  let detectedPath = $state<string | null>(null);
  let isCreatingShortcut = $state(false);
  let shortcutCreated = $state(false);
  let shortcutPath = $state<string | null>(null);

  const totalSteps = 4;

  async function handleDetect() {
    isDetecting = true;
    try {
      const detected = await detectOsuPath();
      if (detected) {
        detectedPath = detected;
        await updateConfig("osu_path", detected);
      }
    } finally {
      isDetecting = false;
    }
  }

  async function handleCreateShortcut() {
    isCreatingShortcut = true;
    try {
      const path = await createDesktopShortcut();
      if (path) {
        shortcutCreated = true;
        shortcutPath = path;
      }
    } finally {
      isCreatingShortcut = false;
    }
  }

  // Check if shortcut already exists when reaching step 4
  async function checkExistingShortcut() {
    const exists = await checkShortcutExists();
    if (exists) {
      shortcutCreated = true;
    }
  }

  function handleNext() {
    if (currentStep < totalSteps - 1) {
      currentStep++;
      // Check if shortcut exists when reaching step 4 (index 3)
      if (currentStep === 3) {
        checkExistingShortcut();
      }
    } else {
      handleComplete();
    }
  }

  function handleBack() {
    if (currentStep > 0) {
      currentStep--;
    }
  }

  function handleComplete() {
    if (dontShowAgain) {
      localStorage.setItem("raiconnect_onboarding_completed", "true");
    }
    onComplete();
  }

  function handleSkip() {
    handleComplete();
  }
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-background/90 backdrop-blur-sm p-4">
  <div class="bg-card border border-border rounded-xl p-8 max-w-lg w-full shadow-2xl">
    <!-- Step indicators -->
    <div class="flex justify-center gap-2 mb-8">
      {#each Array(totalSteps) as _, i}
        <div
          class="w-2 h-2 rounded-full transition-colors {i === currentStep
            ? 'bg-primary'
            : i < currentStep
              ? 'bg-primary/50'
              : 'bg-muted'}"
        ></div>
      {/each}
    </div>

    <!-- Step content -->
    <div class="min-h-[280px] flex flex-col">
      {#if currentStep === 0}
        <!-- Step 1: Welcome -->
        <div class="text-center flex-1 flex flex-col justify-center">
          <div class="mb-6">
            <img src="/favicon.png" alt="rai!connect" class="w-16 h-16 mx-auto mb-4" />
          </div>
          <h2 class="text-2xl font-bold text-foreground mb-3">Welcome to rai!connect</h2>
          <p class="text-muted-foreground leading-relaxed">
            rai!connect is a desktop client for <span class="text-secondary">rai.moe</span>, a fast and reliable osu! beatmap mirror.
          </p>
          <p class="text-muted-foreground leading-relaxed mt-3">
            Download beatmaps directly in-game using osu!direct, even without a supporter tag.
          </p>
        </div>

      {:else if currentStep === 1}
        <!-- Step 2: How it works -->
        <div class="flex-1">
          <h2 class="text-xl font-bold text-foreground mb-6 text-center">How it works</h2>

          <div class="space-y-4">
            <div class="p-4 bg-muted/50 rounded-lg border border-border">
              <div class="flex items-start gap-3">
                <div class="p-2 bg-primary/20 rounded-lg">
                  <Play class="w-5 h-5 text-primary" />
                </div>
                <div>
                  <h3 class="font-semibold text-foreground mb-1">Connect & Launch osu!</h3>
                  <p class="text-sm text-muted-foreground">
                    One-click solution. Starts the proxy and launches osu! automatically.
                  </p>
                </div>
              </div>
            </div>

            <div class="p-4 bg-muted/50 rounded-lg border border-border">
              <div class="flex items-start gap-3">
                <div class="p-2 bg-secondary/20 rounded-lg">
                  <Zap class="w-5 h-5 text-secondary" />
                </div>
                <div>
                  <h3 class="font-semibold text-foreground mb-1">Start Proxy Only</h3>
                  <p class="text-sm text-muted-foreground">
                    For manual setup or if osu! is already running. You'll need to restart osu! to apply changes.
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>

      {:else if currentStep === 2}
        <!-- Step 3: Get Started -->
        <div class="flex-1">
          <h2 class="text-xl font-bold text-foreground mb-2 text-center">Get Started</h2>
          <p class="text-muted-foreground text-center mb-6">
            Let's find your osu! installation
          </p>

          <div class="p-4 bg-muted/50 rounded-lg border border-border mb-6">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div class="p-2 bg-primary/20 rounded-lg">
                  <FolderSearch class="w-5 h-5 text-primary" />
                </div>
                <div>
                  <h3 class="font-semibold text-foreground">osu! Path</h3>
                  {#if detectedPath || store.config.osu_path}
                    <p class="text-xs text-success">Found: {detectedPath || store.config.osu_path}</p>
                  {:else}
                    <p class="text-xs text-muted-foreground">Not detected yet</p>
                  {/if}
                </div>
              </div>
              <Button variant="outline" onclick={handleDetect} loading={isDetecting}>
                {#snippet children()}
                  Detect
                {/snippet}
              </Button>
            </div>
          </div>

          {#if !detectedPath && !store.config.osu_path}
            <p class="text-xs text-muted-foreground text-center">
              Don't worry, you can set the path manually in Settings later.
            </p>
          {/if}
        </div>

      {:else if currentStep === 3}
        <!-- Step 4: Desktop Shortcut -->
        <div class="flex-1">
          <h2 class="text-xl font-bold text-foreground mb-2 text-center">Quick Launch</h2>
          <p class="text-muted-foreground text-center mb-6">
            Create a desktop shortcut to launch osu! with rai in one click
          </p>

          <div class="p-4 bg-muted/50 rounded-lg border border-border mb-6">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div class="p-2 bg-secondary/20 rounded-lg">
                  <ExternalLink class="w-5 h-5 text-secondary" />
                </div>
                <div>
                  <h3 class="font-semibold text-foreground">Desktop Shortcut</h3>
                  {#if shortcutCreated}
                    <p class="text-xs text-success">Shortcut created!</p>
                  {:else}
                    <p class="text-xs text-muted-foreground">Launch osu! with rai in one click</p>
                  {/if}
                </div>
              </div>
              {#if !shortcutCreated}
                <Button variant="outline" onclick={handleCreateShortcut} loading={isCreatingShortcut}>
                  {#snippet children()}
                    Create
                  {/snippet}
                </Button>
              {:else}
                <span class="text-success text-sm">✓ Created</span>
              {/if}
            </div>
          </div>

          <p class="text-xs text-muted-foreground text-center">
            {#if shortcutCreated}
              Double-click the shortcut on your desktop to start playing!
            {:else}
              This step is optional. You can also create the shortcut later in Settings.
            {/if}
          </p>
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="mt-6 pt-6 border-t border-border">
      <!-- Don't show again checkbox -->
      <label class="flex items-center gap-2 mb-4 cursor-pointer group">
        <input
          type="checkbox"
          bind:checked={dontShowAgain}
          class="w-4 h-4 rounded border-input bg-input text-primary accent-primary focus:ring-primary focus:ring-offset-background"
        />
        <span class="text-sm text-muted-foreground group-hover:text-foreground transition-colors">
          Don't show this again
        </span>
      </label>

      <!-- Navigation buttons -->
      <div class="flex justify-between items-center">
        <div>
          {#if currentStep > 0}
            <Button variant="ghost" onclick={handleBack}>
              {#snippet children()}
                Back
              {/snippet}
            </Button>
          {:else}
            <Button variant="ghost" onclick={handleSkip}>
              {#snippet children()}
                Skip
              {/snippet}
            </Button>
          {/if}
        </div>
        <Button variant="primary" onclick={handleNext}>
          {#snippet children()}
            {#if currentStep === totalSteps - 1}
              Done
            {:else}
              Next
            {/if}
          {/snippet}
        </Button>
      </div>
    </div>
  </div>
</div>
