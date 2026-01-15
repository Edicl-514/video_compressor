<script lang="ts">
    import { settingsStore } from "../stores/settings.svelte";
    import {
        CompressionMode,
        type AppSettings,
        type EncoderConfig,
    } from "../types";
    import { invoke } from "@tauri-apps/api/core";
    import ParamsEditorModal from "./ParamsEditorModal.svelte";

    let { close } = $props<{ close: () => void }>();

    let config = $state<AppSettings>(
        JSON.parse(JSON.stringify(settingsStore.value)),
    );
    let currentView = $state<"basic" | "advanced">("basic");
    let editingTarget = $state<{
        type: "video" | "audio" | "filters";
        index: number;
        title: string;
        params: string[];
    } | null>(null);

    function save() {
        settingsStore.value = JSON.parse(JSON.stringify(config));
        close();
    }

    function detectEncoders() {
        // Placeholder for Rust backend call
        console.log("Detecting encoders...");
        // invoke('detect_encoders').then(res => ...)
        alert(
            "Encoder detection will be implemented in the backend integration phase.",
        );
    }

    function toggleResolutionLimit(e: Event) {
        config.maxResolution.enabled = (e.target as HTMLInputElement).checked;
        if (
            config.maxResolution.enabled &&
            config.compressionMode === CompressionMode.VMAF
        ) {
            config.compressionMode = CompressionMode.CRF; // Fallback
        }
    }

    function openParamsEditor(
        type: "video" | "audio" | "filters",
        index: number = 0,
    ) {
        if (type === "filters") {
            editingTarget = {
                type: "filters",
                index: 0,
                title: "Custom Filters",
                params: config.customFilters,
            };
            return;
        }

        const list =
            type === "video"
                ? config.availableVideoEncoders
                : config.availableAudioEncoders;

        editingTarget = {
            type,
            index,
            title: `Edit Parameters: ${list[index].name}`,
            params: list[index].customParams,
        };
    }

    function saveParams(newParams: string[]) {
        if (!editingTarget) return;

        if (editingTarget.type === "filters") {
            config.customFilters = newParams;
        } else if (editingTarget.type === "video") {
            config.availableVideoEncoders[editingTarget.index].customParams =
                newParams;
        } else if (editingTarget.type === "audio") {
            config.availableAudioEncoders[editingTarget.index].customParams =
                newParams;
        }
    }

    function toggleEncoderVisibility(index: number, type: "video" | "audio") {
        if (type === "video") {
            config.availableVideoEncoders[index].visible =
                !config.availableVideoEncoders[index].visible;
        } else {
            config.availableAudioEncoders[index].visible =
                !config.availableAudioEncoders[index].visible;
        }
    }
</script>

<div
    class="backdrop"
    onclick={close}
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === "Escape" && close()}
>
    <div
        class="modal"
        onclick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        tabindex="-1"
        onkeydown={(e) => e.stopPropagation()}
    >
        <header>
            <h2>
                {currentView === "basic" ? "Settings" : "Advanced Settings"}
            </h2>
            <button class="close-btn" onclick={close}>&times;</button>
        </header>

        <div class="content">
            {#if currentView === "basic"}
                <div class="form-group">
                    <label for="compression-mode">Compression Mode</label>
                    <div class="mode-select-row">
                        <select
                            id="compression-mode"
                            bind:value={config.compressionMode}
                            class="compression-select"
                        >
                            <option value={CompressionMode.BITRATE}
                                >Target Bitrate</option
                            >
                            <option value={CompressionMode.CRF}
                                >Target CRF (Quality)</option
                            >
                            <option
                                value={CompressionMode.VMAF}
                                disabled={config.maxResolution.enabled}
                                >Target VMAF {config.maxResolution.enabled
                                    ? "(Disabled by Resolution Limit)"
                                    : ""}</option
                            >
                        </select>
                    </div>
                    <style>
                        .compression-select {
                            width: 436px;
                        }
                    </style>

                    <!-- Dynamic Input for Selected Mode -->
                    <div class="mode-value-input">
                        {#if config.compressionMode === CompressionMode.BITRATE}
                            <label>Bitrate (kbps)</label>
                            <input
                                type="number"
                                bind:value={config.targetBitrate}
                                min="100"
                                step="100"
                            />
                        {:else if config.compressionMode === CompressionMode.CRF}
                            <label>CRF Value (0-51)</label>
                            <div class="slider-row">
                                <input
                                    type="range"
                                    bind:value={config.targetCRF}
                                    min="0"
                                    max="51"
                                    step="1"
                                />
                                <input
                                    type="number"
                                    bind:value={config.targetCRF}
                                    min="0"
                                    max="51"
                                    style="width: 60px;"
                                />
                            </div>
                        {:else if config.compressionMode === CompressionMode.VMAF}
                            <label>Target VMAF (0-100)</label>
                            <input
                                type="number"
                                bind:value={config.targetVMAF}
                                min="0"
                                max="100"
                            />
                        {/if}
                    </div>
                </div>

                <div class="form-group">
                    <label for="concurrent-tasks">Concurrent Tasks</label>
                    <input
                        type="number"
                        id="concurrent-tasks"
                        min="1"
                        max="16"
                        bind:value={config.concurrentTasks}
                    />
                </div>

                <div class="form-group">
                    <span class="group-label">Max Resolution</span>
                    <div class="row">
                        <label class="checkbox-label">
                            <input
                                type="checkbox"
                                checked={config.maxResolution.enabled}
                                onchange={toggleResolutionLimit}
                            />
                            Limit Resolution
                        </label>
                    </div>
                    {#if config.maxResolution.enabled}
                        <div class="row">
                            <input
                                type="number"
                                placeholder="Width"
                                bind:value={config.maxResolution.width}
                            />
                            <span>x</span>
                            <input
                                type="number"
                                placeholder="Height"
                                bind:value={config.maxResolution.height}
                            />
                        </div>
                    {/if}
                </div>

                <div class="form-group">
                    <label for="video-encoder">Video Encoder</label>
                    <select id="video-encoder" bind:value={config.videoEncoder}>
                        {#each config.availableVideoEncoders as enc}
                            {#if enc.visible}
                                <option value={enc.value}>{enc.name}</option>
                            {/if}
                        {/each}
                    </select>
                </div>

                <div class="form-group">
                    <label for="audio-encoder">Audio Encoder</label>
                    <select id="audio-encoder" bind:value={config.audioEncoder}>
                        {#each config.availableAudioEncoders as enc}
                            {#if enc.visible}
                                <option value={enc.value}>{enc.name}</option>
                            {/if}
                        {/each}
                    </select>
                </div>

                <div class="form-group">
                    <label for="target-format">Target Format</label>
                    <select id="target-format" bind:value={config.targetFormat}>
                        <option value="mp4">MP4</option>
                        <option value="mkv">MKV</option>
                        <option value="mov">MOV</option>
                        <option value="webm">WebM</option>
                    </select>
                </div>
            {:else}
                <!-- ADVANCED VIEW -->
                <div class="section">
                    <div class="section-header">
                        <h3>Encoder Management</h3>
                        <button class="secondary-btn" onclick={detectEncoders}
                            >Detect Encoders</button
                        >
                    </div>
                    <div class="encoder-list">
                        <h4>Video Encoders</h4>
                        {#each config.availableVideoEncoders as enc, i}
                            <div class="encoder-row">
                                <label class="checkbox-label">
                                    <input
                                        type="checkbox"
                                        checked={enc.visible}
                                        onchange={() =>
                                            toggleEncoderVisibility(i, "video")}
                                    />
                                    {enc.name} ({enc.value})
                                </label>
                                <button
                                    class="secondary-btn small-btn"
                                    onclick={() => openParamsEditor("video", i)}
                                >
                                    Edit Params
                                </button>
                            </div>
                        {/each}

                        <h4>Audio Encoders</h4>
                        {#each config.availableAudioEncoders as enc, i}
                            <div class="encoder-row">
                                <label class="checkbox-label">
                                    <input
                                        type="checkbox"
                                        checked={enc.visible}
                                        onchange={() =>
                                            toggleEncoderVisibility(i, "audio")}
                                    />
                                    {enc.name} ({enc.value})
                                </label>
                                <button
                                    class="secondary-btn small-btn"
                                    onclick={() => openParamsEditor("audio", i)}
                                >
                                    Edit Params
                                </button>
                            </div>
                        {/each}
                    </div>
                </div>

                {#if editingTarget}
                    <ParamsEditorModal
                        title={editingTarget.title}
                        initialParams={editingTarget.params}
                        close={() => (editingTarget = null)}
                        save={saveParams}
                    />
                {/if}

                <div class="section">
                    <h3>Custom Filters</h3>
                    <div class="warning-box">
                        <label class="checkbox-label">
                            <input
                                type="checkbox"
                                bind:checked={config.enableCustomFiltersEditing}
                            />
                            Enable editing (Caution: Invalid filters may cause failure)
                        </label>
                    </div>
                    <div class="filter-controls">
                        <button
                            class="secondary-btn"
                            disabled={!config.enableCustomFiltersEditing}
                            onclick={() => openParamsEditor("filters")}
                        >
                            Edit Filters
                        </button>
                        {#if config.customFilters.length > 0}
                            <span class="badge"
                                >{config.customFilters.length} active</span
                            >
                        {/if}
                    </div>
                </div>
            {/if}
        </div>

        <footer>
            {#if currentView === "basic"}
                <button
                    class="secondary-btn"
                    onclick={() => (currentView = "advanced")}
                    >Advanced Settings</button
                >
            {:else}
                <button
                    class="secondary-btn"
                    onclick={() => (currentView = "basic")}
                    >Back to Basics</button
                >
            {/if}
            <div class="spacer"></div>
            <button class="secondary-btn" onclick={close}>Cancel</button>
            <button class="primary-btn" onclick={save}>Save Changes</button>
        </footer>
    </div>
</div>

<style>
    .backdrop {
        position: fixed;
        top: 0;
        left: 0;
        width: 100vw;
        height: 100vh;
        background: rgba(0, 0, 0, 0.5);
        backdrop-filter: blur(4px);
        display: flex;
        justify-content: center;
        align-items: center;
        z-index: 1000;
        animation: fadein 0.2s ease-out;
    }

    .modal {
        background: #1e1e1e;
        color: #f0f0f0;
        width: 500px;
        max-width: 90vw;
        max-height: 85vh;
        border-radius: 12px;
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
        display: flex;
        flex-direction: column;
        overflow: hidden;
        border: 1px solid #333;
        animation: scaleup 0.2s ease-out;
    }

    header {
        padding: 16px 20px;
        background: #252525;
        border-bottom: 1px solid #333;
        display: flex;
        justify-content: space-between;
        align-items: center;
    }

    header h2 {
        margin: 0;
        font-size: 1.2rem;
        font-weight: 600;
    }

    .close-btn {
        background: transparent;
        border: none;
        color: #888;
        font-size: 1.5rem;
        cursor: pointer;
        padding: 0;
        line-height: 1;
    }

    .close-btn:hover {
        color: #fff;
    }

    .content {
        padding: 24px;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: 20px;
    }

    .form-group {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    label {
        font-size: 0.9rem;
        color: #aaa;
    }

    input[type="number"],
    select {
        background: #2a2a2a;
        border: 1px solid #444;
        border-radius: 6px;
        padding: 8px 12px;
        color: #fff;
        font-family: inherit;
        font-size: 1rem;
    }

    input:focus,
    select:focus {
        outline: none;
        border-color: #646cff;
    }

    .checkbox-label {
        display: flex;
        align-items: center;
        gap: 8px;
        cursor: pointer;
        color: #eee;
    }

    .row {
        display: flex;
        gap: 10px;
        align-items: center;
    }

    footer {
        padding: 16px 20px;
        background: #252525;
        border-top: 1px solid #333;
        display: flex;
        gap: 10px;
        align-items: center;
    }

    .spacer {
        flex: 1;
    }

    button {
        padding: 8px 16px;
        border-radius: 6px;
        font-weight: 500;
        cursor: pointer;
        font-size: 0.9rem;
        transition: all 0.2s;
    }

    .primary-btn {
        background: #646cff;
        color: white;
        border: none;
    }

    .primary-btn:hover {
        background: #535bf2;
    }

    .secondary-btn {
        background: transparent;
        color: #aaa;
        border: 1px solid #444;
    }

    .secondary-btn:hover {
        border-color: #666;
        color: #fff;
    }

    .section-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 12px;
    }

    .encoder-list {
        display: flex;
        flex-direction: column;
        gap: 8px;
        max-height: 250px;
        overflow-y: auto;
        padding: 8px;
        background: #111;
        border-radius: 6px;
    }

    .encoder-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 10px;
        padding: 4px 0;
    }

    .encoder-row .checkbox-label {
        flex: 1;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .warning-box {
        margin-bottom: 8px;
    }

    @keyframes fadein {
        from {
            opacity: 0;
        }
        to {
            opacity: 1;
        }
    }

    @keyframes scaleup {
        from {
            transform: scale(0.95);
            opacity: 0;
        }
        to {
            transform: scale(1);
            opacity: 1;
        }
    }
    .mode-select-row {
        margin-bottom: 8px;
    }

    .mode-value-input {
        background: #222;
        padding: 12px;
        border-radius: 6px;
        border: 1px solid #333;
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .slider-row {
        display: flex;
        align-items: center;
        gap: 12px;
    }

    .slider-row input[type="range"] {
        flex: 1;
    }

    .small-btn {
        padding: 4px 8px;
        font-size: 0.8rem;
    }

    .filter-controls {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .badge {
        background: #34d399;
        color: #000;
        font-size: 0.75rem;
        padding: 2px 8px;
        border-radius: 100px;
        font-weight: 600;
    }
</style>
