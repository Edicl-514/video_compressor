<script lang="ts">
    import { _ as t } from "svelte-i18n";
    import { invoke } from "@tauri-apps/api/core";
    import { listen, type UnlistenFn } from '@tauri-apps/api/event';

    let { onComplete } = $props<{
        onComplete: (report: any) => void;
    }>();

    type DetectionStatus = {
        type: 'video' | 'audio';
        name: string;
        value: string;
        available: boolean;
    };

    let detectionLog = $state<DetectionStatus[]>([]);
    let isDetecting = $state<boolean>(true);
    let currentPhase = $state<string>('');
    let videoCount = $state<number>(0);
    let audioCount = $state<number>(0);
    let detectionReport: any = null;

    let unlisten: UnlistenFn | null = null;
    let logContainer: HTMLDivElement;

    $effect(() => {
        startDetection();
        return () => {
            if (unlisten) {
                unlisten();
            }
        };
    });

    // 自动滚动到底部
    $effect(() => {
        if (detectionLog.length > 0 && logContainer) {
            logContainer.scrollTop = logContainer.scrollHeight;
        }
    });

    async function startDetection() {
        try {
            // 监听检测事件
            unlisten = await listen<DetectionStatus>('encoder-detection-progress', (event) => {
                const status = event.payload;
                detectionLog.push(status);
                
                if (status.available) {
                    if (status.type === 'video') {
                        videoCount++;
                    } else {
                        audioCount++;
                    }
                }
            });

            // 调用检测函数
            const report = await invoke("detect_encoders");
            
            // 检测完成，保存报告但不自动关闭
            isDetecting = false;
            detectionReport = report;
            
        } catch (e) {
            console.error("Detection failed:", e);
            isDetecting = false;
            detectionReport = { video: [], audio: [] };
        }
    }

    function closeModal() {
        if (detectionReport) {
            onComplete(detectionReport);
        } else {
            onComplete({ video: [], audio: [] });
        }
    }
</script>

<div class="modal-overlay" role="dialog" aria-label="Encoder Detection">
    <div class="modal-container detection-modal">
        <div class="modal-header">
            <h2>{$t("common.detecting_encoders_title")}</h2>
        </div>

        <div class="modal-body">
            <div class="detection-stats">
                <div class="stat-item">
                    <span class="stat-label">{$t("common.video_encoders")}:</span>
                    <span class="stat-value">{videoCount}</span>
                </div>
                <div class="stat-item">
                    <span class="stat-label">{$t("common.audio_encoders")}:</span>
                    <span class="stat-value">{audioCount}</span>
                </div>
            </div>

            <div class="detection-log" bind:this={logContainer}>
                {#each detectionLog as log}
                    <div class="log-entry" class:available={log.available} class:unavailable={!log.available}>
                        <span class="log-type">
                            {log.type === 'video' ? '📹' : '🔊'}
                        </span>
                        <span class="log-name">{log.name}</span>
                        <span class="log-status">
                            {log.available 
                                ? $t("common.encoder_available") 
                                : $t("common.encoder_unavailable")}
                        </span>
                    </div>
                {/each}
            </div>

            {#if !isDetecting}
                <div class="completion-message">
                    ✓ {$t("common.detection_finished")}
                </div>
            {/if}
        </div>

        {#if !isDetecting}
            <div class="modal-footer">
                <button class="close-btn" onclick={closeModal}>
                    {$t("common.close")}
                </button>
            </div>
        {/if}
    </div>
</div>

<style>
    .modal-overlay {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        justify-content: center;
        align-items: center;
        z-index: 2000;
    }

    .detection-modal {
        background: var(--bg-secondary, #2a2a2a);
        border-radius: 8px;
        width: 90%;
        max-width: 600px;
        max-height: 80vh;
        display: flex;
        flex-direction: column;
        box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
    }

    .modal-header {
        padding: 1.5rem;
        border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .modal-header h2 {
        margin: 0;
        font-size: 1.25rem;
        color: var(--text-primary, #ffffff);
    }

    .modal-body {
        padding: 1.5rem;
        overflow-y: auto;
        flex: 1;
    }

    .detection-stats {
        display: flex;
        gap: 2rem;
        margin-bottom: 1.5rem;
        padding: 1rem;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 6px;
    }

    .stat-item {
        display: flex;
        gap: 0.5rem;
        align-items: center;
    }

    .stat-label {
        color: var(--text-secondary, #aaaaaa);
        font-size: 0.9rem;
    }

    .stat-value {
        color: var(--accent-color, #4a9eff);
        font-size: 1.5rem;
        font-weight: bold;
    }

    .detection-log {
        max-height: 400px;
        overflow-y: auto;
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 6px;
        padding: 0.5rem;
        background: rgba(0, 0, 0, 0.2);
    }

    .log-entry {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.5rem;
        margin-bottom: 0.25rem;
        border-radius: 4px;
        font-size: 0.9rem;
        transition: background 0.2s;
    }

    .log-entry.available {
        background: rgba(76, 175, 80, 0.1);
    }

    .log-entry.unavailable {
        background: rgba(244, 67, 54, 0.05);
        opacity: 0.6;
    }

    .log-type {
        font-size: 1.2rem;
        flex-shrink: 0;
    }

    .log-name {
        flex: 1;
        color: var(--text-primary, #ffffff);
        font-family: monospace;
    }

    .log-status {
        color: var(--text-secondary, #aaaaaa);
        font-size: 0.85rem;
    }

    .log-entry.available .log-status {
        color: #4caf50;
    }

    .log-entry.unavailable .log-status {
        color: #f44336;
    }

    .completion-message {
        margin-top: 1rem;
        padding: 1rem;
        background: rgba(76, 175, 80, 0.2);
        border: 1px solid rgba(76, 175, 80, 0.4);
        border-radius: 6px;
        color: #4caf50;
        text-align: center;
        font-weight: 500;
    }

    .modal-footer {
        padding: 1rem 1.5rem;
        border-top: 1px solid rgba(255, 255, 255, 0.1);
        display: flex;
        justify-content: flex-end;
    }

    .close-btn {
        padding: 0.5rem 1.5rem;
        background: var(--accent-color, #4a9eff);
        color: white;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        font-size: 0.95rem;
        font-weight: 500;
        transition: all 0.2s;
    }

    .close-btn:hover {
        background: var(--accent-hover, #3a8eef);
        transform: translateY(-1px);
        box-shadow: 0 2px 8px rgba(74, 158, 255, 0.3);
    }

    .close-btn:active {
        transform: translateY(0);
    }
</style>
