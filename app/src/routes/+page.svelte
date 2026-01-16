<script lang="ts">
  import PathSelector from "$lib/components/PathSelector.svelte";
  import FileList from "$lib/components/FileList.svelte";
  import SystemInfo from "$lib/components/SystemInfo.svelte";
  import Controls from "$lib/components/Controls.svelte";
  import SettingsModal from "$lib/components/SettingsModal.svelte";

  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";

  let inputPath = "";
  let outputPath = "";
  let files: any[] = [];
  let isScanning = false;
  let scanCounter = 0;
  let showSettings = false;

  // Drag state
  let isDragging = false;
  let activeZone: "input" | "output" | "both" | null = null;

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
          files = [...files];
        } catch (e) {
          console.error(`Failed to get metadata for ${file.path}:`, e);
          if (scanId === scanCounter) {
            files[i] = { ...file, status: "Error" };
            files = [...files];
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
      console.log("Active zone updated to:", activeZone);
    } else {
      activeZone = null;
    }
  }

  onMount(() => {
    console.log("Setting up Tauri file drop listeners...");

    let unlisten: (() => void) | undefined;
    const listenPromise = getCurrentWindow().onDragDropEvent((event) => {
      console.log("Drag drop event:", event);
      console.log("Full payload:", JSON.stringify(event.payload, null, 2));

      if (event.payload.type === "enter" || event.payload.type === "over") {
        console.log("File drop hover detected!");
        isDragging = true;

        // Check if payload contains position information
        const payload = event.payload as any;
        if (payload.position) {
          console.log("Position found:", payload.position);
          // Tauri provides PhysicalPosition, need to convert to logical pixels
          const logicalX = payload.position.x / window.devicePixelRatio;
          const logicalY = payload.position.y / window.devicePixelRatio;
          console.log(
            "Converted to logical:",
            logicalX,
            logicalY,
            "devicePixelRatio:",
            window.devicePixelRatio,
          );
          updateActiveZoneFromPoint(logicalX, logicalY);
        } else if (payload.x !== undefined && payload.y !== undefined) {
          console.log("Coordinates found:", payload.x, payload.y);
          const logicalX = payload.x / window.devicePixelRatio;
          const logicalY = payload.y / window.devicePixelRatio;
          updateActiveZoneFromPoint(logicalX, logicalY);
        } else {
          console.log("No position data in payload");
        }
      } else if (event.payload.type === "drop") {
        console.log("File drop detected!", event.payload.paths);
        const paths = event.payload.paths;
        console.log("Active zone at drop:", activeZone);

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

        // Reset
        isDragging = false;
        activeZone = null;
      } else if (event.payload.type === "leave") {
        console.log("File drop cancelled");
        isDragging = false;
        activeZone = null;
      }
    });

    listenPromise.then((u) => {
      unlisten = u;
    });

    // Listen to global mousemove to track mouse position during drag
    // (dragover doesn't work with Tauri file drop on Windows)
    const handleMouseMove = (e: MouseEvent) => {
      if (isDragging) {
        updateActiveZoneFromPoint(e.clientX, e.clientY);
      }
    };

    window.addEventListener("mousemove", handleMouseMove);

    console.log("Drag drop listener registered");

    return () => {
      if (unlisten) {
        unlisten();
      } else {
        listenPromise.then((u) => u());
      }
      window.removeEventListener("mousemove", handleMouseMove);
    };
  });

  // HTML5 drag event handlers for zone detection (kept for fallback)
  function handleZoneDragEnter(zone: "input" | "output" | "both") {
    console.log("Zone drag enter:", zone);
    activeZone = zone;
  }

  function handleZoneDragLeave(event: DragEvent) {
    // Only clear if we're leaving the zone entirely (not entering a child)
    const target = event.currentTarget as HTMLElement;
    const relatedTarget = event.relatedTarget as HTMLElement;

    if (!target.contains(relatedTarget)) {
      activeZone = null;
    }
  }

  function handleStart() {
    console.log("Start clicked");
  }
  function handlePause() {
    console.log("Pause clicked");
  }
  function handleCancel() {
    console.log("Cancel clicked");
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
    <SystemInfo />
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
        on:dragenter={() => handleZoneDragEnter("input")}
        on:dragleave={handleZoneDragLeave}
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
        on:dragenter={() => handleZoneDragEnter("both")}
        on:dragleave={handleZoneDragLeave}
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
        on:dragenter={() => handleZoneDragEnter("output")}
        on:dragleave={handleZoneDragLeave}
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
