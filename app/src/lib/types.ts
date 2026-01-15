
export enum CompressionMode {
    BITRATE = 'bitrate',
    CRF = 'crf',
    VMAF = 'vmaf'
}

export interface EncoderConfig {
    name: string;
    value: string;
    visible: boolean; // Controls visibility in the basic settings dropdown
    customParams: string[]; // Extra arguments specific to this encoder, one per line
}

export interface AppSettings {
    // Basic
    compressionMode: CompressionMode;
    // Values for different modes
    targetBitrate: number; // in kbps
    targetCRF: number; // 0-51 usually
    targetVMAF: number; // 0-100 score target

    concurrentTasks: number;
    maxResolution: {
        enabled: boolean;
        width: number;
        height: number;
    };
    videoEncoder: string;
    audioEncoder: string;
    targetFormat: string;

    // Advanced but stored here
    availableVideoEncoders: EncoderConfig[];
    availableAudioEncoders: EncoderConfig[];

    // Per-encoder presets could be complex, simplifying for now to a string or dictionary
    // For now, let's just hold a custom parameter string as a catch-all for "presets" + "filters"
    customFilters: string[];

    // Safety lock for manual filter editing
    enableCustomFiltersEditing: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
    compressionMode: CompressionMode.CRF,
    targetBitrate: 2000,
    targetCRF: 23,
    targetVMAF: 93,
    concurrentTasks: 1,
    maxResolution: {
        enabled: false,
        width: 1920,
        height: 1080
    },
    videoEncoder: 'libx264',
    audioEncoder: 'aac',
    targetFormat: 'mp4',
    availableVideoEncoders: [
        { name: 'H.264 (x264)', value: 'libx264', visible: true, customParams: [] },
        { name: 'H.265 (x265)', value: 'libx265', visible: true, customParams: [] },
        { name: 'AV1 (SVT-AV1)', value: 'libsvtav1', visible: true, customParams: [] },
        { name: 'NVENC H.264', value: 'h264_nvenc', visible: true, customParams: [] },
        { name: 'NVENC HEVC', value: 'hevc_nvenc', visible: true, customParams: [] },
    ],
    availableAudioEncoders: [
        { name: 'AAC', value: 'aac', visible: true, customParams: [] },
        { name: 'MP3', value: 'libmp3lame', visible: true, customParams: [] },
        { name: 'Opus', value: 'libopus', visible: true, customParams: [] },
    ],
    customFilters: [],
    enableCustomFiltersEditing: false,
};
