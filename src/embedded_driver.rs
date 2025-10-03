use anyhow::{anyhow, Result};
use include_dir::{include_dir, Dir};
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::constants::{INF_BUS_FILE, INF_HID_FILE};

// 1. 在编译时，将项目根目录下的 "drivers" 文件夹完整地嵌入进来。
//    请确保您的驱动文件 (.inf, .sys) 都存放在这个 "drivers" 目录下。
static DRIVER_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/logi/driver");

/// 这个结构体用于管理临时驱动文件。
/// 当它被创建时，会将内置的整个驱动目录解压到临时文件夹。
/// 当它离开作用域时（RAII），会自动清理这些临时文件和目录。
pub struct TmpDriverManager {
    tmp_dir: PathBuf,
}

impl TmpDriverManager {
    pub fn new() -> Result<Self> {
        // 创建一个唯一的临时目录
        let tmp_dir = std::env::temp_dir().join(format!("logi_vhid_{}", std::process::id()));

        println!("[*] 将内置驱动释放到临时目录: {}", tmp_dir.display());

        DRIVER_DIR.extract(&tmp_dir)?;

        Ok(Self { tmp_dir })
    }

    /// 在临时目录中搜索指定的 INF 文件。
    fn find_inf_file(&self, file_name: &str) -> Result<PathBuf> {
        for entry in WalkDir::new(&self.tmp_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name().to_str() == Some(file_name) {
                return Ok(entry.path().to_path_buf());
            }
        }
        Err(anyhow!("在临时目录中未找到驱动文件: {}", file_name))
    }

    /// 获取总线驱动 .inf 文件的路径，无论它在哪个子目录。
    pub fn bus_inf_path(&self) -> Result<PathBuf> {
        
        self.find_inf_file(INF_BUS_FILE)
    }

    /// 获取 HID 驱动 .inf 文件的路径，无论它在哪个子目录。
    pub fn hid_inf_path(&self) -> Result<PathBuf> {
        self.find_inf_file(INF_HID_FILE)
    }
}

impl Drop for TmpDriverManager {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.tmp_dir).ok();
    }
}
