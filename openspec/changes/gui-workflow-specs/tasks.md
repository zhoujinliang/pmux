## 1. Startup Workflow (规格 1)

- [ ] 1.1 实现启动页 UI（Welcome to pmux + CTA 按钮）
- [ ] 1.2 集成文件选择器，验证 Git 仓库
- [ ] 1.3 实现最近工作区列表显示
- [ ] 1.4 添加快捷键 ⌘N 支持
- [ ] 1.5 实现工作区恢复逻辑

## 2. Sidebar Worktree Management (规格 2, 4)

- [ ] 2.1 实现 Sidebar 组件基础布局
- [ ] 2.2 集成 worktree 发现功能
- [ ] 2.3 实现 Sidebar 条目渲染（分支名、状态图标）
- [ ] 2.4 添加 Sidebar 选中态和右键菜单
- [ ] 2.5 实现 ⌘B 切换 Sidebar 显隐
- [ ] 2.6 实现删除 worktree 流程（确认对话框）

## 3. TabBar Pane Switching (规格 4)

- [ ] 3.1 实现 TabBar 组件基础布局
- [ ] 3.2 实现 Pane Tab 渲染和切换
- [ ] 3.3 添加 Tab 关闭功能（× 按钮）
- [ ] 3.4 实现快捷键 ⌘1-8 切换 Tab
- [ ] 3.5 集成 tmux pane 管理

## 4. Branch Worktree Creation (规格 3)

- [ ] 4.1 实现 "+ New Branch" 按钮
- [ ] 4.2 创建新建分支对话框 UI
- [ ] 4.3 实现 git branch + worktree add 流程
- [ ] 4.4 自动创建 tmux window/pane
- [ ] 4.5 自动刷新 Sidebar 并切换
- [ ] 4.6 添加快捷键 ⌘⇧N

## 5. Agent Status Monitoring (规格 5)

- [ ] 5.1 完善 StatusPoller 轮询机制
- [ ] 5.2 优化 detect_status() 检测逻辑
- [ ] 5.3 Sidebar 显示状态图标（● ◐ ✕ ○）
- [ ] 5.4 TopBar 显示整体状态计数
- [ ] 5.5 实现状态变化通知
- [ ] 5.6 添加抖动控制和优先级处理

## 6. Diff View Integration (规格 6)

- [ ] 6.1 实现 Diff 视图触发（⌘⇧R / 右键菜单）
- [ ] 6.2 创建 review tmux window
- [ ] 6.3 启动 nvim diffview
- [ ] 6.4 渲染 review tab 终端输出
- [ ] 6.5 实现关闭 review tab 流程

## 7. Component Integration

- [ ] 7.1 重构 AppRoot 整合所有组件
- [ ] 7.2 解决 GPUI Render 宏递归问题
- [ ] 7.3 实现组件间事件传递
- [ ] 7.4 添加全局错误处理

## 8. Testing & Polish

- [ ] 8.1 编写单元测试
- [ ] 8.2 集成测试验证完整流程
- [ ] 8.3 性能优化（减少不必要的重渲染）
- [ ] 8.4 UI 细节调整（颜色、间距、动画）
