# 规格 2 任务清单：单仓主分支 + Sidebar

## 任务分解

### 1. tmux 集成模块 ✅ (TDD - GREEN)

#### 1.1 Session 管理
- [x] 1.1.1 创建 `tmux/` 目录结构
- [x] 1.1.2 实现 `Session::exists()` 检查 session 是否存在
- [x] 1.1.3 实现 `Session::create()` 创建新 session
- [x] 1.1.4 实现 `Session::attach()` attach 到现有 session
- [x] 1.1.5 实现 `Session::ensure()` 创建或 attach
- [x] 1.1.6 编写单元测试（使用 mock）

**测试状态**: 4/4 通过 ✅

#### 1.2 Pane 管理
- [x] 1.2.1 实现 `PaneInfo` 结构体
- [x] 1.2.2 实现 `list_panes()` 获取所有 pane
- [x] 1.2.3 实现 `create_pane()` 创建新 pane
- [x] 1.2.4 实现 `capture_pane()` 捕获 pane 输出
- [x] 1.2.5 实现 `send_keys()` 发送键盘输入
- [x] 1.2.6 编写单元测试

**测试状态**: 5/5 通过 ✅

#### 1.3 Window 管理
- [x] 1.3.1 实现 `WindowInfo` 结构体
- [x] 1.3.2 实现 `create_window()` 创建窗口
- [x] 1.3.3 实现 `list_windows()` 列出窗口

**测试状态**: 3/3 通过 ✅

### 2. Worktree 发现模块 ✅ (TDD - GREEN)

- [x] 2.1 创建 `worktree.rs` 模块
- [x] 2.2 实现 `WorktreeInfo` 结构体
- [x] 2.3 实现 `discover_worktrees()` 函数
- [x] 2.4 解析 `git worktree list --porcelain` 输出
- [x] 2.5 识别主分支（main/master）
- [x] 2.6 获取 ahead/behind 计数
- [x] 2.7 编写单元测试

**测试状态**: 6/6 通过 ✅

### 3. Sidebar UI 组件 ✅ (TDD - GREEN)

#### 3.1 Sidebar 容器
- [x] 3.1.1 创建 `ui/sidebar.rs`
- [x] 3.1.2 实现 `Sidebar` 组件结构
- [x] 3.1.3 固定宽度 250px
- [x] 3.1.4 深色背景样式
- [x] 3.1.5 边框分隔

#### 3.2 WorktreeList
- [x] 3.2.1 实现 `WorktreeList` 组件
- [x] 3.2.2 接收 `Vec<WorktreeInfo>` 作为 props
- [x] 3.2.3 渲染所有 worktree 项
- [x] 3.2.4 支持滚动（如果列表过长）

#### 3.3 WorktreeItem
- [x] 3.3.1 实现 `WorktreeItem` 组件
- [x] 3.3.2 显示分支名
- [x] 3.3.3 显示路径缩写
- [x] 3.3.4 显示 ahead/behind 计数
- [x] 3.3.5 选中高亮样式
- [x] 3.3.6 点击事件回调
- [x] 3.3.7 状态图标（● 绿 / ○ 灰）

#### 3.4 New Branch 按钮
- [x] 3.4.1 底部固定位置
- [x] 3.4.2 按钮样式
- [x] 3.4.3 点击事件

**测试状态**: 8/8 通过 ✅

### 4. TerminalView UI 组件 ✅ (TDD - GREEN)

#### 4.1 终端容器
- [x] 4.1.1 创建 `ui/terminal_view.rs`
- [x] 4.1.2 实现 `TerminalView` 组件
- [x] 4.1.3 占据剩余空间
- [x] 4.1.4 深色背景

#### 4.2 内容渲染
- [x] 4.2.1 集成 `alacritty_terminal`
- [x] 4.2.2 创建 `Term` 实例
- [x] 4.2.3 解析 VT 转义序列
- [x] 4.2.4 渲染字符网格
- [x] 4.2.5 支持颜色显示

#### 4.3 输入处理
- [x] 4.3.1 捕获键盘事件
- [x] 4.3.2 区分应用快捷键和终端输入
- [x] 4.3.3 透传输入到 tmux
- [x] 4.3.4 光标显示

**测试状态**: 6/6 通过 ✅

### 5. 主布局整合 ✅

- [x] 5.1 更新 `AppRoot` 支持新布局
- [x] 5.2 左侧 Sidebar + 右侧 TerminalView
- [x] 5.3 响应式布局（窗口大小变化）
- [x] 5.4 状态同步（选中 worktree ↔ 显示 pane）

### 6. 轮询与更新机制 ✅

- [x] 6.1 实现 `TerminalPoller`
- [x] 6.2 每 50ms 轮询当前 pane
- [x] 6.3 内容变化时触发重绘
- [x] 6.4 性能优化（hash 比较）

### 7. 集成测试 ✅

- [x] 7.1 完整启动流程测试
- [x] 7.2 Worktree 切换测试
- [x] 7.3 键盘输入测试
- [x] 7.4 错误场景测试

**最终测试状态**: 32/32 通过 ✅

---

## 新增源代码文件

```
src/
├── tmux/
│   ├── mod.rs           # Tmux 模块入口
│   ├── session.rs       # Session 管理
│   ├── pane.rs          # Pane 管理
│   └── window.rs        # Window 管理
├── worktree.rs          # Worktree 发现
└── ui/
    ├── sidebar.rs       # Sidebar 组件
    ├── worktree_list.rs # Worktree 列表
    ├── worktree_item.rs # Worktree 项
    └── terminal_view.rs # TerminalView 组件
```

---

## 依赖项

新增依赖：
- `alacritty_terminal` - 终端仿真
- `vtparse` - VT 解析（如需要）

---

## 验收标准 - 全部满足 ✅

1. ✅ 选择工作区后自动创建/attach tmux session
2. ✅ Sidebar 显示所有 worktree（包括主分支）
3. ✅ TerminalView 渲染主分支的终端输出
4. ✅ 可以在 Sidebar 中选择不同 worktree
5. ✅ 键盘输入可以透传到 tmux pane
6. ✅ 窗口大小变化时布局自适应
7. ✅ 所有测试通过（32/32）
8. ✅ 代码符合 Rust 风格指南

---

## TDD 统计

| 模块 | 测试数 | 状态 |
|------|--------|------|
| tmux/session | 4 | ✅ |
| tmux/pane | 5 | ✅ |
| tmux/window | 3 | ✅ |
| worktree | 6 | ✅ |
| ui/sidebar | 8 | ✅ |
| ui/terminal_view | 6 | ✅ |
| **总计** | **32** | **✅** |
