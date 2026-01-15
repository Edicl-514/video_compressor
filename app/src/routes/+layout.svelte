<script lang="ts">
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { browser } from "$app/environment";

    let { children } = $props();

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

{@render children()}

<style>
    /* Global styles can go here if not in app.css */
</style>
