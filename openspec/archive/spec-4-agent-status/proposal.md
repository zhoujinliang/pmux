# Proposal: 规格 4 - Agent 状态检测与展示

## 背景

pmux 需要实时监控各个 worktree 中 agent 的运行状态，让用户一眼就能看出哪些 agent 正在运行、哪些在等待输入、哪些出现了错误。

## 目标

实现规格 4：每 500ms 轮询所有 agent pane 的输出，检测并展示其状态（Running / Waiting / Error / Idle / Unknown）。

## 范围

### 包含的功能

1. **状态检测引擎**：
   - 每 500ms 轮询所有 tmux pane
   - 基于输出内容分析状态
   - 支持多种状态类型

2. **状态类型定义**：
   - Running（● 绿）- agent 正在执行
   - Waiting（◐ 黄）- 等待用户输入
   - Idle（○ 灰）- 静止状态
   - Error（✕ 红）- 检测到错误
   - Unknown（? 紫）- 无法确定状态

3. **Sidebar 状态展示**：
   - 每个 worktree 显示状态图标
   - 颜色编码直观识别
   - 状态文字说明

4. **抖动控制**：
   - 防抖机制避免闪烁
   - 优先级系统（Error > Waiting > Running > Idle > Unknown）

### 不包含的功能

1. **系统通知**（规格 5）
2. **通知面板**
3. **静音设置**

## 技术方案

- **轮询机制**：独立线程每 500ms 执行
- **文本分析**：关键词匹配 + 正则表达式
- **状态存储**：HashMap<pane_id, AgentStatus>
- **UI 更新**：状态变化时触发重绘

## 用户体验流程

```
应用启动
  ↓
开始状态轮询线程（每 500ms）
  ↓
对每个 pane:
  - capture-pane 获取输出
  - detect_status() 分析状态
  - 状态变化？→ 更新 Sidebar
  ↓
Sidebar 实时显示最新状态
```

## 成功标准

- 状态检测准确率 > 90%
- 轮询间隔稳定在 500ms
- UI 响应流畅无卡顿
- 状态切换自然不闪烁
