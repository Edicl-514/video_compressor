<script lang="ts">
    import { settingsStore } from "../stores/settings.svelte";
    import {
        CompressionMode,
        type AppSettings,
        type EncoderConfig,
        DEFAULT_SETTINGS,
    } from "../types";
    import { invoke } from "@tauri-apps/api/core";
    import ParamsEditorModal from "./ParamsEditorModal.svelte";

    let { close } = $props<{ close: () => void }>();

    let config = $state<AppSettings>(
        JSON.parse(JSON.stringify(settingsStore.value)),
    );

    // Safety initialization for existing encoders
    config.availableVideoEncoders.forEach((e) => {
        if (!e.customParams) e.customParams = [];
        if (e.isSupported === undefined) e.isSupported = true;
    });
    config.availableAudioEncoders.forEach((e) => {
        if (!e.customParams) e.customParams = [];
        if (e.isSupported === undefined) e.isSupported = true;
    });
    let currentView = $state<"basic" | "advanced">("basic");
    let editingTarget = $state<{
        type: "video" | "audio" | "filters" | "vmafParams";
        index: number;
        title: string;
        params: string[];
    } | null>(null);
    let isDetecting = $state<boolean>(false);
    let notification = $state<{
        message: string;
        type: "success" | "error";
    } | null>(null);

    function showNotification(
        message: string,
        type: "success" | "error" = "success",
    ) {
        notification = { message, type };
        setTimeout(() => {
            notification = null;
        }, 5000);
    }

    function save() {
        settingsStore.value = JSON.parse(JSON.stringify(config));
        close();
    }

    async function detectEncoders() {
        console.log("Detecting encoders...");
        isDetecting = true;
        try {
            const report: any = await invoke("detect_encoders");
            console.log("Detected:", report);

            // Reset visibility and supported status for all encoders first
            config.availableVideoEncoders.forEach((e) => {
                e.visible = false;
                e.isSupported = false;
            });
            config.availableAudioEncoders.forEach((e) => {
                e.visible = false;
                e.isSupported = false;
            });

            let addedVideo = 0;
            let addedAudio = 0;

            // Update Video Encoders
            for (const detected of report.video) {
                const existingIndex = config.availableVideoEncoders.findIndex(
                    (e) => e.value === detected.value,
                );
                if (existingIndex >= 0) {
                    config.availableVideoEncoders[existingIndex].visible = true;
                    config.availableVideoEncoders[existingIndex].isSupported =
                        true;
                    // Force update name to ensure consistency (e.g. migrating from old names)
                    config.availableVideoEncoders[existingIndex].name =
                        detected.name;
                } else {
                    config.availableVideoEncoders.push({
                        name: detected.name,
                        value: detected.value,
                        visible: true,
                        isSupported: true,
                        customParams: [],
                    });
                    addedVideo++;
                }
            }

            // Update Audio Encoders
            for (const detected of report.audio) {
                const existingIndex = config.availableAudioEncoders.findIndex(
                    (e) => e.value === detected.value,
                );
                if (existingIndex >= 0) {
                    config.availableAudioEncoders[existingIndex].visible = true;
                    config.availableAudioEncoders[existingIndex].isSupported =
                        true;
                    config.availableAudioEncoders[existingIndex].name =
                        detected.name;
                } else {
                    config.availableAudioEncoders.push({
                        name: detected.name,
                        value: detected.value,
                        visible: true,
                        isSupported: true,
                        customParams: [],
                    });
                    addedAudio++;
                }
            }

            // Ensure selected encoder is valid
            const currentVideo = config.availableVideoEncoders.find(
                (e) => e.value === config.videoEncoder,
            );
            if (!currentVideo || !currentVideo.visible) {
                const firstVisible = config.availableVideoEncoders.find(
                    (e) => e.visible,
                );
                if (firstVisible) config.videoEncoder = firstVisible.value;
            }

            const currentAudio = config.availableAudioEncoders.find(
                (e) => e.value === config.audioEncoder,
            );
            if (!currentAudio || !currentAudio.visible) {
                const firstVisible = config.availableAudioEncoders.find(
                    (e) => e.visible,
                );
                if (firstVisible) config.audioEncoder = firstVisible.value;
            }

            showNotification(
                `Detection complete. Enabled ${report.video.length} video encoders and ${report.audio.length} audio encoders.`,
            );
        } catch (e) {
            console.error(e);
            showNotification("Failed to detect encoders: " + e, "error");
        } finally {
            isDetecting = false;
        }
    }

    function toggleResolutionLimit(e: Event) {
        config.maxResolution.enabled = (e.target as HTMLInputElement).checked;
        if (
            config.maxResolution.enabled &&
            config.compressionMode === CompressionMode.VMAF
        ) {
            config.compressionMode = CompressionMode.CRF; // Fallback
        }
        if (config.maxResolution.enabled) {
            config.enableVmaf = false;
        }
    }

    function openParamsEditor(
        type: "video" | "audio" | "filters" | "vmafParams",
        index: number = 0,
    ) {
        if (type === "filters") {
            editingTarget = {
                type: "filters",
                index: 0,
                title: "Custom Params",
                params: config.customFilters,
            };
            return;
        }

        if (type === "vmafParams") {
            editingTarget = {
                type: "vmafParams",
                index: 0,
                title: "Custom VMAF Params",
                params: config.customVmafParams,
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
        } else if (editingTarget.type === "vmafParams") {
            config.customVmafParams = newParams;
        } else if (editingTarget.type === "video") {
            config.availableVideoEncoders[editingTarget.index].customParams =
                newParams;
        } else if (editingTarget.type === "audio") {
            config.availableAudioEncoders[editingTarget.index].customParams =
                newParams;
        }
    }

    // Encoder filter whitelists
    const VIDEO_ENCODER_WHITELIST = [
        "h264",
        "libx264",
        "hevc",
        "h265",
        "libx265",
        "av1",
        "vp8",
        "libvpx",
        "vp9",
        "mpeg2",
        "mpeg4",
        "wmv",
        "mjpeg",
        "prores",
        "dnxhd",
        "rawvideo",
        "png",
        "bmp",
        "gif",
    ];

    const AUDIO_ENCODER_WHITELIST = [
        "aac",
        "mp3",
        "ac3",
        "flac",
        "opus",
        "vorbis",
        "alac",
        "wavpack",
    ];

    // Hardware encoder keywords (same as backend)
    const HW_KEYWORDS = [
        "nvenc",
        "amf",
        "qsv",
        "cuda",
        "vaapi",
        "vdpau",
        "d3d12va",
    ];

    function shouldShowVideoEncoder(encoderValue: string): boolean {
        // If showing only HW encoders, check if it's a hardware encoder
        if (config.showOnlyHwEncoders) {
            return HW_KEYWORDS.some((k) =>
                encoderValue.toLowerCase().includes(k),
            );
        }

        // If showing all encoders, always show
        if (config.showAllEncoders) {
            return true;
        }

        // Otherwise, filter by whitelist
        return VIDEO_ENCODER_WHITELIST.some((w) =>
            encoderValue.toLowerCase().includes(w.toLowerCase()),
        );
    }

    function shouldShowAudioEncoder(encoderValue: string): boolean {
        // Note: showOnlyHwEncoders does NOT affect audio encoders

        // If showing all encoders, always show
        if (config.showAllEncoders) {
            return true;
        }

        // Otherwise, filter by whitelist
        return AUDIO_ENCODER_WHITELIST.some((w) =>
            encoderValue.toLowerCase().includes(w.toLowerCase()),
        );
    }

    function resetToDefaults() {
        config = JSON.parse(JSON.stringify(DEFAULT_SETTINGS));
        showNotification("Settings reset to defaults");
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
                            <label for="target-bitrate">Bitrate (kbps)</label>
                            <input
                                id="target-bitrate"
                                type="number"
                                bind:value={config.targetBitrate}
                                min="100"
                                step="100"
                            />
                        {:else if config.compressionMode === CompressionMode.CRF}
                            <label for="target-crf">CRF Value (0-51)</label>
                            <div class="slider-row">
                                <input
                                    type="range"
                                    bind:value={config.targetCRF}
                                    min="0"
                                    max="51"
                                    step="1"
                                    aria-label="CRF Value Slider"
                                />
                                <input
                                    id="target-crf"
                                    type="number"
                                    bind:value={config.targetCRF}
                                    min="0"
                                    max="51"
                                    style="width: 60px;"
                                />
                            </div>
                        {:else if config.compressionMode === CompressionMode.VMAF}
                            <label for="target-vmaf">Target VMAF (0-100)</label>
                            <input
                                id="target-vmaf"
                                type="number"
                                bind:value={config.targetVMAF}
                                min="0"
                                max="100"
                            />
                        {/if}
                    </div>

                    <div class="mode-extra-settings">
                        {#if config.compressionMode === CompressionMode.BITRATE}
                            <div class="extra-setting">
                                <label for="bypass-threshold">
                                    Bypass Threshold (kbps)
                                    <span
                                        class="tooltip"
                                        title="Skip processing if input video bitrate is lower than this value"
                                    >
                                        <svg
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2.5"
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                        >
                                            <circle cx="12" cy="12" r="10"
                                            ></circle>
                                            <path d="M12 16v-4"></path>
                                            <path d="M12 8h.01"></path>
                                        </svg>
                                    </span>
                                </label>
                                <input
                                    type="number"
                                    id="bypass-threshold"
                                    bind:value={config.minBitrateThreshold}
                                    min="0"
                                    step="100"
                                    placeholder="0 (Disabled)"
                                />
                            </div>
                            <div class="extra-setting">
                                <label class="checkbox-label">
                                    <input
                                        type="checkbox"
                                        bind:checked={config.twoPass}
                                    />
                                    Enable 2-Pass Encoding
                                    <span
                                        class="tooltip"
                                        title="Runs encoding twice for better quality distribution at the target bitrate. Slower but higher quality."
                                    >
                                        <svg
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2.5"
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                        >
                                            <circle cx="12" cy="12" r="10"
                                            ></circle>
                                            <path d="M12 16v-4"></path>
                                            <path d="M12 8h.01"></path>
                                        </svg>
                                    </span>
                                </label>
                            </div>
                        {:else if config.compressionMode === CompressionMode.CRF}
                            <div class="extra-setting">
                                <label class="checkbox-label">
                                    <input
                                        type="checkbox"
                                        bind:checked={config.crfAutoSkip}
                                    />
                                    Auto-skip if output larger than input
                                    <span
                                        class="tooltip"
                                        title="Stops processing early if detected output bitrate is significantly higher than original"
                                    >
                                        <svg
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            stroke-width="2.5"
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                        >
                                            <circle cx="12" cy="12" r="10"
                                            ></circle>
                                            <path d="M12 16v-4"></path>
                                            <path d="M12 8h.01"></path>
                                        </svg>
                                    </span>
                                </label>
                            </div>
                            {#if config.crfAutoSkip}
                                <div
                                    class="extra-setting"
                                    style="margin-top: 8px;"
                                >
                                    <label for="crf-threshold">
                                        Skip Threshold (%)
                                        <span
                                            class="tooltip"
                                            title="Skip if output bitrate > input bitrate * threshold / 100"
                                        >
                                            <svg
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="2.5"
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                            >
                                                <circle cx="12" cy="12" r="10"
                                                ></circle>
                                                <path d="M12 16v-4"></path>
                                                <path d="M12 8h.01"></path>
                                            </svg>
                                        </span>
                                    </label>
                                    <input
                                        type="number"
                                        id="crf-threshold"
                                        bind:value={config.crfAutoSkipThreshold}
                                        min="10"
                                        max="500"
                                        step="5"
                                    />
                                </div>
                            {/if}
                        {/if}
                    </div>
                </div>

                <div class="form-group-row">
                    <div class="form-group">
                        <label for="ffmpeg-threads">FFmpeg Tasks</label>
                        <input
                            type="number"
                            id="ffmpeg-threads"
                            min="1"
                            max="16"
                            bind:value={config.ffmpegThreads}
                        />
                    </div>
                    <div class="form-group">
                        <label for="ffprobe-threads">FFprobe Tasks</label>
                        <input
                            type="number"
                            id="ffprobe-threads"
                            min="1"
                            max="64"
                            bind:value={config.ffprobeThreads}
                        />
                    </div>
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
                            {#if enc.visible && shouldShowVideoEncoder(enc.value)}
                                <option value={enc.value}>{enc.name}</option>
                            {/if}
                        {/each}
                    </select>
                </div>

                <div class="form-group">
                    <label for="audio-encoder">Audio Encoder</label>
                    <select id="audio-encoder" bind:value={config.audioEncoder}>
                        {#each config.availableAudioEncoders as enc}
                            {#if enc.visible && shouldShowAudioEncoder(enc.value)}
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

                <div class="form-group">
                    <label for="output-suffix">Output Filename Suffix</label>
                    <input
                        type="text"
                        id="output-suffix"
                        bind:value={config.suffix}
                        placeholder="_compressed (leave empty to overwrite)"
                    />
                    <small
                        style="color: #666; font-size: 0.8rem; margin-top: 4px;"
                    >
                        If empty and input/output folders are the same, original
                        files will be overwritten.
                    </small>
                </div>

                <div class="form-group">
                    <span class="group-label">VMAF Configuration</span>

                    <div class="vmaf-settings">
                        <div class="row">
                            <label class="checkbox-label">
                                <input
                                    type="checkbox"
                                    bind:checked={config.enableVmaf}
                                    disabled={config.maxResolution.enabled}
                                />
                                Auto calculate VMAF score after compression
                                {#if config.maxResolution.enabled}
                                    <span class="warning-text"
                                        >(Disabled by Resolution Limit)</span
                                    >
                                {/if}
                            </label>
                        </div>
                        {#if config.maxResolution.enabled && config.enableVmaf}
                            <!-- Automatically uncheck if enabled -->
                            <script>
                                // This is reactive in Svelte 5 logic already by nature of $derived or simple if logic
                                // But here we are in script module. We can use effect or just do it in the change handler
                                // of resolution limit.
                                // We handled it in toggleResolutionLimit roughly for Mode.
                            </script>
                        {/if}

                        <div class="row">
                            <label class="checkbox-label">
                                <input
                                    type="checkbox"
                                    bind:checked={config.vmafFullComputation}
                                />
                                Full Video Calculation (Sluggish)
                            </label>
                        </div>

                        {#if !config.vmafFullComputation}
                            <div class="row">
                                <label class="checkbox-label">
                                    <input
                                        type="checkbox"
                                        bind:checked={config.vmafAutoConfig}
                                    />
                                    Auto Configure Segments
                                </label>
                            </div>
                            <div
                                class="row nested-row"
                                class:disabled={config.vmafAutoConfig}
                            >
                                <div class="sub-input">
                                    <label for="vmaf-seg-count">Segments</label>
                                    <input
                                        type="number"
                                        id="vmaf-seg-count"
                                        bind:value={config.vmafSegmentCount}
                                        min="1"
                                        max="100"
                                    />
                                </div>
                                <div class="sub-input">
                                    <label for="vmaf-seg-dur"
                                        >Duration (s)</label
                                    >
                                    <input
                                        type="number"
                                        id="vmaf-seg-dur"
                                        bind:value={config.vmafSegmentDuration}
                                        min="1"
                                        max="60"
                                    />
                                </div>
                            </div>
                        {/if}

                        <div class="row" style="margin-top: 8px;">
                            <label class="checkbox-label">
                                <input
                                    type="checkbox"
                                    bind:checked={config.vmafUseCuda}
                                />
                                Use CUDA Acceleration (Experimental)
                            </label>
                        </div>
                        <div class="row" style="margin-top: 8px;">
                            <label class="checkbox-label">
                                <input
                                    type="checkbox"
                                    bind:checked={config.vmafNeg}
                                />
                                Anti-filter/Sharpening
                            </label>
                        </div>
                    </div>
                </div>
            {:else}
                <!-- ADVANCED VIEW -->
                <div class="section">
                    <div class="section-header">
                        <h3>Encoder Management</h3>
                        <button
                            class="secondary-btn"
                            onclick={detectEncoders}
                            disabled={isDetecting}
                        >
                            {#if isDetecting}
                                Scanning...
                            {:else}
                                Detect Encoders
                            {/if}
                        </button>
                    </div>

                    <!-- Encoder Filter Options -->
                    <div class="encoder-filter-options">
                        <label class="checkbox-label">
                            <input
                                type="checkbox"
                                bind:checked={config.showAllEncoders}
                            />
                            Show All Available Encoders
                            <span
                                class="tooltip"
                                title="When unchecked, only commonly used encoders are shown"
                            >
                                <svg
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <circle cx="12" cy="12" r="10"></circle>
                                    <path d="M12 16v-4"></path>
                                    <path d="M12 8h.01"></path>
                                </svg>
                            </span>
                        </label>
                        <label class="checkbox-label">
                            <input
                                type="checkbox"
                                bind:checked={config.showOnlyHwEncoders}
                            />
                            Only Show Hardware-Accelerated Video Encoders
                            <span
                                class="tooltip"
                                title="Only show GPU/hardware encoders like NVENC, AMF, QSV"
                            >
                                <svg
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <circle cx="12" cy="12" r="10"></circle>
                                    <path d="M12 16v-4"></path>
                                    <path d="M12 8h.01"></path>
                                </svg>
                            </span>
                        </label>
                    </div>

                    <div class="encoder-list">
                        <h4>Video Encoders</h4>
                        {#each config.availableVideoEncoders as enc, i}
                            {#if enc.isSupported && shouldShowVideoEncoder(enc.value)}
                                <div class="encoder-row">
                                    <label class="checkbox-label">
                                        <input
                                            type="checkbox"
                                            bind:checked={enc.visible}
                                        />
                                        {enc.name}
                                    </label>
                                    <div class="encoder-actions">
                                        {#if enc.customParams?.length > 0}
                                            <span class="badge small"
                                                >{enc.customParams.length} active</span
                                            >
                                        {/if}
                                        <button
                                            class="secondary-btn small-btn"
                                            onclick={() =>
                                                openParamsEditor("video", i)}
                                        >
                                            Edit Params
                                        </button>
                                    </div>
                                </div>
                            {/if}
                        {/each}

                        <h4>Audio Encoders</h4>
                        {#each config.availableAudioEncoders as enc, i}
                            {#if enc.isSupported && shouldShowAudioEncoder(enc.value)}
                                <div class="encoder-row">
                                    <label class="checkbox-label">
                                        <input
                                            type="checkbox"
                                            bind:checked={enc.visible}
                                        />
                                        {enc.name}
                                    </label>
                                    <div class="encoder-actions">
                                        {#if enc.customParams?.length > 0}
                                            <span class="badge small"
                                                >{enc.customParams.length} active</span
                                            >
                                        {/if}
                                        <button
                                            class="secondary-btn small-btn"
                                            onclick={() =>
                                                openParamsEditor("audio", i)}
                                        >
                                            Edit Params
                                        </button>
                                    </div>
                                </div>
                            {/if}
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
                    <h3>Custom Compression Params</h3>
                    <div class="filter-controls">
                        <button
                            class="secondary-btn"
                            onclick={() => openParamsEditor("filters")}
                        >
                            Edit Params
                        </button>
                        {#if config.customFilters.length > 0}
                            <span class="badge"
                                >{config.customFilters.length} active</span
                            >
                        {/if}
                    </div>
                </div>

                <div class="section">
                    <h3>Custom VMAF Params</h3>
                    <div class="filter-controls">
                        <button
                            class="secondary-btn"
                            onclick={() => openParamsEditor("vmafParams")}
                        >
                            Edit Params
                        </button>
                        {#if config.customVmafParams.length > 0}
                            <span class="badge"
                                >{config.customVmafParams.length} active</span
                            >
                        {/if}
                    </div>
                </div>

                <div class="section">
                    <button
                        class="secondary-btn reset-btn"
                        onclick={resetToDefaults}
                    >
                        🔄 Reset to Default Settings
                    </button>
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

        {#if notification}
            <div class={"notification " + notification.type}>
                {#if notification.type === "success"}
                    <span style="color: #34d399;">✓</span>
                {:else}
                    <span style="color: #ef4444;">⚠</span>
                {/if}
                {notification.message}
            </div>
        {/if}
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
    input[type="text"],
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
        margin-top: 0px;
    }

    .row {
        display: flex;
        gap: 10px;
        align-items: center;
    }

    .warning-text {
        color: #eab308; /* yellow-500 */
        font-size: 0.8rem;
        margin-left: 5px;
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

    .encoder-filter-options {
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding: 10px 12px;
        background: rgba(100, 108, 255, 0.1);
        border: 1px solid rgba(100, 108, 255, 0.2);
        border-radius: 6px;
        margin-bottom: 12px;
    }

    .encoder-filter-options .checkbox-label {
        font-size: 0.85rem;
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

    .vmaf-settings {
        margin-top: 8px;
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding: 8px;
        background: rgba(0, 0, 0, 0.2);
        border-radius: 6px;
    }

    .nested-row {
        margin-left: 12px;
        gap: 16px;
    }

    .nested-row.disabled {
        opacity: 0.5;
        pointer-events: none;
    }

    .sub-input {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .sub-input input {
        width: 60px;
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

    .badge.small {
        padding: 0px 6px;
        font-size: 0.7rem;
        min-width: 18px;
        text-align: center;
    }

    .encoder-actions {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .notification {
        position: absolute;
        bottom: 80px;
        left: 50%;
        transform: translateX(-50%);
        background: #333;
        color: white;
        padding: 12px 20px;
        border-radius: 8px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
        z-index: 1100;
        animation: slide-up 0.3s ease-out;
        font-size: 0.9rem;
        display: flex;
        align-items: center;
        gap: 10px;
        border: 1px solid #444;
        max-width: 80%;
        text-align: center;
    }

    .notification.success {
        border-left: 4px solid #34d399;
    }

    .notification.error {
        border-left: 4px solid #ef4444;
    }

    @keyframes slide-up {
        from {
            transform: translate(-50%, 20px);
            opacity: 0;
        }
        to {
            transform: translate(-50%, 0);
            opacity: 1;
        }
    }

    .mode-extra-settings {
        margin-top: 10px;
        border-bottom: 3px solid #333;
        padding-top: 10px;
    }

    .extra-setting {
        display: flex;
        flex-direction: column;
        gap: 6px;
        margin-bottom: 15px;
    }

    .tooltip {
        cursor: help;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 14px;
        height: 14px;
        color: #646cff;
        opacity: 0.6;
        transition: all 0.2s;
        vertical-align: middle;
        margin-left: 4px;
    }

    .tooltip:hover {
        opacity: 1;
        transform: scale(1.15);
    }

    .tooltip svg {
        width: 100%;
        height: 100%;
    }

    .form-group-row {
        display: flex;
        gap: 16px;
    }
    .form-group-row .form-group {
        flex: 1;
    }

    .reset-btn {
        width: 100%;
        color: #ff9999;
        border-color: rgba(255, 153, 153, 0.3);
        margin-top: 8px;
    }

    .reset-btn:hover {
        border-color: rgba(255, 153, 153, 0.6);
        color: #ffb3b3;
        background: rgba(255, 153, 153, 0.1);
    }
</style>
