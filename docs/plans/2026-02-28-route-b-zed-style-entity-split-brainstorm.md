# 路线 B：架构级重构（Zed 风格）— Brainstorming

> 核心：把**单一 AppRoot 树**拆成**多个可独立更新的 Model/Entity**，实现细粒度重绘。

---

## 1. 目标

| 场景 | 当前行为 | 期望行为 |
|------|----------|----------|
| 悬停顶部 icon | AppRoot 全树重绘 | 仅 TopBar 重绘 |
| 切换 worktree | AppRoot 全树重绘 | 仅 Sidebar + Content 重绘 |
| 打开 NewBranchDialog | AppRoot 全树重绘 | 仅 Dialog 重绘 |
| 终端内容更新 | AppRoot 全树重绘 | 仅 TerminalView 重绘 |

---

## 2. 前提：GPUI 是否支持子 Entity 独立 subscribe / notify？

### 2.1 结论：**支持**

根据 [GPUI Ownership 博客](https://zed.dev/blog/gpui-ownership) 和 Zed 源码：

1. **Entity 即独立状态单元**：`cx.new()` 创建的每个 Entity 由 App 拥有，有自己的 `Context<T>`。
2. **observe + notify**：Entity B 可 `cx.observe(&entity_a, |b, a, cx| { ... })`；当 A 调用 `cx.notify()` 时，B 的 callback 被调用。
3. **重绘作用域**：文档提到 "observers are signaled to re-render"、"subtree updates"；当 B 是 View 且 notify 自身时，应只触发 B 的子树重绘。
4. **Entity 可作为 Element**：`Entity<T>` 实现 `Element`，可直接作为 `div().child(topbar_entity)` 嵌入树中。

### 2.2 需验证的点

- GPUI 实际实现中，`apply_notify_effect` 是否将 dirty 标记到**具体 Entity** 而非整窗口。
- 若整窗口 invalidate，重绘时是否只对 dirty Entity 执行 `Render::render`（需查阅 Zed crates/gpui 源码确认）。

**建议**：在实施前做小规模 Spike：创建 `Entity<TopBar>`、`Entity<Sidebar>` 作为 AppRoot 子节点，让 TopBar observe 一个简单 Model，验证仅 TopBar 重绘。

---

## 3. 架构拆解

### 3.1 组件与订阅关系

| 组件 | 独立 Entity? | 订阅状态 | 触发重绘场景 |
|------|--------------|----------|--------------|
| **TopBar** | ✓ Entity | StatusCounts、notification_count、workspace_name | 状态变化、hover |
| **Sidebar** | ✓ Entity | worktree 列表、selected_index、pane_statuses | 切换 worktree、status 变化、hover |
| **NotificationPanel** | ✓ Entity | show_panel、NotificationManager.recent() | toggle、新通知 |
| **NewBranchDialog** | ✓ Entity | is_open、discover_worktrees 结果 | 打开/关闭、创建流程 |
| **TerminalView** | 已是 Component，需升格 | terminal_buffers、pane_target、content_changed | 终端输出、resize、focus |
| **StatusBar** | ✓ Entity | pane_statuses、当前 pane | status 变化 |
| **WorkspaceTabBar** | ✓ Entity | workspace_paths、active_index | 切换 workspace |
| **SplitPaneContainer** | 可保留 Component 或升格 | split_tree、terminal_buffers | layout 变化 |

### 3.2 数据层：共享 Model

将「当前在 AppRoot 内的散落状态」抽成可订阅的 Model：

```
StatusCountsModel      ← EventBus(AgentStateChange) → TopBar, StatusBar 订阅
WorktreeListModel      ← discover_worktrees, branch create/delete → Sidebar 订阅
NotificationModel      ← EventBus(Notification) → TopBar(icon), NotificationPanel 订阅
TerminalBufferModel    ← subscribe_output → TerminalView 订阅
SplitLayoutModel       ← resize, pane add/remove → SplitPaneContainer 订阅
```

---

## 4. 实施路线（分阶段）

### Phase 0：Spike（1–2 天）

1. 在 AppRoot 内 `cx.new()` 创建 `Entity<TopBar>`、`Entity<Sidebar>`。
2. 创建 `StatusCountsModel`（简单 struct + cx.new），TopBar observe 它。
3. 用定时器或按钮修改 Model，调用 notify，观察**是否只有 TopBar 重绘**（可通过 log 或 render 计数器验证）。

### Phase 1：Model 抽取

1. `StatusCountsModel`：从 AppRoot 迁出，EventBus 订阅在 Model 或专门 bridge 中。
2. `WorktreeListModel`：cached_worktrees、active_index、pane_statuses 等。
3. 确保 Model 自身不实现 Render，只被 View Entity observe。

### Phase 2：TopBar / Sidebar 升格为 Entity

1. TopBar：`impl Render for TopBar`，`cx.observe(&status_counts_model, ...)`，仅在自身状态相关时 notify。
2. Sidebar：同理，observe WorktreeListModel、NotificationModel。
3. AppRoot 只负责布局：`div().child(topbar_entity).child(sidebar_entity)...`，不再持有这些组件的内部状态。

### Phase 3：NotificationPanel / Dialogs

1. NotificationPanel：独立 Entity，observe `show_notification_panel`（可放在小 Model 或通过 EventEmitter 传递）。
2. NewBranchDialog、DeleteWorktreeDialog：独立 Entity，is_open 驱动。

### Phase 4：TerminalView 细粒度 notify

1. TerminalView 升格为 Entity（若尚未是）。
2. 终端 output 循环：**仅在 content_changed 时** `terminal_view_entity.update(cx, |_, cx| cx.notify())`，不再 `app_root_entity.update`。
3. 需要把 `terminal_view_entity` 的引用传到 output spawn 中（或通过共享 channel 让 TerminalView 自己订阅 output）。

---

## 5. 与现有 status_change_tx 的关系

当前 `app_root.rs` 有：

```rust
/// Broadcast channel for status changes - Sidebar/StatusBar subscribe, only they re-render (not AppRoot)
status_change_tx: broadcast::Sender<()>,
```

**实际情况**：订阅循环在 AppRoot 的 spawn 中，收到后执行：

```rust
entity.update(cx, |_, cx| cx.notify());  // entity = AppRoot
```

因此仍然触发 **AppRoot 全树重绘**，注释中的「only they re-render」尚未实现。

路线 B 后：EventBus → StatusCountsModel 更新并 notify → TopBar、Sidebar、StatusBar 作为 observer 各自 `cx.notify()`，仅自身子树重绘。

---

## 6. 风险与备选

| 风险 | 缓解 |
|------|------|
| GPUI 实际不提供 Entity 级 dirty | 退到 Phase 1–2 的「条件渲染 + 轻量 render 路径」（见 ui-performance-ultimate Phase 3） |
| 拆 Model 导致数据流复杂 | 先用 1–2 个 Model（如 StatusCounts）验证，再逐步迁移 |
| Entity 生命周期与 clone | 用 `Entity<T>` handle 传递，避免在闭包中捕获过多 state |

---

## 7. 参考

- [GPUI Ownership — Zed Blog](https://zed.dev/blog/gpui-ownership)
- [GPUI contexts.md](https://github.com/zed-industries/zed/blob/main/crates/gpui/docs/contexts.md)
- pmux `docs/plans/2026-02-28-ui-performance-ultimate.md` Phase 3（方案 A vs B）
- Zed `crates/terminal_view`、`crates/terminal` 的 Entity 划分
