# Video Compressor 便携版打包脚本
# 此脚本会创建一个包含所有必要文件的便携版压缩包

param(
    [string]$Version = "0.1.0"
)

$ErrorActionPreference = "Stop"

# 颜色输出函数
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

Write-ColorOutput Green "=========================================="
Write-ColorOutput Green "  Video Compressor 便携版打包工具"
Write-ColorOutput Green "=========================================="
Write-Output ""

# 定义路径
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$AppDir = Join-Path $ProjectRoot "app"
$ReleaseDir = Join-Path $AppDir "src-tauri\target\release"
$DistDir = Join-Path $ProjectRoot "dist"
$PortableDir = Join-Path $DistDir "portable-temp"
$FFmpegBinDir = Join-Path $ProjectRoot "ffmpeg\bin"

# 检查是否已构建
if (-not (Test-Path (Join-Path $ReleaseDir "video-compressor.exe"))) {
    Write-ColorOutput Yellow "未找到构建文件，开始构建..."
    Set-Location $AppDir
    npm run tauri build
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput Red "构建失败！"
        exit 1
    }
    Set-Location $ProjectRoot
}

# 创建临时目录
Write-ColorOutput Cyan "创建临时目录..."
if (Test-Path $PortableDir) {
    Remove-Item -Recurse -Force $PortableDir
}
New-Item -ItemType Directory -Force -Path $PortableDir | Out-Null

# 复制主程序
Write-ColorOutput Cyan "复制主程序..."
Copy-Item (Join-Path $ReleaseDir "video-compressor.exe") (Join-Path $PortableDir "Video Compressor.exe")

# 复制 FFmpeg 及依赖
Write-ColorOutput Cyan "复制 FFmpeg 和依赖库..."
$FFmpegTargetDir = Join-Path $PortableDir "ffmpeg"
New-Item -ItemType Directory -Force -Path $FFmpegTargetDir | Out-Null

# 复制 FFmpeg 可执行文件
Copy-Item (Join-Path $FFmpegBinDir "ffmpeg.exe") $FFmpegTargetDir
Copy-Item (Join-Path $FFmpegBinDir "ffprobe.exe") $FFmpegTargetDir

# 复制所有 DLL
Write-ColorOutput Cyan "复制 DLL 依赖..."
Get-ChildItem -Path $FFmpegBinDir -Filter "*.dll" | ForEach-Object {
    Copy-Item $_.FullName $FFmpegTargetDir
}

# 复制 VMAF 模型文件
$ModelSourceDir = Join-Path $FFmpegBinDir "model"
if (Test-Path $ModelSourceDir) {
    Write-ColorOutput Cyan "复制 VMAF 模型文件..."
    $ModelTargetDir = Join-Path $FFmpegTargetDir "model"
    Copy-Item -Recurse $ModelSourceDir $ModelTargetDir
}

# 创建 README
Write-ColorOutput Cyan "创建说明文件..."
$ReadmeContent = @"
Video Compressor 便携版 v$Version
=====================================

使用说明：
1. 直接运行 "Video Compressor.exe" 启动程序
2. FFmpeg 及所有依赖已包含在 ffmpeg 目录中
3. 配置文件将保存在%APPDATA%\Local\com.edicl.video-compressor

系统要求：
- Windows 10/11 (64位)
- 建议 8GB 以上内存
- 支持硬件加速需要对应的显卡驱动

功能特性：
- 支持多种视频编码格式（H.264, H.265, AV1）
- 硬件加速支持（NVIDIA NVENC, Intel QSV, AMD AMF）
- VMAF 质量评估
- 批量处理
- 自定义 FFmpeg 参数
"@

Set-Content -Path (Join-Path $PortableDir "README.txt") -Value $ReadmeContent -Encoding UTF8

# 创建压缩包
Write-ColorOutput Cyan "创建压缩包..."
$ZipFileName = "Video-Compressor-Portable-v$Version.zip"
$ZipPath = Join-Path $DistDir $ZipFileName

if (Test-Path $ZipPath) {
    Remove-Item $ZipPath
}

# 使用 .NET 压缩
Add-Type -AssemblyName System.IO.Compression.FileSystem
[System.IO.Compression.ZipFile]::CreateFromDirectory($PortableDir, $ZipPath)

# 清理临时目录
Write-ColorOutput Cyan "清理临时文件..."
Remove-Item -Recurse -Force $PortableDir

# 计算文件大小
$ZipSize = (Get-Item $ZipPath).Length / 1MB

Write-Output ""
Write-ColorOutput Green "=========================================="
Write-ColorOutput Green "  打包完成！"
Write-ColorOutput Green "=========================================="
Write-Output ""
Write-ColorOutput Yellow "输出文件: $ZipPath"
Write-ColorOutput Yellow "文件大小: $([math]::Round($ZipSize, 2)) MB"
Write-Output ""
Write-ColorOutput Cyan "便携版使用说明："
Write-ColorOutput White "1. 解压 ZIP 文件到任意目录"
Write-ColorOutput White "2. 运行 'Video Compressor.exe'"
Write-ColorOutput White "3. 开始使用！"
Write-Output ""
