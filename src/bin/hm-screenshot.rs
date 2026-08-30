//! 截图对比工具 — 捕获GUI窗口截图并与参考图对比
//!
//! 使用方法：
//!   cargo run --bin hm-screenshot              # 捕获当前窗口
//!   cargo run --bin hm-screenshot -- compare    # 捕获并与参考图对比
//!
//! 输出：
//!   screenshots/current/  — 当前截图（BMP）
//!   hm_screenshot_report.json — 对比报告（供AI视觉审查）
//!
//! AI 工作流：
//! 1. 运行此工具捕获截图
//! 2. 读取 JSON 报告查看相似度分数
//! 3. 对低分项用视觉能力做详细对比并修复

#![cfg(target_os = "windows")]

use std::io::Write;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const WINDOW_TITLE: &str = "HackMagic Music Player";

// windows 0.61 未导出 PrintWindow，直接绑定 user32（Vista 起可用）
#[link(name = "user32")]
extern "system" {
    fn PrintWindow(hwnd: HWND, hdcdest: HDC, nflags: u32) -> i32;
    fn SetProcessDPIAware() -> i32;
}

fn main() {
    // 声明 DPI 感知：否则 GetWindowRect 返回虚拟化坐标（175% 缩放下
    // 只能得到模糊的缩小截图，且宽高与实际窗口不符）
    unsafe { SetProcessDPIAware() };

    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("capture");

    println!("=== HackMagic 截图工具 ===");

    // 确保输出目录存在
    let current_dir = std::path::Path::new("screenshots/current");
    let reference_dir = std::path::Path::new("screenshots/reference");
    let _ = std::fs::create_dir_all(current_dir);
    let _ = std::fs::create_dir_all(reference_dir);

    // 1. 查找窗口
    let hwnd = find_window();
    match hwnd {
        Some(h) => {
            println!("✓ 找到窗口: {:?}", h);
            // 2. 捕获截图
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
            let filename = format!("screenshot_{timestamp}.bmp");
            let filepath = current_dir.join(&filename);
            match capture_window(h, &filepath) {
                Ok(dimensions) => {
                    println!("✓ 截图已保存: {} ({}x{})", filepath.display(), dimensions.0, dimensions.1);
                    // 3. 生成元数据
                    let meta = ScreenshotEntry {
                        file: filepath.to_string_lossy().to_string(),
                        timestamp: timestamp.clone(),
                        width: dimensions.0,
                        height: dimensions.1,
                        description: "自动捕获".to_string(),
                    };
                    let _ = std::fs::write(
                        "screenshots/current/latest.json",
                        serde_json::to_string_pretty(&meta).unwrap_or_default(),
                    );
                    // 4. 如果是对比模式，与参考图对比
                    if mode == "compare" {
                        generate_comparison_report();
                    } else {
                        println!("\n提示：运行 `cargo run --bin hm-screenshot -- compare` 与参考图对比");
                    }
                }
                Err(e) => {
                    eprintln!("✗ 截图失败: {e}");
                    std::process::exit(1);
                }
            }
        }
        None => {
            println!("✗ 未找到窗口「{WINDOW_TITLE}」");
            println!("  请先运行 `cargo run --bin hm-gui` 启动应用");
            // 即使没有窗口，也生成对比报告（基于已有截图）
            if mode == "compare" {
                println!("\n尝试基于已有截图生成对比报告...");
                generate_comparison_report();
            }
        }
    }
}

fn find_window() -> Option<HWND> {
    let title: Vec<u16> = WINDOW_TITLE.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        match FindWindowW(None, windows::core::PCWSTR(title.as_ptr())) {
            Ok(hwnd) if hwnd != HWND::default() => Some(hwnd),
            _ => None,
        }
    }
}

/// 捕获窗口截图，返回 (width, height)
fn capture_window(hwnd: HWND, path: &std::path::Path) -> std::io::Result<(u32, u32)> {
    unsafe {
        // 获取窗口尺寸
        let mut rect = RECT::default();
        let ok = GetWindowRect(hwnd, &mut rect);
        if ok.is_err() {
            return Err(std::io::Error::other("GetWindowRect 失败"));
        }
        let width = (rect.right - rect.left).max(1) as u32;
        let height = (rect.bottom - rect.top).max(1) as u32;

        // 获取窗口DC
        let hdc_window = GetDC(Some(hwnd));
        if hdc_window.is_invalid() {
            return Err(std::io::Error::other("GetDC 失败"));
        }

        // 创建兼容DC和位图
        let hdc_mem = CreateCompatibleDC(Some(hdc_window));
        let h_bitmap = CreateCompatibleBitmap(hdc_window, width as i32, height as i32);

        // 选入位图
        let old_bitmap = SelectObject(hdc_mem, h_bitmap.into());

        // 拷贝窗口内容 — GPUI 经 DirectX 渲染，GDI BitBlt 只能拿到全黑画面，
        // 必须用 PrintWindow + PW_RENDERFULLCONTENT 抓取 DWM 合成后的内容。
        let printed = PrintWindow(hwnd, hdc_mem, PW_RENDERFULLCONTENT);
        if printed == 0 {
            // 传统 GDI 窗口回退路径
            let _ = BitBlt(hdc_mem, 0, 0, width as i32, height as i32, Some(hdc_window), 0, 0, SRCCOPY);
        }

        // 读取像素数据
        let mut bmp_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: height as i32, // 正数 = 自下而上
                biPlanes: 1,
                biBitCount: 32, // BGR + alpha
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD::default()],
        };

        let pixel_count = (width * height) as usize;
        let mut pixels: Vec<u8> = vec![0u8; pixel_count * 4];
        let got = GetDIBits(
            hdc_mem,
            h_bitmap,
            0,
            height,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmp_info,
            DIB_RGB_COLORS,
        );

        // 恢复并清理
        let _ = SelectObject(hdc_mem, old_bitmap);
        let _ = DeleteObject(h_bitmap.into());
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(Some(hwnd), hdc_window);

        if got == 0 {
            return Err(std::io::Error::other("GetDIBits 失败"));
        }

        // 写BMP文件
        write_bmp(path, &pixels, width, height)?;
        Ok((width, height))
    }
}

/// 写BMP文件（32位BGRA，自下而上）
fn write_bmp(path: &std::path::Path, pixels: &[u8], width: u32, height: u32) -> std::io::Result<()> {
    let row_size = width * 4;
    let pixel_data_size = row_size * height;
    let file_size = 14 + 40 + pixel_data_size;

    let mut file = std::fs::File::create(path)?;
    // BITMAPFILEHEADER (14 bytes)
    file.write_all(&[0x42, 0x4D])?; // "BM"
    file.write_all(&file_size.to_le_bytes())?; // bfSize
    file.write_all(&0u16.to_le_bytes())?; // bfReserved1
    file.write_all(&0u16.to_le_bytes())?; // bfReserved2
    file.write_all(&54u32.to_le_bytes())?; // bfOffBits
    // BITMAPINFOHEADER (40 bytes)
    file.write_all(&40u32.to_le_bytes())?; // biSize
    file.write_all(&(width as i32).to_le_bytes())?; // biWidth
    file.write_all(&(height as i32).to_le_bytes())?; // biHeight
    file.write_all(&1u16.to_le_bytes())?; // biPlanes
    file.write_all(&32u16.to_le_bytes())?; // biBitCount
    file.write_all(&0u32.to_le_bytes())?; // biCompression (BI_RGB)
    file.write_all(&pixel_data_size.to_le_bytes())?; // biSizeImage
    file.write_all(&0u32.to_le_bytes())?; // biXPelsPerMeter
    file.write_all(&0u32.to_le_bytes())?; // biYPelsPerMeter
    file.write_all(&0u32.to_le_bytes())?; // biClrUsed
    file.write_all(&0u32.to_le_bytes())?; // biClrImportant
    // 像素数据（已经是自下而上）
    file.write_all(pixels)?;
    Ok(())
}

/// 生成对比报告：将 current/ 中的截图与 reference/ 中的参考图对比
fn generate_comparison_report() {
    println!("\n=== 生成对比报告 ===");

    let current_dir = std::path::Path::new("screenshots/current");
    let reference_dir = std::path::Path::new("screenshots/reference");

    let mut entries: Vec<ComparisonEntry> = Vec::new();

    // 列出参考图
    let references: Vec<_> = std::fs::read_dir(reference_dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|ext| ext == "bmp").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();

    // 列出当前截图
    let currents: Vec<_> = std::fs::read_dir(current_dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|ext| ext == "bmp").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();

    println!("  参考图数量: {}", references.len());
    println!("  当前截图数量: {}", currents.len());

    if references.is_empty() {
        println!("  ⚠️ 没有参考图。请将原版MusicPlayer2截图放入 screenshots/reference/");
        println!("     参考图命名建议：main_view.bmp, playlist_view.bmp, lyrics_view.bmp 等");
    }

    // 对每张参考图，找最相似的当前截图
    for ref_entry in &references {
        let ref_path = ref_entry.path();
        let ref_name = ref_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let ref_hash = match compute_ahash(&ref_path) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("  ✗ 无法计算参考图哈希 {}: {}", ref_name, e);
                continue;
            }
        };

        let mut best_match: Option<(String, u32, f64)> = None; // (filename, hamming_distance, similarity%)

        for cur_entry in &currents {
            let cur_path = cur_entry.path();
            let cur_name = cur_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            match compute_ahash(&cur_path) {
                Ok(cur_hash) => {
                    let dist = hamming_distance(&ref_hash, &cur_hash);
                    let similarity = (1.0 - dist as f64 / 64.0) * 100.0;
                    if best_match.is_none() || dist < best_match.as_ref().unwrap().1 {
                        best_match = Some((cur_name, dist, similarity));
                    }
                }
                Err(e) => {
                    eprintln!("  ✗ 无法计算当前截图哈希 {}: {}", cur_name, e);
                }
            }
        }

        let entry = match best_match {
            Some((matched_file, dist, similarity)) => {
                println!("  {} → 最佳匹配: {} (相似度: {:.1}%, 汉明距离: {})", ref_name, matched_file, similarity, dist);
                ComparisonEntry {
                    reference: ref_name,
                    best_match: matched_file,
                    similarity_percent: (similarity * 10.0).round() / 10.0,
                    hamming_distance: dist,
                    status: if similarity >= 90.0 { "match".to_string() } else if similarity >= 70.0 { "similar".to_string() } else { "different".to_string() },
                }
            }
            None => {
                println!("  {} → 无可对比的截图", ref_name);
                ComparisonEntry {
                    reference: ref_name,
                    best_match: String::new(),
                    similarity_percent: 0.0,
                    hamming_distance: 64,
                    status: "no_match".to_string(),
                }
            }
        };
        entries.push(entry);
    }

    // 输出JSON报告
    let report = ComparisonReport {
        generated_at: chrono::Utc::now().to_rfc3339(),
        reference_count: references.len(),
        current_count: currents.len(),
        entries,
        ai_instructions: "AI审查指南：1. 关注status为'different'的项；2. 对低相似度项，用视觉能力对比两张图；3. 根据差异修复UI代码；4. 重新截图验证".to_string(),
    };

    let report_json = serde_json::to_string_pretty(&report).unwrap_or_default();
    let _ = std::fs::write("hm_screenshot_report.json", &report_json);
    println!("\n✓ 对比报告已保存到 hm_screenshot_report.json");
}

/// 计算图片的平均哈希（aHash）— 8x8灰度
fn compute_ahash(path: &std::path::Path) -> std::io::Result<[u64; 8]> {
    let data = std::fs::read(path)?;
    if data.len() < 54 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "BMP文件过短"));
    }

    // 解析BMP头
    let pixel_offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
    let width = i32::from_le_bytes([data[18], data[19], data[20], data[21]]);
    let height_signed = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
    let bit_count = u16::from_le_bytes([data[28], data[29]]);

    if width <= 0 || height_signed == 0 || bit_count != 32 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("不支持的BMP格式: {}x{} {}bit", width, height_signed, bit_count)));
    }

    let width = width as u32;
    let height = height_signed.unsigned_abs();
    // biHeight 为正 = 自下而上（BMP 默认），为负 = 自上而下
    let bottom_up = height_signed > 0;

    // 采样8x8网格的灰度值
    let mut grid: [[f64; 8]; 8] = [[0.0; 8]; 8];
    let row_size = width * 4;

    for (gy, row_grid) in grid.iter_mut().enumerate() {
        for (gx, cell) in row_grid.iter_mut().enumerate() {
            // 采样位置（中心）
            let px = (gx as f64 + 0.5) / 8.0 * width as f64;
            let py = (gy as f64 + 0.5) / 8.0 * height as f64;
            let px = px.min(width as f64 - 1.0) as u32;
            let py = py.min(height as f64 - 1.0) as u32;
            // BMP自下而上：行索引反转
            let row = if bottom_up { height - 1 - py } else { py };
            let offset = pixel_offset + (row * row_size + px * 4) as usize;
            if offset + 3 > data.len() {
                continue;
            }
            let b = data[offset] as f64;
            let g = data[offset + 1] as f64;
            let r = data[offset + 2] as f64;
            // 灰度 = 0.299R + 0.587G + 0.114B
            *cell = 0.299 * r + 0.587 * g + 0.114 * b;
        }
    }

    // 计算平均值
    let avg: f64 = grid.iter().flat_map(|row| row.iter()).sum::<f64>() / 64.0;

    // 生成哈希
    let mut hash = [0u64; 8];
    for (gy, row_grid) in grid.iter().enumerate() {
        for (gx, &g) in row_grid.iter().enumerate() {
            if g >= avg {
                hash[gy] |= 1 << gx;
            }
        }
    }
    Ok(hash)
}

/// 计算两个哈希的汉明距离
fn hamming_distance(a: &[u64; 8], b: &[u64; 8]) -> u32 {
    a.iter().zip(b.iter())
        .map(|(x, y)| (x ^ y).count_ones())
        .sum()
}

// ============================================================================
// JSON报告结构
// ============================================================================

#[derive(serde::Serialize)]
struct ScreenshotEntry {
    file: String,
    timestamp: String,
    width: u32,
    height: u32,
    description: String,
}

#[derive(serde::Serialize)]
struct ComparisonEntry {
    reference: String,
    best_match: String,
    similarity_percent: f64,
    hamming_distance: u32,
    status: String,
}

#[derive(serde::Serialize)]
struct ComparisonReport {
    generated_at: String,
    reference_count: usize,
    current_count: usize,
    entries: Vec<ComparisonEntry>,
    ai_instructions: String,
}
