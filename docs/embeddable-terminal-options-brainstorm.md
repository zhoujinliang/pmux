# 可嵌入终端组件选项（Brainstorm）

> 目标：在 pmux (Rust + GPUI) 中嵌入终端，尽量少管渲染管线

---

## 一、与 GPUI 天然兼容

| 组件 | 类型 | 解析 | 渲染 | 说明 |
|------|------|------|------|------|
| **gpui-terminal** | Rust crate | alacritty_terminal | GPUI 自带 | 专为 GPUI 设计，`TerminalView` 实现 `Render`，接受任意 `Read`/`Write` 流。支持 ANSI、真色、OSC 52、动态配置。**限制**：无鼠标选文、无 scrollback 导航。gpui ^0.2.2，需确认与 pmux 的 gpui (Zed git) 兼容性。 |
| **alacritty_terminal** | Rust crate | ✓ | ✗ | 解析+状态，无渲染。pmux 当前用法。 |

---

## 二、完整终端库（需 FFI / 平台集成）

| 组件 | 语言 | 解析 | 渲染 | 说明 |
|------|------|------|------|------|
| **libghostty** | Zig/C | ✓ | ✓ | Ghostty 核心，C API（内部），OrbStack/cmux 已用。需预编译 .so，`ghostty-sys` 有 bindgen。要解决与 GPUI 窗口/渲染的集成。 |
| **libghostty-vt** | Zig | ✓ | ✗ | 仅解析+状态，C API 规划中。适合自渲染场景。 |
| **libvte** | C (GNOME) | ✓ | ✓ | GTK 终端 widget，完整嵌入。依赖 GTK，Linux 为主，与 GPUI 不直接兼容。 |
| **Contour** (vtbackend) | C++ | ✓ | ✓ | 模块化：vtparser、vtbackend、vtrasterizer 等，可嵌入。C++ 集成，Rust 需 cxx 或 bindgen。 |

---

## 三、Web / 跨端

| 组件 | 平台 | 说明 |
|------|------|------|
| **xterm.js** | 浏览器 | VS Code、Jupyter、Theia 等在用。在原生 app 里需 WebView，架构重。 |
| **JediTerm** | Java | JetBrains 系终端，JVM 生态。 |

---

## 四、应用级（非库）

| 组件 | 说明 |
|------|------|
| **WezTerm** | Rust 终端+multiplexer，无公开嵌入 API。 |
| **Foot** | Wayland 终端，有 `terminal.h` 但非设计为库，集成需直接拉源码。 |
| **Alacritty** | 独立应用，核心在 alacritty_terminal，无嵌入 API。 |

---

## 五、特殊用途

| 组件 | 说明 |
|------|------|
| **embedded-term** | no_std，嵌入式/内核，基于 embedded-graphics，功能有限。 |
| **portable-pty** | 仅 PTY，无解析/渲染，pmux 已用。 |

---

## 六、对 pmux 的推荐排序

1. **gpui-terminal** — 与 GPUI 最契合，若版本兼容可直接替换当前 `TerminalEngine`+自渲染。
2. **libghostty** — 完整终端，成熟度较高，代价是 C FFI 与 GPUI 集成。
3. **继续 alacritty_terminal + 自渲染** — 现状，可先抽象接口再考虑迁移。
4. **libghostty-vt** — 等 C API 稳定后，替代 alacritty_terminal 解析层。

---

## 七、gpui-terminal 快速验证

```bash
# 检查 gpui-terminal 与 pmux 的依赖兼容
cargo tree -p gpui-terminal
# 对比 pmux: gpui (git), alacritty_terminal 0.25.1, flume 0.11, portable-pty 0.8
```

若 gpui 版本冲突，可 fork gpui-terminal 适配 pmux 的 gpui 来源。
