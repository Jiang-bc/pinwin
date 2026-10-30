# PinWin Rust Modernization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 交付保留 PinWin 功能、使用现代 Rust 依赖和实际 AI 重绘图标的独立 Windows 原生应用。

**Architecture:** 沿用原生 Win32 消息循环和现有 Rust 业务接口，替换手写 ABI 与 JSON 解析，明确系统资源所有权和窗口状态恢复。将 UI 按主界面、托盘和设置拆分，资产独立于旧 C# 目录，构建链生成 EXE 资源和安装产物。

**Tech Stack:** Rust 2024、windows-sys、serde/serde_json、clap、embed-resource；独立 Rust 图像转换工具使用 image；Windows 10/11 x86_64 MSVC、Inno Setup、GitHub Actions。

**Spec:** `docs/superpowers/specs/2026-10-31-rust-modernization-design.md`

## Approved amendments — override original task wording

用户最终修订：portable 唯一交付（EXE 同目录 settings.json、无安装器/无 AppData）；输出 snake_case 和空字符串禁用快捷键，旧格式兼容可选；图像 AI/程序绘制酌情；授权本地验证通过后自动提交及推送新开发分支。任务 1 按此实现并测试路径；任务 5 使用 AI 主图标搭配程序绘制光标/徽标；任务 7 删除 install.iss，生成 portable ZIP。每个任务本地全套测试通过后提交，最终本地实机验证与审查通过后再推送。Native 在当前目录新分支执行，无需另外询问执行方法。

## Global Constraints

- Windows 10/11、x86_64 MSVC、原生桌面控件、现有英文界面文案。
- 保留 PinWin 名称、MIT 许可证及原作者署名；不默认以管理员身份运行。
- Rust 构建、测试、安装包不再依赖 C# 项目目录。
- 保留 CTRL + F11 / CTRL + F12、旧 PascalCase 配置字段和 null 禁用语义。
- 优先读取 %APPDATA%\PinWin\settings.json，不存在时读取工作目录 settings.json。
- 保留透明度范围 30..=255；未知配置字段兼容。
- 仅管理本应用拥有的置顶记录；取消全部与退出不得改动其他应用的无关窗口。
- 图像必须实际通过 AI 工具生成，保留源图、提示词和可复现转换过程；生产构建不依赖 Python 或在线生成。
- 保留安装器 AppId {712FE69B-B957-41FE-83F0-CDA7393444BF}。
- 新本地分支工作；本任务不自动提交、推送、发布或创建远端 PR。各任务结束检查 diff，代替技能示例中的提交步骤。
- 最新依赖版本在实施时核实。已查询参考版本 windows-sys 0.61.2、serde_json 1.0.154、clap 4.6.7、embed-resource 3.0.12、image 0.25.10；只有兼容且实际使用的依赖进入清单。

## Review Focus

1. 外部窗口原本已置顶、带色键或半透明：取消 PinWin 管理后恢复已读取的原始属性；任务 3 覆盖。
2. 钩子在 DestroyWindow 或 SetWindowPos 中同步重入、旧 HWND 被复用：不得借用崩溃或改动新窗口；任务 3 覆盖。
3. 配置包含代理对转义、相对路径、负数或超大透明度，写入中发生替换失败：解析兼容且旧文件完整；任务 1 覆盖。
4. CLI 多个动作、孤立 --opacity、已有无关置顶窗口：参数错误明确，批量操作限定所有权；任务 4 覆盖。
5. 图标文件有效但不能按光标语义加载，或缩小后丢失对比度：检查资源类型、热点、alpha 和不同 DPI 尺寸；任务 5、6 覆盖。

## File Structure

- `Cargo.toml` / `Cargo.lock`：主应用的运行及构建依赖，2024 edition 和实测最低工具链。
- `src/settings.rs`、`src/settings/shortcut_serde.rs`：兼容旧配置的序列化、路径选择及原子保存。
- `src/winapi/ffi.rs`：仅重导出官方绑定及几何扩展 trait，删除自定义 ABI；完成后可按使用情况进一步收窄导入。
- `src/winapi/{mod,resources,window_state}.rs`：窗口操作、拥有资源的 RAII、外部窗口身份和状态快照。
- `src/pin_manager.rs`、`src/overlay.rs`：管理记录、钩子通知、徽标和选窗生命周期。
- `src/cli.rs`、`src/main.rs`：参数解析、显式窗口动作、受限批量动作、控制台附加和启动。
- `src/ui/{mod,main_window,tray,settings}.rs`：由现有 `src/ui.rs` 迁移，分别负责消息调度、管理窗口、托盘和设置。
- `assets/{source,icons}`、`assets/README.md`：AI 源图、生成说明、发布资源。
- `tools/assetgen/{Cargo.toml,src/main.rs}`：隔离图像处理依赖，确定性输出 ICO/CUR/徽标 PNG。
- `build.rs`、`resources/{pinwin.rc,pinwin.manifest}`：图标、版本及运行级别和 DPI 声明。
- `tests/{settings_compat_test,cli_test,asset_test,win32_integration_test}.rs`：配置、命令行、资源和实机窗口行为。
- `install.iss`、`.github/workflows/windows.yml`、`README.md`、`SECURITY.md`：安装、验证、交付说明。

## Task 1: 配置解析与可靠保存

**Files:** Modify `Cargo.toml`, `Cargo.lock`, `src/settings.rs`; create `src/settings/shortcut_serde.rs`, `tests/settings_compat_test.rs`.

**Interfaces:** 保留 `ApplicationSettings::from_json_str(json: &str) -> Result<Self, String>`、`to_json_pretty(&self) -> String`、`save_to_explicit_path(&self, path: &Path) -> io::Result<()>`、`load_from_file(file_name: Option<&str>) -> Self`。新增 `pub fn load_from_path(path: &Path) -> Result<Self, String>` 供需要显示错误的 UI 使用。快捷键序列化模块使用 Serde 标准 `serialize<S: Serializer>(value: &KeyCombo, serializer: S) -> Result<S::Ok, S::Error>` 和 `deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<KeyCombo, D::Error>`。

- [ ] 使用 git-worktrees 技能建立隔离工作区或确认采用本地新分支；先确认现有 docs 未丢失，记录工具链和 baseline，后续命令在选定工作目录运行。
- [ ] 在 `tests/settings_compat_test.rs` 添加 `legacy_settings_and_null_shortcuts`：旧四字段 JSON 使用 CTRL + F11、CTRL + F12；null 得到 KeyCombo::NONE；缺失字段 default_opacity == 255 且 highlight_target_on_hover == true。
- [ ] 添加 `unicode_and_malformed_json`：`中📌` 解码为 `中📌`；路径反斜杠 roundtrip 相等；`{broken}`、重复字段和非对象 JSON 返回 Err；未知字段忽略。
- [ ] 添加 `opacity_is_clamped`：-1 -> 30，999 -> 255，30 和 255 不变；采用有符号或任意精度 JSON 数值中间表示，避免先转 u8 导致解析失败。
- [ ] 添加 `atomic_save_preserves_previous_file_on_failure`：正常重复保存后内容可解析；保持目标文件独占打开使替换失败，断言返回 Err 且原字节不变；清理只删除测试自己创建的唯一临时目录。路径选择测试抽取接受 AppData/CWD 参数的纯函数，避免修改进程全局环境影响并发测试。
- [ ] 运行 `cargo test --test settings_compat_test`，确认新测试在旧解析器上暴露 Unicode/语法/保存问题。
- [ ] 引入 Serde derive、PascalCase 和 default；删除 `JsonValue`、`extract_json_field`、`escape_json_str`。同目录 create_new 临时文件、write_all、sync_all，通过 Windows 原子替换 API 完成覆盖；失败清理临时文件并保留原目标。Windows API 在任务 2 统一到官方绑定。
- [ ] 运行 `cargo test --test settings_compat_test` 与已有配置单测，全部通过；检查 diff。

## Task 2: 官方 Win32 绑定与资源所有权

**Files:** Modify `Cargo.toml`, `Cargo.lock`, `src/winapi/{ffi,mod}.rs`, all Win32 consumers and `tests/win32_integration_test.rs`; create `src/winapi/resources.rs`.

**Interfaces:** `ffi` 重导出 windows-sys 原始类型，HWND/HICON/HBITMAP 使用官方指针类型；新增 `pub trait RectExt { fn width(&self) -> i32; fn height(&self) -> i32; }`。在 resources 定义 `OwnedIcon::from_owned(raw: HICON) -> Option<Self>`、`as_raw(&self) -> HICON`，OwnedCursor、OwnedBitmap 同等模式；共享系统图标和光标不进入这些构造器。保留已有高级 Api* 调用接口，按官方类型调整内部代码。

- [ ] 给现有位图/图标加载测试补充 DestroyIcon/DestroyCursor/DeleteObject 的正确释放；新增 `owned_resources_release_on_drop`，使用 GetGuiResources 在同一测试线程重复创建释放资源后检查 GDI/USER 计数未持续增长，允许一次类注册常驻资源。
- [ ] 添加 `mutex_error_does_not_allow_second_instance`，将互斥体创建失败作为 Result 错误处理，而不是返回“首实例”；正常持锁、释放和再次获取仍工作。
- [ ] 添加 windows-sys，功能覆盖 Foundation、Gdi、Dwm、LibraryLoader、Threading、FileSystem、Console、HiDpi、Input/KeyboardAndMouse、Accessibility、Controls/Dialogs、Shell 和 WindowsAndMessaging，依编译器提示补足必要功能；不整包启用全部 API。
- [ ] 改写 `ffi.rs` 为官方重导出和 RectExt，移除自定义 extern、GUID/RECT/MSG 等 ABI。修改 null、句柄比较、HMENU 控件 ID、SendMessage 参数、回调及结构体字段初始化；避免用全局 isize 类型别名伪装迁移。
- [ ] 运行 `cargo check --all-targets`；Rust2024 下为 unsafe 操作添加局部 unsafe 块，保持回调 ABI 正确。将 edition 升至 2024，并记录 `cargo tree` 的依赖最低版本，后续任务 7 验证 rust-version。
- [ ] 资源 RAII 用于实际所有权路径，关闭 mutex、钩子、字体、内存 DC 和临时菜单；选窗 DC 使用成对 ReleaseDC/DeleteDC，错误分支同样释放。
- [ ] 运行 `cargo test --test win32_integration_test -- --test-threads=1` 与 `cargo clippy --all-targets -- -D warnings`；检查真实资源测试通过及 diff。

## Task 3: 置顶事务、恢复与事件生命周期

**Files:** Create `src/winapi/window_state.rs`; modify `src/winapi/mod.rs`, `src/pin_manager.rs`, `src/overlay.rs`, `tests/win32_integration_test.rs`.

**Interfaces:** `WindowIdentity { hwnd: HWND, process_id: u32, thread_id: u32, generation: usize }`；`WindowIdentity::capture(hwnd: HWND) -> io::Result<Self>`、`is_current(&self) -> bool`。generation 为本进程安装到目标的唯一窗口属性标记，用于区分同进程同线程的 HWND 复用；属性创建失败不宣称身份验证可靠。`WindowSnapshot::capture(hwnd: HWND) -> io::Result<Self>`、`restore(&self, identity: &WindowIdentity) -> io::Result<()>` 保存 topmost、layered/transparent 原位、GetLayeredWindowAttributes 实际可读的 alpha/色键/flags。保留 PinManager 公开接口，entry 增加身份、快照及资源所有权。

- [ ] 添加 `unpin_restores_original_topmost_and_layered_attributes`：测试窗口预置 topmost、alpha=180 和色键，管理期间改为 alpha=100/穿透，取消后原状态恢复；另测原本未 layered 的窗口恢复后无多余 layered 位。
- [ ] 添加 `destroy_target_removes_only_its_badge` 与 `stale_identity_is_rejected`：清理不改变另一测试窗口的 topmost；移除/替换 generation 标记后旧快照不得应用。
- [ ] 添加 `pin_failure_rolls_back`，通过可控测试钩子让徽标创建失败，断言没有残留 entry 或被改变的 topmost/alpha；测试钩子仅 cfg(test) 且不干扰并发用例。
- [ ] 添加 `reentrant_notifications_are_deferred`：创建/销毁徽标和调用 SetWindowPos 的路径能收到通知，消息排空后状态正确且不出现 RefCell already borrowed panic。
- [ ] 运行相关集成测试，记录旧实现在原始状态恢复上的失败。
- [ ] 实现 capture -> apply -> create badge -> publish entry 的事务；任一步失败回滚。移出 entry 后再做外部销毁/恢复，避免持有 RefCell 可变借用期间触发回调。不可读取原属性的 layered 窗口，不覆盖其不可恢复的透明效果；给出可解释错误。
- [ ] WinEvent 回调仅 PostMessage 通知 UI；主线程使用身份校验处理移动/销毁，移除过期 entry。保留低频健康检查，不允许 panic 穿过 extern system 回调。窗口关闭、程序退出和 Escape 取消遮罩都走同一资源释放路径。
- [ ] 运行完整 Win32 集成测试，核对负坐标矩形和超时标题测试；检查 diff。

## Task 4: 现代 CLI 与 GUI 启动行为

**Files:** Modify `src/{cli,main}.rs`, `src/pin_manager.rs`; create `tests/cli_test.rs`; UI 主消息处理预留任务 6 移动。

**Interfaces:** 保留 `CliAction`、`parse_cli_args<I, S>(args: I) -> Result<CliAction, String>` 和 `execute_cli_action(action: CliAction) -> i32`；clap 结构为私有实现。新增 `pub fn request_unpin_all() -> Result<usize, String>`，通过有超时的 SendMessageTimeoutW 向同一登录会话的已运行主窗口请求 WM_APP 消息，在 UI 线程调用 PinManager::unpin_all，返回实际数量。

- [ ] 添加 `legacy_aliases_and_handle_formats`：/?、-v、--tray/--minimized、--list/--list-windows、十进制/十六进制 HWND，符合原有 CliAction；指针转换不得截断。
- [ ] 添加 `conflicting_actions_and_orphan_opacity_are_errors`：--list 与 --pin-hwnd 冲突、孤立 --opacity、缺参数、未知参数返回 Err；合法透明度 clamp 到 30..=255，超出 u8 的输入报错，与帮助一致。
- [ ] 添加 `unpin_all_leaves_unrelated_topmost_untouched`：无主程序时返回清晰错误且不改变外部测试窗口；主消息处理的真实窗口测试只取消 PinManager 已有记录。不要改为枚举全部桌面 topmost。
- [ ] 运行 `cargo test --test cli_test` 和 CLI 单测，确认参数冲突在旧实现上失败。
- [ ] 引入 clap，保留旧命令名与退出码 0 成功/1 操作失败/2 参数错误。显式 --pin-hwnd、--unpin-hwnd 保留直接窗口动作；帮助解释它们不创建 GUI 管理记录，不能假称跨进程保存原始状态。
- [ ] 主程序仅 GUI 路径进入单实例互斥；所有 CLI 路径先 AttachConsole(ATTACH_PARENT_PROCESS)，GUI 使用 windows_subsystem=windows，无额外终端。版本去掉硬编码 GNU 平台。
- [ ] `cargo build --release` 后用 PowerShell 实际运行 EXE 的 --help、--version、--print-settings、错误参数并等待进程，重定向 stdout/stderr 验证文本与退出码；GUI 启动无附加控制台在任务 7 实机检查。

## Task 5: AI 图标与可重复资源生成

**Files:** Create `assets/source/*`, `assets/icons/{app.ico,tray.ico,pinned.png,target.cur}`, `assets/README.md`, `tools/assetgen/{Cargo.toml,Cargo.lock,src/main.rs}`, `tests/asset_test.rs`.

**Interfaces:** 独立工具命令 `cargo run --manifest-path tools/assetgen/Cargo.toml -- --source assets/source/pin.png --target assets/source/target.png --out assets/icons`。输出 app.ico 包含 16/20/24/32/48/64/128/256，tray.ico 包含 16/20/24/32，pinned.png 为 64x64 alpha PNG，target.cur 包含 32/48/64 且每帧热点位于中心 (size/2,size/2)。工具 image 依赖只开启 PNG/ICO 必需格式。

- [ ] 调用可用 AI 图像工具，生成透明背景、无文字的红色斜向图钉；生成匹配的高对比度瞄准光标。请求将源文件保存到上述 source 路径；工具仍运行时等完成通知，不编造图像生成结果。
- [ ] 用图片读取工具查看源图；确认单个主体、无棋盘格伪透明、完整尖端、留白一致。若工具无法输出真实 alpha，确定性移除纯色背景并在说明记录处理方法。
- [ ] 在 `tests/asset_test.rs` 添加 `ico_and_cursor_directories_are_valid`：断言类型 1/2、帧数、每帧边界、尺寸、CUR 热点。添加 `badge_has_transparent_and_opaque_pixels`，验证实际 alpha 0 和 >0 均存在。
- [ ] 运行资源测试，确认资源缺失时失败。
- [ ] 实现工具 `fn main() -> Result<(), Box<dyn std::error::Error>>`：使用 alpha 感知缩放，同一输入字节输出稳定；将 image 的 ICO 帧写为标准目录，CUR 使用类型 2 及热点字段，帧数据可复用支持的 DIB/PNG 编码。清理小尺寸边缘，避免透明 RGB 渗色。
- [ ] 运行转换工具两次比对 SHA256；运行 `cargo test --test asset_test`；在浅/深背景上展示 16/20/24/32 图标预览并实际读取图像确认识别度。
- [ ] 写 `assets/README.md` 记录模型、完整提示词、源图路径、转换命令、各角色及热点。完成独立资产后才能移除旧资源。

## Task 6: UI 拆分与新资源接入

**Files:** Replace `src/ui.rs` with `src/ui/{mod,main_window,tray,settings}.rs`; modify `src/{overlay,pin_manager}.rs`, `src/winapi/{mod,resources}.rs`, asset/integration tests; replace `tests/regression_source_test.rs` 的字符串断言。

**Interfaces:** `ui::run_gui_app(start_minimized: bool) -> i32` 保留；父模块持有原有 MainUiState、SettingsUiState 与 thread_local 状态，共享项使用 pub(super)，不跨线程传原始 UI 句柄。`resources::load_embedded_icon(bytes: &[u8], size: i32) -> io::Result<OwnedIcon>`、`load_embedded_cursor(bytes: &[u8], size: i32) -> io::Result<OwnedCursor>`、`load_badge_bitmap() -> io::Result<OwnedBitmap>` 负责内置资源。保持自定义图标加载和可靠回退。

- [ ] 添加 `embedded_resources_load_at_supported_sizes`，真实调用 Windows 加载 ICO 与 CUR；GetIconInfo 断言 cursor fIcon == FALSE 且热点缩放正确，清理返回的 mask/color 位图。
- [ ] 添加 `selection_model_preserves_explicit_selection`，将列表选择状态更新提取为纯函数，验证刷新后只保留原先选中的 HWND；替换依赖旧 ui.rs 文本布局的源码检查。
- [ ] 添加 `hover_target_changes_on_pointer_move`，为 overlay 的目标解析提供受控窗口和位置输入，断言移动会更新目标，负坐标矩形仍可表示；真实 WM_MOUSEMOVE 联调在任务 7 完成。
- [ ] 拆分既有函数并保持消息 ID、快捷键与行为；托盘模块负责 Shell_NotifyIcon、TaskbarCreated 重建和图标生命周期；设置模块负责草稿、预览、快捷键冲突和保存错误反馈。
- [ ] 将 include_bytes! 切换至 assets/icons；徽标通过带 alpha 的图标/位图绘制，去掉 BMP 色键假设。根据窗口 DPI 使用 MulDiv/系统缩放计算徽标及图标尺寸；处理 WM_DPICHANGED。CUR 正确解码热点，不强制把 HICON 转 HCURSOR。
- [ ] 设置 Save 仅在写入成功后应用；若快捷键冲突，展示错误并保留可用注册。替换托盘、设置预览或关闭对话框时释放旧资源。用户自定义无效图标不崩溃并显示回退状态。
- [ ] 运行 `cargo test`、`cargo fmt --check`、严格 Clippy；确认每个旧 UI 行为仍有对应处理路径，检查 diff。

## Task 7: 发布工程、旧工程清理与最终验证

**Files:** Create `build.rs`, `resources/{pinwin.rc,pinwin.manifest}`, `.github/workflows/windows.yml`; modify `Cargo.toml`, `Cargo.lock`, `install.iss`, `README.md`, `SECURITY.md`, `.gitignore`; remove legacy C# projects/solutions/NuGet/resource-only files after查阅实际目录。

**Interfaces:** `cargo build --release --locked` 产生 `target/release/PinWin.exe`；build.rs `fn main()` 调用 embed-resource 编译 resources/pinwin.rc，RC 版本信息与 Cargo 版本一致，manifest asInvoker、Common Controls v6、PerMonitorV2。安装器仅依赖该 EXE 和明确列出的许可证文件。

- [ ] 添加 EXE 资源检查：PowerShell 读取 VersionInfo，Win32 ExtractIconEx 验证应用图标；manifest 检查 asInvoker 与 DPI；不要仅凭 RC 源码存在宣称已嵌入。
- [ ] 加入 embed-resource 和资源文件，构建 release；声明 rust-version 后使用该版本工具链实际 `cargo check --locked`，不可用则选择已安装且验证过的版本并说明。不要只写未经验证的最低版本。
- [ ] 更新 install.iss：保留 AppId、桌面/自启动可选项、启动菜单及卸载；版本从 Rust EXE 获取；指向 Rust release，去掉旧 DLL/PDB/Images 必需项。先查 ISCC；存在则编译，不存在记录未生成安装器。
- [ ] 查阅 legacy 目录、项目文件和所有 include_bytes!/README/installer 引用，确认已脱钩；删除 PinWin/、PinWinTest/、PinWin.sln、PinWin.sln.DotSettings 及仅旧工程使用的配置，具体名字以实际 inventory 为准，保留 LICENSE。不要删除用户新增文件或未识别目录。
- [ ] CI 使用 windows-latest 和 MSVC，运行 cargo fmt --check、cargo clippy --all-targets --locked -- -D warnings、cargo test --locked、cargo build --release --locked；单独检查 tools/assetgen 的格式、构建和资源可再生性，上传 EXE。CI 可能无交互桌面：标记真实桌面测试并在不可运行时明确跳过，纯逻辑测试必须运行，不能把跳过记作实机通过。
- [ ] 更新文档：构建、CLI 新批量语义、旧配置兼容、AI 资产来源、GUI 启动、安装器及权限限制；移除未经实测的体积/性能断言，实际测量 release 大小并注明构建平台。
- [ ] 最终一次执行 `cargo fmt --check`、`cargo clippy --all-targets --locked -- -D warnings`、`cargo test --locked`、`cargo build --release --locked`；对独立工具执行对应格式和检查；失败必须修复后重跑受影响检查。
- [ ] 实际启动本次 EXE，使用本任务创建的测试窗口演示选择/置顶/徽标移动/取消、透明度/穿透/恢复、列表多选、设置保存及托盘菜单；记录实际观察结果。GUI 无额外控制台，退出后本次目标状态恢复且无遗留 PinWin 进程。不要终止用户已有实例。
- [ ] 核对 git diff --check、git status、旧工程引用、产物路径和资源尺寸；报告完成项、测试结果、EXE/安装器是否实际生成。混合 DPI、多显示器、UIPI 不可覆盖时列出明确人工步骤，不声称通过。

## Self-Review Record

- 范围覆盖：配置/路径/保存任务 1；ABI/资源任务 2；事务/身份/回调任务 3；CLI/批量所有权/控制台任务 4；AI 和转换任务 5；UI/托盘/光标/高 DPI 任务 6；清理/资源/安装/CI/文档/实机验证任务 7。
- 类型一致：所有平台句柄在任务 2 后使用官方 pointer handle；RectExt 在 CLI 和 UI 导入；配置兼容接口和 ui::run_gui_app 保留。
- Review Focus 五项均有对应测试；不可自动验证的视觉及真实多显示器场景明确保留人工核对。
- 实施默认建议 Native：Win32 类型迁移和 UI 状态接口耦合，单一执行者连续处理更省上下文；完成后独立审查整个分支。执行方法等待用户选择。
