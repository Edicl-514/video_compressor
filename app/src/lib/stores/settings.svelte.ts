
import { DEFAULT_SETTINGS, type AppSettings } from '../types';

function createSettingsStore() {
    let settings = $state<AppSettings>(DEFAULT_SETTINGS);
    let loaded = false;

    // Load from localStorage on client side
    if (typeof localStorage !== 'undefined') {
        const stored = localStorage.getItem('app_settings');
        if (stored) {
            try {
                settings = { ...DEFAULT_SETTINGS, ...JSON.parse(stored) };
            } catch (e) {
                console.error("Failed to parse settings", e);
            }
        }
        loaded = true;
    }

    // Persistence is handled in the root layout component to avoid effect_orphan errors.

    return {
        get value() { return settings; },
        set value(v: AppSettings) { settings = v; },
        // Helper to update specific fields easily if needed
        update: (fn: (s: AppSettings) => void) => {
            fn(settings);
        }
    };
}

export const settingsStore = createSettingsStore();
