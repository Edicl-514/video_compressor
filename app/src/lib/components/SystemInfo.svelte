<script lang="ts" module>
    function getMeterColor(value: number) {
        if (value < 60) return "linear-gradient(90deg, #34d399, #10b981)";
        if (value < 85) return "linear-gradient(90deg, #fbbf24, #f59e0b)";
        return "linear-gradient(90deg, #f87171, #ef4444)";
    }
</script>

<script lang="ts">
    import { listen } from "@tauri-apps/api/event";
    import { onMount } from "svelte";
    import { t } from "svelte-i18n";

    let {
        processingSpeed = 0,
        processingBitrate = 0,
        totalProgress = 0,
    } = $props<{
        processingSpeed?: number;
        processingBitrate?: number;
        totalProgress?: number;
    }>();

    let cpu = $state(0);
    let ram = $state(0);
    let gpu = $state(0);

    onMount(() => {
        const unlisten = listen("system-stats", (event: any) => {
            const stats = event.payload;
            cpu = Math.round(stats.cpu_usage);
            ram = Math.round(stats.memory_usage);
            gpu = Math.round(stats.gpu_usage);
        });

        return () => {
            unlisten.then((u) => u());
        };
    });
</script>

<div class="system-info">
    <div class="row">
        <div class="info-item">
            <span class="label">CPU</span>
            <div class="meter-container">
                <div
                    class="meter-bar"
                    style="width: {cpu}%; background: {getMeterColor(cpu)}"
                ></div>
            </div>
            <span class="value">{cpu}%</span>
        </div>

        <div class="info-divider"></div>

        <div class="info-item">
            <span class="label">RAM</span>
            <div class="meter-container">
                <div
                    class="meter-bar"
                    style="width: {ram}%; background: {getMeterColor(ram)}"
                ></div>
            </div>
            <span class="value">{ram}%</span>
        </div>

        <div class="info-divider"></div>

        <div class="info-item">
            <span class="label">GPU</span>
            <div class="meter-container">
                <div
                    class="meter-bar"
                    style="width: {gpu}%; background: {getMeterColor(gpu)}"
                ></div>
            </div>
            <span class="value">{gpu}%</span>
        </div>
    </div>

    {#if processingSpeed > 0 || processingBitrate > 0 || totalProgress > 0}
        <div class="row stats-row">
            {#if processingSpeed > 0}
                <div class="info-item min-fit">
                    <span class="label">{$t("common.speed")}</span>
                    <span class="value speed-value"
                        >{processingSpeed.toFixed(2)}x</span
                    >
                </div>
            {/if}

            {#if processingSpeed > 0 && processingBitrate > 0}
                <div class="info-divider"></div>
            {/if}

            {#if processingBitrate > 0}
                <div class="info-item min-fit">
                    <span class="label">{$t("common.bitrate")}</span>
                    <span class="value bitrate-value"
                        >{processingBitrate.toFixed(1)}k</span
                    >
                </div>
            {/if}

            {#if (processingSpeed > 0 || processingBitrate > 0) && totalProgress > 0}
                <div class="info-divider"></div>
            {/if}

            {#if totalProgress > 0}
                <div class="info-item min-fit progress-item">
                    <span class="label">{$t("common.total")}</span>
                    <div class="meter-container">
                        <div
                            class="meter-bar"
                            style="width: {totalProgress}%; background: linear-gradient(90deg, #3b82f6, #06b6d4);"
                        ></div>
                    </div>
                    <span class="value total-value"
                        >{totalProgress.toFixed(0)}%</span
                    >
                </div>
            {/if}
        </div>
    {/if}
</div>

<style>
    .system-info {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        background-color: #1a1a1a;
        padding: 0.6rem 1.2rem;
        border-radius: 10px;
        color: #e0e0e0;
        font-size: 0.8rem;
        border: 1px solid #333;
        width: fit-content;
    }
    .row {
        display: flex;
        align-items: center;
        gap: 1.5rem;
        width: 100%;
    }
    .stats-row {
        justify-content: flex-start;
        padding-top: 0.2rem;
        border-top: 1px solid #333;
        margin-top: 0.2rem;
        padding-top: 0.5rem;
    }
    .info-item {
        display: flex;
        align-items: center;
        gap: 0.8rem;
        min-width: 120px;
    }
    .info-item.min-fit {
        min-width: auto;
    }
    .meter-container {
        flex: 1;
        height: 6px;
        background-color: #333;
        border-radius: 3px;
        overflow: hidden;
        min-width: 60px;
    }
    .meter-bar {
        height: 100%;
        transition:
            width 0.5s ease-out,
            background 0.5s ease;
    }
    .info-divider {
        width: 1px;
        height: 20px;
        background-color: #444;
    }
    .label {
        font-weight: 600;
        color: #888;
        font-size: 0.7rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }
    .value {
        font-family: "JetBrains Mono", "Consolas", monospace;
        color: #fff;
        font-weight: 600;
        min-width: 35px;
        text-align: right;
    }
    .speed-value {
        color: #60a5fa;
    }
    .bitrate-value {
        color: #a78bfa;
    }
    .total-value {
        color: #22d3ee;
    }
    .progress-item {
        flex: 1;
    }
    .progress-item .meter-container {
        min-width: 80px;
    }
</style>
