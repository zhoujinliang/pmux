# pmux — AI Agent 多分支开发工作台

## 1. 背景与目标

随着 AI 编程工具（Claude Code / OpenCode / CodeX 等）逐渐成为主要代码生产者，
开发者的角色正在从「写代码」转向「Review / 架构 / 决策」。

pmux 是一个原生桌面应用，用于：

- 管理多个 AI Agent 并行工作（每个 agent 一个 git worktree）
- 实时监控 agent 状态（Running / Waiting / Error）
- 主动通知（系统通知 + 应用内通知）
- 快速 Review Diff（内置或跳转 nvim diffview）

对标产品：[cmux](https://github.com/manaflow-ai/cmux)（UI 布局）+ [agent-of-empires](https://github.com/njbrake/agent-of-empires)（多 agent 管理逻辑）

---

## 2. 核心设计原则

### 2.1 角色分离

| 角色 | 职责 |
|----|-----|
| AI Agent | 写代码、提交 commit |
| Human | Review、修改、合并 |

### 2.2 Git 是唯一真实源

- 所有 Review 基于：`main...HEAD`
- 所有 Agent 产出必须进入 Git 才能被 Review

### 2.3 Branch / Worktree First

- 一个 Agent = 一个 worktree = 一个终端 tab
- 不同 Agent 之间物理隔离

---

## 3. 核心架构：UI 壳 + tmux 后端

### 3.1 设计理念

pmux = **tmux 的高级前端**。和 cmux 对 libghostty 的关系一样。

```
┌─────────────────────────────────┐
│  pmux (GPUI 原生窗口)            │  ← 只是 UI 壳，关掉窗口 agent 继续跑
│  ┌───────────────────────────┐  │
│  │  终端渲染（GPU）           │  │
│  │  读取 tmux pane 的输出     │  │
│  └───────────────────────────┘  │
└──────────┬──────────────────────┘
           │ attach / capture-pane / send-keys
           ▼
┌─────────────────────────────────┐
│  tmux (后台常驻)                │  ← 真正的进程管理器
│  ├─ session: sdlc-myproject     │
│  │  ├─ window: control-tower    │
│  │  │  ├─ pane 0: shell (main)  │  agent 进程活在这里
│  │  │  ├─ pane 1: claude (feat) │
│  │  │  └─ pane 2: claude (fix)  │
│  │  └─ window: review-feat-x   │
│  └─ session: sdlc-other         │
└─────────────────────────────────┘
```

### 3.2 为什么必须保留 tmux

| 问题 | 没有 tmux | 有 tmux |
|------|----------|---------|
| 关掉 pmux 窗口 | 所有 agent 进程被杀 | agent 继续跑，重开 pmux 恢复 |
| 崩溃恢复 | 全部丢失 | tmux session 完好，重新 attach |
| SSH 远程 | 无法使用 | tmux 在远程服务器，pmux 本地连接 |
| 多人协作 | 不可能 | 多个 pmux 实例 attach 同一 session |

### 3.3 职责划分

| 层 | 职责 | 技术 |
|----|------|------|
| **tmux** | 进程生命周期、session 持久化、pane 管理 | tmux server（后台常驻） |
| **pmux** | GPU 终端渲染、UI 交互、通知、快捷键 | GPUI + alacritty_terminal |

pmux 通过以下方式与 tmux 交互：

- `tmux capture-pane -p -t <target>` → 读取 pane 内容（状态检测）
- `tmux send-keys -t <target> <keys>` → 向 pane 发送输入
- `tmux list-panes / list-windows / list-sessions` → 发现 session 结构
- `tmux pipe-pane` 或 PTY passthrough → 实时终端输出流

---

## 4. 技术选型

### 4.3 核心依赖

| crate | 用途 |
|-------|------|
| `gpui` | UI 框架（窗口、布局、渲染、事件）- Zed 编辑器同款 |
| `gpui_platform` | GPUI 平台适配层（带 font-kit feature） |
| `alacritty_terminal` | 终端仿真（VT 解析、grid buffer） |
| `notify-rust` | 系统通知（macOS / Linux） |
| `rfd` | 跨平台文件选择器 |

---

## 5. UI 布局设计（对标 cmux）

### 5.1 整体布局

```
┌──────────────────────────────────────────────────────────────────────────┐
│ [≡] [🔔 2] [📂]  [myproject ×] [other-repo ●×]        [⌨][🌐][⊞][⊟]  │ ← 顶部栏 (Workspace Tabs)
├─────────────────┬────────────────────────────────────────────────────────┤
│                 │ [🖥 main ×] [🖥 feat-x ×] [🖥 fix-bug ×]              │ ← Tab 栏 (Pane Tabs)
│  liziliu@lizi…  ├────────────────────────────────────────────────────────┤
│  ~              │                                                        │
│  ■ 选中高亮     │  终端内容区                                             │
│                 │                                                        │
│  liziliu@lizi…  │  GPU 渲染的终端（alacritty_terminal）                  │
│  ~              │  读取 tmux pane 输出                                    │
│                 │                                                        │
│  OC | Create…   │  $ claude code                                         │
│  main · ~/Doc…  │  > Thinking...                                         │
│                 │                                                        │
│                 │                                                        │
└─────────────────┴────────────────────────────────────────────────────────┘
```

**层级关系：**
- **顶部栏 (TopBar)** = Workspace 级别的 Tab，每个 Tab 代表一个 Git 仓库
- **Tab 栏 (TabBar)** = Pane 级别的 Tab，每个 Tab 代表当前仓库内的一个 tmux pane（worktree/agent）

### 5.2 顶部栏 (Workspace Tabs)

顶部栏包含全局控制按钮和 **Workspace Tabs**（多仓库切换）。

```
┌──────────────────────────────────────────────────────────────────────────┐
│ [≡]  [🔔 2]  [📂]  [myproject ×] [other-repo ●×] [+ ]    [⌨][🌐][⊞][⊟] │
└──────────────────────────────────────────────────────────────────────────┘
  │      │      │         │              │        │         │   │   │   │
  │      │      │         │              │        │         │   │   │   └─ 水平分屏
  │      │      │         │              │        │         │   │   └─ 垂直分屏
  │      │      │         │              │        │         │   └─ 浏览器（可选）
  │      │      │         │              │        │         └─ 快捷键帮助
  │      │      │         │              │        └─ 新建 Workspace Tab
  │      │      │         │              └─ 另一个仓库（● = 有未读通知）
  │      │      │         └─ 当前激活的仓库 Tab
  │      │      └─ 打开 git 目录（新建 Workspace）
  │      └─ 通知铃铛 + 未读数（点击弹出通知面板）
  └─ 收缩/展开侧边栏（⌘B）
```

**Workspace Tab 特性：**
- 每个 Tab 显示仓库名称（从路径提取）
- `×` 关闭按钮：关闭该 Workspace（不删除仓库，只是从 pmux 移除管理）
- `●` 蓝色圆点：该仓库下有 Agent 需要关注（Waiting/Error）
- 快捷键：`⌘1-8` 切换到第 N 个 Workspace
- 支持拖拽排序

**通知面板**（浮层，从铃铛下方弹出）：

```
┌─ Notifications ──────────────── ×─┐
│                                    │
│  ● feat-x: Agent 等待输入          │
│    2 min ago                       │
│                                    │
│  ✕ experiment: build failed        │
│    5 min ago                       │
│                                    │
│  ● fix-bug: 3 files changed       │
│    8 min ago                       │
│                                    │
└────────────────────────────────────┘
```

空状态：

```
┌─ Notifications ──────────────── ×─┐
│                                    │
│         🔕                         │
│    No notifications yet            │
│    Desktop notifications will      │
│    appear here.                    │
│                                    │
└────────────────────────────────────┘
```

### 5.3 侧边栏

每个条目对应一个 tmux pane（= 一个 agent worktree）。

**pmux 侧边栏条目**：

```
┌─────────────────┐
│  ● feat-x       │  ← 状态图标 + 分支名
│  Running · +2   │  ← 状态文字 · ahead count
│                 │
│  ◐ fix-bug [!]  │  ← [!] = 需要关注（蓝色环，同 cmux）
│  Waiting · +1   │
│                 │
│  ✕ experiment   │
│  Error          │
└─────────────────┘
```

选中状态：蓝色背景高亮（同 cmux）。

侧边栏收缩后：完全隐藏，主区域占满宽度。

### 5.4 Tab 栏

```
[🖥 main ×]  [🖥 feat-x ●×]  [🖥 fix-bug ×]          [⌨] [🌐] [⊞] [⊟]
                      │                                  │    │    │    │
                      └─ 蓝色圆点 = 有通知               │    │    │    │
                                                         │    │    │    └─ 水平分屏
                                                         │    │    └─ 垂直分屏
                                                         │    └─ 浏览器（可选，v2）
                                                         └─ 键盘快捷键面板
```

- 每个 tmux pane 一个 tab
- 点击 tab 切换终端视图（= 切换 attach 的 tmux pane）
- 点击侧边栏条目也切换到对应 tab
- `×` 关闭 tab（= 关闭 tmux pane，可选删除 worktree）
- 右侧按钮：分屏操作（⌘D 垂直分屏，⌘⇧D 水平分屏）

### 5.5 主区域（终端）

终端渲染流程：

```
tmux pane (后台)
  → tmux pipe-pane / PTY passthrough
  → alacritty_terminal::Term (VT 解析)
  → GPUI GPU 渲染（字符网格 + 颜色 + 光标）
```

用户输入流程：

```
键盘事件 → GPUI 事件循环
  → 应用级快捷键？→ 拦截处理
  → 否 → tmux send-keys -t <pane> <key>
```

**分屏布局**（对标 cmux 截图 3）：

```
┌──────────────────────────┬──────────────────────────┐
│ 🖥 pane-1 ×              │ 🖥 pane-2 ×              │
│                          │                          │
│ $ claude code            │ $ claude code            │
│ > Thinking...            │ > Waiting for input      │
│                          ├──────────────────────────┤
│                          │ 🖥 pane-3 ×              │
│                          │                          │
│                          │ $ nvim                   │
│                          │                          │
└──────────────────────────┴──────────────────────────┘
```

分屏 = 同时显示多个 tmux pane 的输出。tmux 负责 pane 管理，pmux 负责渲染布局。

### 5.6 Diff 视图

任何 agent tab 中，用户可通过快捷键 `⌘⇧D` 或侧边栏右键菜单「View Diff」打开 diff 窗口。

**实现方式：** 在当前 tab 旁新开一个 tmux window，启动 nvim + diffview：

```
tmux new-window -t <session> -n "review-<branch>"
tmux send-keys -t <session>:review-<branch> \
  "NVIM_APPNAME=sdlc-review nvim -c 'DiffviewOpen main...HEAD'" C-m
```

pmux 自动切换到该 review tab，显示 nvim diffview 的终端输出。

**Diff 视图布局**（nvim diffview 在 pmux 嵌入终端中）：

```
┌─────────────────┬────────────────────────────────────────────────────────┐
│  侧边栏         │ [● main] [◐ feat-x] [📝 review-feat-x ×]            │
│                 ├────────────────────────────────────────────────────────┤
│  ● feat-x       │ ┌─ NvimTree ──────┬─ DiffView ─────────────────────┐ │
│  Running · +2   │ │ ▼ src/          │ feat-x vs main │ 3 changed     │ │
│                 │ │   Auth.tsx  [M]  │ @@ -12,6 +12,18 @@             │ │
│  ◐ fix-bug [!]  │ │   Button.tsx [A] │  - const old = useOldAuth()   │ │
│  Waiting · +1   │ │                  │  + const auth = useAuth()     │ │
│                 │ └──────────────────┴────────────────────────────────┘ │
└─────────────────┴────────────────────────────────────────────────────────┘
```

关闭 diff：在 nvim 中 `:q` 或 `⌘W` 关闭 review tab。

---

## 6. 业务流程

### 6.1 首次打开 git repo

```
用户点击 📂 → 选择 git 目录（如 ~/workspace/myproject）
  │
  ├─ 验证是 git repo（检查 .git 目录）
  ├─ 读取 .sdlc.yaml（如有，获取 base_branch / exclude 等配置）
  ├─ worktree::discover() → 发现主 repo + 所有已有 worktree
  │
  ├─ tmux new-session -d -s "sdlc-myproject"
  │   ├─ 为主 repo 创建 pane（window: control-tower, pane 0）
  │   ├─ 为每个 worktree 创建 pane（pane 1, 2, ...）
  │   └─ cd 到对应 worktree 目录
  │
  ├─ pmux 侧边栏：显示所有 worktree 条目
  ├─ pmux tab 栏：为每个 pane 创建 tab
  └─ 开始状态轮询
```

### 6.2 新建 branch + worktree

侧边栏底部 `[+ New Branch]` 按钮，或快捷键 `⌘⇧N`。

```
用户点击 [+ New Branch]
  → 弹出输入框：分支名（如 "feat/auth"）
  │
  ├─ git branch feat/auth                          # 创建分支
  ├─ git worktree add ../myproject-feat-auth feat/auth  # 创建 worktree
  │
  ├─ tmux split-window / new-window                # 在 session 中创建新 pane
  │   └─ cd ../myproject-feat-auth
  │
  ├─ 侧边栏：新增条目「? feat/auth」（Unknown 状态）
  ├─ tab 栏：新增 tab
  └─ 自动切换到新 tab
```

### 6.3 删除 worktree

侧边栏右键菜单「Remove」，或选中后按 `⌘⌫`。

```
用户选择 "feat/auth" → Remove
  → 确认对话框："Remove worktree feat/auth? This will kill the agent process."
  │
  ├─ tmux kill-pane -t <pane>                      # 终止 agent 进程
  ├─ git worktree remove ../myproject-feat-auth     # 删除 worktree 目录
  ├─ git branch -d feat/auth                        # 可选：删除分支（需确认）
  │
  ├─ 侧边栏：移除条目
  ├─ tab 栏：移除 tab
  └─ 如果删除的是当前 tab → 切换到相邻 tab
```

注意：如果 worktree 有未推送的 commit，弹出警告。

### 6.4 退出 / 重新打开 pmux

```
退出 pmux（⌘Q 或关闭窗口）：
  → pmux 保存 UI 状态到 ~/.config/pmux/state.json
    （当前选中 tab、侧边栏宽度、分屏布局）
  → tmux session 不受影响，所有 agent 继续运行

重新打开 pmux：
  → 读取 state.json
  → tmux has-session -t "sdlc-myproject" → 存在
  → tmux list-panes → 恢复 pane 结构
  → worktree::discover() → 刷新 worktree 列表
  → 恢复 UI 状态（tab 顺序、选中项等）
  → 重新开始状态轮询
```

### 6.5 查看 Diff

在任何 agent tab 中：

```
用户按 ⌘⇧R（Review Diff）或点击侧边栏条目的 [diff] 图标
  │
  ├─ 检查 review-<branch> tmux window 是否已存在
  │   ├─ 存在 → 切换到该 tab（nvim 已在运行）
  │   └─ 不存在 → 创建：
  │       ├─ tmux new-window -t <session> -n "review-<branch>"
  │       ├─ tmux send-keys "NVIM_APPNAME=sdlc-review nvim -c 'DiffviewOpen main...HEAD'" C-m
  │       └─ pmux 新增 review tab
  │
  ├─ pmux 切换到 review tab
  └─ 用户在 nvim diffview 中审查代码

关闭 diff：
  → nvim :q → tmux window 关闭 → pmux 移除 review tab → 回到 agent tab
```

---

## 7. 快捷键设计

### 7.1 应用级快捷键（GPUI 层拦截，不传给终端）

| 快捷键 | 功能 |
|--------|------|
| ⌘B | 收缩/展开侧边栏 |
| ⌘N | 新建 workspace（打开 git 目录选择器） |
| ⌘⇧N | 新建 branch + worktree（弹出输入框） |
| ⌘1-8 | 切换到第 N 个 tab |
| ⌘D | 垂直分屏 |
| ⌘⇧D | 水平分屏 |
| ⌘W | 关闭当前 tab/pane |
| ⌘⌫ | 删除选中的 worktree（确认对话框） |
| ⌘⇧R | 打开当前分支的 Diff 视图（nvim diffview） |
| ⌘I | 打开/关闭通知面板 |
| ⌘⇧U | 跳转到最近一条未读通知对应的 tab |
| ⌥⌘←/→/↑/↓ | 在分屏 pane 间切换焦点 |

### 7.2 终端内快捷键

所有非应用级快捷键通过 `tmux send-keys` 透传给 pane 内程序（neovim 等完全正常）。

---

## 8. 数据模型

### 8.1 Workspace

```rust
struct Workspace {
    repo_root: PathBuf,           // git repo 根目录
    base_branch: String,          // 默认 "main"
    tmux_session: String,         // tmux session 名（如 "sdlc-myproject"）
    worktrees: Vec<WorktreeState>,
}
```

### 8.2 WorktreeState

```rust
struct WorktreeState {
    worktree: Worktree,           // branch, path, ahead, behind
    status: AgentStatus,          // Running / Waiting / Idle / Error / Unknown
    tmux_pane: String,            // tmux pane target（如 "sdlc-myproject:control-tower.1"）
    notified: bool,               // 未读通知标记
}
```

### 8.3 AgentStatus（复用现有）

```rust
enum AgentStatus {
    Running,   // ● 绿  — agent 正在执行
    Waiting,   // ◐ 黄  — 等待用户输入
    Idle,      // ○ 灰  — 静止
    Error,     // ✕ 红  — 检测到错误
    Unknown,   // ? 紫  — pane 未启动
}
```

### 8.4 Notification

```rust
struct Notification {
    worktree_branch: String,
    message: String,              // "Agent 等待输入" / "build failed"
    timestamp: Instant,
    read: bool,
}
```

---

## 9. tmux 交互层

### 9.1 终端输出获取

两种方案，按优先级：

**方案 A：tmux control mode（推荐）**

```
pmux → tmux -CC attach-session -t <session>
```

tmux control mode 通过 stdout 以结构化文本协议输出所有 pane 的内容变化。
pmux 解析协议 → 喂给 alacritty_terminal → GPU 渲染。

优点：实时、低延迟、官方支持。
cmux / iTerm2 都用这个方案。

**方案 B：轮询 capture-pane（fallback）**

```
每 50ms: tmux capture-pane -p -t <pane> -e  (带 ANSI 转义)
→ alacritty_terminal 解析 → 渲染
```

优点：简单。缺点：有延迟，高频轮询消耗 CPU。

### 9.2 输入发送

```
用户按键 → pmux 判断是否应用级快捷键
  → 否 → tmux send-keys -t <pane> -l <key>
```

### 9.3 Session 生命周期

```
pmux 启动：
  1. 检查 tmux session 是否存在（tmux has-session -t <name>）
  2. 存在 → attach（读取现有 pane 结构）
  3. 不存在 → 创建 session + window + panes

pmux 关闭：
  → 什么都不做。tmux session 继续运行。

pmux 重新打开：
  → 重新 attach，恢复所有终端状态。
```

### 9.4 状态检测

复用现有 `tmux.rs` 的 `detect_status()` 逻辑：

```
每 500ms:
  tmux capture-pane -p -t <pane>
  → detect_status(content)
  → 状态变化 → 更新侧边栏 + 可能触发通知
```

| 状态 | 匹配规则 |
|------|---------|
| Running | "esc to interrupt" / "thinking" / "writing" / "running tool" |
| Waiting | "? " / "> " / "human turn" |
| Error | "error" / "failed" / "traceback" |
| Idle | 无上述关键词且内容非空 |
| Unknown | 内容为空 |

---

## 10. 架构

### 10.1 线程模型

```
main thread (GPUI event loop)
  ├── 窗口渲染（GPU）
  ├── 键盘/鼠标事件 → tmux send-keys
  └── 状态更新（从 poller 接收）

tmux reader thread
  ├── tmux control mode 协议解析（方案 A）
  │   或 capture-pane 轮询（方案 B）
  ├── → alacritty_terminal::Term 解析
  └── → channel → main thread 重绘

status poller thread (每 500ms)
  ├── tmux capture-pane × N panes
  ├── detect_status() per pane
  ├── 状态变化 → channel → main thread
  └── 触发通知（notify-rust）
```



### 10.3 数据流

```
pmux 启动
  → tmux::session::ensure_session()     # 创建或 attach tmux session
  → worktree::discover()                # 发现 git worktrees
  → tmux::session::ensure_panes()       # 确保每个 worktree 有对应 pane
  → tmux::control_mode::connect()       # 建立 control mode 连接
  → GPUI 窗口渲染

tmux pane 输出
  → tmux control mode 协议 / capture-pane
  → alacritty_terminal::Term::advance()
  → terminal::renderer 重绘

用户按键
  → GPUI 事件 → 应用级快捷键？
    → 是：处理（切换 tab、收缩侧边栏等）
    → 否：tmux send-keys -t <pane>

状态轮询（每 500ms）
  → tmux capture-pane × N
  → detect_status()
  → 状态变化 → 更新侧边栏 + 通知

用户点击 📂
  → 系统文件选择器
  → 选择 git 目录
  → tmux::session::create_session()     # 新建 tmux session
  → worktree::discover()                # 发现 worktrees
  → 添加到侧边栏
```

---

## 11. 规格拆解（GUI 实现，阶段性交付）

pmux 完全通过 GUI 界面操作，无需 CLI。以下规格按依赖顺序排列，每个规格完成后都可独立运行和体验。

**图例：** ✅ 已完成 · 🔄 进行中 · 📋 待开始

---

### 规格 1：启动页与工作区选择 ✅
**目标**：应用启动流程，引导用户选择首个工作区。

**已实现：**
- `AppRoot` 根据是否有工作区自动切换启动页 / 工作区视图
- 启动页（`startup_page.rs` + `app_root.rs::render_startup_page`）：
  - 居中「Welcome to pmux」标题
  - 「Select Workspace」按钮，点击调用系统文件夹选择器（`rfd`）
  - 非 git 仓库时显示红色错误提示
- `Config`（`config.rs`）持久化最近工作区路径，下次启动自动恢复
- `git_utils.rs` 验证所选目录是否为 git 仓库

---

### 规格 2：单仓主分支 + Sidebar ✅
**目标**：建立基础窗口布局，管理单个仓库的主分支。

**已实现：**
- `AppRoot::render_workspace_view` 渲染完整三栏布局（Sidebar + TabBar + TerminalView）
- `Sidebar`（`sidebar.rs`）：worktree 列表、状态图标、选中高亮、`+ New Branch` 按钮
- `TerminalView`（`terminal_view.rs`）：支持 `with_content(Arc<Mutex<TerminalContent>>)` 接收外部共享 buffer
- `⌘B` 收缩/展开 Sidebar（`sidebar_visible` 状态）
- `AppRoot::start_tmux_session`：打开工作区时调用 `Session::ensure()`，启动后台轮询任务（每 200ms `capture-pane` → 更新 `TerminalContent` → `cx.notify()` 触发重绘）
- 键盘输入透传：`handle_key_down` 已接入 `InputHandler::send_key`，非 Cmd 按键转发到 tmux
- 启动恢复：`init_workspace_restoration` 在启动时调用，有已保存工作区时自动触发 `start_tmux_session`

---

### 规格 3：新建 Branch + Worktree（GUI） 🔄
**目标**：通过 GUI 创建新分支和工作区。

**已完成：**
- `NewBranchDialogUi`、`NewBranchOrchestrator`、`create_branch_async` 完整流程
- `create_branch` 已实现：校验 → `git worktree add` → 创建 tmux pane → 刷新 Sidebar

**待完成：**
- Sidebar「+ New Branch」按钮点击 → 调用 `open_new_branch_dialog`（当前回调仅 `println!`）
- 对话框 Create 按钮 → 调用 `create_branch`（当前 `on_create` 回调未接入）
- 分支名输入框：需改为可编辑组件（当前为只读 `div`）

---

### 规格 4：多 Worktree 管理与 TabBar 🔄
**目标**：支持在同一仓库内管理多个 worktree。

**UI 层已完成：**
- `TabBar`（`tabbar.rs`）：pane tab 渲染、激活状态、关闭按钮、`+` 新建按钮
- `WorkspaceManager`（`workspace_manager.rs`）：多 tab 数据模型，支持增删切换
- `worktree.rs`：`WorktreeInfo` 数据结构，`short_branch_name()`、`ahead` 计数

**待实现：**
- Sidebar 条目与 TabBar 联动切换
- 删除 worktree 确认对话框
- `tmux kill-pane` + `git worktree remove` 流程

---

### 规格 5：Agent 状态检测与展示 ✅
**目标**：实时监控并可视化 agent 状态。

**已实现：**
- `AgentStatus`（`agent_status.rs`）：Running / Waiting / Idle / Error / Unknown
- `StatusDetector`（`status_detector.rs`）：基于 pane 输出文本的关键词匹配
- `StatusPoller`（`status_poller.rs`）：独立线程，每 500ms 轮询所有 pane
- `PaneStatusTracker`（`pane_status_tracker.rs`）：跟踪状态变化历史
- StatusPoller 状态变化 → `pane_statuses` HashMap → `update_status_counts` → `cx.notify()` 重绘
- `Sidebar` 每个条目显示状态图标（●◐○✕?）和颜色
- `TopBar` 显示 `StatusCounts`（error + waiting 计数）

---

### 规格 6：Diff 视图集成 📋
**目标**：内置代码审查能力。

- 快捷键 ⌘⇧R 或 Sidebar 右键「View Diff」打开 diff
- 新开 tmux window 运行 `nvim -c 'DiffviewOpen main...HEAD'`
- pmux 新增 review tab，显示 nvim 终端输出

---

### 规格 7：通知面板与系统通知 🔄
**目标**：主动提醒需要关注的事件。

**已完成：**
- `Notification` / `NotificationType`（`notification.rs`）：Error / Waiting / Info 三类
- `NotificationManager`（`notification_manager.rs`）：通知队列、合并、已读管理
- `NotificationPanel`（`notification_panel.rs`）：浮层 UI，列表 + 清空 + 关闭
- `TopBar` 通知铃铛图标，`has_notifications` 时红色高亮

**待连接：**
- `StatusPoller` 状态变化 → `NotificationManager::add()` → 更新 `AppRoot.notifications`
- `notify-rust` 系统通知调用
- ⌘I 快捷键绑定

---

### 规格 8：多仓库（Workspace Tabs） 🔄
**目标**：支持同时管理多个 git 仓库。

**已完成：**
- `TopBar` 渲染 Workspace Tabs（仓库名、× 关闭、● 通知圆点）
- `WorkspaceManager` 支持多仓库 tab 增删切换
- 📂 按钮触发文件选择器，已实现 `handle_add_workspace`

**待实现：**
- 切换 Workspace Tab 时恢复对应仓库的 Sidebar / TabBar 状态
- ⌘1-8 快捷键切换

---

### 规格 9：状态持久化与启动恢复 🔄
**目标**：保存用户偏好，提升启动体验。

**已完成：**
- `Config`（`config.rs`）：保存/读取最近工作区路径（`~/.config/pmux/config.json`）
- `AppRoot::new()` 启动时自动加载上次工作区

**待实现：**
- 保存多仓库列表、激活 tab、Sidebar 宽度、窗口尺寸
- 启动时重新 attach 所有 tmux sessions

---

### 规格 10：控制模式与性能优化 📋
**目标**：迁移到更高效的终端输出获取方案。

- 从轮询 `capture-pane` 迁移到 tmux control mode（`tmux -CC`）
- 独立线程解析 control mode 协议 → `alacritty_terminal::Term` → GPU 渲染
- 刷新节流（60fps 上限）

---

### 规格 11：分屏布局 📋
**目标**：同时查看多个 pane。

- ⌘D 垂直分屏、⌘⇧D 水平分屏
- 拖拽调整分屏边界
- ⌥⌘←/→/↑/↓ 在 pane 间切换焦点

---

## 当前进展总结

| 层 | 状态 | 说明 |
|----|------|------|
| GPUI UI 组件 | ✅ 全部编译通过 | TopBar / TabBar / Sidebar / TerminalView / NotificationPanel / AppRoot |
| tmux 后端模块 | ✅ 框架完整 | session / pane / window / capture_pane |
| 状态检测 | ✅ 已连接 UI | StatusPoller → pane_statuses → Sidebar/TopBar |
| 通知系统 | ✅ 逻辑完整 | Notification + NotificationManager（待接入 StatusPoller） |
| 配置持久化 | ✅ 基础完成 | 单工作区路径保存/恢复 |
| **规格 1/2/5** | ✅ **已完成** | MVP 核心：启动、选仓库、看终端、键盘透传、状态展示 |

**下一步优先级：** 规格 3 收尾 — 打通 New Branch 按钮 → 对话框 → Create 流程，完成 v0.2 可新建分支版本。

---

## 交付路线图

| 阶段 | 规格 | 核心能力 | 可验证里程碑 |
|------|------|----------|--------------|
| MVP | 1-2 | 启动、选仓库、看终端 | 能打开仓库，看到主分支终端输出 |
| v0.2 | 3-4 | 多 worktree、TabBar | 能新建分支，多 tab 切换 |
| v0.3 | 5-7 | 状态监控、通知 | 能看到 agent 状态变化，收到通知 |
| v0.4 | 6-8 | Diff 审查、多仓库 | 能看代码 diff，管理多个项目 |
| v0.5 | 9-11 | 持久化、性能、分屏 | 重启不丢状态，分屏对比，完整对标 cmux |

每阶段完成后都应是一个可用的版本，可提前收集反馈。

---

## 下一阶段建议（v0.2 冲刺）

**目标**：打通规格 3 + 规格 4 剩余项，实现「新建分支 → 多 tab 切换」完整闭环。

### 优先级 1：规格 3 收尾（约 1–2 天）

1. **Sidebar + New Branch 按钮**  
   - 修改 `sidebar.on_new_branch` 回调，通过 `cx.update_entity` 调用 `open_new_branch_dialog`  
   - 参考 `on_select` 的 `(idx, window, cx)` 模式，将 `on_new_branch` 改为接收 `(Window, App)` 以获取 `update_entity`

2. **NewBranchDialogUi 可编辑输入**  
   - 用 GPUI 的 `Editor` 或 `Input` 替代只读 `div`，实现分支名输入  
   - 将输入值同步到 `NewBranchDialog::set_branch_name`

3. **Create 按钮回调**  
   - `on_create` 中通过 `entity.update` 调用 `create_branch`  
   - 确保 `set_branch_name` 在 Create 前已更新（或由 `on_create(branch_name)` 传入）

### 优先级 2：规格 4 剩余项（约 1 天）

4. **删除 worktree**  
   - Sidebar 右键菜单「Remove」或 ⌘⌫  
   - 确认对话框 → `tmux kill-pane` + `git worktree remove`  
   - 刷新 Sidebar 和 TabBar

5. **多 pane TabBar 联动**  
   - 当前 `build_pane_tabs` 仅返回单 tab，需根据 `worktree::discover_worktrees` 动态生成  
   - 切换 tab 时更新 `active_pane_target` 并切换 TerminalView 的 capture 目标

### 可选（按需）

- **规格 7 通知接入**：StatusPoller 状态变化时调用 `NotificationManager::add`，并绑定 ⌘I  
- **规格 8 多仓库切换**：Workspace Tab 切换时恢复对应 Sidebar/TabBar 状态

---