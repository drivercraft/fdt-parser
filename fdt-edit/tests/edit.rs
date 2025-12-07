#![cfg(unix)]

use dtb_file::*;
use fdt_edit::*;
use std::fs;
use std::process::Command;

#[test]
fn test_parse_and_rebuild() {
    // 解析原始 DTB
    let raw_data = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw_data).unwrap();
    let fdt_data = fdt.encode();

    // 创建临时文件
    let temp_dir = std::env::temp_dir();
    let original_dtb_path = temp_dir.join("original.dtb");
    let rebuilt_dtb_path = temp_dir.join("rebuilt.dtb");
    let original_dts_path = temp_dir.join("original.dts");
    let rebuilt_dts_path = temp_dir.join("rebuilt.dts");

    // 清理函数
    let cleanup = || {
        let _ = fs::remove_file(&original_dtb_path);
        let _ = fs::remove_file(&rebuilt_dtb_path);
        let _ = fs::remove_file(&original_dts_path);
        let _ = fs::remove_file(&rebuilt_dts_path);
    };

    // 确保清理临时文件
    cleanup();

    // 保存原始数据和重建数据到临时文件
    fs::write(&original_dtb_path, &*raw_data).expect("无法写入原始DTB文件");
    fs::write(&rebuilt_dtb_path, &fdt_data).expect("无法写入重建DTB文件");

    // 检查dtc命令是否可用
    let dtc_check = Command::new("dtc").arg("--version").output();

    if dtc_check.is_err() {
        cleanup();
        panic!("dtc命令不可用，请安装device-tree-compiler");
    }

    // 使用dtc将DTB文件转换为DTS文件
    let original_output = Command::new("dtc")
        .args([
            "-I",
            "dtb",
            "-O",
            "dts",
            "-o",
            original_dts_path.to_str().unwrap(),
        ])
        .arg(original_dtb_path.to_str().unwrap())
        .output()
        .expect("执行dtc命令失败（原始文件）");

    if !original_output.status.success() {
        cleanup();
        panic!(
            "dtc转换原始DTB失败: {}",
            String::from_utf8_lossy(&original_output.stderr)
        );
    }

    let rebuilt_output = Command::new("dtc")
        .args([
            "-I",
            "dtb",
            "-O",
            "dts",
            "-o",
            rebuilt_dts_path.to_str().unwrap(),
        ])
        .arg(rebuilt_dtb_path.to_str().unwrap())
        .output()
        .expect("执行dtc命令失败（重建文件）");

    if !rebuilt_output.status.success() {
        cleanup();
        panic!(
            "dtc转换重建DTB失败: {}",
            String::from_utf8_lossy(&rebuilt_output.stderr)
        );
    }

    // 读取生成的DTS文件并进行逐字对比
    let original_dts = fs::read_to_string(&original_dts_path).expect("无法读取原始DTS文件");
    let rebuilt_dts = fs::read_to_string(&rebuilt_dts_path).expect("无法读取重建DTS文件");

    // 进行逐字对比
    if original_dts != rebuilt_dts {
        println!("原始DTS文件内容:\n{}", original_dts);
        println!("\n重建DTS文件内容:\n{}", rebuilt_dts);

        // 找到第一个不同的位置
        let original_chars: Vec<char> = original_dts.chars().collect();
        let rebuilt_chars: Vec<char> = rebuilt_dts.chars().collect();

        let min_len = original_chars.len().min(rebuilt_chars.len());
        let mut diff_pos = None;

        for i in 0..min_len {
            if original_chars[i] != rebuilt_chars[i] {
                diff_pos = Some(i);
                break;
            }
        }

        match diff_pos {
            Some(pos) => {
                let context_start = pos.saturating_sub(50);
                let context_end = (pos + 50).min(min_len);

                println!("\n发现差异，位置: {}", pos);
                println!(
                    "原始文件片段: {}>>>DIFF<<<{}",
                    &original_dts[context_start..pos],
                    &original_dts[pos..context_end]
                );
                println!(
                    "重建文件片段: {}>>>DIFF<<<{}",
                    &rebuilt_dts[context_start..pos],
                    &rebuilt_dts[pos..context_end]
                );
            }
            None => {
                if original_chars.len() != rebuilt_chars.len() {
                    println!(
                        "文件长度不同: 原始={}, 重建={}",
                        original_chars.len(),
                        rebuilt_chars.len()
                    );
                }
            }
        }

        cleanup();
        panic!("原始DTS和重建DTS不完全匹配");
    }

    // 清理临时文件
    cleanup();

    println!("✅ 测试通过：原始DTB和重建DTB的DTS表示完全一致");
}

// TODO: 需要为 Fdt 实现 Display trait
// #[test]
// fn test_display_dts() {
//     // 解析 DTB
//     let raw_data = fdt_qemu();
//     let fdt = Fdt::from_bytes(&raw_data).unwrap();

//     // 使用 Display 输出 DTS
//     let dts = format!("{}", fdt);

//     // 验证输出格式
//     assert!(dts.starts_with("/dts-v1/;"), "DTS 应该以 /dts-v1/; 开头");
//     assert!(dts.contains("/ {"), "DTS 应该包含根节点");
//     assert!(dts.contains("};"), "DTS 应该包含节点闭合");

//     // 验证包含一些常见节点
//     assert!(dts.contains("compatible"), "DTS 应该包含 compatible 属性");

//     println!("✅ Display 测试通过");
//     println!("DTS 输出前 500 字符:\n{}", &dts[..dts.len().min(500)]);
// }
