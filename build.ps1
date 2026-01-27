# Video Compressor 快速构建脚本
# 一键构建所有分发版本

$ErrorActionPreference = "Stop"

function Write-ColorOutput($ForegroundColor, $Message) {
    # 修正函数参数传递方式，避免参数解析歧义
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($Message) {
        Write-Output $Message
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

Write-ColorOutput -ForegroundColor Green -Message "=========================================="
Write-ColorOutput -ForegroundColor Green -Message "  Video Compressor 构建工具"
Write-ColorOutput -ForegroundColor Green -Message "=========================================="
Write-Output ""

$ProjectRoot = $PSScriptRoot
$AppDir = Join-Path $ProjectRoot "app"

# 检查 Node.js
Write-ColorOutput -ForegroundColor Cyan -Message "检查环境..."
try {
    $nodeVersion = node --version
    Write-ColorOutput -ForegroundColor White -Message "Node.js: $nodeVersion"
}
catch {
    Write-ColorOutput -ForegroundColor Red -Message "错误: 未安装 Node.js"
    Write-ColorOutput -ForegroundColor Yellow -Message "请从 https://nodejs.org/ 下载安装"
    exit 1
}

# 检查 Rust
try {
    $rustVersion = rustc --version
    Write-ColorOutput -ForegroundColor White -Message "Rust: $rustVersion"
}
catch {
    Write-ColorOutput -ForegroundColor Red -Message "错误: 未安装 Rust"
    Write-ColorOutput -ForegroundColor Yellow -Message "请从 https://rustup.rs/ 下载安装"
    exit 1
}

# 检查 FFmpeg
$FFmpegPath = Join-Path $ProjectRoot "ffmpeg\bin\ffmpeg.exe"
if (-not (Test-Path $FFmpegPath)) {
    Write-ColorOutput -ForegroundColor Red -Message "错误: 未找到 FFmpeg"
    Write-ColorOutput -ForegroundColor Yellow -Message "请确保 FFmpeg 位于 ffmpeg\bin\ 目录"
    exit 1
}
Write-ColorOutput -ForegroundColor White -Message "FFmpeg: 已找到"

Write-Output ""

# 安装依赖
Write-ColorOutput -ForegroundColor Cyan -Message "安装依赖..."
Set-Location $AppDir
if (-not (Test-Path "node_modules")) {
    npm install
    if ($LASTEXITCODE -ne 0) {
        Write-ColorOutput -ForegroundColor Red -Message "依赖安装失败！"
        exit 1
    }
}

Write-Output ""

# 构建应用
Write-ColorOutput -ForegroundColor Cyan -Message "开始构建应用..."
Write-ColorOutput -ForegroundColor Yellow -Message "这可能需要几分钟时间，请耐心等待..."
Write-Output ""

npm run tauri build

if ($LASTEXITCODE -ne 0) {
    Write-ColorOutput -ForegroundColor Red -Message "构建失败！"
    exit 1
}

Set-Location $ProjectRoot

Write-Output ""
Write-ColorOutput -ForegroundColor Green -Message "=========================================="
Write-ColorOutput -ForegroundColor Green -Message "  构建成功！"
Write-ColorOutput -ForegroundColor Green -Message "=========================================="
Write-Output ""

# 显示输出文件
$BundleDir = Join-Path $AppDir "src-tauri\target\release\bundle"

Write-ColorOutput -ForegroundColor Cyan -Message "安装程序位置："
Write-Output ""

$NsisPath = Join-Path $BundleDir "nsis"
if (Test-Path $NsisPath) {
    Get-ChildItem -Path $NsisPath -Filter "*.exe" | ForEach-Object {
        # 关键修复：将 1MB 替换为 1048576（1MB=1024*1024 字节），避免字符串内解析冲突
        $size = [math]::Round($_.Length / 1048576, 2)
        # 修正字符串格式化写法，明确参数传递
        $msg = "  [NSIS] {0} ({1} MB)" -f $_.Name, $size
        Write-ColorOutput -ForegroundColor White -Message $msg
        $pathMsg = "         {0}" -f $_.FullName
        Write-ColorOutput -ForegroundColor Gray -Message $pathMsg
    }
}

$MsiPath = Join-Path $BundleDir "msi"
if (Test-Path $MsiPath) {
    Get-ChildItem -Path $MsiPath -Filter "*.msi" | ForEach-Object {
        # 关键修复：将 1MB 替换为 1048576
        $size = [math]::Round($_.Length / 1048576, 2)
        # 修正字符串格式化写法
        $msg = "  [MSI]  {0} ({1} MB)" -f $_.Name, $size
        Write-ColorOutput -ForegroundColor White -Message $msg
        $pathMsg = "         {0}" -f $_.FullName
        Write-ColorOutput -ForegroundColor Gray -Message $pathMsg
    }
}

Write-Output ""
Write-ColorOutput -ForegroundColor Yellow -Message "推荐分发: NSIS 安装程序 (.exe)"
Write-Output ""

# 询问是否创建便携版
Write-ColorOutput -ForegroundColor Cyan -Message "是否创建便携版压缩包? (Y/N)"
$response = Read-Host

if ($response -eq 'Y' -or $response -eq 'y') {
    Write-Output ""
    & (Join-Path $ProjectRoot "scripts\create-portable.ps1")
}

Write-Output ""
Write-ColorOutput -ForegroundColor Green -Message "全部完成！"
Write-ColorOutput -ForegroundColor White -Message "查看 BUILD.md 了解更多分发信息"
Write-Output ""