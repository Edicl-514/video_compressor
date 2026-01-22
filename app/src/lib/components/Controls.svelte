<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { _ as t } from "svelte-i18n";
    const dispatch = createEventDispatcher();

    export let isProcessing = false;
    export let isPaused = false;
</script>

<div class="controls">
    <div class="main-actions">
        {#if !isProcessing || isPaused}
            <button class="btn btn-primary" on:click={() => dispatch(isPaused ? "resume" : "start")}
                >{isPaused ? $t("common.resume") : $t("common.start")}</button
            >
        {:else}
            <button class="btn btn-secondary" on:click={() => dispatch("pause")}
                >{$t("common.pause")}</button
            >
        {/if}
        <button class="btn btn-danger" on:click={() => dispatch("cancel")}
            >{$t("common.cancel")}</button
        >
    </div>

    <div class="secondary-actions">
        <button
            class="btn btn-icon"
            on:click={() => dispatch("settings")}
            title={$t("common.settings")}
        >
            ⚙️
        </button>
    </div>
</div>

<style>
    .controls {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1rem;
        background-color: #2d2d2d;
        border-radius: 8px;
        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        margin-top: auto; /* Push to bottom if needed */
    }
    .main-actions {
        display: flex;
        gap: 1rem;
    }
    .btn {
        padding: 0.6rem 1.5rem;
        border: none;
        border-radius: 6px;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s;
        font-size: 0.95rem;
    }
    .btn:active {
        transform: scale(0.98);
    }
    .btn-primary {
        background-color: #646cff;
        color: white;
    }
    .btn-primary:hover {
        background-color: #535bf2;
        box-shadow: 0 0 10px rgba(100, 108, 255, 0.4);
    }
    .btn-secondary {
        background-color: #4a4a4a;
        color: white;
    }
    .btn-secondary:hover {
        background-color: #5a5a5a;
    }
    .btn-danger {
        background-color: #cf352e;
        color: white;
    }
    .btn-danger:hover {
        background-color: #e0453d;
    }
    .btn-icon {
        background-color: transparent;
        color: #e0e0e0;
        font-size: 1.4rem;
        padding: 0.4rem;
        border-radius: 50%;
    }
    .btn-icon:hover {
        background-color: #3d3d3d;
    }
</style>
