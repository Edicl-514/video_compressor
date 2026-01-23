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
    import EncoderDetectionModal from "./EncoderDetectionModal.svelte";
    import LanguageSwitcher from "./LanguageSwitcher.svelte";
    import { _ as t } from "svelte-i18n";

    let { close } = $props<{ close: () => void }>();

    const initialData = JSON.parse(JSON.stringify(settingsStore.value));

    // Safety initialization for existing encoders
    initialData.availableVideoEncoders.forEach((e: any) => {
        if (!e.customParams) e.customParams = [];
        if (e.isSupported === undefined) e.isSupported = true;
    });
    initialData.availableAudioEncoders.forEach((e: any) => {
        if (!e.customParams) e.customParams = [];
        if (e.isSupported === undefined) e.isSupported = true;
    });

    let config = $state<AppSettings>(initialData);
    let currentView = $state<"basic" | "advanced">("basic");
    let editingTarget = $state<{
        type: "video" | "audio" | "filters" | "vmafParams";
        index: number;
        title: string;
        params: string[];
    } | null>(null);
    let isDetecting = $state<boolean>(false);
    let showDetectionModal = $state<boolean>(false);
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

    function detectEncoders() {
        showDetectionModal = true;
    }

    function handleDetectionComplete(report: any) {
        showDetectionModal = false;
        applyDetectionReport(report);
    }

    function applyDetectionReport(report: any) {
        console.log("Detection complete:", report);

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
                config.availableVideoEncoders[existingIndex].isSupported = true;
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
                config.availableAudioEncoders[existingIndex].isSupported = true;
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
            $t("common.detection_complete", {
                values: {
                    video_count: report.video.length,
                    audio_count: report.audio.length,
                },
            }),
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
                title: $t("common.custom_params"),
                params: config.customFilters,
            };
            return;
        }

        if (type === "vmafParams") {
            editingTarget = {
                type: "vmafParams",
                index: 0,
                title: $t("common.custom_vmaf_params"),
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
            title: $t("common.edit_params_encoder", {
                values: { name: list[index].name },
            }),
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
        showNotification($t("common.settings_reset"));
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
                {currentView === "basic"
                    ? $t("common.settings")
                    : $t("common.advanced_settings")}
            </h2>
            <button class="close-btn" onclick={close}>&times;</button>
        </header>

        <div class="content">
            {#if currentView === "basic"}
                <div class="form-group">
                    <label for="language-switcher">Language / 语言</label>
                    <LanguageSwitcher />
                </div>

                <div class="form-group">
                    <label for="compression-mode"
                        >{$t("common.compression_mode")}</label
                    >
                    <div class="mode-select-row">
                        <select
                            id="compression-mode"
                            bind:value={config.compressionMode}
                            class="compression-select"
                        >
                            <option value={CompressionMode.BITRATE}
                                >{$t("common.target_bitrate")}</option
                            >
                            <option value={CompressionMode.CRF}
                                >{$t("common.target_crf")}</option
                            >
                            <option
                                value={CompressionMode.VMAF}
                                disabled={config.maxResolution.enabled}
                                >{$t("common.target_vmaf")}
                                {config.maxResolution.enabled
                                    ? $t("common.disabled_by_resolution_limit")
                                    : ""}</option
                            >
                            <option value={CompressionMode.COPY}
                                >{$t("common.copy_mode")}</option
                            >
                            <option value={CompressionMode.CUSTOM}
                                >{$t("common.custom_mode")}</option
                            >
                        </select>
                    </div>
                    <style>
                        .compression-select {
                            width: 440px;
                        }
                    </style>

                    <!-- Dynamic Input for Selected Mode -->
                    <div class="mode-value-input">
                        {#if config.compressionMode === CompressionMode.BITRATE}
                            <label for="target-bitrate"
                                >{$t("common.bitrate_kbps")}</label
                            >
                            <input
                                id="target-bitrate"
                                type="number"
                                bind:value={config.targetBitrate}
                                min="100"
                                step="100"
                            />
                        {:else if config.compressionMode === CompressionMode.CRF}
                            <label for="target-crf"
                                >{$t("common.target_crf")}</label
                            >
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
                            <label for="target-vmaf"
                                >{$t("common.target_vmaf_0_100")}</label
                            >
                            <input
                                id="target-vmaf"
                                type="number"
                                bind:value={config.targetVMAF}
                                min="0"
                                max="100"
                            />
                        {:else if config.compressionMode === CompressionMode.COPY}
                            <small class="copy-mode-hint">
                                {$t("common.copy_mode_hint")}
                            </small>
                        {:else if config.compressionMode === CompressionMode.CUSTOM}
                            <div class="mode-extra-settings">
                                <label for="custom-command"
                                    >{$t("common.custom_command_label")}</label
                                >
                                <textarea
                                    id="custom-command"
                                    class="custom-command-textarea"
                                    bind:value={config.customCommand}
                                    rows="3"
                                    placeholder="ffmpeg -i %INPUT% -c copy %OUTPUT%"
                                ></textarea>
                                <small class="copy-mode-hint"
                                    >{$t("common.custom_command_hint")}</small
                                >
                            </div>
                        {/if}
                    </div>

                    <div class="mode-extra-settings">
                        {#if config.compressionMode === CompressionMode.BITRATE}
                            <div class="extra-setting">
                                <label for="bypass-threshold">
                                    {$t("common.bypass_threshold_kbps")}
                                    <span
                                        class="tooltip"
                                        title={$t(
                                            "common.bypass_threshold_hint",
                                        )}
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
                                    placeholder={$t(
                                        "common.bypass_threshold_hint",
                                    )}
                                />
                            </div>
                            <div class="extra-setting">
                                <label class="checkbox-label">
                                    <input
                                        type="checkbox"
                                        bind:checked={config.twoPass}
                                    />
                                    {$t("common.enable_2_pass")}
                                    <span
                                        class="tooltip"
                                        title={$t("common.2pass_hint")}
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
                                    {$t("common.crf_auto_skip")}
                                    <span
                                        class="tooltip"
                                        title={$t("common.crf_auto_skip_hint")}
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
                                        {$t("common.crf_skip_threshold")}
                                        <span
                                            class="tooltip"
                                            title={$t(
                                                "common.crf_skip_threshold_hint",
                                            )}
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
                        <label for="ffmpeg-threads"
                            >{$t("common.ffmpeg_tasks")}</label
                        >
                        <input
                            type="number"
                            id="ffmpeg-threads"
                            min="1"
                            max="16"
                            bind:value={config.ffmpegThreads}
                        />
                    </div>
                    <div class="form-group">
                        <label for="ffprobe-threads"
                            >{$t("common.ffprobe_tasks")}</label
                        >
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
                    <span class="group-label"
                        >{$t("common.max_resolution")}</span
                    >
                    <div class="row">
                        <label class="checkbox-label">
                            <input
                                type="checkbox"
                                checked={config.maxResolution.enabled}
                                onchange={toggleResolutionLimit}
                                disabled={config.compressionMode ===
                                    CompressionMode.COPY ||
                                    config.compressionMode ===
                                        CompressionMode.CUSTOM}
                            />
                            {$t("common.limit_resolution")}
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
                    <label for="video-encoder"
                        >{$t("common.video_encoder")}</label
                    >
                    <select
                        id="video-encoder"
                        bind:value={config.videoEncoder}
                        disabled={config.compressionMode ===
                            CompressionMode.COPY ||
                            config.compressionMode === CompressionMode.CUSTOM}
                    >
                        {#each config.availableVideoEncoders as enc}
                            {#if enc.visible && shouldShowVideoEncoder(enc.value)}
                                <option value={enc.value}>{enc.name}</option>
                            {/if}
                        {/each}
                    </select>
                </div>

                <div class="form-group">
                    <label for="audio-encoder"
                        >{$t("common.audio_encoder")}</label
                    >
                    <select
                        id="audio-encoder"
                        bind:value={config.audioEncoder}
                        disabled={config.compressionMode ===
                            CompressionMode.COPY ||
                            config.compressionMode === CompressionMode.CUSTOM}
                    >
                        {#each config.availableAudioEncoders as enc}
                            {#if enc.visible && shouldShowAudioEncoder(enc.value)}
                                <option value={enc.value}>{enc.name}</option>
                            {/if}
                        {/each}
                    </select>
                </div>

                <div class="form-group">
                    <label for="target-format"
                        >{$t("common.target_format")}</label
                    >
                    <select id="target-format" bind:value={config.targetFormat}>
                        <option value="mp4">MP4</option>
                        <option value="mkv">MKV</option>
                        <option value="mov">MOV</option>
                        <option value="webm">WebM</option>
                    </select>
                </div>

                <div class="form-group">
                    <label for="output-suffix">{$t("common.suffix")}</label>
                    <input
                        type="text"
                        id="output-suffix"
                        bind:value={config.suffix}
                        placeholder={$t("common.suffix_hint")}
                    />
                    <small
                        style="color: #666; font-size: 0.8rem; margin-top: 4px;"
                    >
                        {$t("common.suffix_hint")}
                    </small>
                </div>

                <div class="form-group">
                    <span class="group-label">{$t("common.vmaf_config")}</span>

                    <div class="vmaf-settings">
                        <div class="row">
                            <label class="checkbox-label">
                                <input
                                    type="checkbox"
                                    bind:checked={config.enableVmaf}
                                    disabled={config.maxResolution.enabled}
                                />
                                {$t("common.auto_calculate_vmaf_score")}
                                {#if config.maxResolution.enabled}
                                    <span class="warning-text"
                                        >{$t(
                                            "common.disabled_by_resolution_limit",
                                        )}</span
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
                                {$t("common.vmaf_full_computation")}
                            </label>
                        </div>

                        {#if !config.vmafFullComputation}
                            <div class="row">
                                <label class="checkbox-label">
                                    <input
                                        type="checkbox"
                                        bind:checked={config.vmafAutoConfig}
                                    />
                                    {$t("common.vmaf_auto_config")}
                                </label>
                            </div>
                            <div
                                class="row nested-row"
                                class:disabled={config.vmafAutoConfig}
                            >
                                <div class="sub-input">
                                    <label for="vmaf-seg-count"
                                        >{$t(
                                            "common.vmaf_segment_count",
                                        )}</label
                                    >
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
                                        >{$t(
                                            "common.vmaf_segment_duration",
                                        )}</label
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
                                {$t("common.vmaf_use_cuda")}
                            </label>
                        </div>
                        <div class="row" style="margin-top: 8px;">
                            <label class="checkbox-label">
                                <input
                                    type="checkbox"
                                    bind:checked={config.vmafNeg}
                                />
                                {$t("common.vmaf_neg")}
                            </label>
                        </div>

                        {#if config.compressionMode === CompressionMode.VMAF}
                            <div class="row" style="margin-top: 12px;">
                                <label class="checkbox-label">
                                    <input
                                        type="checkbox"
                                        bind:checked={
                                            config.vmafSearchOptimization
                                        }
                                    />
                                    {$t("common.vmaf_search_optimization")}
                                </label>
                            </div>
                            {#if config.vmafSearchOptimization}
                                <div
                                    class="warning-box"
                                    style="margin-top: 8px;"
                                >
                                    {$t(
                                        "common.vmaf_search_optimization_warning",
                                    )}
                                </div>
                            {/if}
                        {/if}
                    </div>
                </div>
            {:else}
                <!-- ADVANCED VIEW -->
                <div class="section">
                    <div class="section-header">
                        <h3>{$t("common.encoder_management")}</h3>
                        <button class="secondary-btn" onclick={detectEncoders}>
                            {$t("common.detect_encoders")}
                        </button>
                    </div>

                    <!-- Encoder Filter Options -->
                    <div class="encoder-filter-options">
                        <label class="checkbox-label">
                            <input
                                type="checkbox"
                                bind:checked={config.showAllEncoders}
                            />
                            {$t("common.show_all_encoders")}
                            <span
                                class="tooltip"
                                title={$t("common.show_all_encoders_hint")}
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
                            {$t("common.show_only_hw_encoders")}
                            <span
                                class="tooltip"
                                title={$t("common.show_only_hw_encoders_hint")}
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
                        <h4>{$t("common.video_encoders")}</h4>
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
                                                >{enc.customParams.length}
                                                {$t("common.activated")}</span
                                            >
                                        {/if}
                                        <button
                                            class="secondary-btn small-btn"
                                            onclick={() =>
                                                openParamsEditor("video", i)}
                                        >
                                            {$t("common.edit_params")}
                                        </button>
                                    </div>
                                </div>
                            {/if}
                        {/each}

                        <h4>{$t("common.audio_encoders")}</h4>
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
                                                >{enc.customParams.length}
                                                {$t("common.activated")}</span
                                            >
                                        {/if}
                                        <button
                                            class="secondary-btn small-btn"
                                            onclick={() =>
                                                openParamsEditor("audio", i)}
                                        >
                                            {$t("common.edit_params")}
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

                {#if showDetectionModal}
                    <EncoderDetectionModal
                        onComplete={handleDetectionComplete}
                    />
                {/if}

                <div class="section">
                    <h3>{$t("common.custom_compression_params")}</h3>
                    <div class="filter-controls">
                        <button
                            class="secondary-btn"
                            onclick={() => openParamsEditor("filters")}
                        >
                            {$t("common.edit_params")}
                        </button>
                        {#if config.customFilters.length > 0}
                            <span class="badge"
                                >{config.customFilters.length}
                                {$t("common.activated")}</span
                            >
                        {/if}
                    </div>
                </div>

                <div class="section">
                    <h3>{$t("common.custom_vmaf_params")}</h3>
                    <div class="filter-controls">
                        <button
                            class="secondary-btn"
                            onclick={() => openParamsEditor("vmafParams")}
                        >
                            {$t("common.edit_params")}
                        </button>
                        {#if config.customVmafParams.length > 0}
                            <span class="badge"
                                >{config.customVmafParams.length}
                                {$t("common.activated")}</span
                            >
                        {/if}
                    </div>
                </div>

                <div class="section">
                    <button
                        class="secondary-btn reset-btn"
                        onclick={resetToDefaults}
                    >
                        {$t("common.reset_to_default")}
                    </button>
                </div>
            {/if}
        </div>

        <footer>
            {#if currentView === "basic"}
                <button
                    class="secondary-btn"
                    onclick={() => (currentView = "advanced")}
                    >{$t("common.advanced_settings")}</button
                >
            {:else}
                <button
                    class="secondary-btn"
                    onclick={() => (currentView = "basic")}
                    >{$t("common.back_to_basics")}</button
                >
            {/if}
            <div class="spacer"></div>
            <button class="secondary-btn" onclick={close}
                >{$t("common.cancel")}</button
            >
            <button class="primary-btn" onclick={save}
                >{$t("common.save")}</button
            >
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
        background: rgba(0, 0, 0, 0.65);
        backdrop-filter: blur(8px);
        -webkit-backdrop-filter: blur(8px);
        display: flex;
        justify-content: center;
        align-items: center;
        z-index: 1000;
        animation: fadein 0.3s cubic-bezier(0.2, 0.8, 0.2, 1);
    }

    .modal {
        background: var(--bg-color);
        background: linear-gradient(to bottom, #252525, #1e1e1e);
        color: var(--text-main);
        width: 500px;
        max-width: 90vw;
        max-height: 85vh;
        border-radius: var(--radius-lg);
        box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5), 0 0 0 1px var(--border-color);
        display: flex;
        flex-direction: column;
        overflow: hidden;
        border: 1px solid #333;
        animation: scaleup 0.2s ease-out;
    }

    header {
        padding: 18px 24px;
        background: rgba(255, 255, 255, 0.03);
        border-bottom: 1px solid var(--border-color);
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

    h3 {
        margin: 0;
        font-size: 1.1rem;
        font-weight: 600;
        color: var(--text-main);
    }

    h4 {
        margin: 16px 0 8px 0;
        font-size: 0.85rem;
        font-weight: 600;
        color: var(--text-muted);
        text-transform: uppercase;
        letter-spacing: 0.05em;
    }

    .group-label {
        font-size: 0.85rem;
        font-weight: 600;
        color: var(--text-muted);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        display: block;
        margin-bottom: 8px;
    }

    label:not(.checkbox-label) {
        font-size: 0.82rem;
        font-weight: 600;
        color: var(--text-muted);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin-top: 4px;
    }

    label {
        font-size: 0.95rem;
        color: var(--text-main);
    }

    input[type="number"],
    input[type="text"],
    select {
        background: #121212;
        border: 1px solid var(--border-color);
        border-radius: var(--radius-sm);
        padding: 10px 12px;
        color: #ffffff;
        font-family: inherit;
        font-size: 0.95rem;
        font-weight: 500;
        transition: all 0.2s ease;
    }

    select option {
        background-color: #121212;
        color: #ffffff;
    }

    input:focus,
    select:focus {
        outline: none;
        border-color: var(--primary-color);
        box-shadow: 0 0 0 3px var(--primary-glow);
        background: #1a1a1a;
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
        background: var(--primary-color);
        background: linear-gradient(135deg, var(--primary-color), var(--primary-active));
        color: white;
        border: none;
        box-shadow: 0 4px 6px rgba(100, 108, 255, 0.25);
    }

    .primary-btn:hover {
        background: var(--primary-hover);
        box-shadow: 0 6px 12px rgba(100, 108, 255, 0.4);
        transform: translateY(-1px);
    }

    .secondary-btn {
        background: transparent;
        color: var(--text-muted);
        border: 1px solid var(--border-color);
    }

    .secondary-btn:hover {
        border-color: var(--text-muted);
        color: var(--text-main);
        background: rgba(255, 255, 255, 0.05);
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

    .copy-mode-hint {
        color: #888;
        font-size: 0.85rem;
        line-height: 1.5;
        padding: 8px 0;
        display: block;
    }

    .custom-command-textarea {
        width: 100%;
        box-sizing: border-box;
        background: #2a2a2a;
        color: #fff;
        border: 1px solid #444;
        border-radius: 6px;
        padding: 8px 12px;
        font-family: monospace;
        resize: vertical;
        min-height: 64px;
        max-height: 360px;
        font-size: 0.95rem;
    }

    .custom-command-textarea:focus {
        outline: none;
        border-color: #646cff;
        box-shadow: 0 0 0 3px rgba(100, 108, 255, 0.06);
    }

    .warning-box {
        background: rgba(251, 191, 36, 0.15);
        border: 1px solid rgba(251, 191, 36, 0.4);
        border-radius: 6px;
        padding: 10px 12px;
        font-size: 0.8rem;
        color: #fbbf24;
        line-height: 1.5;
    }
</style>
