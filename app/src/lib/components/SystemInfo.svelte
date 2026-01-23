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
        
        /* Glass Style matching Controls */
        background: rgba(45, 45, 45, 0.6);
        background: linear-gradient(to bottom, rgba(35, 35, 35, 0.8), rgba(45, 45, 45, 0.6));
        backdrop-filter: blur(12px);
        -webkit-backdrop-filter: blur(12px);
        border: 1px solid var(--border-color);
        box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);

        padding: 0.8rem 1.4rem;
        border-radius: var(--radius-md);
        color: var(--text-main);
        font-size: 0.8rem;
        width: fit-content;
        transition: all 0.3s ease;
    }
    
    .system-info:hover {
        background: rgba(50, 50, 50, 0.8);
        box-shadow: 0 6px 24px rgba(0, 0, 0, 0.3);
        border-color: rgba(255, 255, 255, 0.15);
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
        background-color: rgba(255,255,255,0.1);
        border-radius: 100px;
        overflow: hidden;
        min-width: 60px;
    }
    .meter-bar {
        height: 100%;
        transition:
            width 0.5s ease-out,
            background 0.5s ease;
    }
    .progress-item .meter-bar {
        position: relative;
        overflow: hidden;
    }
    .progress-item .meter-bar::after {
        content: "";
        position: absolute;
        top: 0;
        left: 0;
        bottom: 0;
        right: 0;
        background: linear-gradient(90deg, transparent, rgba(255,255,255,0.3), transparent);
        transform: translateX(-100%);
        animation: shimmer 1.5s infinite;
    }
    @keyframes shimmer {
        100% {
            transform: translateX(100%);
        }
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
