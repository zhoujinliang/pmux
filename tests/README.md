# pmux 测试套件

pmux 的测试采用分层架构，从快速回归测试到完整的 E2E 场景。

## 测试分层概览

```
tests/
├── regression/     # 核心回归测试（精简，自动化）
├── functional/     # 功能测试（按模块划分）
├── performance/    # 性能基准测试
├── e2e/            # 端到端场景测试
├── lib/            # 共享测试工具
└── *.rs            # Rust 集成测试
```

## 快速开始

```bash
# 运行所有测试（完整套件，约 30 分钟）
./tests/run_full_suite.sh

# 仅回归测试（快速，约 5 分钟）
cd tests/regression && ./run_all.sh

# 仅功能测试（中等，约 15 分钟）
cd tests/functional && ./run_all.sh

# 仅性能测试
./tests/run_performance_suite.sh

# 仅 E2E 测试
cd tests/e2e && ./run_all.sh
```

## 各层级详解

### 1. 回归测试 (`tests/regression/`)

**目标**：快速验证核心功能完整性，适合 CI/CD。

**覆盖范围**：
- Sidebar 状态颜色检测（自动化图像分析）
- 终端光标位置检测
- ANSI 颜色显示检测

**运行时间**：~5 分钟

```bash
cd tests/regression
./run_all.sh
```

### 2. 功能测试 (`tests/functional/`)

**目标**：按模块详细验证功能。

**模块划分**：

| 模块 | 测试文件 | 描述 |
|------|----------|------|
| `window/` | `window_creation_test.sh` | 窗口创建、定位、激活 |
| `workspace/` | `workspace_switching_test.sh` | Workspace 切换、状态恢复 |
| `terminal/` | `basic_commands_test.sh` | 常用命令执行 |
| `pane/` | `split_operations_test.sh` | Pane 分屏操作 |
| `input/` | `keyboard_input_test.sh` | 键盘输入处理 |
| `tui/` | `vim_compatibility_test.sh` | TUI 应用兼容性 |
| `status/` | `agent_status_test.sh` | 状态检测 |
| `render/` | `ansi_colors_test.sh` | 渲染功能 |

**运行时间**：~15 分钟

```bash
cd tests/functional
./run_all.sh

# 仅运行特定模块
./run_all.sh --test window
```

### 3. 性能测试 (`tests/performance/`)

**目标**：测量关键性能指标。

**指标**：
- 冷启动时间
- 热启动时间
- 渲染帧率（FPS）
- 打字延迟
- 输入吞吐量
- 快捷键响应时间

**运行时间**：~10 分钟

```bash
# 运行性能测试
./tests/run_performance_suite.sh

# 建立性能基线
cd tests/performance && ./run_all.sh --baseline
```

### 4. E2E 场景测试 (`tests/e2e/`)

**目标**：模拟真实用户场景。

**场景**：
- 新分支创建工作流
- Workspace 恢复工作流
- Claude Code TUI 交互场景

**运行时间**：~10 分钟

```bash
cd tests/e2e
./run_all.sh
```

### 5. Rust 集成测试

**目标**：底层组件单元测试。

```bash
cargo test

# 特定模块测试
cargo test terminal_engine::
cargo test tui_compatibility::
```

## 选择测试层级

### CI/CD 流水线
```yaml
# GitHub Actions 示例
jobs:
  regression:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run Regression Tests
        run: |
          cd tests/regression
          ./run_all.sh --ci
```

### 本地开发
```bash
# 快速验证
./tests/run_regression.sh

# 完整验证（发布前）
./tests/run_full_suite.sh
```

### 特定模块开发
```bash
# 修改了 pane 相关代码
cd tests/functional/pane
./split_operations_test.sh

# 修改了 window 相关代码
cd tests/functional/window
./window_creation_test.sh
```

## 测试工具共享

所有测试共享 `tests/regression/lib/test_utils.sh` 中的工具函数：

```bash
source "tests/regression/lib/test_utils.sh"

# 启动 pmux
start_pmux

# 发送按键
send_keystroke "hello"
send_keycode 36  # Return

# 截图验证
capture_screenshot "my_test.png"

# 报告结果
add_report_result "Test Name" "PASS"
```

## 报告输出

所有测试都生成结构化报告：

```
tests/{level}/results/YYYY-MM-DD_HH-MM-SS/
├── report.md           # 人类可读报告
├── summary.json        # 机器可读摘要
├── screenshots/        # 测试截图
│   ├── *.png
│   └── analyzed/       # 分析后的截图
└── logs/              # 详细日志
```

## 添加新测试

### 功能测试

1. 创建测试文件：
   ```bash
   touch tests/functional/{module}/my_feature_test.sh
   chmod +x tests/functional/{module}/my_feature_test.sh
   ```

2. 使用模板：
   ```bash
   #!/bin/bash
   source "tests/regression/lib/test_utils.sh"
   
   test_my_feature() {
       log_info "Test: My Feature"
       # ... 测试逻辑
       add_report_result "My Feature" "PASS"
   }
   
   # 运行
   start_pmux || exit 1
   sleep 5
   test_my_feature
   stop_pmux
   ```

3. 注册到 `tests/functional/run_all.sh`

## 常见问题

### 测试无法启动
- 确保 pmux 已编译：`cargo build`
- 检查权限：`chmod +x tests/**/*.sh`

### AppleScript 权限问题
- 在系统设置中允许终端控制 GUI
- 或手动运行一次测试脚本授权

### 图像分析失败
- 确保安装了 Python 和 PIL：`pip install Pillow`

## 下一步

查看各层级详细文档：
- [回归测试说明](regression/README.md)
- [功能测试模块](functional/)
- [性能测试指南](performance/)
- [E2E 场景文档](e2e/)
