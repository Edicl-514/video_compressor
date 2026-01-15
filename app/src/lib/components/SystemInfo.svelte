<script lang="ts">
    let cpu = 15;
    let ram = 45;
    let gpu = 10;
    let speed = 1.2;
</script>

<div class="system-info">
    <div class="info-item">
        <span class="label">CPU:</span>
        <span class="value">{cpu}%</span>
    </div>
    <div class="info-divider"></div>
    <div class="info-item">
        <span class="label">RAM:</span>
        <span class="value">{ram}%</span>
    </div>
    <div class="info-divider"></div>
    <div class="info-item">
        <span class="label">GPU:</span>
        <span class="value">{gpu}%</span>
    </div>
    <div class="info-divider"></div>
    <div class="info-item">
        <span class="label">Speed:</span>
        <span class="value">{speed}x</span>
    </div>
</div>

<style>
    .system-info {
        display: flex;
        align-items: center;
        gap: 1.5rem;
        background-color: #2a2a2a;
        padding: 0.8rem 1.5rem;
        border-radius: 8px;
        color: #e0e0e0;
        font-size: 0.9rem;
        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    }
    .info-item {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    .info-divider {
        width: 1px;
        height: 16px;
        background-color: #444;
    }
    .label {
        font-weight: 500;
        color: #aaa;
    }
    .value {
        font-family: "Consolas", monospace;
        color: #646cff;
        font-weight: bold;
    }
</style>
