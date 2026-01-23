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
        padding: 1rem 1.5rem;
        background: rgba(30, 30, 30, 0.45);
        backdrop-filter: blur(20px);
        -webkit-backdrop-filter: blur(20px);
        border-top: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 16px;
        box-shadow: 0 -8px 32px rgba(0, 0, 0, 0.4);
        margin-top: auto; 
        z-index: 100;
        position: relative;
    }
    .main-actions {
        display: flex;
        gap: 1rem;
    }
    .btn {
        padding: 0.7rem 1.8rem;
        border: none;
        border-radius: var(--radius-md);
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
        font-size: 0.95rem;
        letter-spacing: 0.02em;
        position: relative;
        overflow: hidden;
    }
    .btn:active {
        transform: scale(0.96);
    }
    /* Add a shine effect */
    .btn::after {
        content: '';
        position: absolute;
        top: 0;
        left: -100%;
        width: 100%;
        height: 100%;
        background: linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent);
        transition: 0.5s;
    }
    .btn:hover::after {
        left: 100%;
    }

    .btn-primary {
        background-color: var(--primary-color);
        background: linear-gradient(135deg, var(--primary-color), var(--primary-active));
        color: white;
        box-shadow: 0 4px 12px rgba(100, 108, 255, 0.3);
    }
    .btn-primary:hover {
        background-color: var(--primary-hover);
        box-shadow: 0 6px 16px rgba(100, 108, 255, 0.5);
        transform: translateY(-1px);
    }
    .btn-secondary {
        background-color: var(--surface-color);
        color: var(--text-main);
        border: 1px solid var(--border-color);
    }
    .btn-secondary:hover {
        background-color: var(--surface-hover);
        border-color: var(--border-hover);
        transform: translateY(-1px);
    }
    .btn-danger {
        background-color: #cf352e;
        background: linear-gradient(135deg, #ef4444, #dc2626);
        color: white;
        box-shadow: 0 4px 12px rgba(239, 68, 68, 0.3);
    }
    .btn-danger:hover {
        background-color: #e0453d;
        box-shadow: 0 6px 16px rgba(239, 68, 68, 0.5);
        transform: translateY(-1px);
    }
    .btn-icon {
        background-color: transparent;
        color: var(--text-muted);
        font-size: 1.4rem;
        padding: 0.5rem;
        border-radius: 50%;
        transition: all 0.3s ease;
    }
    .btn-icon:hover {
        background-color: rgba(255, 255, 255, 0.1);
        color: var(--text-main);
        transform: rotate(30deg);
    }
</style>
