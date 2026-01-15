<script lang="ts" context="module">
    function getMeterColor(value: number) {
        if (value < 60) return "linear-gradient(90deg, #34d399, #10b981)";
        if (value < 85) return "linear-gradient(90deg, #fbbf24, #f59e0b)";
        return "linear-gradient(90deg, #f87171, #ef4444)";
    }
</script>

<script lang="ts">
    import { listen } from "@tauri-apps/api/event";
    import { onMount } from "svelte";

    let cpu = 0;
    let ram = 0;
    let gpu = 0;
    let speed = 0;

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

    {#if speed > 0}
        <div class="info-divider"></div>
        <div class="info-item">
            <span class="label">SPEED</span>
            <span class="value speed-value">{speed.toFixed(1)}x</span>
        </div>
    {/if}
</div>

<style>
    .system-info {
        display: flex;
        align-items: center;
        gap: 1.5rem;
        background-color: #1a1a1a;
        padding: 0.6rem 1.2rem;
        border-radius: 10px;
        color: #e0e0e0;
        font-size: 0.8rem;
        border: 1px solid #333;
    }
    .info-item {
        display: flex;
        align-items: center;
        gap: 0.8rem;
        min-width: 120px;
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
</style>
