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
  import { sortStore } from "$lib/stores/sortStore.svelte";
  import { t } from "svelte-i18n";

  let inputPath = $state("");
  let outputPath = $state("");
  let files = $state<VideoInfo[]>([]);
  let isScanning = $state(false);
  let isProcessing = $state(false);
  let isPaused = $state(false);
  let shouldStop = $state(false);
  let scanCounter = $state(0);
  let showSettings = $state(false);

  let processingStats = $derived.by(() => {
    // Active processing stats - check for all processing-related statuses
    const isProcessingStatus = (status: string) =>
      status === "Processing" ||
      status === "Processing (Pass 1/2)" ||
      status === "Processing (Pass 2/2)" ||
      status.startsWith("Searching CRF") ||
      status.startsWith("Found CRF");

    const processing = files.filter(
      (f) =>
        isProcessingStatus(f.status) &&
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
        if (
          f.status === "Done" ||
          f.status === "Evaluating" ||
          f.status === "Waiting for VMAF" ||
          f.status === "Skipped"
        ) {
          progressSum += 1;
        } else if (f.status === "Processing (Pass 1/2)") {
          // 2-pass mode: Pass 1 sends progress 0-50, use directly
          progressSum += (f.progress || 0) / 100;
        } else if (f.status === "Processing (Pass 2/2)") {
          // 2-pass mode: Pass 2 sends progress 50-100, use directly
          progressSum += (f.progress || 0) / 100;
        } else if (f.status.startsWith("Searching CRF")) {
          // Target VMAF mode: search phase sends progress 0-50
          // Backend sends progress 0-50 directly, so use progress/100
          progressSum += (f.progress || 0) / 100;
        } else if (f.status.startsWith("Found CRF")) {
          // Target VMAF mode: compression phase sends progress 50-100
          // Backend sends progress 50-100 directly, so use progress/100
          progressSum += (f.progress || 0) / 100;
        } else if (f.status.startsWith("Processing")) {
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

  // Status whitelist for "no active task" - if all files have these statuses, no task is in progress
  const IDLE_STATUSES = [
    "pending",
    "cancelled",
    "done",
    "error",
    "skipped",
    "scanning",
  ];

  // Check if any task is currently in progress (any file has a status NOT in the whitelist)
  let hasActiveTask = $derived.by(() => {
    return files.some((f) => !IDLE_STATUSES.includes(f.status.toLowerCase()));
  });

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
    outputPath = ""; // Reset output path when input changes
    scanVideos();
  }

  function handleOutputChange(event: CustomEvent<string>) {
    outputPath = event.detail;
  }

  function handleInputVideosChange(event: CustomEvent<string[]>) {
    const paths = event.detail;
    if (paths && paths.length > 0) {
      outputPath = ""; // Reset output path when input videos are selected
      handleMultiDrop(paths, "input");
    }
  }

  // Handle dropping multiple folders or video files
  async function handleMultiDrop(
    paths: string[],
    zone: "input" | "output" | "both",
  ) {
    if (hasActiveTask) {
      console.log("Drop rejected: active task in progress");
      return;
    }

    // Categorize paths
    const categorization: any = await invoke("categorize_paths", { paths });
    console.log("Path categorization:", categorization);

    // If all paths are invalid, do nothing
    if (
      categorization.videos.length === 0 &&
      categorization.directories.length === 0
    ) {
      console.warn("No valid videos or directories found in drop");
      return;
    }

    // Single folder drop to input - use existing logic
    if (
      categorization.directories.length === 1 &&
      categorization.videos.length === 0 &&
      zone === "input"
    ) {
      inputPath = categorization.directories[0];
      outputPath = ""; // Reset output path when input folder is selected
      scanVideos();
      return;
    }

    // Single folder drop to output - set output path
    if (
      categorization.directories.length === 1 &&
      categorization.videos.length === 0 &&
      zone === "output"
    ) {
      outputPath = categorization.directories[0];
      return;
    }

    // Single folder drop to both - use existing logic
    if (
      categorization.directories.length === 1 &&
      categorization.videos.length === 0 &&
      zone === "both"
    ) {
      inputPath = categorization.directories[0];
      outputPath = categorization.directories[0];
      scanVideos();
      return;
    }

    // Multiple folders/videos - cannot drop to output only
    if (zone === "output") {
      console.warn("Multiple paths cannot be dropped to output-only zone");
      return;
    }

    // Single video file handling
    if (
      categorization.videos.length === 1 &&
      categorization.directories.length === 0
    ) {
      const videoPath = categorization.videos[0];
      const separator = videoPath.includes("\\") ? "\\" : "/";
      const videoDir = videoPath.substring(0, videoPath.lastIndexOf(separator));

      // For multiple paths or videos: scan all paths and add to queue
      scanCounter++;
      const currentScanId = scanCounter;
      isScanning = true;

      try {
        const result: any = await invoke("scan_multiple_paths", {
          paths: [videoPath],
        });
        const newVideos = result.videos || [];

        if (zone === "both") {
          // Single video to both: input shows path, output shows parent dir
          inputPath = videoPath;
          outputPath = videoDir;
          // Mark with originalOutputDir for processing
          for (const video of newVideos) {
            video.originalOutputDir = videoDir;
          }
        } else {
          // Single video to input: just show the path
          inputPath = videoPath;
          outputPath = ""; // Reset output path when input video is selected
        }

        files = newVideos;
        fetchMetadata(currentScanId);
      } catch (e) {
        console.error("Failed to scan video:", e);
      } finally {
        if (currentScanId === scanCounter) {
          isScanning = false;
        }
      }
      return;
    }

    // For multiple paths or videos: scan all paths and add to queue
    scanCounter++;
    const currentScanId = scanCounter;
    isScanning = true;

    try {
      // Collect all paths to scan
      const allPaths = [
        ...categorization.directories,
        ...categorization.videos,
      ];

      const result: any = await invoke("scan_multiple_paths", {
        paths: allPaths,
      });
      const newVideos = result.videos || [];

      // Build display message for input path
      const folderCount = categorization.directories.length;
      const videoCount = categorization.videos.length;
      let inputDisplayParts: string[] = [];
      if (folderCount > 0) {
        inputDisplayParts.push(
          $t("common.folder_count", { values: { count: folderCount } }),
        );
      }
      if (videoCount > 0) {
        inputDisplayParts.push(
          $t("common.video_count", { values: { count: videoCount } }),
        );
      }
      const inputDisplay = $t("common.selected_display", {
        values: { parts: inputDisplayParts.join(" + ") },
      });

      if (zone === "both") {
        // For "both" zone: each video outputs to its original location
        // We need to mark each video with its original directory as output
        for (const video of newVideos) {
          // Store the original directory as the video's intended output location
          const separator = video.path.includes("\\") ? "\\" : "/";
          const originalDir = video.path.substring(
            0,
            video.path.lastIndexOf(separator),
          );
          video.originalOutputDir = originalDir;
        }
        inputPath = inputDisplay;
        outputPath = $t("common.output_to_original_directories");
      } else {
        // For "input" zone: videos go to the set outputPath (or default location)
        inputPath = inputDisplay;
        outputPath = ""; // Reset output path when input videos/folders are selected
      }

      // Add all found videos to the files list
      files = newVideos;
      console.log(`Found ${files.length} videos from ${allPaths.length} paths`);

      // Start fetching metadata
      fetchMetadata(currentScanId);

      if (result.errors && result.errors.length > 0) {
        console.warn("Scan errors:", result.errors);
      }
    } catch (e) {
      console.error("Failed to scan multiple paths:", e);
    } finally {
      if (currentScanId === scanCounter) {
        isScanning = false;
      }
    }
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
    let unlistenVmafSearch: (() => void) | undefined;

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
          if (
            files[index].status === "Cancelled" &&
            status.startsWith("Processing")
          ) {
            console.log(
              `Ignoring stale 'Processing' update for cancelled file: ${path}`,
            );
            return;
          }

          // Calculate adjusted progress for VMAF mode compression phase
          // Backend sends progress 50-100 for compression, but we want smooth transition
          // from wherever the search ended (vmafSearchEndProgress) to 100
          let adjustedProgress = progress;
          if (status.startsWith("Found CRF")) {
            const searchEndProgress = files[index].vmafSearchEndProgress ?? 50;
            // Backend sends 50-100, map back to 0-100 for compression phase
            const compressionPhaseProgress = Math.max(0, (progress - 50) * 2);
            // Map to [searchEndProgress, 100]
            const remainingRange = 100 - searchEndProgress;
            adjustedProgress = Math.round(
              searchEndProgress +
                (compressionPhaseProgress / 100) * remainingRange,
            );
          }

          // Map both camelCase and snake_case to be safe, but use sprawled object for reactivity
          files[index] = {
            ...files[index],
            progress: adjustedProgress,
            status,
            speed: speed ?? files[index].speed,
            bitrateKbps:
              bitrateKbps ?? bitrate_kbps ?? files[index].bitrateKbps,
            outputInfo: outputInfo ?? output_info ?? files[index].outputInfo,
            // Check nested vmaf updates from outputInfo if available
            vmaf: outputInfo?.vmaf ?? output_info?.vmaf ?? files[index].vmaf,
            vmafDevice:
              outputInfo?.vmafDevice ??
              output_info?.vmafDevice ??
              files[index].vmafDevice,
            vmafDetail:
              outputInfo?.vmafDetail ??
              output_info?.vmafDetail ??
              files[index].vmafDetail,
            vmafTotalSegments:
              outputInfo?.vmafTotalSegments ??
              output_info?.vmafTotalSegments ??
              files[index].vmafTotalSegments,
            vmafModel:
              outputInfo?.vmafModel ??
              output_info?.vmafModel ??
              files[index].vmafModel,
          };
          console.log(
            `Update ${path}: ${adjustedProgress}% ${status} vmaf:${files[index].vmaf}`,
          );
          console.log(`Update ${path}: ${adjustedProgress}% ${status}`);
        }
      });

      // Listen for VMAF CRF search progress
      unlistenVmafSearch = await listen(
        "vmaf-search-progress",
        (event: any) => {
          const {
            path,
            iteration,
            maxIterations,
            currentCrf,
            currentVmaf,
            targetVmaf,
            bestCrf,
            bestVmaf,
          } = event.payload;
          const index = files.findIndex((f) => f.path === path);
          if (index !== -1) {
            // Update status with search progress
            let statusText = `Searching CRF (${iteration}/${maxIterations})`;
            if (currentVmaf > 0) {
              statusText += ` | CRF ${Math.round(currentCrf)} → VMAF ${currentVmaf.toFixed(1)}`;
            } else {
              statusText += ` | Testing CRF ${Math.round(currentCrf)}...`;
            }
            if (bestCrf !== undefined && bestCrf !== null) {
              statusText += ` | Best: CRF ${Math.round(bestCrf)}`;
            }

            // Calculate progress: map iteration to 0-50% range
            // The search phase can take up to 50% of total progress
            const searchProgress = Math.round((iteration / maxIterations) * 50);

            files[index] = {
              ...files[index],
              status: statusText,
              progress: searchProgress,
              // Store the current search progress so compression can continue from here
              vmafSearchEndProgress: searchProgress,
            };
          }
        },
      );

      console.log("Setting up Tauri file drop listeners...");

      const u = await getCurrentWindow().onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          // Block dragging if there's an active task
          if (hasActiveTask) {
            isDragging = false;
            return;
          }

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
          // Block dropping if there's an active task
          if (hasActiveTask) {
            console.log("Drop blocked: active task in progress");
            isDragging = false;
            activeZone = null;
            return;
          }

          console.log("File drop detected!", event.payload.paths);
          const paths = event.payload.paths;

          if (paths && paths.length > 0 && activeZone) {
            // Use the new multi-drop handler for all cases
            handleMultiDrop(paths, activeZone);
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
      if (unlistenVmafSearch) unlistenVmafSearch();
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

    // Check if output path is set (but not the special translation text for originalOutputDir)
    if (!outputPath || outputPath.trim() === "") {
      await ask($t("common.output_not_set_message"), {
        title: $t("common.output_not_set_title"),
        kind: "warning",
      });
      return;
    }

    // Overwrite Check
    const emptySuffix = !settings.suffix;

    // Check if any video would be overwritten
    // A video will be overwritten if:
    // 1. No suffix is set, AND
    // 2. Output format same as input format (or format unchanged), AND
    // 3. The video's output directory is the same as its source directory
    let willOverwrite = false;

    if (emptySuffix) {
      for (const file of pendingFiles) {
        // Get the video's source directory
        const separator = file.path.includes("\\") ? "\\" : "/";
        const sourceDir = file.path.substring(
          0,
          file.path.lastIndexOf(separator),
        );

        // Get the effective output directory for this file
        const effectiveOutputDir =
          (file as any).originalOutputDir || outputPath;

        // Compare directories
        if (normalizePath(sourceDir) === normalizePath(effectiveOutputDir)) {
          // Also check if input extension matches output format
          const inputExt = file.path
            .substring(file.path.lastIndexOf(".") + 1)
            .toLowerCase();
          const outputExt = settings.targetFormat.toLowerCase();

          if (inputExt === outputExt) {
            willOverwrite = true;
            break;
          }
        }
      }
    }

    if (willOverwrite) {
      const confirmed = await ask($t("common.overwrite_warning_message"), {
        title: $t("common.overwrite_warning_title"),
        kind: "warning",
      });

      if (!confirmed) return;
    }

    console.log("Start clicked");
    isProcessing = true;
    isPaused = false;
    shouldStop = false;

    // Helper functions for sorting (same as FileList.svelte)
    function parseResolution(res: string): number {
      const parts = res.split("x").map(Number);
      if (parts.length !== 2) return 0;
      const pixels = parts[0] * parts[1];
      return isNaN(pixels) ? 0 : pixels;
    }

    function parseBitrate(bitrate: string): number {
      // Only use the static bitrate string from initial scan
      // Don't use bitrateKbps which changes during processing
      const match = bitrate.match(/(\d+)/);
      return match ? parseInt(match[1], 10) : 0;
    }

    // Create items with original indices
    let itemsWithIndices = files.map((f, i) => ({ f, i }));

    // Apply sorting if a sort column is set
    if (sortStore.column) {
      itemsWithIndices.sort((a, b) => {
        let result = 0;
        switch (sortStore.column) {
          case "name":
            result = a.f.name.localeCompare(b.f.name, undefined, {
              numeric: true,
            });
            break;
          case "size":
            result = a.f.size - b.f.size;
            break;
          case "resolution":
            result =
              parseResolution(a.f.resolution) - parseResolution(b.f.resolution);
            break;
          case "bitrate":
            result = parseBitrate(a.f.bitrate) - parseBitrate(b.f.bitrate);
            break;
          case "encoder":
            result = a.f.encoder.localeCompare(b.f.encoder);
            break;
        }
        return sortStore.direction === "asc" ? result : -result;
      });
    }

    // Queue of indices to process (now in sorted order)
    const queue = itemsWithIndices
      .filter((item) => item.f.status !== "Done" && item.f.status !== "Error")
      .map((item) => item.i);

    const concurrency = settings.ffmpegThreads || 1;

    async function worker() {
      while (queue.length > 0) {
        if (shouldStop || isPaused) break;

        const i = queue.shift();
        if (i === undefined) break;

        const file = files[i];

        // Prepare output path
        // Check if video has originalOutputDir set (from multi-drop to "both" zone)
        const effectiveOutputPath =
          (file as any).originalOutputDir || outputPath;
        const effectiveInputRoot =
          (file as any).originalOutputDir ||
          inputPath ||
          file.path.substring(
            0,
            file.path.lastIndexOf(file.path.includes("\\") ? "\\" : "/"),
          );

        const outPath = getOutputFilePath(
          file.path,
          effectiveOutputPath,
          settings.targetFormat,
          settings.suffix,
          effectiveInputRoot,
        );

        // Update status to Processing
        files[i].status = "Processing";
        files[i].progress = 0;
        files = [...files];

        try {
          // Check stop or pause flag again before starting expensive operation
          if (shouldStop || isPaused) {
            files[i].status = "Pending"; // Revert status if stopped or paused right before start
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
    isPaused = !isPaused;
    if (!isPaused && isProcessing) {
      // If we unpaused, we need to restart workers if none are running?
      // Actually handleStart is already designed to process the queue.
      // But workers are already gone. So we just call handleStart again.
      handleStart();
    }
  }

  async function handleCancel() {
    console.log("Cancel clicked");
    shouldStop = true;
    isPaused = false; // Reset pause state on cancel

    // Find currently processing files and send cancel command
    // Use a loop over index to update state correctly
    for (let i = 0; i < files.length; i++) {
      if (
        files[i].status.startsWith("Processing") ||
        files[i].status.startsWith("Searching CRF") ||
        files[i].status.startsWith("Found CRF") ||
        files[i].status === "Waiting for VMAF" ||
        files[i].status === "Evaluating"
      ) {
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
  <section class="path-section">
    <PathSelector
      bind:inputPath
      bind:outputPath
      disabled={hasActiveTask}
      on:inputChange={handleInputChange}
      on:outputChange={handleOutputChange}
      on:inputVideosChange={handleInputVideosChange}
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
      {isProcessing}
      {isPaused}
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
          <span>{$t("common.as_input_folder")}</span>
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
          <span>{$t("common.as_both_folders")}</span>
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
          <span>{$t("common.as_output_folder")}</span>
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
    max-width: 1400px;
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
