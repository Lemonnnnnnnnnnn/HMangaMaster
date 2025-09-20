pub mod common;  // 共享工具和RequestContext
pub mod telegraph;
pub mod ehentai;
pub mod nhentai; // Nhentai 解析器
pub mod hitomi;
pub mod wnacg;   // Wnacg 解析器
pub mod comic18; // 18comic 解析器
pub mod pixiv;   // Pixiv 解析器

pub fn register_all() {
    // 分模块注册站点解析器与 host 匹配器
    telegraph::register();
    ehentai::register();
    nhentai::register();
    hitomi::register();
    wnacg::register();
    comic18::register();
    pixiv::register();
}


