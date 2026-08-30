//! E2E 测试运行器 — 启动应用、执行操作序列、收集日志和截图
//!
//! 使用方法：
//!   cargo run --bin hm-e2e-test
//!
//! AI 可以：
//! 1. 运行此二进制启动自动化测试
//! 2. 读取输出的 JSON 报告
//! 3. 根据报告修复发现的问题

use std::process::Command;
use std::time::Duration;

fn main() {
    println!("=== HackMagic E2E 测试运行器 ===\n");

    // 1. 编译确保最新
    println!("[1/5] 编译项目...");
    let build = Command::new("cargo")
        .args(["build", "--bin", "hm-gui"])
        .status();
    match build {
        Ok(s) if s.success() => println!("✓ 编译成功"),
        Ok(_) => {
            println!("✗ 编译失败");
            std::process::exit(1);
        }
        Err(e) => {
            println!("✗ 无法运行 cargo: {e}");
            std::process::exit(1);
        }
    }

    // 2. 运行单元测试
    println!("\n[2/5] 运行单元测试...");
    let test = Command::new("cargo")
        .args(["test", "--lib"])
        .output();
    match test {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{stdout}{stderr}");
            if combined.contains("test result: ok") {
                // 提取测试数量
                for line in combined.lines() {
                    if line.contains("test result:") {
                        println!("  {line}");
                    }
                }
                println!("✓ 单元测试通过");
            } else {
                println!("✗ 单元测试失败");
                // 打印失败的测试
                for line in combined.lines() {
                    if line.contains("FAILED") || line.contains("error[") {
                        println!("  {line}");
                    }
                }
                std::process::exit(1);
            }
        }
        Err(e) => {
            println!("✗ 无法运行测试: {e}");
            std::process::exit(1);
        }
    }

    // 3. 启动 GUI 应用（后台）
    println!("\n[3/5] 启动 GUI 应用...");
    let mut gui = match Command::new("cargo")
        .args(["run", "--bin", "hm-gui"])
        .spawn()
    {
        Ok(child) => {
            println!("✓ GUI 进程已启动 (PID: {})", child.id());
            child
        }
        Err(e) => {
            println!("✗ 无法启动 GUI: {e}");
            std::process::exit(1);
        }
    };

    // 等待应用启动
    std::thread::sleep(Duration::from_secs(5));

    // 4. 检查日志文件
    println!("\n[4/5] 检查日志文件...");
    let log_files = ["hm_gui.log", "hm_panic.log"];
    for f in &log_files {
        if std::path::Path::new(f).exists() {
            let content = std::fs::read_to_string(f).unwrap_or_default();
            let lines = content.lines().count();
            let errors = content.lines().filter(|l| l.contains("[ERROR]") || l.contains("[PANIC]")).count();
            let warnings = content.lines().filter(|l| l.contains("[WARN]")).count();
            let ui_events = content.lines().filter(|l| l.contains("[UI_EVENT]")).count();
            println!("  {f}: {lines} 行, {errors} 错误, {warnings} 警告, {ui_events} UI事件");

            if errors > 0 {
                println!("  ⚠️ 错误详情:");
                for line in content.lines().filter(|l| l.contains("[ERROR]") || l.contains("[PANIC]")).take(10) {
                    println!("    {line}");
                }
            }
        } else {
            println!("  {f}: 不存在");
        }
    }

    // 5. 生成测试报告
    println!("\n[5/5] 生成测试报告...");
    let report = generate_report();
    let report_path = "hm_e2e_report.json";
    std::fs::write(report_path, &report).ok();
    println!("✓ 报告已保存到 {report_path}");

    // 等待用户手动测试（或等待AI通过Computer Use操作）
    println!("\n=== 应用已启动，可以通过以下方式测试 ===");
    println!("1. 手动操作应用界面");
    println!("2. AI 通过 Computer Use 自动化操作");
    println!("3. 按 Ctrl+C 结束测试");
    println!("\n事件日志会实时写入 hm_gui.log");
    println!("查找 [UI_EVENT] 标记可看到所有UI交互事件");

    // 等待 GUI 进程结束
    let _ = gui.wait();
}

fn generate_report() -> String {
    let mut report = String::new();
    report.push_str("{\n");
    report.push_str("  \"test_type\": \"e2e_manual\",\n");

    // 检查编译状态
    report.push_str("  \"build\": \"success\",\n");

    // 检查测试结果
    report.push_str("  \"unit_tests\": \"passed\",\n");

    // 检查日志
    let log_content = std::fs::read_to_string("hm_gui.log").unwrap_or_default();
    let error_count = log_content.lines().filter(|l| l.contains("[ERROR]")).count();
    let panic_count = log_content.lines().filter(|l| l.contains("[PANIC]")).count();
    let ui_event_count = log_content.lines().filter(|l| l.contains("[UI_EVENT]")).count();

    report.push_str(&format!("  \"errors\": {error_count},\n"));
    report.push_str(&format!("  \"panics\": {panic_count},\n"));
    report.push_str(&format!("  \"ui_events\": {ui_event_count},\n"));

    if error_count == 0 && panic_count == 0 {
        report.push_str("  \"status\": \"healthy\"\n");
    } else {
        report.push_str("  \"status\": \"issues_found\"\n");
    }

    report.push_str("}\n");
    report
}
