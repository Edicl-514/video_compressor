<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { createEventDispatcher } from 'svelte';

  export let inputPath = "";
  export let outputPath = "";

  const dispatch = createEventDispatcher();

  async function browseInput() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Input Video Folder"
      });
      if (selected && typeof selected === 'string') {
        inputPath = selected;
        dispatch('inputChange', inputPath);
      }
    } catch (e) {
      console.error("Failed to open dialog", e);
    }
  }

  async function browseOutput() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Output Folder"
      });
      if (selected && typeof selected === 'string') {
        outputPath = selected;
        dispatch('outputChange', outputPath);
      }
    } catch (e) {
      console.error("Failed to open dialog", e);
    }
  }
</script>

<div class="path-selector">
  <div class="path-row">
    <label for="input">Input:</label>
    <div class="input-wrapper">
        <input 
            id="input" 
            type="text" 
            bind:value={inputPath} 
            placeholder="Select video folder... (or drag & drop)" 
            on:change={() => dispatch('inputChange', inputPath)}
        />
        <!-- Optional: Instruction tooltip or icon could go here -->
    </div>
    <button on:click={browseInput}>Browse</button>
  </div>
  
  <div class="path-row">
    <label for="output">Output:</label>
    <div class="input-wrapper">
        <input 
            id="output" 
            type="text" 
            bind:value={outputPath} 
            placeholder="Select output folder..." 
            on:change={() => dispatch('outputChange', outputPath)}
        />
    </div>
    <button on:click={browseOutput}>Browse</button>
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
    font-family: 'JetBrains Mono', monospace;
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
</style>
