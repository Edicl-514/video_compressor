# 构建验证脚本
# 用于检查构建输出是否完整

param(
    [switch]$Detailed
)

$ErrorActionPreference = "Stop"

function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

function Test-File($Path, $Name) {
    if (Test-Path $Path) {
        $size = (Get-Item $Path).Length / 1MB
        Write-ColorOutput Green "  ? $Name ($([math]::Round($size, 2)) MB)"
        return $true
    }
    else {
        Write-ColorOutput Red "  ? $Name - 未找到"
        return $false
    }
}

Write-ColorOutput Cyan "=========================================="
Write-ColorOutput Cyan "  构建验证工具"
Write-ColorOutput Cyan "=========================================="
Write-Output ""

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$BundleDir = Join-Path $ProjectRoot "app\src-tauri\target\release\bundle"
$DistDir = Join-Path $ProjectRoot "dist"

$allGood = $true

# 检查安装程序
Write-ColorOutput Yellow "检查安装程序..."

$NsisDir = Join-Path $BundleDir "nsis"
if (Test-Path $NsisDir) {
    $nsisFiles = Get-ChildItem -Path $NsisDir -Filter "*.exe"
    if ($nsisFiles.Count -gt 0) {
        foreach ($file in $nsisFiles) {
            Test-File $file.FullName "NSIS 安装程序: $($file.Name)" | Out-Null
        }
    }
    else {
        Write-ColorOutput Red "  ? 未找到 NSIS 安装程序"
        $allGood = $false
    }
}
else {
    Write-ColorOutput Red "  ? NSIS 目录不存在"
    $allGood = $false
}

$MsiDir = Join-Path $BundleDir "msi"
if (Test-Path $MsiDir) {
    $msiFiles = Get-ChildItem -Path $MsiDir -Filter "*.msi"
    if ($msiFiles.Count -gt 0) {
        foreach ($file in $msiFiles) {
            Test-File $file.FullName "MSI 安装程序: $($file.Name)" | Out-Null
        }
    }
    else {
        Write-ColorOutput Yellow "  ? 未找到 MSI 安装程序（可选）"
    }
}
else {
    Write-ColorOutput Yellow "  ? MSI 目录不存在（可选）"
}

Write-Output ""

# 检查便携版
Write-ColorOutput Yellow "检查便携版..."
if (Test-Path $DistDir) {
    $zipFiles = Get-ChildItem -Path $DistDir -Filter "*.zip"
    if ($zipFiles.Count -gt 0) {
        foreach ($file in $zipFiles) {
            Test-File $file.FullName "便携版: $($file.Name)" | Out-Null
        }
    }
    else {
        Write-ColorOutput Yellow "  ? 未找到便携版（可选）"
    }
}
else {
    Write-ColorOutput Yellow "  ? dist 目录不存在（可选）"
}

Write-Output ""

# 详细检查（如果启用）
if ($Detailed) {
    Write-ColorOutput Yellow "详细检查..."
    
    # 检查主程序
    $ExePath = Join-Path $ProjectRoot "app\src-tauri\target\release\video-compressor.exe"
    if (Test-File $ExePath "主程序") {
        # 检查依赖
        Write-ColorOutput Cyan "  检查依赖..."
        
        # 这里可以添加更多检查
        # 例如检查 FFmpeg 是否被正确打包
    }
    
    Write-Output ""
}

# 总结
Write-ColorOutput Cyan "=========================================="
if ($allGood) {
    Write-ColorOutput Green "  ? 所有必需文件都已找到"
    Write-ColorOutput Green "  构建成功！"
}
else {
    Write-ColorOutput Red "  ? 缺少某些必需文件"
    Write-ColorOutput Yellow "  请运行构建脚本: .\build.ps1"
}
Write-ColorOutput Cyan "=========================================="
Write-Output ""

# 显示分发建议
if ($allGood) {
    Write-ColorOutput Yellow "分发建议："
    Write-ColorOutput White "  1. 普通用户: 使用 NSIS 安装程序 (.exe)"
    Write-ColorOutput White "  2. 企业部署: 使用 MSI 安装程序 (.msi)"
    Write-ColorOutput White "  3. 便携使用: 使用便携版压缩包 (.zip)"
    Write-Output ""
}

exit $(if ($allGood) { 0 } else { 1 })
