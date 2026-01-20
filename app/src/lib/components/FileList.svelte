<script lang="ts">
    import type { VideoInfo } from "$lib/types";
    import { invoke } from "@tauri-apps/api/core";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { t } from "svelte-i18n";

    export let files: VideoInfo[] = [];

    function canComputeVmaf(file: VideoInfo): boolean {
        return (
            file.status === "Done" &&
            !!file.outputInfo &&
            file.resolution === file.outputInfo.resolution &&
            file.path !== file.outputInfo.path &&
            (file.vmaf === undefined || file.vmaf === null)
        );
    }

    async function triggerVmaf(file: VideoInfo) {
        if (!canComputeVmaf(file)) return;

        try {
            await invoke("compute_vmaf", {
                inputPath: file.path,
                outputPath: file.outputInfo!.path,
                config: settingsStore.value,
                durationSec: file.durationSec || 0.0,
            });
        } catch (e) {
            console.error("Failed to start VMAF computation:", e);
        }
    }

    function formatSize(bytes: number): string {
        if (bytes === 0) return "0 B";
        const k = 1024;
        const sizes = ["B", "KB", "MB", "GB", "TB"];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
    }

    function getCompressionRatio(oldSize: number, newSize: number): string {
        if (oldSize === 0) return "0%";
        const ratio = ((oldSize - newSize) / oldSize) * 100;
        if (ratio >= 0) {
            return "↓" + ratio.toFixed(1) + "%";
        } else {
            return "↑" + Math.abs(ratio).toFixed(1) + "%";
        }
    }

    // Check if status text is long enough to need scrolling
    function isLongStatus(status: string): boolean {
        return status.length > 16;
    }
</script>

<div class="file-list-container">
    {#if files.length === 0}
        <div class="empty-state">
            <p>{$t("common.no_video_files")}</p>
        </div>
    {:else}
        <table class="file-list">
            <thead>
                <tr>
                    <th class="col-name">{$t("common.file_name")}</th>
                    <th class="col-size">{$t("common.file_size")}</th>
                    <th class="col-resolution">{$t("common.resolution")}</th>
                    <th class="col-bitrate">{$t("common.bitrate")}</th>
                    <th class="col-encoder">{$t("common.encoder")}</th>
                    <th class="col-status">{$t("common.status")}</th>
                    <th class="col-vmaf">{$t("common.vmaf_score")}</th>
                    <th class="col-progress">{$t("common.progress")}</th>
                </tr>
            </thead>
            <tbody>
                {#each files as file}
                    <tr>
                        <td class="col-name" title={file.path}>{file.name}</td>

                        <!-- Size -->
                        <td class="col-size">
                            {#if file.status === "Done" && file.outputInfo}
                                <div
                                    class="info-cell"
                                    title="Original: {formatSize(
                                        file.size,
                                    )} → New: {formatSize(
                                        file.outputInfo.size,
                                    )}"
                                >
                                    <span class="new-value"
                                        >{formatSize(
                                            file.outputInfo.size,
                                        )}</span
                                    >
                                    <span class="ratio-tag"
                                        >{getCompressionRatio(
                                            file.size,
                                            file.outputInfo.size,
                                        )}</span
                                    >
                                </div>
                            {:else}
                                {formatSize(file.size)}
                            {/if}
                        </td>

                        <!-- Resolution -->
                        <td class="col-resolution">
                            {#if file.status === "Done" && file.outputInfo}
                                <div
                                    class="info-cell"
                                    title="Original: {file.resolution} → New: {file
                                        .outputInfo.resolution}"
                                >
                                    <span class="new-value"
                                        >{file.outputInfo.resolution}</span
                                    >
                                </div>
                            {:else}
                                {file.resolution}
                            {/if}
                        </td>

                        <!-- Bitrate -->
                        <td class="col-bitrate">
                            {#if file.status === "Done" && file.outputInfo}
                                <div
                                    class="info-cell"
                                    title="Original: {file.bitrate} → New: {file
                                        .outputInfo.bitrate}"
                                >
                                    <span class="new-value"
                                        >{file.outputInfo.bitrate}</span
                                    >
                                </div>
                            {:else}
                                {file.bitrate}
                            {/if}
                        </td>

                        <!-- Encoder -->
                        <td class="col-encoder">
                            {#if file.status === "Done" && file.outputInfo}
                                <div
                                    class="info-cell"
                                    title="Original: {file.encoder} → New: {file
                                        .outputInfo.encoder}"
                                >
                                    <span class="new-value"
                                        >{file.outputInfo.encoder}</span
                                    >
                                </div>
                            {:else}
                                {file.encoder}
                            {/if}
                        </td>

                        <td class="col-status">
                            <div class="status-wrapper">
                                <span
                                    class="status-badge"
                                    class:status-scanning={file.status ===
                                        "Scanning"}
                                    class:status-pending={file.status ===
                                        "Pending"}
                                    class:status-processing={file.status.startsWith(
                                        "Processing",
                                    ) || file.status.startsWith("Found CRF")}
                                    class:status-searching={file.status.startsWith(
                                        "Searching CRF",
                                    )}
                                    class:status-done={file.status === "Done"}
                                    class:status-error={file.status === "Error"}
                                    class:status-cancelled={file.status ===
                                        "Cancelled"}
                                    class:status-skipped={file.status ===
                                        "Skipped"}
                                    class:status-waiting-for-vmaf={file.status ===
                                        "Waiting for VMAF"}
                                    class:status-evaluating={file.status ===
                                        "Evaluating"}
                                    class:is-long={isLongStatus(file.status)}
                                    title={file.status}
                                >
                                    <span class="status-text"
                                        >{file.status}</span
                                    >
                                </span>
                            </div>
                        </td>

                        <!-- VMAF Score -->
                        <td
                            class="col-vmaf"
                            class:clickable={canComputeVmaf(file)}
                            ondblclick={() => triggerVmaf(file)}
                        >
                            {#if file.status === "Evaluating"}
                                <div class="vmaf-cell">
                                    <span
                                        class="vmaf-evaluating"
                                        title="Evaluating VMAF Segments"
                                    >
                                        {file.vmafDetail
                                            ? file.vmafDetail.length
                                            : 0} / {file.vmafTotalSegments ||
                                            "?"}
                                    </span>
                                    {#if file.vmafDevice}
                                        <span
                                            class="vmaf-device"
                                            title="Computing using {file.vmafDevice}{file.vmafModel
                                                ? ` (${file.vmafModel})`
                                                : ''}"
                                            >{file.vmafDevice === "CUDA"
                                                ? "⚡"
                                                : "🖥️"}</span
                                        >
                                    {/if}
                                </div>
                            {:else if file.vmaf !== undefined && file.vmaf !== null}
                                <div class="vmaf-cell">
                                    <span
                                        class="vmaf-score"
                                        class:high-score={file.vmaf >= 93}
                                        class:med-score={file.vmaf >= 80 &&
                                            file.vmaf < 93}
                                        class:low-score={file.vmaf < 80}
                                        title={file.vmafDetail &&
                                        file.vmafDetail.length > 0
                                            ? `Avg: ${file.vmaf.toFixed(2)}\nSegments:\n${file.vmafDetail.map((s, i) => `#${i + 1}: ${s.toFixed(2)}`).join("\n")}`
                                            : undefined}
                                    >
                                        {file.vmaf.toFixed(2)}
                                    </span>
                                    {#if file.vmafDevice}
                                        <span
                                            class="vmaf-device"
                                            title="Computed using {file.vmafDevice}{file.vmafModel
                                                ? ` (${file.vmafModel})`
                                                : ''}"
                                            >{file.vmafDevice === "CUDA"
                                                ? "⚡"
                                                : "🖥️"}</span
                                        >
                                    {/if}
                                </div>
                            {:else}
                                <span class="vmaf-placeholder">-</span>
                            {/if}
                        </td>

                        <td class="col-progress">
                            <div class="progress-bar">
                                <div
                                    class="progress-fill"
                                    style="width: {file.progress}%"
                                ></div>
                            </div>
                            <span class="progress-text">{file.progress}%</span>
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>

<style>
    .file-list-container {
        flex: 1;
        background-color: #1e1e1e;
        border-radius: 12px;
        overflow-y: auto;
        border: 1px solid #333;
        min-height: 200px;
        box-shadow: inset 0 2px 10px rgba(0, 0, 0, 0.2);
        position: relative;
    }

    .empty-state {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        color: #666;
        font-size: 1.1rem;
    }

    .file-list {
        width: 100%;
        border-collapse: collapse;
        color: #ddd;
        font-size: 0.9rem;
        table-layout: fixed;
    }
    th,
    td {
        padding: 0.8rem 1rem;
        text-align: left;
        border-bottom: 1px solid #2d2d2d;
        overflow: hidden;
    }
    th {
        background-color: #252525;
        position: sticky;
        top: 0;
        font-weight: 600;
        color: #aaa;
        z-index: 10;
        box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    }
    tr:hover {
        background-color: #2a2a2a;
    }
    .col-name {
        width: auto;
        max-width: 250px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        font-weight: 500;
        color: #fff;
    }
    /* Fixed width columns */
    .col-size {
        width: 120px;
        min-width: 120px;
        max-width: 120px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .col-resolution {
        width: 90px;
        min-width: 90px;
        max-width: 90px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .col-bitrate {
        width: 90px;
        min-width: 90px;
        max-width: 90px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .col-encoder {
        width: 80px;
        min-width: 80px;
        max-width: 80px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    /* Status column with marquee scrolling */
    .col-status {
        width: 140px;
        min-width: 140px;
        max-width: 140px;
        text-align: center;
    }
    .col-vmaf {
        width: 70px;
        min-width: 70px;
        max-width: 70px;
        text-align: center;
    }
    .col-progress {
        width: 130px;
        min-width: 130px;
        max-width: 130px;
        white-space: nowrap;
    }
    .progress-bar {
        height: 6px;
        background-color: #333;
        border-radius: 3px;
        overflow: hidden;
        margin-right: 8px;
        display: inline-block;
        vertical-align: middle;
        width: 80px;
    }
    .progress-fill {
        height: 100%;
        background-color: #646cff;
        transition: width 0.3s ease;
    }
    .progress-text {
        font-size: 0.8rem;
        color: #888;
    }
    .status-wrapper {
        display: flex;
        justify-content: center;
        width: 100%;
    }
    .status-badge {
        display: inline-block;
        max-width: 160px;
        padding: 0.25rem 0.6rem;
        border-radius: 4px;
        font-size: 0.75rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        overflow: hidden;
    }
    .status-text {
        display: inline-block;
        white-space: nowrap;
    }
    /* Apply marquee animation automatically for long status text */
    .status-badge.is-long .status-text {
        animation: marquee-scroll 5s ease-in-out infinite;
    }
    @keyframes marquee-scroll {
        0%,
        15% {
            transform: translateX(0);
        }
        35%,
        50% {
            transform: translateX(calc(-100% + 140px));
        }
        65%,
        85% {
            transform: translateX(0);
        }
        100% {
            transform: translateX(0);
        }
    }
    .status-scanning {
        background-color: #4b3b00;
        color: #fbbf24;
    }
    .status-error {
        background-color: #450a0a;
        color: #f87171;
    }
    .status-pending {
        background-color: #333;
        color: #aaa;
    }
    .status-processing {
        background-color: #1e3a8a;
        color: #93c5fd;
    }
    .status-searching {
        background-color: #134e4a;
        color: #2dd4bf;
    }
    .status-done {
        background-color: #14532d;
        color: #86efac;
    }
    .status-cancelled {
        background-color: #451a03;
        color: #fbbf24;
    }
    .status-skipped {
        background-color: #1e293b;
        color: #94a3b8;
    }
    .status-waiting-for-vmaf {
        background-color: #1e3a8a;
        color: #93c5fd;
    }
    .status-evaluating {
        background-color: #3b0764;
        color: #d8b4fe;
    }

    .col-vmaf.clickable {
        cursor: pointer;
    }
    .col-vmaf.clickable:hover {
        background-color: rgba(100, 108, 255, 0.1);
    }

    /* New styles for updated info */
    .info-cell {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        cursor: help;
    }
    .new-value {
        color: #86efac; /* Green-ish */
        font-weight: 500;
    }
    .ratio-tag {
        font-size: 0.75rem;
        background-color: rgba(134, 239, 172, 0.1);
        color: #86efac;
        padding: 2px 4px;
        border-radius: 4px;
    }

    .vmaf-cell {
        display: flex;
        align-items: center;
        gap: 6px;
    }
    .vmaf-device {
        font-size: 0.8rem;
        cursor: help;
        opacity: 0.8;
    }
    .vmaf-score {
        font-weight: bold;
        font-family: monospace;
    }
    .high-score {
        color: #86efac; /* Green-400 */
    }
    .med-score {
        color: #fbbf24; /* Amber-400 */
    }
    .low-score {
        color: #f87171; /* Red-400 */
    }
    .vmaf-placeholder {
        color: #555;
    }
    .vmaf-evaluating {
        font-family: monospace;
        color: #fbbf24;
        font-weight: 500;
    }
</style>
