
export enum CompressionMode {
    BITRATE = 'bitrate',
    CRF = 'crf',
    VMAF = 'vmaf'
}

export interface VideoInfo {
    name: string;
    path: string;
    size: number;
    resolution: string;
    bitrate: string;
    encoder: string;
    status: string; // "Scanning", "Pending", "Processing", "Done", "Error", "Cancelled"
    progress: number;
    durationSec: number;
    speed?: number;
    bitrateKbps?: number;
    outputInfo?: VideoInfo;
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

    ffmpegThreads: number;
    ffprobeThreads: number;
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

    // Output suffix
    suffix: string;
}

export const DEFAULT_SETTINGS: AppSettings = {
    compressionMode: CompressionMode.CRF,
    targetBitrate: 2000,
    targetCRF: 23,
    targetVMAF: 93,
    ffmpegThreads: 2,
    ffprobeThreads: 4,
    maxResolution: {
        enabled: false,
        width: 1920,
        height: 1080
    },
    videoEncoder: 'libx264',
    audioEncoder: 'aac',
    targetFormat: 'mp4',
    availableVideoEncoders: [
        { name: 'libx264 (CPU)', value: 'libx264', visible: true, customParams: [] },
        { name: 'libx265 (CPU)', value: 'libx265', visible: true, customParams: [] },
        { name: 'libsvtav1 (CPU)', value: 'libsvtav1', visible: true, customParams: [] },
        { name: 'h264_nvenc (HW)', value: 'h264_nvenc', visible: true, customParams: [] },
        { name: 'hevc_nvenc (HW)', value: 'hevc_nvenc', visible: true, customParams: [] },
    ],
    availableAudioEncoders: [
        { name: 'aac (CPU)', value: 'aac', visible: true, customParams: [] },
        { name: 'libmp3lame (CPU)', value: 'libmp3lame', visible: true, customParams: [] },
        { name: 'libopus (CPU)', value: 'libopus', visible: true, customParams: [] },
    ],
    customFilters: [],
    enableCustomFiltersEditing: false,
    suffix: '_compressed'
};
