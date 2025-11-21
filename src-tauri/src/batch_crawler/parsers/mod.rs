pub mod common;
pub mod ehentai_batch;

/// 注册所有批量解析器
pub fn register_all() {
    ehentai_batch::register();
}