<script lang="ts">
    import { locale, locales, waitLocale } from "svelte-i18n";

    const languages = [
        { code: "en", name: "English", flag: "🇺🇸" },
        { code: "zh", name: "简体中文", flag: "🇨🇳" },
    ];

    function setLanguage(code: string) {
        locale.set(code);
        localStorage.setItem("preferred_language", code);
    }
</script>

<div class="language-switcher">
    {#each languages as lang}
        <button
            class="lang-btn"
            class:active={$locale?.startsWith(lang.code)}
            onclick={() => setLanguage(lang.code)}
        >
            <span class="flag">{lang.flag}</span>
            <span class="name">{lang.name}</span>
        </button>
    {/each}
</div>

<style>
    .language-switcher {
        display: flex;
        gap: 8px;
        background: rgba(0, 0, 0, 0.05);
        padding: 4px;
        border-radius: 8px;
        width: fit-content;
    }

    .lang-btn {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 6px 12px;
        border: none;
        background: transparent;
        border-radius: 6px;
        cursor: pointer;
        font-size: 0.9rem;
        color: #666;
        transition: all 0.2s ease;
    }

    .lang-btn:hover {
        background: rgba(255, 255, 255, 0.5);
        color: #333;
    }

    .lang-btn.active {
        background: white;
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
        color: #000;
        font-weight: 500;
    }

    .flag {
        font-size: 1.1rem;
    }

    @media (prefers-color-scheme: dark) {
        .language-switcher {
            background: rgba(255, 255, 255, 0.05);
        }
        .lang-btn {
            color: #aaa;
        }
        .lang-btn:hover {
            background: rgba(255, 255, 255, 0.1);
            color: #eee;
        }
        .lang-btn.active {
            background: rgba(255, 255, 255, 0.15);
            color: #fff;
        }
    }
</style>
