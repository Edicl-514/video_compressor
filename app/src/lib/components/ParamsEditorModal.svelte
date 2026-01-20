<script lang="ts">
    import { t } from "svelte-i18n";

    // Make this component generic, so it can be used for encoders or generic filters
    let { title, initialParams, close, save } = $props<{
        title: string;
        initialParams: string[];
        close: () => void;
        save: (params: string[]) => void;
    }>();

    // Local copy for editing
    let params = $state<string[]>([...(initialParams || [])]);

    // Track new param input
    let newParam = $state("");

    function addParam() {
        if (newParam.trim()) {
            params.push(newParam.trim());
            newParam = "";
        }
    }

    function removeParam(index: number) {
        params = params.filter((_, i) => i !== index);
    }

    function moveUp(index: number) {
        if (index > 0) {
            const temp = params[index];
            params[index] = params[index - 1];
            params[index - 1] = temp;
        }
    }

    function moveDown(index: number) {
        if (index < params.length - 1) {
            const temp = params[index];
            params[index] = params[index + 1];
            params[index + 1] = temp;
        }
    }

    function handleSave() {
        if (newParam.trim()) {
            params.push(newParam.trim());
        }
        save(params);
        close();
    }
</script>

<div
    class="backdrop"
    role="button"
    tabindex="-1"
    onclick={close}
    onkeydown={(e) => e.key === "Escape" && close()}
>
    <div
        class="modal"
        role="dialog"
        tabindex="-1"
        aria-modal="true"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
    >
        <header>
            <h3>{title}</h3>
            <button class="close-btn" onclick={close}>&times;</button>
        </header>

        <div class="content">
            <p class="description">
                {$t("common.custom_params_hint")}
            </p>

            <div class="param-input">
                <input
                    type="text"
                    placeholder={$t("common.custom_params_placeholder")}
                    bind:value={newParam}
                    onkeydown={(e) => e.key === "Enter" && addParam()}
                />
                <button class="add-btn" onclick={addParam}
                    >{$t("common.add")}</button
                >
            </div>

            <ul class="param-list">
                {#each params as param, i}
                    <li>
                        <span class="param-text">{param}</span>
                        <div class="actions">
                            <button
                                class="icon-btn"
                                onclick={() => moveUp(i)}
                                disabled={i === 0}>↑</button
                            >
                            <button
                                class="icon-btn"
                                onclick={() => moveDown(i)}
                                disabled={i === params.length - 1}>↓</button
                            >
                            <button
                                class="icon-btn delete-btn"
                                onclick={() => removeParam(i)}>&times;</button
                            >
                        </div>
                    </li>
                {/each}
                {#if params.length === 0}
                    <li class="empty">{$t("common.no_params")}</li>
                {/if}
            </ul>
        </div>

        <footer>
            <div class="spacer"></div>
            <button class="secondary-btn" onclick={close}
                >{$t("common.cancel")}</button
            >
            <button class="primary-btn" onclick={handleSave}
                >{$t("common.confirm")}</button
            >
        </footer>
    </div>
</div>

<style>
    .backdrop {
        position: fixed;
        top: 0;
        left: 0;
        width: 100vw;
        height: 100vh;
        background: rgba(0, 0, 0, 0.6);
        z-index: 2000; /* Higher than settings modal */
        display: flex;
        justify-content: center;
        align-items: center;
        backdrop-filter: blur(2px);
    }
    .modal {
        background: #222;
        width: 400px;
        max-width: 90vw;
        border-radius: 8px;
        border: 1px solid #444;
        display: flex;
        flex-direction: column;
        box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
    }
    header {
        padding: 12px 16px;
        border-bottom: 1px solid #333;
        display: flex;
        justify-content: space-between;
        align-items: center;
        background: #2a2a2a;
        color: #fff;
    }
    header h3 {
        margin: 0;
        font-size: 1rem;
    }
    .close-btn {
        background: none;
        border: none;
        color: #888;
        font-size: 1.2rem;
        cursor: pointer;
        padding: 0;
    }
    .close-btn:hover {
        color: #fff;
    }

    .content {
        padding: 16px;
        display: flex;
        flex-direction: column;
        gap: 12px;
    }
    .description {
        margin: 0;
        color: #aaa;
        font-size: 0.85rem;
    }

    .param-input {
        display: flex;
        gap: 8px;
    }
    input {
        flex: 1;
        background: #111;
        border: 1px solid #444;
        color: #fff;
        padding: 6px 10px;
        border-radius: 4px;
    }
    input:focus {
        outline: none;
        border-color: #666;
    }

    .add-btn {
        padding: 0 12px;
        background: #646cff;
        border: none;
        color: white;
        border-radius: 4px;
        cursor: pointer;
    }

    .param-list {
        list-style: none;
        padding: 0;
        margin: 0;
        background: #1a1a1a;
        border: 1px solid #333;
        border-radius: 4px;
        max-height: 200px;
        overflow-y: auto;
    }
    li {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 6px 10px;
        border-bottom: 1px solid #2a2a2a;
    }
    li:last-child {
        border-bottom: none;
    }
    li.empty {
        color: #666;
        font-style: italic;
        padding: 12px;
        text-align: center;
        justify-content: center;
    }

    .param-text {
        color: #ddd;
        font-family: monospace;
    }

    .actions {
        display: flex;
        gap: 4px;
    }
    .icon-btn {
        background: #333;
        border: none;
        color: #aaa;
        width: 24px;
        height: 24px;
        border-radius: 4px;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 0.8rem;
    }
    .icon-btn:hover:not(:disabled) {
        background: #444;
        color: #fff;
    }
    .icon-btn:disabled {
        opacity: 0.3;
        cursor: default;
    }
    .delete-btn:hover {
        background: #a33;
        color: white;
    }

    footer {
        padding: 12px 16px;
        border-top: 1px solid #333;
        display: flex;
        justify-content: flex-end;
        gap: 10px;
        background: #2a2a2a;
    }

    button {
        padding: 6px 12px;
        border-radius: 4px;
        cursor: pointer;
        font-size: 0.9rem;
    }
    .primary-btn {
        background: #646cff;
        color: white;
        border: none;
    }
    .secondary-btn {
        background: transparent;
        border: 1px solid #444;
        color: #ccc;
    }
    .secondary-btn:hover {
        border-color: #666;
        color: white;
    }
</style>
