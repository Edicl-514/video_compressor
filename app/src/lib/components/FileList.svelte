<script lang="ts">
    export let files: any[] = []; // Using any[] temporarily, or define interface

    interface VideoInfo {
        name: string;
        path: string;
        size: number;
        resolution: string;
        bitrate: string;
        encoder: string;
        status: string;
        progress: number;
    }

    function formatSize(bytes: number): string {
        if (bytes === 0) return "0 B";
        const k = 1024;
        const sizes = ["B", "KB", "MB", "GB", "TB"];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
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
                    <th class="progress-col">Progress</th>
                </tr>
            </thead>
            <tbody>
                {#each files as file}
                    <tr>
                        <td class="col-name" title={file.path}>{file.name}</td>
                        <td>{formatSize(file.size)}</td>
                        <td>{file.resolution}</td>
                        <td>{file.bitrate}</td>
                        <td>{file.encoder}</td>
                        <td>
                            <span
                                class="status-badge status-{file.status.toLowerCase()}"
                            >
                                {file.status}
                            </span>
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
</style>
