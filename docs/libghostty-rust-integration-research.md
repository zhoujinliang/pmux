# libghostty 在 Rust/GPUI 中的接入调研

> 目标：保持 GPUI，用 libghostty 替换 alacritty_terminal 作为 VT 解析和终端状态层

## 一、pmux 当前对 alacritty_terminal 的依赖

| 模块 | 用途 |
|------|------|
| `alacritty_terminal::vte::ansi::Processor` | VT 序列解析，`advance(&mut term, bytes)` |
| `alacritty_terminal::term::Term` | 网格、光标、模式、历史 |
| `alacritty_terminal::term::RenderableContent` | 可渲染内容（cells、cursor、colors） |
| `alacritty_terminal::grid::GridIterator` / `Indexed<Cell>` | 迭代 cell 用于 GPUI 渲染 |
| `alacritty_terminal::vte::ansi::{Color, Rgb, CursorShape}` | 颜色、光标形状 |

pmux 自己做渲染：`TerminalEngine` 只负责解析 + 状态，`terminal_rendering.rs`、`TerminalElement` 用 GPUI 画出来。所以需要的是：**VT 解析 + 终端状态 + 可迭代的 cell 数据**，不需要 alacritty 的 GPU 渲染。

---

## 二、libghostty 生态现状

### 2.1 libghostty-vt（首选目标）

- **职责**：VT 解析 + 终端状态（cursor、styles、wrapping 等）
- **依赖**：零依赖（连 libc 都不要）
- **语言**：Zig 实现，设计为暴露 C API
- **状态**（Mitchell Hashimoto 博客，2025 年 9 月）：
  - Zig API：已有，可用于测试
  - C API：尚未就绪，计划近期推出
  - 整体：alpha，API 不稳定

与 pmux 需求高度吻合：只替换「解析 + 状态」层，渲染继续用 GPUI。

### 2.2 完整 libghostty（ghostty.h）

- **用途**：macOS 版 Ghostty、OrbStack 等产品通过 C API 嵌入整个终端
- **状态**：内部 API，官方明确表示「不是通用库」
- **ghostty-sys**：bindgen 绑定 `ghostty.h`，需要 `GHOSTTY_LOCATION` 指向预编译的 `libghostty.so`（或等价物）
- **ghostty crate**：基于 ghostty-sys 的高层封装，提供 App/Surface/Inspector 等

问题：面向「嵌入完整终端窗口」，而不是「只给解析后的 cell 数据」。要适配 pmux 的自渲染管线，需要深入理解并裁剪 API，工作量大且可能踩内部接口的雷。

### 2.3 可选路径对比

| 路径 | 依赖 | 工作量 | 风险 |
|------|------|--------|------|
| **A. 等 libghostty-vt C API** | 官方 C API | 中等 | 时间不确定，API 仍会变 |
| **B. 直接用 libghostty-vt Zig 模块** | Zig 编译器，Zig 编译产物 | 中高 | Zig↔Rust FFI 需自行设计 |
| **C. 用 ghostty-sys + 完整 libghostty** | 预编译 libghostty | 高 | 内部 API，与自渲染模型可能不匹配 |
| **D. 自行封装 libghostty-vt** | Zig 源码，自建 C 封装 | 高 | 需维护 C 封装和 Zig 编译 |

---

## 三、推荐路线

### 3.1 短期（1–3 个月）：准备 + 参与生态

1. **加入 Ghostty Discord**  
   - Mitchell 明确欢迎早期用户参与 API 设计  
   - 说明 pmux 场景：GPUI 自渲染，只需要「bytes → parsed cells」接口  

2. **抽象终端解析层**  
   - 定义 `TerminalParser` trait，例如：
     ```rust
     pub trait TerminalParser: Send + Sync {
         fn advance(&mut self, bytes: &[u8]);
         fn renderable_content(&self) -> RenderableSnapshot;  // 或迭代器
         fn resize(&mut self, cols: usize, rows: usize);
         // ...
     }
     ```
   - 先把 `alacritty_terminal` 包在实现里，为未来替换 libghostty-vt 做准备  

3. **关注 libghostty-vt 进展**  
   - 订阅 [ghostty-org/ghostty](https://github.com/ghostty-org/ghostty) 的 libghostty 相关 PR/issue  
   - C API 一出即可评估是否满足 pmux 的 cell 迭代需求  

### 3.2 中期（C API 稳定后）：实现 libghostty-vt 绑定

1. **建 `ghostty-vt-sys` crate**  
   - 用 Zig 或官方构建脚本编译 libghostty-vt  
   - bindgen 生成 C API 的 Rust 绑定  

2. **建 `ghostty-vt` 安全封装**  
   - 提供 `impl TerminalParser for GhosttyVtParser`  
   - 做一次 libghostty-vt 与 alacritty 的对比测试（vttest、大量日志回放）  

3. **在 pmux 中切换后端**  
   - 用 feature flag 或配置选择 `alacritty` / `libghostty-vt`  

### 3.3 若必须提前动手（C API 未出）

可尝试 **Zig 直接输出 C 兼容符号**：

1. 用 Zig 的 `export` 暴露 libghostty-vt 的 C 接口  
2. 在 Rust 的 `build.rs` 里调用 `zig build` 或等价命令，编译成静态库  
3. 用 `#[link(name = "ghostty_vt")]` 链接，bindgen 生成绑定  

风险：需要自行设计 C ABI，且可能与官方未来 C API 不一致，后续可能要重写一层。

---

## 四、与 tmux 的关系

接入 libghostty **不解决** tmux 带来的双状态、capture-pane 拼接等问题。

- libghostty 替代的是：**VT 解析 + 终端状态**（对应 alacritty_terminal）
- tmux 的 `pipe-pane`、`capture-pane`、`send-keys` 等逻辑仍然存在  

但 libghostty-vt 有 **Tmux Control Mode** 支持，这为未来可能的「结构化 tmux 集成」留了空间，值得在 API 设计中考虑。

---

## 五、结论

| 问题 | 建议 |
|------|------|
| 是否值得做？ | 值得，有利于长期正确性和统一终端生态 |
| 什么时候做？ | 先做抽象层和社区联系，等 libghostty-vt C API 稳定再实现绑定 |
| 能否马上换掉 alacritty？ | 不行，C API 和 Rust 绑定都不成熟 |
| tmux 问题会一起解决吗？ | 不会，需单独处理 tmux 集成架构 |

建议优先完成 **TerminalParser 抽象**，再在 Ghostty 社区表达需求并跟踪 libghostty-vt 进展。这样既降低未来迁移成本，又能在 API 设计中影响「可嵌入、可自渲染」的使用场景。
