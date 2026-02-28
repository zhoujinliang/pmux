# pmux 回归测试

回归测试专注于**核心的自动化视觉验证**，确保在代码变更后关键 UI 元素仍然正确渲染。

## 测试分层

pmux 测试现在分为四层：

| 层级 | 目录 | 关注点 | 运行频率 |
|------|------|--------|----------|
| **回归测试** | `tests/regression/` | 核心功能完整性 | 每次提交 |
| **功能测试** | `tests/functional/` | 功能模块详细验证 | 每日/每周 |
| **性能测试** | `tests/performance/` | 性能基准 | 发布前 |
| **E2E 测试** | `tests/e2e/` | 用户场景 | 发布前 |

## 回归测试内容（精简）

回归测试只包含**自动化图像分析测试**：

1. **Sidebar 状态颜色检测** (`test_sidebar_status_auto.sh`)
   - 使用 Python PIL 分析截图
   - 检测运行/错误/输入状态的颜色指示器

2. **终端光标位置检测** (`test_cursor_position_auto.sh`)
   - 自动检测光标在终端中的位置
   - 验证光标渲染正确性

3. **ANSI 颜色显示检测** (`test_colors_auto.sh`)
   - 分析颜色输出截图
   - 验证颜色渲染

## 快速开始

```bash
# 运行所有回归测试
cd /Users/matt.chow/workspace/pmux/tests/regression
./run_all.sh

# 跳过编译
./run_all.sh --skip-build

# CI 模式
./run_all.sh --ci
```

## 文件结构

```
tests/regression/
├── lib/
│   ├── test_utils.sh       # 共享测试工具
│   └── image_analysis.py    # Python 图像分析
├── test_*.sh                 # 核心回归测试（已精简）
├── run_all.sh               # 主运行脚本
└── README.md                # 本文档
```

## 迁移的功能测试

以下测试已移至 `tests/functional/`：

- `test_window.sh` → `functional/window/window_creation_test.sh`
- `test_basic_commands.sh` → `functional/terminal/basic_commands_test.sh`
- `test_interaction.sh` → `functional/input/keyboard_input_test.sh`
- `test_pane_shortcuts.sh` → `functional/pane/split_operations_test.sh`
- `test_tui.sh` → `functional/tui/vim_compatibility_test.sh`
- `test_colors.sh` → `functional/render/ansi_colors_test.sh`
- `test_status_detection.sh` → `functional/status/agent_status_test.sh`
- `test_new_workspace.sh` → `functional/workspace/workspace_switching_test.sh`

## 迁移的场景测试

以下测试已移至 `tests/e2e/`：

- `test_new_branch.sh` → `e2e/workflow_new_branch.sh`
- `test_workspace_restore.sh` → `e2e/workflow_restore.sh`
- `test_claude_code_tui.sh` → `e2e/scenario_claude_code.sh`

## 迁移的性能测试

以下测试已移至 `tests/performance/`：

- `test_startup_performance.sh` → `performance/startup_benchmark.sh`
- `test_framerate.sh` → `performance/framerate_benchmark.sh`
- `test_typing_performance.sh` → `performance/typing_benchmark.sh`

## 报告位置

测试结果和截图保存在：

```
tests/regression/results/
├── YYYY-MM-DD_HH-MM-SS/
│   ├── report.md
│   ├── screenshot_*.png
│   └── logs/
```

## 与 CI 集成

回归测试设计为在 CI 中快速运行（约 5-10 分钟）：

```yaml
# GitHub Actions 示例
- name: Run Regression Tests
  run: |
    cd tests/regression
    ./run_all.sh --ci
```

## 下一步

回归测试通过后，建议运行：

1. **功能测试**：验证各个功能模块
   ```bash
   cd tests/functional && ./run_all.sh
   ```

2. **性能测试**：验证性能指标
   ```bash
   cd tests/performance && ./run_all.sh
   ```

3. **E2E 测试**：验证用户场景
   ```bash
   cd tests/e2e && ./run_all.sh
   ```
