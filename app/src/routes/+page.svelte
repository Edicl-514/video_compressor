<script lang="ts">
  import PathSelector from "$lib/components/PathSelector.svelte";
  import FileList from "$lib/components/FileList.svelte";
  import SystemInfo from "$lib/components/SystemInfo.svelte";
  import Controls from "$lib/components/Controls.svelte";
  import SettingsModal from "$lib/components/SettingsModal.svelte";

  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { onMount, onDestroy } from "svelte";
  import type { VideoInfo } from "$lib/types";
  import { settingsStore } from "$lib/stores/settings.svelte";

  let inputPath = $state("");
  let outputPath = $state("");
  let files = $state<VideoInfo[]>([]);
  let isScanning = $state(false);
  let isProcessing = $state(false);
  let shouldStop = $state(false);
  let scanCounter = $state(0);
  let showSettings = $state(false);

  let processingStats = $derived.by(() => {
    // Active processing stats
    const processing = files.filter(
      (f) =>
        f.status === "Processing" &&
        (f.speed !== undefined || f.bitrateKbps !== undefined),
    );

    let totalSpeed = 0;
    let totalBitrate = 0;

    if (processing.length > 0) {
      for (const f of processing) {
        totalSpeed += f.speed || 0;
        totalBitrate += f.bitrateKbps || 0;
      }
    }

    // Total Progress Calculation
    const totalCount = files.length;
    let progressSum = 0;

    if (totalCount > 0) {
      for (const f of files) {
        if (f.status === "Done") {
          progressSum += 1;
        } else if (f.status === "Processing") {
          progressSum += (f.progress || 0) / 100;
        }
        // Error, Pending, Cancelled count as 0 progress for now?
        // Or should Error count as done? Usually Error stops the flow for that file, so technically it's "processed" (failed).
        // But let's stick to the user's formula: "Finished + Processing".
        // If "Error" means "Finished with error", maybe we should count it?
        // Let's assume strict "Done" for now.
      }
    }

    const totalProgress = totalCount > 0 ? (progressSum / totalCount) * 100 : 0;

    // Only return null if there are absolutely no files to work on?
    // Or just return the object with 0s.
    // If files.length is 0, we can return null to hide everything?
    if (totalCount === 0) return null;

    return {
      speed: totalSpeed,
      bitrate: processing.length > 0 ? totalBitrate / processing.length : 0,
      totalProgress,
    };
  });

  // Drag state
  let isDragging = $state(false);
  let activeZone = $state<"input" | "output" | "both" | null>(null);

  let unlistenProgress: (() => void) | null = null;

  async function scanVideos() {
    if (!inputPath) return;
    scanCounter++;
    const currentScanId = scanCounter;
    isScanning = true;
    try {
      console.log("Scanning directory:", inputPath);
      const result: any = await invoke("scan_directory", { path: inputPath });
      files = result.videos || [];
      console.log("Found files:", files.length);

      // Start fetching metadata in the background
      fetchMetadata(currentScanId);

      if (result.errors && result.errors.length > 0) {
        console.warn("Scan errors:", result.errors);
      }
    } catch (e) {
      console.error("Failed to scan videos:", e);
    } finally {
      if (currentScanId === scanCounter) {
        isScanning = false;
      }
    }
  }

  async function fetchMetadata(scanId: number) {
    const threads = settingsStore.value.ffprobeThreads || 4;
    const queue = [...files.keys()].filter(
      (i) => files[i].status === "Scanning",
    );

    async function worker() {
      while (queue.length > 0) {
        if (scanId !== scanCounter) return;
        const i = queue.shift();
        if (i === undefined) break;

        const file = files[i];
        try {
          const info: any = await invoke("get_video_metadata", {
            path: file.path,
          });

          if (scanId !== scanCounter) return;
          files[i] = info;
        } catch (e) {
          console.error(`Failed to get metadata for ${file.path}:`, e);
          if (scanId === scanCounter) {
            files[i].status = "Error";
          }
        }
      }
    }

    // Start workers
    const workers = Array(Math.min(threads, queue.length))
      .fill(null)
      .map(() => worker());

    await Promise.all(workers);
  }

  function handleInputChange(event: CustomEvent<string>) {
    inputPath = event.detail;
    scanVideos();
  }

  function handleOutputChange(event: CustomEvent<string>) {
    outputPath = event.detail;
  }

  // --- Drag and Drop Logic ---

  // Track mouse position during drag to determine which zone is active
  function updateActiveZoneFromPoint(clientX: number, clientY: number) {
    const elem = document.elementFromPoint(clientX, clientY);
    if (!elem) {
      activeZone = null;
      return;
    }

    const zoneElement = elem.closest(".drop-zone");
    if (zoneElement) {
      if (zoneElement.classList.contains("zone-input")) {
        activeZone = "input";
      } else if (zoneElement.classList.contains("zone-output")) {
        activeZone = "output";
      } else if (zoneElement.classList.contains("zone-both")) {
        activeZone = "both";
      } else {
        activeZone = null;
      }
      // console.log("Active zone updated to:", activeZone);
    } else {
      activeZone = null;
    }
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;
    let unlistenMouseMove: (() => void) | undefined;

    const setup = async () => {
      // Listen for progress
      unlistenProgress = await listen("video-progress", (event: any) => {
        const {
          path,
          progress,
          status,
          outputInfo,
          bitrateKbps,
          speed,
          bitrate_kbps,
          output_info,
        } = event.payload;
        const index = files.findIndex((f) => f.path === path);
        if (index !== -1) {
          // Don't overwrite 'Cancelled' status with stale backend updates
          if (files[index].status === "Cancelled" && status === "Processing") {
            console.log(
              `Ignoring stale 'Processing' update for cancelled file: ${path}`,
            );
            return;
          }

          // Map both camelCase and snake_case to be safe, but use sprawled object for reactivity
          files[index] = {
            ...files[index],
            progress,
            status,
            speed: speed ?? files[index].speed,
            bitrateKbps:
              bitrateKbps ?? bitrate_kbps ?? files[index].bitrateKbps,
            outputInfo: outputInfo ?? output_info ?? files[index].outputInfo,
          };
          console.log(`Update ${path}: ${progress}% ${status}`);
        }
      });

      console.log("Setting up Tauri file drop listeners...");

      const u = await getCurrentWindow().onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          isDragging = true;

          const payload = event.payload as any;
          if (payload.position) {
            const logicalX = payload.position.x / window.devicePixelRatio;
            const logicalY = payload.position.y / window.devicePixelRatio;
            updateActiveZoneFromPoint(logicalX, logicalY);
          } else if (payload.x !== undefined && payload.y !== undefined) {
            const logicalX = payload.x / window.devicePixelRatio;
            const logicalY = payload.y / window.devicePixelRatio;
            updateActiveZoneFromPoint(logicalX, logicalY);
          }
        } else if (event.payload.type === "drop") {
          console.log("File drop detected!", event.payload.paths);
          const paths = event.payload.paths;

          if (paths && paths.length > 0) {
            const path = paths[0];

            if (activeZone === "input") {
              inputPath = path;
              scanVideos();
            } else if (activeZone === "output") {
              outputPath = path;
            } else if (activeZone === "both") {
              inputPath = path;
              outputPath = path;
              scanVideos();
            }
          }

          isDragging = false;
          activeZone = null;
        } else if (event.payload.type === "leave") {
          isDragging = false;
          activeZone = null;
        }
      });
      unlisten = u;

      // Listen to global mousemove to track mouse position during drag
      const handleMouseMove = (e: MouseEvent) => {
        if (isDragging) {
          updateActiveZoneFromPoint(e.clientX, e.clientY);
        }
      };

      window.addEventListener("mousemove", handleMouseMove);
      unlistenMouseMove = () =>
        window.removeEventListener("mousemove", handleMouseMove);

      console.log("Drag drop listener registered");
    };

    setup();

    return () => {
      if (unlisten) unlisten();
      if (unlistenProgress) unlistenProgress();
      if (unlistenMouseMove) unlistenMouseMove();
    };
  });

  // HTML5 drag event handlers for zone detection (kept for fallback)
  function handleZoneDragEnter(zone: "input" | "output" | "both") {
    activeZone = zone;
  }

  function handleZoneDragLeave(event: DragEvent) {
    const target = event.currentTarget as HTMLElement;
    const relatedTarget = event.relatedTarget as HTMLElement;

    if (!target.contains(relatedTarget)) {
      activeZone = null;
    }
  }

  function getOutputFilePath(
    input: string,
    outputDir: string,
    format: string,
    suffix: string,
    inputRoot: string, // Add inputRoot parameter
  ): string {
    // Normalize paths for processing
    const normalize = (p: string) => p.replace(/\\/g, "/");
    const normInput = normalize(input);
    const normInputRoot = normalize(inputRoot).replace(/\/$/, ""); // Remove trailing slash if present
    const normOutputDir = normalize(outputDir).replace(/\/$/, "");

    const separator = input.includes("\\") ? "\\" : "/";
    const fileName = input.split(separator).pop() || "unknown";
    const nameNoExt =
      fileName.substring(0, fileName.lastIndexOf(".")) || fileName;

    // Determine target directory
    let targetDir = "";

    // Check if we are outputting to a different directory structure
    // If outputDir is set and different from inputRoot
    const isDifferentDir =
      normOutputDir &&
      normOutputDir.toLowerCase() !== normInputRoot.toLowerCase();

    if (
      isDifferentDir &&
      normInput.toLowerCase().startsWith(normInputRoot.toLowerCase())
    ) {
      // Calculate relative path from input root
      // input: /root/subdir/file.mp4
      // root: /root
      // relative: /subdir/file.mp4 (we want /subdir part)

      const relativePath = normInput.substring(normInputRoot.length); // starts with / usually
      // relativePath includes filename: /subdir/file.mp4

      const relativeDir = relativePath.substring(
        0,
        relativePath.lastIndexOf("/"),
      );
      // relativeDir: /subdir (or empty string if at root)

      targetDir = normOutputDir + relativeDir;
    } else {
      // Same directory mode (or input is not under inputRoot? shouldn't happen with scan)
      // Use the file's current directory
      targetDir = input.substring(0, input.lastIndexOf(separator));

      // If outputDir is set but same as inputRoot, technically we are in "Same Dir" mode for the root,
      // but for subdirectories, we just keep them where they are relative to input root?
      // Actually, if inputRoot == outputDir, we just want to save in place (subdir structure is implicitly preserved).
      // So targetDir = input's dir is correct.
    }

    // Determine suffix
    let finalSuffix = "";

    // Logic for suffix:
    // If saving to the SAME folder as original file, and suffix is empty -> Overwrite warning case.
    // If saving to DIFFERENT folder, typically no suffix needed unless user wants one.

    // We compare targetDir with input's dir
    const normInDir = normalize(
      input.substring(0, input.lastIndexOf(separator)),
    ).toLowerCase();
    const normTargetDir = normalize(targetDir).toLowerCase();

    if (normInDir === normTargetDir) {
      finalSuffix = suffix; // If empty, it means overwrite
    } else {
      finalSuffix = suffix;
    }

    let outName = `${nameNoExt}${finalSuffix}.${format}`;

    // Handle separator for final string reconstruction
    // We used forward slashes for calculation, better stick to it or convert back?
    // Windows accepts forward slashes in API usually, but let's be consistent with OS if possible.
    // Actually, simple concatenation with "/" is mostly fine in JS/Tauri as Rust handles it.

    if (targetDir && !targetDir.endsWith("/")) {
      targetDir += "/";
    }

    return targetDir + outName;
  }

  function normalizePath(p: string): string {
    return p.replace(/\\/g, "/").toLowerCase().replace(/\/$/, "");
  }

  async function handleStart() {
    if (isProcessing) return;

    // Check if there are any files to process
    const pendingFiles = files.filter(
      (f) => f.status !== "Done" && f.status !== "Error",
    );
    if (pendingFiles.length === 0) return;

    const settings = settingsStore.value;

    // Overwrite Check
    let effectiveOut = outputPath;
    if (!effectiveOut && inputPath) {
      effectiveOut = inputPath;
    }

    const sameDir = normalizePath(effectiveOut) === normalizePath(inputPath);
    const emptySuffix = !settings.suffix;

    if (sameDir && emptySuffix) {
      const confirmed = await ask(
        "The source files will be overwritten. Continue?",
        {
          title: "Overwrite Warning",
          kind: "warning",
        },
      );

      if (!confirmed) return;
    }

    console.log("Start clicked");
    isProcessing = true;
    shouldStop = false;

    // Queue of indices to process
    const queue = files
      .map((f, i) => ({ f, i }))
      .filter((item) => item.f.status !== "Done" && item.f.status !== "Error")
      .map((item) => item.i);

    const concurrency = settings.ffmpegThreads || 1;

    async function worker() {
      while (queue.length > 0) {
        if (shouldStop) break;

        const i = queue.shift();
        if (i === undefined) break;

        const file = files[i];

        // Prepare output path
        const outPath = getOutputFilePath(
          file.path,
          outputPath,
          settings.targetFormat,
          settings.suffix,
          inputPath,
        );

        // Update status to Processing
        files[i].status = "Processing";
        files[i].progress = 0;
        files = [...files];

        try {
          // Check stop flag again before starting expensive operation
          if (shouldStop) {
            files[i].status = "Pending"; // Revert status if stopped right before start
            files = [...files];
            break;
          }

          await invoke("start_processing", {
            inputPath: file.path,
            outputPath: outPath,
            config: settingsStore.value,
            durationSec: file.durationSec || 0.0,
          });
        } catch (e: any) {
          console.error("Processing error:", e);
          // Only update to Error if it wasn't already marked as Cancelled
          if (files[i] && files[i].status !== "Cancelled") {
            files[i].status = "Error";
            files = [...files];
          }
        }
      }
    }

    const workers = Array(Math.min(concurrency, queue.length))
      .fill(null)
      .map(() => worker());

    await Promise.all(workers);

    isProcessing = false;
  }

  function handlePause() {
    console.log("Pause clicked (Suspend/Pause not fully implemented yet)");
  }

  async function handleCancel() {
    console.log("Cancel clicked");
    shouldStop = true;

    // Find currently processing files and send cancel command
    // Use a loop over index to update state correctly
    for (let i = 0; i < files.length; i++) {
      if (files[i].status === "Processing") {
        const path = files[i].path;
        // Immediate UI feedback
        files[i] = { ...files[i], status: "Cancelled", progress: 0 };

        try {
          await invoke("cancel_processing", { path });
        } catch (e) {
          console.error(`Failed to cancel ${path}:`, e);
        }
      }
    }
  }

  function handleSettings() {
    console.log("Settings clicked");
    showSettings = true;
  }
</script>

<main class="container">
  <div class="header">
    <h1>Video Compressor</h1>
  </div>

  <section class="path-section">
    <PathSelector
      bind:inputPath
      bind:outputPath
      on:inputChange={handleInputChange}
      on:outputChange={handleOutputChange}
    />
  </section>

  <section class="content-area">
    <FileList {files} />
    {#if isScanning}
      <div class="loading-overlay">Scanning...</div>
    {/if}
  </section>

  <section class="bottom-panel">
    <SystemInfo
      processingSpeed={processingStats?.speed}
      processingBitrate={processingStats?.bitrate}
      totalProgress={processingStats?.totalProgress}
    />
    <Controls
      on:start={handleStart}
      on:pause={handlePause}
      on:cancel={handleCancel}
      on:settings={handleSettings}
    />
  </section>

  {#if isDragging}
    <div class="drag-overlay">
      <div
        class="drop-zone zone-input {activeZone === 'input' ? 'active' : ''}"
        role="button"
        tabindex="0"
        ondragenter={() => handleZoneDragEnter("input")}
        ondragleave={handleZoneDragLeave}
      >
        <div class="zone-content">
          <span class="icon">📂</span>
          <span>Set Input Folder</span>
        </div>
      </div>

      <div
        class="drop-zone zone-both {activeZone === 'both' ? 'active' : ''}"
        role="button"
        tabindex="0"
        ondragenter={() => handleZoneDragEnter("both")}
        ondragleave={handleZoneDragLeave}
      >
        <div class="zone-content">
          <span class="icon">✨</span>
          <span>Set Both (Input & Output)</span>
        </div>
      </div>

      <div
        class="drop-zone zone-output {activeZone === 'output' ? 'active' : ''}"
        role="button"
        tabindex="0"
        ondragenter={() => handleZoneDragEnter("output")}
        ondragleave={handleZoneDragLeave}
      >
        <div class="zone-content">
          <span class="icon">💾</span>
          <span>Set Output Folder</span>
        </div>
      </div>
    </div>
  {/if}
  {#if showSettings}
    <SettingsModal close={() => (showSettings = false)} />
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family:
      "Inter",
      system-ui,
      -apple-system,
      sans-serif;
    background-color: #202020;
    color: #ffffff;
    height: 100vh;
    overflow: hidden;
  }

  .container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 1.5rem;
    gap: 1.5rem;
    box-sizing: border-box;
    max-width: 1200px;
    margin: 0 auto;
    position: relative;
  }

  .header h1 {
    font-size: 1.5rem;
    font-weight: 700;
    margin: 0;
    color: #f0f0f0;
    letter-spacing: -0.02em;
  }

  .content-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    position: relative;
  }

  .loading-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.2rem;
    border-radius: 12px;
    backdrop-filter: blur(2px);
    z-index: 10;
  }

  .bottom-panel {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    background-color: #252525;
    padding: 1rem;
    border-radius: 12px;
    box-shadow: 0 -4px 10px rgba(0, 0, 0, 0.2);
    border: 1px solid #333;
  }

  @media (min-width: 800px) {
    .bottom-panel {
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
    }
  }

  .drag-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.9);
    z-index: 9999;
    display: flex;
    padding: 2rem;
    gap: 1.5rem;
    backdrop-filter: blur(4px);
    /* Ensure it captures events so elementFromPoint works on children? 
         其实 elementFromPoint works based on visual layout. 
         Pointer events need to be active.
      */
  }

  .drop-zone {
    flex: 1;
    border: 3px dashed #555;
    border-radius: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.1s;
    background-color: rgba(255, 255, 255, 0.05);
  }

  .drop-zone.active {
    transform: scale(1.02);
    box-shadow: 0 0 40px rgba(0, 0, 0, 0.6);
  }

  .zone-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    pointer-events: none;
  }

  .icon {
    font-size: 3rem;
  }

  .zone-input.active {
    border-color: #34d399;
    background-color: rgba(52, 211, 153, 0.15);
    color: #34d399;
  }
  .zone-output.active {
    border-color: #f472b6;
    background-color: rgba(244, 114, 182, 0.15);
    color: #f472b6;
  }
  .zone-both.active {
    border-color: #60a5fa;
    background-color: rgba(96, 165, 250, 0.15);
    color: #60a5fa;
  }
</style>
