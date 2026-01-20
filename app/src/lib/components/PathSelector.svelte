<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { createEventDispatcher } from "svelte";

  export let inputPath = "";
  export let outputPath = "";
  export let disabled = false;

  import { _ as t } from "svelte-i18n";

  const dispatch = createEventDispatcher();

  // Video file extensions for file filter
  const VIDEO_EXTENSIONS = [
    "mp4",
    "mkv",
    "avi",
    "mov",
    "flv",
    "wmv",
    "webm",
    "m4v",
    "mpg",
    "mpeg",
    "3gp",
    "ts",
    "asf",
    "rmvb",
    "vob",
    "m2ts",
    "f4v",
    "mts",
    "ogv",
    "divx",
    "xvid",
    "rm",
  ];

  async function browseInputFolder() {
    if (disabled) return;
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Input Video Folder",
      });
      if (selected && typeof selected === "string") {
        inputPath = selected;
        dispatch("inputChange", inputPath);
      }
    } catch (e) {
      console.error("Failed to open dialog", e);
    }
  }

  async function browseInputVideos() {
    if (disabled) return;
    try {
      const selected = await open({
        directory: false,
        multiple: true,
        title: "Select Video Files",
        filters: [
          {
            name: "Video Files",
            extensions: VIDEO_EXTENSIONS,
          },
        ],
      });
      if (selected) {
        // Handle array of paths or single path
        const paths = Array.isArray(selected) ? selected : [selected];
        if (paths.length > 0) {
          // Emit multiple paths - parent component will handle scanning
          dispatch("inputVideosChange", paths);
        }
      }
    } catch (e) {
      console.error("Failed to open dialog", e);
    }
  }

  async function browseOutput() {
    if (disabled) return;
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Output Folder",
      });
      if (selected && typeof selected === "string") {
        outputPath = selected;
        dispatch("outputChange", outputPath);
      }
    } catch (e) {
      console.error("Failed to open dialog", e);
    }
  }
</script>

<div class="path-selector">
  <div class="path-row">
    <div class="path-row-label">
      <label for="input">{$t("common.input_path")}:</label>
    </div>
    <div class="input-wrapper">
      <input
        id="input"
        type="text"
        bind:value={inputPath}
        placeholder={$t("common.input_place_holder")}
        on:change={() => dispatch("inputChange", inputPath)}
      />
    </div>
    <div class="browse-buttons">
      <button
        class="browse-btn"
        on:click={browseInputFolder}
        {disabled}
        title={$t("common.select_folder")}>📁</button
      >
      <button
        class="browse-btn"
        on:click={browseInputVideos}
        {disabled}
        title={$t("common.select_video_files")}>🎬</button
      >
    </div>
  </div>

  <div class="path-row">
    <div class="path-row-label">
      <label for="output">{$t("common.output_path")}:</label>
    </div>
    <div class="input-wrapper">
      <input
        id="output"
        type="text"
        bind:value={outputPath}
        placeholder={$t("common.output_place_holder")}
        on:change={() => dispatch("outputChange", outputPath)}
      />
    </div>
    <button class="browse-output-btn" on:click={browseOutput} {disabled}
      >{$t("common.browse")}</button
    >
  </div>
</div>

<style>
  .path-selector {
    padding: 1.5rem;
    background-color: #2a2a2a;
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
    border: 1px solid #333;
  }
  .path-row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .path-row-label {
    width: auto;
    text-align: right;
  }
  label {
    width: 60px;
    color: #a0a0a0;
    font-weight: 500;
    font-size: 0.9rem;
  }
  .input-wrapper {
    flex: 1;
    position: relative;
  }
  input {
    width: 100%;
    padding: 0.7rem 1rem;
    background-color: #1a1a1a;
    color: #e0e0e0;
    border: 1px solid #444;
    border-radius: 8px;
    outline: none;
    transition: all 0.2s ease;
    font-family: "JetBrains Mono", monospace;
    font-size: 0.85rem;
    box-sizing: border-box;
  }
  input:focus {
    border-color: #646cff;
    box-shadow: 0 0 0 2px rgba(100, 108, 255, 0.1);
    background-color: #222;
  }
  button {
    padding: 0.7rem 1.4rem;
    background-color: #3a3a3a;
    color: #fff;
    border: 1px solid #555;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.2s;
    font-size: 0.9rem;
  }
  button:hover {
    background-color: #646cff;
    border-color: #646cff;
    transform: translateY(-1px);
  }
  button:active {
    transform: translateY(0);
  }
  button:disabled {
    background-color: #2a2a2a;
    color: #666;
    cursor: not-allowed;
    border-color: #444;
  }
  button:disabled:hover {
    transform: none;
    background-color: #2a2a2a;
    border-color: #444;
  }
  .browse-buttons {
    display: flex;
    gap: 0.3rem;
  }
  .browse-btn {
    padding: 0.7rem 0.9rem;
    font-size: 1rem;
    min-width: auto;
  }
  .browse-output-btn {
    width: 110px;
    padding-left: 0;
    padding-right: 0;
    text-align: center;
  }
</style>
