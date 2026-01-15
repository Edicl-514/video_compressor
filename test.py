import subprocess
import re
import sys

def check_video_encoders():
    """检测视频编码器（包含CPU编码器+硬件编码器）"""
    # ========== 配置区 ==========
    # CPU编码器：常用高性能/通用CPU编码方案
    cpu_video_encoders = [
        # H.264/AVC
        'libx264',      # 最主流的H.264 CPU编码器
        'x264',         # 备用别名
        # H.265/HEVC
        'libx265',      # 主流H.265 CPU编码器
        'x265',         # 备用别名
        # VP9
        'libvpx',       # VP8/VP9 基础编码器
        'libvpx-vp9',   # 专用VP9编码器
        'vp9',          # 原生VP9
        # AV1
        'libaom-av1',   # AV1 CPU编码器（画质好但慢）
        'av1',          # 原生AV1
        # 其他通用编码器
        'mpeg4',        # MPEG-4 Part 2
        'libmpeg4',     # MPEG-4 库版本
        'wmv1',         # WMV 1
        'wmv2',         # WMV 2
        'mpeg2video',   # MPEG-2
        'msmpeg4v2',    # 微软MPEG4 v2
    ]
    # 硬件编码器关键词（用于从ffmpeg列表中筛选）
    hw_keywords = ['nvenc', 'amf', 'qsv', 'cuda', 'vaapi', 'vdpau']
    # 视频验证分辨率（满足硬件编码器最小尺寸要求）
    test_resolution = '1280x720'
    # ===========================

    print("=" * 80)
    print("📹 视频编码器检测")
    print("=" * 80)

    # 第一步：获取FFmpeg所有编码器列表
    try:
        result = subprocess.run(
            ['ffmpeg', '-encoders'],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding='utf-8',
            errors='ignore'
        )
        all_encoder_output = result.stdout
        encoder_pattern = re.compile(r'^\s*V[A-Z.]+\s+([a-z0-9_\-]+)\s+(.*)$')
        all_video_encoders = {}
        for line in all_encoder_output.split('\n'):
            match = encoder_pattern.match(line)
            if match:
                enc_name = match.group(1)
                desc = match.group(2)
                all_video_encoders[enc_name] = desc
    except FileNotFoundError:
        print("❌ 错误：未找到 ffmpeg，请确保它在系统 PATH 中。")
        return {"cpu": [], "hw": []}

    # 第二步：检测CPU视频编码器
    print("\n【CPU编码器检测】")
    print("-" * 60)
    cpu_available = []
    for enc_name in cpu_video_encoders:
        if enc_name not in all_video_encoders:
            print(f"⚪ [不存在] {enc_name:<20} (FFmpeg 未编译该编码器)")
            continue
        
        # 验证编码器是否可实际调用
        cmd = [
            'ffmpeg', '-y', '-hide_banner', '-v', 'error',
            '-f', 'lavfi', '-i', f'color=size={test_resolution}:rate=30',
            '-frames:v', '1', '-pix_fmt', 'yuv420p',
            '-c:v', enc_name, '-f', 'null', '-'
        ]
        try:
            subprocess.run(
                cmd, check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                encoding='utf-8'
            )
            desc = all_video_encoders[enc_name][:50]  # 截断过长描述
            print(f"✅ [可用] {enc_name:<20} {desc}")
            cpu_available.append(enc_name)
        except subprocess.CalledProcessError:
            print(f"❌ [失败] {enc_name:<20} (存在但无法调用)")

    # 第三步：检测硬件视频编码器
    print("\n【硬件编码器检测】")
    print("-" * 60)
    # 筛选所有含硬件关键词的编码器
    hw_candidates = [(name, desc) for name, desc in all_video_encoders.items()
                     if any(kw in name for kw in hw_keywords)]
    hw_available = []
    if not hw_candidates:
        print("ℹ️  未检测到潜在的硬件编码器")
    else:
        for enc_name, desc in hw_candidates:
            cmd = [
                'ffmpeg', '-y', '-hide_banner',
                '-f', 'lavfi', '-i', f'color=size={test_resolution}:rate=30',
                '-frames:v', '1', '-pix_fmt', 'yuv420p',
                '-c:v', enc_name, '-f', 'null', '-'
            ]
            try:
                subprocess.run(
                    cmd, check=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                    encoding='utf-8'
                )
                print(f"✅ [可用] {enc_name:<20} {desc[:50]}")
                hw_available.append(enc_name)
            except subprocess.CalledProcessError as e:
                # 差异化输出错误信息
                base_msg = f"❌ [失败] {enc_name:<20}"
                if 'qsv' in enc_name:
                    print(f"{base_msg} (硬件不支持/驱动未加载)")
                else:
                    print(f"{base_msg} {desc[:50]}")
                    # 输出最后3行错误日志
                    error_lines = e.stderr.strip().split('\n')
                    for err_line in error_lines[-3:]:
                        print(f"    | {err_line.strip()}")
                    print()

    return {"cpu": cpu_available, "hw": hw_available}

def check_audio_encoders():
    """检测音频编码器（扩展更多常用类型）"""
    # 扩展后的音频编码器检测列表
    target_encoders = [
        # AAC 系列
        'aac',          # FFmpeg 原生 AAC
        'aac_mf',       # Windows 系统 AAC
        'libfdk_aac',   # 高质量AAC（需单独编译）
        # MP3 系列
        'libmp3lame',   # 标准MP3编码器
        'mp3_mf',       # Windows 系统 MP3
        # Opus/FLAC/ALAC
        'libopus',      # 高性能Opus
        'opus',         # 原生Opus
        'flac',         # 原生FLAC
        'alac',         # Apple Lossless
        # 杜比/AC3系列
        'ac3',          # AC3 (杜比数字)
        'eac3',         # E-AC3 (杜比数字+)
        # Windows 媒体音频
        'wmav2',        # WMA v2
        'wmav1',        # WMA v1
        # 其他常用
        'mp2',          # MPEG-1 Audio Layer II
        'pcm_s16le',    # PCM 16位小端
        'libvorbis',    # OGG Vorbis
    ]

    print("\n" + "=" * 80)
    print("🎵 音频编码器检测")
    print("=" * 80)

    # 获取所有编码器列表
    try:
        result = subprocess.run(
            ['ffmpeg', '-encoders'],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding='utf-8',
            errors='ignore'
        )
        all_encoders = result.stdout
    except FileNotFoundError:
        print("❌ 未找到 ffmpeg")
        return []

    audio_available = []
    for enc_name in target_encoders:
        # 1. 检查编码器是否存在于FFmpeg列表中
        if not re.search(f'A..... {enc_name} ', all_encoders):
            print(f"⚪ [不存在] {enc_name:<15} (FFmpeg 未编译该编码器)")
            continue

        # 2. 实际调用验证
        cmd = [
            'ffmpeg', '-y', '-hide_banner', '-v', 'error',
            '-f', 'lavfi', '-i', 'anullsrc=r=44100:cl=stereo',
            '-t', '1', '-c:a', enc_name, '-f', 'null', '-'
        ]
        try:
            subprocess.run(
                cmd, check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE
            )
            # 补充推荐说明
            note = ""
            if enc_name == 'aac': note = "(推荐: 兼容性最好)"
            if enc_name == 'libmp3lame': note = "(推荐: MP3 标准)"
            if enc_name == 'libopus': note = "(推荐: 低码率高音质)"
            if enc_name == 'libfdk_aac': note = "(推荐: 高质量AAC)"
            if 'mf' in enc_name: note = "(依赖Windows系统组件)"
            
            print(f"✅ [可用] {enc_name:<15} {note}")
            audio_available.append(enc_name)
        except subprocess.CalledProcessError:
            print(f"❌ [失败] {enc_name:<15} (存在但无法调用)")

    return audio_available

def main():
    """主函数：整合视频+音频编码器检测"""
    print("🔍 开始检测系统FFmpeg编码器可用性...\n")
    
    # 检测视频编码器
    video_result = check_video_encoders()
    cpu_video = video_result["cpu"]
    hw_video = video_result["hw"]
    
    # 检测音频编码器
    audio_available = check_audio_encoders()

    # 生成最终结论
    print("\n" + "=" * 80)
    print("📊 编码器检测最终结论")
    print("=" * 80)
    
    # 视频编码器结论
    print("\n🎬 视频编码器推荐:")
    if hw_video:
        print(f"   硬件编码器 (优先推荐): {', '.join(hw_video)}")
    if cpu_video:
        print(f"   CPU编码器 (备用): {', '.join(cpu_video)}")
    if not hw_video and not cpu_video:
        print("   无可用的视频编码器")
    
    # 音频编码器结论
    print("\n🎧 音频编码器推荐:")
    if audio_available:
        print(f"   {', '.join(audio_available)}")
    else:
        print("   无可用的音频编码器")
    
    # 额外建议
    print("\n💡 实用建议:")
    if 'h264_nvenc' in hw_video:
        print("   - 视频编码优先使用 h264_nvenc (NVIDIA显卡硬件加速)")
    if 'libx264' in cpu_video:
        print("   - 无硬件加速时，libx264 是CPU编码H.264的最佳选择")
    if 'libmp3lame' in audio_available:
        print("   - 音频MP3编码优先使用 libmp3lame，兼容性和音质最优")
    if 'aac' in audio_available:
        print("   - 音频AAC编码优先使用原生aac，无需依赖系统组件")

if __name__ == "__main__":
    main()