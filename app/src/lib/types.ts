
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
    vmaf?: number; // VMAF score (0-100)
    vmafDevice?: string; // "CPU" or "CUDA"
    vmafDetail?: number[]; // Detail scores per segment
    vmafTotalSegments?: number;
    vmafModel?: string;
    vmafSearchEndProgress?: number; // Progress when VMAF search ended (for smooth progress)
    originalOutputDir?: string; // For multi-drop mode: output to original directory
}

// VMAF CRF search progress event payload
export interface VmafSearchProgress {
    path: string;
    iteration: number;
    maxIterations: number;
    currentCrf: number;
    currentVmaf: number;
    targetVmaf: number;
    bestCrf?: number;
    bestVmaf?: number;
    samples: [number, number][]; // [crf, vmaf] pairs
}

// ... (imports remain same, not restated here)

export interface EncoderConfig {
    name: string;
    value: string;
    visible: boolean; // Controls visibility in the basic settings dropdown
    customParams: string[]; // Extra arguments specific to this encoder, one per line
    isSupported?: boolean; // Whether the encoder is currently supported/detected
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

    // Custom VMAF calculation params
    customVmafParams: string[];

    // Safety lock for manual filter editing


    // Output suffix
    suffix: string;

    // Bitrate mode settings
    twoPass: boolean;
    minBitrateThreshold: number; // kbps, 0 to disable

    // CRF mode settings
    crfAutoSkip: boolean; // skip if output bitrate > input bitrate * threshold
    crfAutoSkipThreshold: number; // percentage, e.g. 100 for 1.0x

    // VMAF Settings
    enableVmaf: boolean;
    vmafFullComputation: boolean;
    vmafSegmentCount: number;
    vmafSegmentDuration: number;
    vmafAutoConfig: boolean;
    vmafUseCuda: boolean;
    vmafNeg: boolean;

    // Encoder filter settings
    showAllEncoders: boolean; // Show all available encoders without filtering
    showOnlyHwEncoders: boolean; // Only show hardware-accelerated video encoders
}

export const DEFAULT_SETTINGS: AppSettings = {
    compressionMode: CompressionMode.CRF,
    targetBitrate: 8000,
    targetCRF: 22,
    targetVMAF: 97,
    ffmpegThreads: 1,
    ffprobeThreads: 8, // More parallel checks
    maxResolution: {
        enabled: false,
        width: 1920,
        height: 1080
    },
    videoEncoder: 'libx264',
    audioEncoder: 'aac',
    targetFormat: 'mp4',
    availableVideoEncoders: [
        { name: 'libx264 (CPU)', value: 'libx264', visible: true, isSupported: true, customParams: ['-preset medium', '-profile:v high'] },
        { name: 'libx265 (CPU)', value: 'libx265', visible: true, isSupported: true, customParams: ['-preset medium', '-tag:v hvc1'] },
        { name: 'libsvtav1 (CPU)', value: 'libsvtav1', visible: true, isSupported: true, customParams: ['-preset 8', '-svtav1-params tune=0'] },
        { name: 'h264_nvenc (HW)', value: 'h264_nvenc', visible: true, isSupported: true, customParams: ['-preset p4'] },
        { name: 'hevc_nvenc (HW)', value: 'hevc_nvenc', visible: true, isSupported: true, customParams: ['-preset p4', '-tag:v hvc1'] },
        { name: 'av1_nvenc (HW)', value: 'av1_nvenc', visible: true, isSupported: true, customParams: ['-preset p4', '-tune hq'] },
    ],
    availableAudioEncoders: [
        { name: 'aac (CPU)', value: 'aac', visible: true, isSupported: true, customParams: ['-b:a 192k'] },
        { name: 'libmp3lame (CPU)', value: 'libmp3lame', visible: true, isSupported: true, customParams: ['-ac 2'] },
        { name: 'libopus (CPU)', value: 'libopus', visible: true, isSupported: true, customParams: ['-b:a 128k', '-vbr on'] },
    ],
    customFilters: ['-pix_fmt yuv420p', '-map_metadata 0', '-movflags +faststart'],
    customVmafParams: ['n_subsample=5'],

    suffix: '_compressed',
    twoPass: false,
    minBitrateThreshold: 0,
    crfAutoSkip: false,
    crfAutoSkipThreshold: 100,

    // VMAF Settings
    enableVmaf: false,
    vmafFullComputation: false, // Option 2: Full computation (Slow)
    vmafSegmentCount: 4,        // Option 3: Number of segments (if not full)
    vmafSegmentDuration: 10,     // Option 3: Duration per segment (if not full)
    vmafAutoConfig: true,       // Option 3: Auto set segments/duration
    vmafUseCuda: true,          // Option 4: Experimental CUDA
    vmafNeg: false,

    // Encoder filter settings
    showAllEncoders: false,
    showOnlyHwEncoders: false
};
