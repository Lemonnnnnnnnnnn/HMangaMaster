## HMangaMaster 后端迁移到 Tauri/Rust 的实施计划（v1）

本计划将 `ImageMaster` 的 Go/Wails 后端重构为 `HMangaMaster` 的 Tauri/Rust 后端；前端已迁入，仅需替换原 `wailsjs` 导出函数为 Tauri `invoke`，以及事件监听改为 Tauri `listen`。

### 目标与范围
- 将 `D:\code\mine\ImageMaster\core` 下的后端能力按模块迁移到 `D:\code\mine\HMangaMaster\src-tauri\src`，保持对前端的 API/DTO/事件语义基本一致。
- 优先保证基础功能可用：配置、日志、库扫描、历史；随后实现下载/任务；最后实现爬虫解析器。
- 事件命名与主要 RPC 映射保持不变（或最小差异），降低前端改动量。

### Rust 模块结构（建议目录）
```
src-tauri/src/
  app.rs                # 全局状态 AppState（logger/config/history/library/tasks）
  commands.rs           # #[tauri::command] 对外 RPC 映射（与 Wails API 对应）
  types/                # 公共类型与 DTO
    mod.rs
    dto.rs
  utils/
    mod.rs
    semaphore.rs        # 基于 tokio::sync::Semaphore
    path.rs             # 数据/配置目录、路径工具
  logger/
    mod.rs              # tracing/tracing-subscriber 初始化与滚动日志
    api.rs              # get_log_info
  config/
    mod.rs
    manager.rs          # JSON 配置 + 目录选择对话框
  request/
    mod.rs
    client.rs           # reqwest::Client 包装，默认头/Cookie/代理/HTTP2
  library/
    mod.rs
    manager.rs          # 库扫描、排序、DataURL
  history/
    mod.rs
    manager.rs          # download_history.json 持久化
  download/
    mod.rs
    downloader.rs       # 单/批量下载、重试、并发、取消、进度
  task/
    mod.rs
    manager.rs          # 任务生命周期、事件派发
    model.rs
    updater.rs
  crawler/
    mod.rs
    factory.rs          # host 识别/创建
    parsers/            # 各站点解析器
```

### RPC 与事件映射（Wails → Tauri）
- 事件（保持名称/新增）：
  - `download:completed`, `download:cancelled`, `download:failed`（必要时新增 `download:progress`）
- Config（对应 `core/config/api.go`）：
  - `GetActiveLibrary` → `config_get_active_library()`
  - `SetActiveLibrary(library)` → `config_set_active_library(library)`
  - `GetOutputDir()` → `config_get_output_dir()`
  - `SetOutputDir()` → `config_set_output_dir()`（Tauri 目录对话框）
  - `GetProxy()` → `config_get_proxy()`
  - `SetProxy(proxy)` → `config_set_proxy(proxy)`
  - `GetLibraries()` → `config_get_libraries()`
  - `AddLibrary()` → `config_add_library()`（Tauri 目录对话框）
- Library（对应 `core/library/api.go`）：
  - `InitializeLibraryManager()` → `library_init()`
  - `LoadLibrary(path)` → `library_load(path)`
  - `LoadActiveLibrary()` → `library_load_active()`
  - `LoadAllLibraries()` → `library_load_all()`
  - `GetAllMangas()` → `library_get_all_mangas()`
  - `GetMangaImages(path)` → `library_get_manga_images(path)`
  - `DeleteManga(path)` → `library_delete_manga(path)`
  - `GetImageDataUrl(path)` → `library_get_image_data_url(path)`
- History（对应 `core/history/api.go`）：
  - `GetDownloadHistory()` → `history_get()`
  - `AddDownloadRecord(task)` → `history_add(record)`
  - `ClearDownloadHistory()` → `history_clear()`
- Crawler/Task：
  - `StartCrawl(url)` → `task_start_crawl(url) -> String`（taskId）
  - `CancelCrawl(taskId)` → `task_cancel(task_id) -> bool`
  - `GetAllTasks()` → `task_all()`
  - `GetActiveTasks()` → `task_active()`
  - `GetHistoryTasks()` → `task_history()`
  - `ClearHistory()` → `task_clear_history()`
  - `GetTaskByID(taskId)` → `task_by_id(task_id)`
  - `GetTaskProgress(taskId)` → `task_progress(task_id)`
- Logger：
  - `GetLogInfo()` → `logger_get_info()`

### 优先级与里程碑
1) 核心共享层（优先）
   - types / utils / logger / request / config
2) 低耦合功能
   - library / history（前端 Home/Library/History 页可用）
3) 下载与任务
   - download / task（事件派发与取消）
4) 爬虫
   - crawler 工厂 + generic 站点，后续逐站点移植（ehentai, nhentai, hitomi...）
5) TLS/反爬增强（按需）

### 实施步骤（可度量输出）
- 里程碑 1：骨架 + 基础命令
  - [x] 建立模块与 `AppState`，注册 commands（`app.rs`、`lib.rs`、`commands.rs`）
  - [x] 实现 logger 初始化与 `logger_get_info`
  - [x] 实现 config 读写与目录选择（`config_*` 全部）
- 里程碑 2：库与历史
  - [x] library 扫描/排序/图片 DataURL（`library_*` 全部）
  - [x] history JSON 持久化（`history_*` 全部）
  - [x] 前端 Home/History 页改造为 `invoke` 调用
- 里程碑 3：网络与下载
  - [x] request::Client（默认头/Cookie/代理/HTTP2）
  - [x] downloader：单/批量下载、重试/延时、并发控制（CancellationToken 可取消）
  - [x] 事件：下载进度、完成/取消（emit）
- 里程碑 4：任务与事件
  - [x] task manager：任务 Map、取消、进度与完成事件（emit）
  - [x] 任务 RPC：`task_history` / `task_progress` 已实现并注册（后端），新增解析阶段状态 `parsing`
  - [x] 历史写入：任务完成/失败/取消均写入 `download_history.json`（解析失败/取消也会记录），与 Go 端行为一致
  - [x] 前端事件监听替换并联调
- 里程碑 5：爬虫
  - [x] crawler 工厂 + host 注册表，未匹配到任何解析器时，返回报错并在前端弹出 toast 提示
  - [x] 站点解析器分阶段移植（ehentai / nhentai / hitomi / ...）
    - 已完成：Telegraph、eHentai（含 ExHentai 基本兼容）、Nhentai、Hitomi、Wnacg、Comic18（均已模块化并注册 host 规则）
    - 已完成：并发抓取（eHentai/wnacg 已使用 `buffer_unordered` 并发解析分页与图片页，限流并发=8）。
    - [x] 解析阶段的任务进度上报（`task_start_crawl` 实现 `ProgressReporter`，通过 `download:progress` 事件上报 `parsingTotal`/`parsingProgress`；且将 `parsing` 视为活跃任务并保留在内存任务列表）。
    - [x] 解析器进度接入：Nhentai / Wnacg / 18Comic 已在解析阶段上报进度（stage: `parsing:images` 等），与 eHentai 一致。
    - 差异与对齐说明：
      - eHentai：并发抓取与解析阶段进度上报已接入（`parsingTotal`/`parsingProgress`）；URL 解析语义与 Go 端一致。
      - Telegraph：对相对路径 `src` 自动补全为 `https://telegra.ph/`，并去重排序，语义与 Go 端一致（解析阶段无需进度）。
      - Nhentai：基于缩略图 data-src 转原图，现已上报解析阶段进度；已模块化到 `crawler/parsers/nhentai.rs` 并注册 host 规则。
      - Wnacg：并发抓取与解析阶段进度均已接入；与 Go 端解析语义一致。
      - 18Comic：实现 `.scramble-page > img` 的 `data-original`/`src` 抓取，现已上报解析阶段进度。
      - Hitomi：
        - 已对齐下载阶段的 Referer 头（`https://hitomi.la/`），避免 403。
        - 生成图片 URL 的 `gg.b` 已解析；`gg.s(hash)` 路径映射当前采用近似实现，通常可用；若遇 404 将进一步精准移植 gg.js 的算法（或内嵌 JS 运行时）。
- 里程碑 6：检查
  - [x] 再次检查并调整，使 ImageMaster 和 HMangaMaster 的功能保持一致
    - 对齐事件派发：失败与取消分别派发 `download:failed` 与 `download:cancelled`，不再在失败时误发 `download:completed`。
    - 历史 DTO 与任务状态字段均采用 camelCase，时间字符串为 RFC3339，与前端展示逻辑匹配。

### 关键设计要点
- 取消模型：`tokio` + `Semaphore` 控并发，任务使用 `CancellationToken` 取消，语义对齐 Go 的 `context`。
- 事件派发：`app_handle.emit_all("download:completed", payload)` 与 `download:cancelled`，按需新增 `download:progress`。
- 配置与数据路径：使用 Tauri `path_resolver`；历史文件名与结构对齐 `download_history.json`。
- TLS 指纹：首版使用 `reqwest + rustls + h2`；如遇拦截再评估 `isahc`/无头浏览器方案。
  - 变更：移除强制 `http2_prior_knowledge`，避免对普通 HTTPS 站点错误使用 h2c，兼容性更好。

### 验收标准（每里程碑）
- 提供 commands 的集成测试/手动验证清单；前端页面可完成相应操作（含事件响应）。
- 日志可在文件中查看，配置可持久化，历史可增删查，下载/取消与事件可观测。

### 变更与版本化
- 本文件作为“单一事实来源”，遇到实现差异或阻塞需在此更新：
  - 记录原因、替代方案与影响面
  - 更新里程碑/任务清单
  - 打上版本：v1, v1.1, ... （文件顶部）

### 附：前端最小改造指南
- RPC：`wailsjs` → `@tauri-apps/api/core` `invoke("command", payload)`
- 事件：`Wails EventsOn` → `@tauri-apps/api/event` `listen("event", cb)`；事件名不变
- DTO：保持字段与时间格式对齐（`DownloadTaskDTO` 等，已采用 camelCase，通过 serde rename）


