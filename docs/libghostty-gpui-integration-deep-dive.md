# libghostty + GPUI 集成深度调研

> 调研 ghostty-sys/ghostty API、GPUI 嵌入能力、OrbStack 参考，以及 GPUI 窗口内嵌 libghostty 终端的 PoC 可行性

---

## 一、ghostty-sys 与 ghostty crate 的 API

### 1.1 ghostty-sys

- **作用**：对 `include/ghostty.h` 的 bindgen 绑定
- **构建**：需 `GHOSTTY_LOCATION` 指向含 `libghostty.so`（或等价物）的目录
- **依赖**：Zig 编译器、预编译动态库

### 1.2 ghostty C API（ghostty.h）概要

| 类型 | 说明 |
|------|------|
| `ghostty_app_t` | 应用句柄 |
| `ghostty_surface_t` | 终端 surface |
| `ghostty_config_t` | 配置 |
| `ghostty_inspector_t` | Inspector（调试/元视图） |

**平台嵌入相关**：

```c
typedef struct {
  void* nsview;   // macOS: NSView
} ghostty_platform_macos_s;

typedef struct {
  ghostty_platform_e platform_tag;
  ghostty_platform_u platform;   // 含 nsview
  void* userdata;
  double scale_factor;
  float font_size;
  const char* working_directory;
  const char* command;
  // ...
} ghostty_surface_config_s;

ghostty_surface_t ghostty_surface_new(ghostty_app_t, const ghostty_surface_config_s*);
void ghostty_surface_set_size(ghostty_surface_t, uint32_t, uint32_t);
void ghostty_surface_draw(ghostty_surface_t);
// 输入转发
bool ghostty_surface_key(ghostty_surface_t, ghostty_input_key_s);
void ghostty_surface_text(ghostty_surface_t, const char*, uintptr_t);
void ghostty_surface_mouse_*(...);
```

**Inspector Metal 路径**（用于渲染到 Metal 层）：

```c
#ifdef __APPLE__
bool ghostty_inspector_metal_init(ghostty_inspector_t, void*);   // void* 可能为 CAMetalLayer
void ghostty_inspector_metal_render(ghostty_inspector_t, void*, void*);
bool ghostty_inspector_metal_shutdown(ghostty_inspector_t);
#endif
```

### 1.3 nsview 的语义

`ghostty_platform_macos_s.nsview` 在创建 Surface 时传入，可能有两种用法：

1. **输入**：宿主提供一个容器 NSView，libghostty 在其内创建/管理终端子视图
2. **输出**：libghostty 创建 NSView，通过该字段返回给宿主嵌入

结合 Ghostty macOS 的实现方式，更可能是 (1)：宿主提供容器，libghostty 在其中渲染。无论如何，都需要宿主能拿到或创建 NSView。

### 1.4 ghostty crate（Rust 封装）

- **已暴露**：`init()`、`Config`、`Info`、`cli_main`
- **README 中列为完成**：App、Surface、Inspector
- **实际**：`src/ghostty.rs` 只封装了 `Ghostty` 和 Config，**没有 App/Surface 的 Rust 封装**

结论：要用 libghostty 的 Surface，需直接通过 **ghostty-sys** 调用 C API，或自行在 ghostty crate 中封装 App/Surface。

---

## 二、GPUI 的嵌入能力

### 2.1 现状

- GPUI 是 Metal 优先、自成体系的 UI 框架
- 以元素树 + 每帧渲染为主，**未发现**公开的「嵌入任意 NSView」或「获取某区域 Metal Layer」的 API
- #5040（窗口透明度）、#7955（NSVisualEffectView）主要讨论视觉效果，不涉及通用 native view 嵌入

### 2.2 可能的扩展方向（需查 Zed 源码确认）

1. **PlatformView / NativeView**
   - 若 GPUI 有类似 `div().native_view(...)` 或 `PlatformView` 的扩展点，可尝试传入 libghostty 的 NSView
   - 需在 Zed 仓库中搜索 `NSView`、`PlatformView`、`native` 等

2. **Metal Layer 共享**
   - `ghostty_inspector_metal_*` 暗示可把渲染输出到外部 Metal 层
   - 前提是 GPUI 能提供 `CAMetalLayer` 或等价句柄供 libghostty 使用
   - GPUI 使用 Metal 做渲染，但未必对外暴露可复用的 layer

3. **Texture / 离屏渲染**
   - 若 libghostty 支持渲染到 MTLTexture，而 GPUI 能渲染纹理，则可走「离屏渲染 → 纹理 → GPUI 显示」的路径
   - ghostty.h 中未见此类 API

### 2.3 结论

**当前 GPUI 未提供清晰的「嵌入外部 native view 或 Metal 层」的公共接口**。要做集成，需要：

- 深入 Zed/GPUI 源码，确认是否有未文档化的扩展点，或
- 向 GPUI 提 issue/PR，增加 PlatformView 或 Metal layer 共享能力

---

## 三、OrbStack 的集成方式

### 3.1 已知信息

- OrbStack v2.0 使用「由 Ghostty 驱动的终端」
- OrbStack 为 **Swift + AppKit** 的 macOS 应用
- 与 Ghostty 的集成方式与 pmux 不同：
  - OrbStack 完全控制 NSWindow/NSView 层级
  - 可以创建 NSView，填入 `ghostty_surface_config_s.platform.macos.nsview`，再传给 `ghostty_surface_new`
  - 不需要经过 GPUI 的渲染管线

### 3.2 对 pmux 的启示

- pmux 是 **Rust + GPUI**，没有直接访问 AppKit 的 view 层级
- 要在 pmux 中复用 OrbStack 的集成方式，需要 GPUI 提供「在某块区域插入 NSView」的能力
- 否则只能考虑独立窗口、Metal 共享等替代方案

---

## 四、PoC 设计思路

### 4.1 方案 A：独立子窗口（权宜之计）

1. 用 `cx.open_window` 为每个终端开一个 GPUI 窗口
2. 通过 `gpui_platform` 或平台层拿到底层 `NSWindow`
3. 在 NSWindow 的 contentView 中创建容器 NSView，传入 libghostty Surface 配置
4. 将终端窗口在逻辑上「绑定」到主窗口（位置、焦点等）

**优点**：不依赖 GPUI 新增能力  
**缺点**：每个 pane 一个窗口，与 pmux 的 split 布局模型不符；需要处理窗口管理、焦点、关闭等

### 4.2 方案 B：扩展 GPUI，支持 PlatformView

1. 在 Zed 仓库提 PR，增加 `PlatformView` 或类似元素
2. 在该元素内部创建 NSView，并支持传入自定义 view 或 view 工厂
3. pmux 中：`div().child(PlatformView::new(|cx| create_ghostty_nsview(cx)))`
4. 创建 libghostty Surface 时，使用该 NSView 作为 `platform.macos.nsview`

**优点**：与 GPUI 布局自然融合  
**缺点**：依赖 Zed 合并改动，周期不确定

### 4.3 方案 C：Metal 共享（若 API 存在）

1. 确认 `ghostty_inspector_metal_init` 的参数（例如是否为 `CAMetalLayer*`）
2. 查找 GPUI 是否在某处暴露 Metal layer 或可挂载的渲染目标
3. 若能获得 layer，则：libghostty 渲染到该 layer，GPUI 将该区域作为「纹理/layer 显示」处理

**优点**：不引入额外窗口  
**缺点**：GPUI 可能不暴露所需 API；Inspector 可能并非完整终端视图，需要验证

### 4.4 方案 D：暂不集成 libghostty，改用 gpui-terminal

- gpui-terminal 与 GPUI 同栈，直接实现 `Render`
- 无需 FFI、NSView、Metal 共享
- 功能较 libghostty 少（如无鼠标选文、scrollback 导航），但可作为过渡方案

---

## 五、建议行动顺序

| 优先级 | 行动 |
|--------|------|
| 1 | 在 Zed 仓库中搜索 `NSView`、`platform`、`native`、`Metal`、`CAMetalLayer`，确认 GPUI 是否已有未文档化的嵌入能力 |
| 2 | 若确认无现成能力，评估向 GPUI 提 PlatformView 类 feature 的可行性和优先级 |
| 3 | 在等待 GPUI 支持期间，用 **gpui-terminal** 替换当前自建终端管线，降低复杂度 |
| 4 | 保持关注 libghostty 的 C API 与 Rust 生态，待 GPUI 具备嵌入能力后再做 libghostty 集成 PoC |

---

## 六、结论

- **ghostty-sys / ghostty**：C API 有 App、Surface、平台 nsview；ghostty crate 尚未封装 Surface，需直接用 ghostty-sys 或自行封装
- **GPUI**：未发现「嵌入 NSView 或 Metal 层」的公共 API，需进一步查源码或推动 GPUI 扩展
- **OrbStack**：在 Swift/AppKit 下直接使用 libghostty，与 pmux 的 GPUI 架构不直接可比
- **PoC 可行性**：在 GPUI 支持 PlatformView 或 Metal 共享之前，在 GPUI 窗口内「原生」嵌入 libghostty 终端的路径不清晰；更现实的选择是先用 gpui-terminal，再视 GPUI 和 libghostty 的演进决定是否迁移
