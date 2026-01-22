<script lang="ts">
    import { createEventDispatcher, onMount } from "svelte";
    import { t, locale } from "svelte-i18n";
    import { invoke } from "@tauri-apps/api/core";
    import { settingsStore } from "$lib/stores/settings.svelte.js";
    import type { EncoderConfig } from "$lib/types";
    import { CompressionMode } from "$lib/types";

    const dispatch = createEventDispatcher();

    let currentStep = $state(0);
    let isDetecting = $state(false);
    let detectedCount = $state(0);
    let recommendedEncoder = $state<EncoderConfig | null>(null);
    let recommendedMode = $state<CompressionMode>(CompressionMode.CRF);

    // Available languages
    const languages = [
        { code: "en", name: "English" },
        { code: "zh", name: "中文" },
    ];

    async function nextStep() {
        if (currentStep === 0) {
            // Language selected, now detect encoders
            currentStep++;
            await detectEncoders();
        } else if (currentStep === 1) {
            // Encoders detected, recommended one set.
            // Now decide mode based on recommendation
            decideMode();
            currentStep++;
        } else {
            finish();
        }
    }

    function selectLanguage(lang: string) {
        locale.set(lang);
    }

    async function detectEncoders() {
        isDetecting = true;
        try {
            // Call the backend detection
            const report: any = await invoke("detect_encoders");

            // Update settings with detected encoders
            settingsStore.update((s) => {
                // Mark all as unsupported first
                s.availableVideoEncoders.forEach(
                    (e) => (e.isSupported = false),
                );
                s.availableAudioEncoders.forEach(
                    (e) => (e.isSupported = false),
                );

                // Update based on report
                report.video.forEach((d: any) => {
                    const index = s.availableVideoEncoders.findIndex(
                        (e) => e.value === d.value,
                    );
                    if (index >= 0) {
                        s.availableVideoEncoders[index].isSupported = true;
                    } else {
                        s.availableVideoEncoders.push({
                            name: d.name,
                            value: d.value,
                            visible: true,
                            isSupported: true,
                            customParams: [],
                        });
                    }
                });

                report.audio.forEach((d: any) => {
                    const index = s.availableAudioEncoders.findIndex(
                        (e) => e.value === d.value,
                    );
                    if (index >= 0) {
                        s.availableAudioEncoders[index].isSupported = true;
                    } else {
                        s.availableAudioEncoders.push({
                            name: d.name,
                            value: d.value,
                            visible: true,
                            isSupported: true,
                            customParams: [],
                        });
                    }
                });

                // Filter valid encoders
                const validEncoders = s.availableVideoEncoders.filter(
                    (e) => e.isSupported,
                );
                detectedCount = validEncoders.length;

                // Recommendation Logic
                // Priority: HW > SW
                // Vendor: Nvidia > QSV > AMF > Other
                // Codec: AV1 > HEVC > H264 > Other

                const sorted = validEncoders.sort((a, b) => {
                    // 1. Hardware vs Software
                    // "nvenc", "qsv", "amf", "vaapi" vs others
                    const isHwA = isHardware(a.value);
                    const isHwB = isHardware(b.value);
                    if (isHwA && !isHwB) return -1;
                    if (!isHwA && isHwB) return 1;

                    // 2. Vendor Rank
                    const rankVendor = (val: string) => {
                        if (val.includes("nvenc")) return 3;
                        if (val.includes("qsv")) return 2;
                        if (val.includes("amf")) return 1;
                        return 0; // Other HW or SW
                    };
                    const diffVendor =
                        rankVendor(b.value) - rankVendor(a.value);
                    if (diffVendor !== 0) return diffVendor;

                    // 3. Codec Rank
                    const rankCodec = (val: string) => {
                        if (val.includes("av1")) return 3;
                        if (val.includes("hevc") || val.includes("265"))
                            return 2;
                        if (val.includes("h264") || val.includes("264"))
                            return 1;
                        return 0;
                    };
                    return rankCodec(b.value) - rankCodec(a.value);
                });

                if (sorted.length > 0) {
                    recommendedEncoder = sorted[0];
                    s.videoEncoder = recommendedEncoder.value;
                }
            });
        } catch (e) {
            console.error("Detection failed", e);
        } finally {
            isDetecting = false;
        }
    }

    function isHardware(name: string) {
        return (
            name.includes("nvenc") ||
            name.includes("qsv") ||
            name.includes("amf") ||
            name.includes("vaapi")
        );
    }

    function decideMode() {
        if (recommendedEncoder && recommendedEncoder.value.includes("nvenc")) {
            recommendedMode = CompressionMode.VMAF; // Target VMAF
        } else {
            recommendedMode = CompressionMode.CRF; // Target CRF
        }

        settingsStore.update((s) => {
            s.compressionMode = recommendedMode;
        });
    }

    function setMode(mode: CompressionMode) {
        recommendedMode = mode;
        settingsStore.update((s) => {
            s.compressionMode = mode;
        });
    }

    function finish() {
        settingsStore.update((s) => {
            s.firstRun = false;
        });
        dispatch("close");
    }

    // Auto-detect language on mount
    onMount(() => {
        // If user has system language set to zh, default to it
        const sysLang = navigator.language.toLowerCase();
        if (sysLang.startsWith("zh")) {
            locale.set("zh");
        } else {
            locale.set("en");
        }
    });
</script>

<div class="backdrop">
    <div class="wizard-card">
        <header>
            <h1>{$t("welcome_wizard.title")}</h1>
            <p>{$t("welcome_wizard.intro")}</p>
        </header>

        <!-- Steps Indicator -->
        <div class="stepper">
            {#each [0, 1, 2] as step}
                <div class="step-item" class:active={currentStep >= step}>
                    <div class="step-circle">
                        {step + 1}
                    </div>
                </div>
                {#if step < 2}
                    <div
                        class="step-line"
                        class:active={currentStep > step}
                    ></div>
                {/if}
            {/each}
        </div>

        <div class="wizard-content">
            <!-- Step 1: Language -->
            {#if currentStep === 0}
                <div class="step-section">
                    <h2>{$t("welcome_wizard.step_language")}</h2>
                    <div class="options-grid">
                        {#each languages as lang}
                            <button
                                class="option-btn"
                                onclick={() => selectLanguage(lang.code)}
                            >
                                {lang.name}
                            </button>
                        {/each}
                    </div>
                </div>

                <!-- Step 2: Encoders -->
            {:else if currentStep === 1}
                <div class="step-section center-text">
                    <h2>{$t("welcome_wizard.step_encoders")}</h2>

                    {#if isDetecting}
                        <div class="loading-state">
                            <div class="spinner"></div>
                            <p class="pulse">
                                {$t("welcome_wizard.detecting")}
                            </p>
                        </div>
                    {:else if recommendedEncoder}
                        <div class="recommendation-box">
                            <div class="check-icon">✓</div>
                            <p class="label">
                                {$t("welcome_wizard.recommended_encoder")}
                            </p>
                            <p class="value">{recommendedEncoder.name}</p>
                            <p class="sub-text">
                                {$t("welcome_wizard.detection_result", {
                                    values: { count: detectedCount },
                                })}
                            </p>
                        </div>
                    {:else}
                        <div class="error-box">
                            Detection finished but no optimal encoder found.
                        </div>
                    {/if}
                </div>

                <!-- Step 3: Mode -->
            {:else if currentStep === 2}
                <div class="step-section">
                    <h2>{$t("welcome_wizard.step_mode")}</h2>
                    <p class="desc">
                        {$t("welcome_wizard.mode_selection_desc")}
                    </p>

                    <div class="options-list">
                        <!-- Recommended Option -->
                        <div
                            class="option-card recommended"
                            role="button"
                            tabindex="0"
                            onclick={() => setMode(recommendedMode)}
                            onkeydown={(e) =>
                                e.key === "Enter" && setMode(recommendedMode)}
                        >
                            <div class="badge">
                                {$t("welcome_wizard.recommended_mode")}
                            </div>
                            <h3>
                                {recommendedMode === CompressionMode.VMAF
                                    ? $t("common.target_vmaf")
                                    : $t("common.target_crf")}
                            </h3>
                            <p>
                                {recommendedMode === CompressionMode.VMAF
                                    ? $t("welcome_wizard.gpu_detected_hint")
                                    : $t("welcome_wizard.cpu_detected_hint")}
                            </p>
                        </div>

                        <!-- Alternative Option -->
                        {#if recommendedMode === CompressionMode.VMAF}
                            <div
                                class="option-card"
                                role="button"
                                tabindex="0"
                                onclick={() => setMode(CompressionMode.CRF)}
                                onkeydown={(e) =>
                                    e.key === "Enter" &&
                                    setMode(CompressionMode.CRF)}
                            >
                                <h3>{$t("common.target_crf")}</h3>
                            </div>
                        {:else}
                            <div
                                class="option-card"
                                role="button"
                                tabindex="0"
                                onclick={() => setMode(CompressionMode.VMAF)}
                                onkeydown={(e) =>
                                    e.key === "Enter" &&
                                    setMode(CompressionMode.VMAF)}
                            >
                                <h3>{$t("common.target_vmaf")}</h3>
                            </div>
                        {/if}
                    </div>
                </div>
            {/if}
        </div>

        <footer>
            {#if currentStep > 0}
                <!-- Spacer -->
                <div></div>
            {/if}
            <div class="actions">
                <button
                    class="primary-btn"
                    onclick={nextStep}
                    disabled={isDetecting}
                >
                    {currentStep === 2
                        ? $t("welcome_wizard.finish")
                        : $t("welcome_wizard.next")}
                </button>
            </div>
        </footer>
    </div>
</div>

<style>
    .backdrop {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(0, 0, 0, 0.85);
        backdrop-filter: blur(8px);
        z-index: 9999;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .wizard-card {
        background: #252525;
        width: 500px;
        max-width: 90%;
        border-radius: 16px;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.6);
        border: 1px solid #444;
        overflow: hidden;
        display: flex;
        flex-direction: column;
        animation: scaleup 0.3s ease-out;
    }

    header {
        padding: 30px 30px 20px;
        text-align: center;
        background: linear-gradient(180deg, #2a2a2a 0%, #252525 100%);
        border-bottom: 1px solid #333;
    }

    header h1 {
        margin: 0;
        font-size: 1.8rem;
        font-weight: 700;
        background: linear-gradient(90deg, #646cff, #a78bfa);
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
        margin-bottom: 8px;
    }

    header p {
        margin: 0;
        color: #aaa;
        font-size: 0.95rem;
    }

    .stepper {
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 20px 0;
        background: #222;
        border-bottom: 1px solid #333;
    }

    .step-item {
        display: flex;
        align-items: center;
    }

    .step-circle {
        width: 32px;
        height: 32px;
        border-radius: 50%;
        background: #333;
        color: #666;
        display: flex;
        align-items: center;
        justify-content: center;
        font-weight: bold;
        font-size: 0.9rem;
        transition: all 0.3s;
        border: 2px solid #444;
    }

    .step-item.active .step-circle {
        background: #646cff;
        color: #fff;
        border-color: #646cff;
        box-shadow: 0 0 10px rgba(100, 108, 255, 0.4);
    }

    .step-line {
        width: 40px;
        height: 4px;
        background: #333;
        margin: 0 8px;
        border-radius: 2px;
        transition: all 0.3s;
    }

    .step-line.active {
        background: #646cff;
    }

    .wizard-content {
        padding: 30px;
        min-height: 250px;
        display: flex;
        flex-direction: column;
        justify-content: center;
    }

    .step-section {
        display: flex;
        flex-direction: column;
        gap: 20px;
        animation: fadein 0.3s ease-out;
    }

    .step-section h2 {
        margin: 0;
        font-size: 1.4rem;
        text-align: center;
        color: #fff;
    }

    .center-text {
        text-align: center;
    }

    .desc {
        text-align: center;
        color: #999;
        margin: -10px 0 10px;
    }

    .options-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 16px;
    }

    .option-btn {
        padding: 20px;
        background: #333;
        border: 2px solid #444;
        border-radius: 12px;
        color: #fff;
        font-size: 1.1rem;
        cursor: pointer;
        transition: all 0.2s;
    }

    .option-btn:hover {
        background: #3a3a3a;
        border-color: #646cff;
        transform: translateY(-2px);
    }

    .loading-state {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 16px;
        padding: 20px;
    }

    .spinner {
        width: 40px;
        height: 40px;
        border: 4px solid rgba(100, 108, 255, 0.1);
        border-top-color: #646cff;
        border-radius: 50%;
        animation: spin 1s linear infinite;
    }

    .pulse {
        color: #aaa;
        animation: pulse 1.5s ease-in-out infinite;
    }

    .recommendation-box {
        background: rgba(52, 211, 153, 0.1);
        border: 1px solid rgba(52, 211, 153, 0.3);
        padding: 24px;
        border-radius: 12px;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
    }

    .check-icon {
        background: #34d399;
        color: #000;
        width: 40px;
        height: 40px;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 1.5rem;
        font-weight: bold;
        margin-bottom: 8px;
    }

    .label {
        font-size: 0.9rem;
        color: #34d399;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        margin: 0;
    }

    .value {
        font-size: 1.5rem;
        font-weight: bold;
        color: #fff;
        margin: 0;
    }

    .sub-text {
        font-size: 0.9rem;
        color: #aaa;
        margin: 0;
    }

    .options-list {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .option-card {
        padding: 16px;
        background: #333;
        border: 1px solid #444;
        border-radius: 10px;
        cursor: pointer;
        transition: all 0.2s;
        position: relative;
    }

    .option-card:hover {
        background: #3a3a3a;
        border-color: #666;
    }

    .option-card:focus {
        outline: none;
        border-color: #646cff;
        box-shadow: 0 0 0 2px rgba(100, 108, 255, 0.2);
    }

    .option-card.recommended {
        border: 2px solid #646cff;
        background: rgba(100, 108, 255, 0.05);
    }

    .option-card h3 {
        margin: 0 0 4px;
        font-size: 1.1rem;
        color: #fff;
    }

    .option-card.recommended h3 {
        color: #818cf8;
    }

    .option-card p {
        margin: 0;
        font-size: 0.9rem;
        color: #aaa;
        line-height: 1.4;
    }

    .badge {
        position: absolute;
        top: 0;
        right: 0;
        background: #646cff;
        color: white;
        font-size: 0.75rem;
        padding: 4px 8px;
        border-bottom-left-radius: 8px;
        font-weight: bold;
    }

    footer {
        padding: 20px 30px;
        background: #222;
        border-top: 1px solid #333;
        display: flex;
        justify-content: flex-end;
    }

    .primary-btn {
        background: #646cff;
        color: white;
        border: none;
        padding: 10px 24px;
        border-radius: 8px;
        font-weight: 600;
        font-size: 1rem;
        cursor: pointer;
        transition: all 0.2s;
        box-shadow: 0 4px 12px rgba(100, 108, 255, 0.3);
    }

    .primary-btn:hover:not(:disabled) {
        background: #535bf2;
        transform: translateY(-1px);
        box-shadow: 0 6px 16px rgba(100, 108, 255, 0.4);
    }

    .primary-btn:disabled {
        background: #444;
        color: #888;
        cursor: not-allowed;
        box-shadow: none;
    }

    @keyframes fadein {
        from {
            opacity: 0;
            transform: translateY(10px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }

    @keyframes scaleup {
        from {
            opacity: 0;
            transform: scale(0.95);
        }
        to {
            opacity: 1;
            transform: scale(1);
        }
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    @keyframes pulse {
        0%,
        100% {
            opacity: 1;
        }
        50% {
            opacity: 0.5;
        }
    }
</style>
