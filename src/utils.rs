use anyhow::{anyhow, Result};
use std::path::{ PathBuf};

/// 查找INF文件，支持子目录搜索
pub fn find_inf_file(filename: &str) -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    
    // 首先检查当前目录
    let current_path = current_dir.join(filename);
    if current_path.exists() {
        return Ok(current_path);
    }
    
    // 递归搜索子目录
    for entry in walkdir::WalkDir::new(&current_dir)
        .max_depth(5) // 限制搜索深度
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() && entry.file_name() == filename {
            return Ok(entry.path().to_path_buf());
        }
    }
    
    Err(anyhow!("未找到INF文件: {} (已搜索当前目录及子目录)", filename))
}

/// 获取Windows最后错误信息的现代化实现
/// 注意：这个函数主要用于那些不返回 Result 的旧式 IOCTL 调用，
/// 对于大多数 windows-rs 函数，直接处理返回的 Error 会更好。
pub fn get_last_error() -> String {
    // Error::from_win32() 会自动调用 GetLastError() 并获取对应的错误信息
    let error = windows::core::Error::from_thread();
    format!("[WinError {}] {}", error.code().0, error.message())
}
