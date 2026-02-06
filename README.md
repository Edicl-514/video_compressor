# Video Compressor

一个功能强大的视频压缩工具，基于 Tauri + SvelteKit 开发，内置 FFmpeg 支持。

[简体中文](#video-compressor) | [English](#english-version)

## ✨ 主要特性

- 🎬 **多种编码格式**: 支持 H.264, H.265, AV1 等主流编码格式
- ⚡ **硬件加速**: 支持 NVIDIA NVENC, Intel QSV, AMD AMF 硬件加速
- 📊 **VMAF 质量评估**: 内置 VMAF 质量评估，精确控制压缩质量
- 🔄 **批量处理**: 支持批量压缩多个视频文件
- 🎯 **多种压缩模式**: 
  - 目标 CRF 模式
  - 目标 VMAF 模式
  - 目标码率模式
  - 自定义 FFmpeg 命令模式
  - 流复制模式
- 🌍 **多语言支持**: 支持中文和英文界面
- 🎨 **现代化界面**: 基于 Svelte 5 的响应式界面

## 📋 系统要求

- Windows 10 或更高版本
- 建议至少 4GB RAM，更大的视频文件建议 8GB+

### ⚠️ FFmpeg 编译要求

**重要提示**：本程序需要使用带有 `libvmaf_cuda` 的 FFmpeg，这会导致许可证冲突。因此：

- 源代码和 Release 版本**均不包含** FFmpeg 二进制文件
- **需要自己编译** FFmpeg，并将编译结果放入 `ffmpeg/bin/` 目录
- 编译步骤和详细说明请参考：[compile_cuda_vmaf_ffmpeg_on_windows](https://github.com/Edicl-514/compile_cuda_vmaf_ffmpeg_on_windows)

## 🚀 快速开始

### 用户使用

如果你只是想使用这个应用，请下载发布的安装程序：

1. 前往Releases下载最新版本并解压
2. **从 [compile_cuda_vmaf_ffmpeg_on_windows](https://github.com/Edicl-514/compile_cuda_vmaf_ffmpeg_on_windows) 编译 FFmpeg**，或从其他来源获取带有 `libvmaf_cuda` 的 FFmpeg 二进制文件
3. 将编译好的 FFmpeg 文件放入应用安装目录下的 `ffmpeg` 文件夹
4. 启动应用，开始压缩视频！

### 开发者构建

#### 前提条件

- Node.js 18+ 和 npm
- Rust 和 Cargo（用于 Tauri）
- Visual Studio Build Tools 或完整的 Visual Studio（仅 Windows）
- **编译好的 FFmpeg**（带有 `libvmaf_cuda` 支持，参考 [compile_cuda_vmaf_ffmpeg_on_windows](https://github.com/Edicl-514/compile_cuda_vmaf_ffmpeg_on_windows) 项目），放入`ffmpeg/bin/`文件夹

#### 最简单的方法（推荐）

在项目根目录运行：

```powershell
.\build.ps1
```

这个脚本会自动完成所有构建步骤，生成可分发的安装程序。



## 📁 项目结构

```
video_compressor/
├── app/                      # Svelte 前端应用
│   ├── src/
│   │   ├── lib/
│   │   │   ├── components/   # Svelte 组件
│   │   │   ├── i18n/         # 国际化
│   │   │   ├── stores/       # Svelte 存储
│   │   │   └── types.ts      # TypeScript 类型定义
│   │   └── routes/           # SvelteKit 路由
│   ├── src-tauri/            # Tauri 后端
│   │   ├── src/
│   │   │   ├── main.rs       # Tauri 主程序入口
│   │   │   ├── lib.rs        # 库代码
│   │   │   └── video.rs      # 视频处理逻辑
│   │   ├── Cargo.toml        # Rust 依赖
│   │   └── tauri.conf.json   # Tauri 配置
│   ├── package.json          # Node.js 依赖
│   └── tsconfig.json         # TypeScript 配置
├── ffmpeg/                   # FFmpeg 可执行文件和模型文件
│   └── bin/                  # FFmpeg.exe 放在这里
│       └── model/            # VMAF 质量评估模型
├── scripts/                  # 构建脚本
└── build.ps1                 # 主构建脚本
```



## 📝 许可证

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。


## 📚 相关资源

- [Tauri 文档](https://tauri.app/)
- [SvelteKit 文档](https://kit.svelte.dev/)
- [FFmpeg 文档](https://ffmpeg.org/)
- [VMAF 文档](https://github.com/Netflix/vmaf)

---

## English Version

# Video Compressor

A powerful video compression tool developed with Tauri + SvelteKit and built-in FFmpeg support.

## ✨ Features

- 🎬 **Multiple Codec Formats**: Support for H.264, H.265, AV1, and other mainstream codecs
- ⚡ **Hardware Acceleration**: Support for NVIDIA NVENC, Intel QSV, AMD AMF hardware acceleration
- 📊 **VMAF Quality Assessment**: Built-in VMAF quality assessment for precise quality control
- 🔄 **Batch Processing**: Compress multiple video files at once
- 🎯 **Multiple Compression Modes**:
  - Target CRF mode
  - Target VMAF mode
  - Target bitrate mode
  - Custom FFmpeg command mode
  - Stream copy mode
- 🌍 **Multi-Language Support**: English and Chinese interface support
- 🎨 **Modern UI**: Responsive interface based on Svelte 5

## 📋 System Requirements

- Windows 10 or higher
- Recommended 4GB RAM minimum, 8GB+ for larger video files

### ⚠️ FFmpeg Compilation Requirements

**Important**: This application requires FFmpeg compiled with `libvmaf_cuda` support, which creates a license conflict. Therefore:

- The source code and Release versions **do not include** FFmpeg binaries
- **You must compile FFmpeg yourself** and place the compiled binaries in the `ffmpeg/bin/` directory
- For compilation steps and detailed instructions, please refer to: [compile_cuda_vmaf_ffmpeg_on_windows](https://github.com/Edicl-514/compile_cuda_vmaf_ffmpeg_on_windows)

## 🚀 Getting Started

### For Users

To use the application, download the installer:

1. Go to Releases,download and unzip
2. **Compile FFmpeg** with `libvmaf_cuda` support from [compile_cuda_vmaf_ffmpeg_on_windows](https://github.com/Edicl-514/compile_cuda_vmaf_ffmpeg_on_windows), or obtain pre-compiled FFmpeg binaries with `libvmaf_cuda` support from other sources
3. Place the compiled FFmpeg binaries into the `ffmpeg` folder in your application installation directory
4. Launch the application and start compressing videos!

### For Developers

#### Prerequisites

- Node.js 18+ and npm
- Rust and Cargo (for Tauri)
- Visual Studio Build Tools or full Visual Studio (Windows only)
- **Compiled FFmpeg** with `libvmaf_cuda` support (refer to [compile_cuda_vmaf_ffmpeg_on_windows](https://github.com/Edicl-514/compile_cuda_vmaf_ffmpeg_on_windows)),Place it in the `ffmpeg/bin/` folder

#### Recommended Method

Run in the project root directory:

```powershell
.\build.ps1
```

This script automatically completes all build steps and generates a distributable installer.



## 📝 License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.


## 📚 Resources

- [Tauri Documentation](https://tauri.app/)
- [SvelteKit Documentation](https://kit.svelte.dev/)
- [FFmpeg Documentation](https://ffmpeg.org/)
- [VMAF Documentation](https://github.com/Netflix/vmaf)
