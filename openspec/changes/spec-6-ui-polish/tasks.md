# 规格 6 任务清单：UI 布局与交互完善

## 任务分解

### 1. Layout System 布局系统 ✅ (TDD - GREEN)

- [ ] 1.1 创建 `LayoutConfig` 结构体
  - sidebar_width: u32
  - window_size: (u32, u32)
  - window_position: (i32, i32)
- [ ] 1.2 实现响应式布局计算
- [ ] 1.3 实现 Sidebar 宽度限制（min: 200px, max: 400px）
- [ ] 1.4 实现 TerminalView 自适应填充
- [ ] 1.5 实现分屏布局管理（预留）
- [ ] 1.6 编写单元测试（6 个测试）

**测试状态**: 0/6

### 2. Resizable Sidebar 可调整侧边栏 ✅ (TDD - GREEN)

- [x] 2.1 实现拖拽检测（鼠标在边框上）
- [x] 2.2 实现拖拽调整宽度
- [x] 2.3 实现实时预览（可选）
- [x] 2.4 实现双击自动展开/收缩
- [x] 2.5 实现 ⌘B 快捷键切换
- [x] 2.6 记住用户调整的宽度
- [x] 2.7 编写单元测试（6 个测试）

**测试状态**: 16/16 ✓

### 3. Keyboard Shortcuts 键盘快捷键 ✅ (TDD - GREEN)

- [x] 3.1 创建 `KeyBinding` 结构体
- [x] 3.2 定义所有快捷键映射
  - ⌘N: 新建 workspace
  - ⌘⇧N: 新建 branch
  - ⌘1-8: 切换 tab
  - ⌘W: 关闭当前 tab
  - ⌘D: 垂直分屏
  - ⌘⇧D: 水平分屏
  - ⌘B: 切换 sidebar
  - ⌘I: 打开通知面板
  - ⌘⇧U: 跳转到最近未读
  - ⌘?: 显示快捷键帮助
- [x] 3.3 实现快捷键注册系统
- [x] 3.4 实现快捷键冲突检测
- [x] 3.5 编写单元测试（10 个测试）

**测试状态**: 15/15 ✓

### 4. Help Panel 快捷键帮助面板 ✅ (TDD - GREEN)

- [x] 4.1 创建 `HelpPanel` 组件
- [x] 4.2 实现快捷键列表展示
- [x] 4.3 按类别分组（General、Navigation、Workspace、View）
- [x] 4.4 实现搜索过滤
- [x] 4.5 实现 ⌘? 打开/关闭
- [x] 4.6 实现 ESC 关闭
- [x] 4.7 编写单元测试（6 个测试）

**测试状态**: 13/13 ✓

### 5. Window State Persistence 窗口状态持久化 ✅ (TDD - GREEN)

- [x] 5.1 创建 `WindowState` 结构体
  - size: (u32, u32)
  - position: (i32, i32)
  - maximized: bool
- [x] 5.2 创建 `AppState` 聚合结构
  - window_state
  - sidebar_width
  - active_workspace_index
  - recent_workspaces: Vec<PathBuf>
- [x] 5.3 实现保存到配置文件
- [x] 5.4 实现启动时恢复
- [x] 5.5 实现定期自动保存
- [x] 5.6 编写单元测试（8 个测试）

**测试状态**: 12/12 ✓

### 6. Empty States 空状态优化 ✅ (TDD - GREEN)

- [x] 6.1 设计空状态视觉风格
- [x] 6.2 实现 "No workspace selected" 空状态
- [x] 6.3 实现 "No notifications" 空状态
- [x] 6.4 实现 "Empty worktree list" 引导
- [x] 6.5 添加插图/图标
- [x] 6.6 添加操作按钮（CTA）
- [x] 6.7 编写单元测试（4 个测试）

**测试状态**: 12/12 ✓

### 7. Loading & Error States 加载和错误状态 ✅ (TDD - GREEN)

- [x] 7.1 实现加载指示器（spinner）
- [x] 7.2 实现骨架屏（可选）
- [x] 7.3 实现错误提示 toast
- [x] 7.4 实现重试机制
- [x] 7.5 实现网络断开提示
- [x] 7.6 编写单元测试（4 个测试）

**测试状态**: 15/15 ✓

---

## 依赖关系

```
1. Layout System ←── 2. Resizable Sidebar
   ↓                    ↓
3. Keyboard Shortcuts ←┴── 4. Help Panel
   ↓
5. State Persistence
   ↓
6. Empty States ←── 7. Loading & Error States
```

---

## 验收标准

完成以上所有任务后，应满足以下验收标准：

1. ✅ Sidebar 支持拖拽调整宽度（200-400px）
2. ✅ ⌘B 快速展开/收缩 Sidebar
3. ✅ 所有快捷键有明确反馈
4. ✅ ⌘? 显示快捷键帮助面板
5. ✅ 窗口大小和位置自动保存恢复
6. ✅ 空状态有友好的视觉引导和 CTA
7. ✅ 加载和错误状态有适当反馈
8. ✅ 所有测试通过（目标：44+ 个测试）
9. ✅ 代码符合 Rust 风格指南

---

## 预估工作量

- **总时间**: 约 2-3 天
- **主要难点**:
  - 拖拽交互的流畅性
  - 状态持久化的时机选择
  - 跨平台窗口状态兼容性

---

## 下一步

完成本规格的所有任务后，将进入 **规格 7：控制模式集成与性能优化**，实现：
- tmux control mode 集成
- 从轮询迁移到事件驱动
- 性能监控和优化
