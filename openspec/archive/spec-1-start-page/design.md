# 规格 1 设计：默认启动页（选择工作区）

## 1. 需求分析

### 功能需求

- **FR1.1**: 应用启动时检查是否已有保存的工作区路径
- **FR1.2**: 若无保存工作区，则显示启动页界面
- **FR1.3**: 启动页包含居中的「选择工作区」CTA 按钮
- **FR1.4**: 点击按钮后打开系统文件夹选择器
- **FR1.5**: 验证所选路径是否为有效的 git 仓库
- **FR1.6**: 验证通过后保存工作区路径到本地存储
- **FR1.7**: 验证失败时显示错误提示

### 非功能需求

- **NFR1.1**: 界面简洁直观，符合现代桌面应用设计规范
- **NFR1.2**: 文件选择响应迅速，不超过 2 秒响应时间
- **NFR1.3**: Git 仓库验证快速完成，不超过 1 秒

## 2. 架构设计

### 2.1 组件架构

```
AppRootComponent
  ├── condition: has_saved_workspace()
  ├── true  → WorkspaceComponent (规格 2+)
  └── false → StartupPageComponent
              └── FileSelectorIntegration
```

### 2.2 技术栈

- **UI 框架**: `gpui-component` 
- **UI 原语**: `div()`, `flex()`, `button()`, 等
- **文件系统**: Rust 标准库 `std::fs`
- **文件选择**: `rfd` crate（跨平台文件选择器）
- **Git 检测**: 检查路径下是否存在 `.git` 目录或文件

## 3. 组件设计

### 3.1 AppRootComponent

```rust
struct AppRootComponent {
    workspace_path: Option<PathBuf>,
    startup_page: StartupPageComponent,
    workspace_component: Option<WorkspaceComponent>,
}

impl ViewComponent for AppRootComponent {
    fn render(&self, cx: &mut AppContext) -> Element {
        match self.workspace_path {
            Some(_) => self.workspace_component.as_ref().unwrap().into_element(cx),
            None => self.startup_page.into_element(cx),
        }
    }
}
```

### 3.2 StartupPageComponent

```rust
struct StartupPageComponent {
    on_select_workspace: Box<dyn Fn(PathBuf)>,
}

impl ViewComponent for StartupPageComponent {
    fn render(&self, cx: &mut AppContext) -> Element {
        // 居中布局的启动页
        flex()
            .flex_1()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .items_center()
                    .gap_6()
                    .child(
                        div()
                            .text_xl()
                            .font_medium()
                            .child("Welcome to pmux")
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_muted()
                            .child("Select a Git repository to manage your AI agents")
                    )
                    .child(
                        button()
                            .label("📁 Select workspace")
                            .primary()
                            .on_click(move |_| {
                                // 打开文件选择器
                            })
                    )
            )
    }
}
```

## 4. 数据模型

### 4.1 配置存储

```rust
struct AppConfig {
    recent_workspace: Option<String>,
    // 其他配置...
}

// 保存位置：~/.config/pmux/config.json 或平台特定的位置
```

### 4.2 Git 仓库验证

```rust
fn is_git_repository(path: &Path) -> bool {
    path.join(".git").exists() || 
    path.join(".git").is_file() // bare repo
}
```

## 5. 交互流程

### 5.1 主流程

```
1. App 启动
2. AppRootComponent::init()
   a. 读取配置文件 (~/.config/pmux/config.json)
   b. 检查 recent_workspace 字段
   c. 若存在且有效 → 设置 workspace_path
   d. 若不存在 → workspace_path = None
3. 渲染
   a. workspace_path != None → 渲染 WorkspaceComponent (规格 2+)
   b. workspace_path == None → 渲染 StartupPageComponent
4. 用户点击「选择工作区」
5. 调用系统文件选择器
6. 用户选择路径
7. 验证路径是否为 Git 仓库
8. 保存路径到配置文件
9. 重新渲染，进入规格 2+ 流程
```

### 5.2 错误处理流程

```
1. 用户选择非 Git 目录
2. Git 仓库验证失败
3. 显示错误提示：「所选目录不是 Git 仓库，请选择包含 .git 的目录」
4. 保持在启动页，允许重新选择
```

## 6. UI/UX 设计

### 6.1 启动页布局

```
┌─────────────────────────────────┐
│           pmux Window           │
│                                 │
│                                 │
│                                 │
│         [ Welcome to pmux ]     │
│                                 │
│   Select a Git repository to    │
│   manage your AI agents         │
│                                 │
│         [ 📁 Select workspace ]  │
│                                 │
│                                 │
│                                 │
│                                 │
└─────────────────────────────────┘
```

### 6.2 视觉规范

- **主色调**: 与 cmux 类似的深色主题
- **按钮样式**: 主按钮使用品牌色，圆角，适当阴影
- **字体**: 系统默认字体，启动页标题稍大
- **间距**: 适中的元素间距，视觉舒适

## 7. 实现约束

### 7.1 性能约束

- 文件选择器打开延迟 < 2 秒
- Git 仓库验证 < 1 秒
- 应用启动时间 < 3 秒

### 7.2 平台约束

- 支持 macOS、Linux、Windows
- 使用跨平台文件选择器库
- 配置文件存储遵循各平台规范

## 8. 测试策略

### 8.1 单元测试

- Git 仓库验证函数测试
- 配置文件读写测试
- 路径有效性验证测试

### 8.2 集成测试

- 启动页显示逻辑测试
- 文件选择流程测试
- 配置保存与恢复测试

## 9. 风险与缓解

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| 系统文件选择器不可用 | 高 | 低 | 准备备用文件选择实现 |
| 配置文件权限问题 | 中 | 低 | 使用标准配置目录，优雅处理权限错误 |
| Git 仓库验证误报 | 中 | 低 | 多种验证方法结合 |
