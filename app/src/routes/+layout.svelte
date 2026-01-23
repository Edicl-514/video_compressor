<script lang="ts">
    import "../app.css";
    import "$lib/i18n"; // Import to initialize i18n
    import { waitLocale } from "svelte-i18n";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { browser } from "$app/environment";
    import WelcomeWizard from "$lib/components/WelcomeWizard.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { onMount, onDestroy } from "svelte";

    let { children } = $props();
    let bgUrl = $state<string | null>(null);

    onMount(async () => {
        if (browser) {
            try {
                console.log("Invoking get_background_image...");
                const bytes = await invoke<number[] | null>("get_background_image");
                if (bytes && bytes.length > 0) {
                    console.log(`Received background image data: ${bytes.length} bytes`);
                    const blob = new Blob([new Uint8Array(bytes)]);
                    bgUrl = URL.createObjectURL(blob);
                } else {
                    console.log("No background image data received");
                }
            } catch (err) {
                console.error("Failed to load background image:", err);
            }
        }
    });

    onDestroy(() => {
        if (bgUrl && browser) {
            URL.revokeObjectURL(bgUrl);
        }
    });

    $effect(() => {
        if (browser) {
            // Access the value to properly subscribe to changes
            const s = settingsStore.value;
            localStorage.setItem("app_settings", JSON.stringify(s));
        }
    });

    // Also need to initialize the store if it hasn't loaded (though store logic runs on import)
    // The store's internal load logic ran once on import, which is fine.
    // However, if we are SSR'ing, we might want to avoid inconsistencies.
    // But since this is a Tauri app (SPA mode usually), it's fine.
</script>

{#await waitLocale()}
    <!-- Loading translations... -->
{:then}
    <div class="main-layout" style:background-image={bgUrl ? `linear-gradient(rgba(26, 26, 26, 0.6), rgba(26, 26, 26, 0.6)), url(${bgUrl})` : 'none'}>
        {@render children()}
    </div>

    {#if settingsStore.value.firstRun}
        <WelcomeWizard />
    {/if}
{/await}

<style>
    .main-layout {
        width: 100vw;
        height: 100vh;
        background-size: cover;
        background-position: center;
        background-repeat: no-repeat;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        background-color: #1a1a1a;
    }
</style>
