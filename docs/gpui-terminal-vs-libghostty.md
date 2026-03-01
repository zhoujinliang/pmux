# gpui-terminal vs libghostty 对比

---

## 一、概览

| 维度 | gpui-terminal | libghostty |
|------|---------------|------------|
| **定位** | GPUI 专用终端组件 | 跨语言/跨平台可嵌入终端库 |
| **语言** | 纯 Rust | Zig + C API |
| **解析** | alacritty_terminal (VTE) | Ghostty 自研 (SIMD 优化) |
| **渲染** | GPUI（与 pmux 同栈） | Metal/OpenGL（自带） |
| **依赖** | gpui, alacritty_terminal, portable-pty | 预编译 libghostty.so，ghostty-sys |
| **生态** | 24 stars，2025-12 发布 | Ghostty/OrbStack/cmux 使用，成熟 |

---

## 二、集成复杂度

| 项目 | gpui-terminal | libghostty |
|------|---------------|------------|
| **与 pmux 集成** | 直接 `TerminalView::new(reader, writer, config)`，接受任意 Read/Write 流 | 需 C FFI、预编译库、处理与 GPUI 的窗口/渲染整合 |
| **与 tmux 配合** | 流式：`pipe-pane` → fifo → Read；输入 → Write。与 pmux 现有模式一致 | 同上，但 libghostty 若自带 PTY 封装，可能需对接其 I/O 模型 |
| **GPUI 版本** | gpui 0.2.2 (crates.io)；pmux 用 Zed git，可能需 fork 适配 | 与 GPUI 无关，但要解决「libghostty 渲染面」如何嵌入 GPUI 窗口 |
| **构建** | `cargo add gpui-terminal` | 需 Zig、GHOSTTY_LOCATION、动态库 |

---

## 三、能力对比

| 能力 | gpui-terminal | libghostty |
|------|---------------|------------|
| ANSI / 256 色 / 真色 | ✓ | ✓ |
| Bold/italic/underline | ✓ | ✓ |
| OSC 52 剪贴板 | ✓ | ✓ (需确认) |
| Kitty Graphics | 依赖 alacritty | ✓ 原生支持 |
| Tmux Control Mode | 依赖 alacritty | ✓ 原生支持 |
| 鼠标选文 | ✗ 未实现 | ✓ |
| Scrollback 导航 | ✗ 未实现 | ✓ |
| 字体/连字 | GPUI 管线 | Ghostty 管线 |
| 性能 | 依赖 alacritty + GPUI | SIMD 解析，Metal 渲染 |

---

## 四、适用场景

| 场景 | 更合适 |
|------|--------|
| 快速替换 pmux 现有终端、少改架构 | **gpui-terminal** |
| 需要最高兼容性（Kitty/Tmux 协议） | **libghostty** |
| 不想碰 C/FFI，纯 Rust | **gpui-terminal** |
| 想要完整终端能力（选文、scrollback 等） | **libghostty** |
| 与 GPUI 渲染完全统一 | **gpui-terminal** |
| 跨平台、跨 UI 框架复用 | **libghostty** |

---

## 五、风险与代价

| 项目 | gpui-terminal | libghostty |
|------|---------------|------------|
| **维护** | 社区项目，作者单主力 | Ghostty 官方，生态更大 |
| **API 稳定** | 0.1.0，可能变动 | 内部 API，官方称不稳定 |
| **pmux 改动量** | 替换 TerminalEngine + 渲染链路，保留 I/O 层 | 替换整条终端管线，外加 FFI 与窗口嵌入 |
| **tmux 问题** | 不解决 capture-pane/pipe-pane 拼接等结构问题 | 若放弃 tmux、直接用 PTY，可避免；若仍用 tmux，问题依旧 |

---

## 六、结论

| 若你优先考虑… | 建议 |
|---------------|------|
| 实现快、改动小、少踩坑 | **gpui-terminal** |
| 终端能力、协议兼容、长期投入 | **libghostty** |
| 先验证「少管渲染」的可行性 | **gpui-terminal** 做 PoC |

**建议路径**：先用 gpui-terminal 替换现有管线，验证效果；若后续需要 Kitty/Tmux 协议或更强终端能力，再评估 libghostty。两者都依赖 tmux 时，结构性问题不会自动消失，需单独处理。
