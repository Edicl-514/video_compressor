<script lang="ts">
    import type { VideoInfo } from "$lib/types";
    export let files: VideoInfo[] = [];

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
</script>

<div class="file-list-container">
    {#if files.length === 0}
        <div class="empty-state">
            <p>No video files found. Select a folder to scan.</p>
        </div>
    {:else}
        <table class="file-list">
            <thead>
                <tr>
                    <th>File Name</th>
                    <th>Size</th>
                    <th>Resolution</th>
                    <th>Bitrate</th>
                    <th>Encoder</th>
                    <th>Status</th>
                    <th>VMAF</th>
                    <th class="progress-col">Progress</th>
                </tr>
            </thead>
            <tbody>
                {#each files as file}
                    <tr>
                        <td class="col-name" title={file.path}>{file.name}</td>

                        <!-- Size -->
                        <td>
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
                        <td>
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
                        <td>
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
                        <td>
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

                        <td>
                            <span
                                class="status-badge status-{file.status.toLowerCase()}"
                            >
                                {file.status}
                            </span>
                        </td>

                        <!-- VMAF Score -->
                        <td>
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
                                            title="Computing using {file.vmafDevice}"
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
                                            title="Computed using {file.vmafDevice}"
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

                        <td>
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
    }
    th,
    td {
        padding: 0.8rem 1rem;
        text-align: left;
        border-bottom: 1px solid #2d2d2d;
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
        max-width: 250px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        font-weight: 500;
        color: #fff;
    }
    .progress-col {
        width: 150px;
    }
    .progress-bar {
        width: 100%;
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
    .status-badge {
        padding: 0.25rem 0.6rem;
        border-radius: 4px;
        font-size: 0.75rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
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
    .status-done {
        background-color: #14532d;
        color: #86efac;
    }
    .status-cancelled {
        background-color: #451a03;
        color: #fbbf24;
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
