# P1: Config 支持 backend 配置

> **For Claude:** Use TDD when implementing. Consider `subagent-driven-development` for parallel tasks.

**Goal:** Add backend selection via config.json or PMUX_BACKEND env; StatusBar display and config validation.

**Architecture:** `resolve_backend(config)` in `runtime/backends/mod.rs` returns env > config > default. `create_runtime_from_env` takes `config: Option<&Config>`. Config struct gets `backend` field with validation on load.

**Tech Stack:** Rust, GPUI, serde

**来源**：`docs/design-gap-analysis.md` § 二.2 (P1) | **预估**：~1 天

---

## 1. 背景与现状

### 1.1 设计要求

| 来源 | 要求 |
|------|------|
| design.md §12 | 通过 config.json 或环境变量指定 backend |
| runtime-completion design | config.json 中 `"backend": "tmux"` |
| design-gap-analysis | Config 增加 `backend` 字段；`create_runtime_from_env` 优先读 Config，其次环境变量 |

### 1.2 当前实现

- `Config` 无 `backend` 字段
- `default_backend()` 已存在于 `config.rs` 但未使用
- `create_runtime_from_env` 仅读 `PMUX_BACKEND` 环境变量
- 调用点：`app_root.rs` 的 `start_local_session`、`switch_to_worktree`（含 recover 路径）

### 1.3 可选扩展（本计划包含）

1. **UI 展示**：StatusBar 显示当前 backend（如 `local` / `tmux`），悬停提示来源（config/env/default）
2. **配置校验**：`Config::load()` 时校验 `backend` 合法性，非法值时 log warning 并 fallback

---

## 2. 设计决策

### 2.1 优先级顺序

**Env > Config > Default**

- CI/临时调试可用 `PMUX_BACKEND=tmux pmux` 覆盖 config
- 与 12-factor 实践一致

解析链：`PMUX_BACKEND` → `config.backend` → `"local"`

### 2.2 无效 backend 处理

- 白名单：仅 `"local"`、`"tmux"` 有效
- 无效值：fallback 到 `"local"`，并 log warning（config 加载时）或静默（env 覆盖时，env 无效也 fallback）

### 2.3 实现位置

- **resolve_backend(config: Option<&Config>) -> String**：放在 `config.rs`，避免 backends 依赖 config 形成循环
- **create_runtime_from_env**：接受 `config: Option<&Config>`，内部调用 `resolve_backend(config)`

### 2.4 recover 与 config

- `recover_runtime` 使用 `WorktreeState.backend`（创建时持久化的值）
- config 的 `backend` 仅用于**新建** session；recover 不读 config ✓

---

## 3. 需要变更的文件

| 文件 | 变更 |
|------|------|
| `src/config.rs` | 添加 `backend` 字段；实现 `resolve_backend()`；`load_from_path` 中校验并 log warning |
| `src/runtime/backends/mod.rs` | `create_runtime_from_env` 接受 `config: Option<&Config>`，使用 `Config::resolve_backend` |
| `src/ui/app_root.rs` | 调用 `create_runtime_from_env` 时传入 `Config::load().ok().as_ref()`；resolve 并传给 StatusBar |
| `src/ui/status_bar.rs` | `from_context` 增加 `backend: Option<&str>`，左侧显示 `backend: local` 或 `backend: tmux` |

---

## 4. 实施任务 (Task-by-Task)

> **For Claude:** 按顺序逐项执行，每项完成后运行 `cargo test` 验证。建议 TDD：先写测试再实现。

---

### Task 1: Config 添加 backend 字段

**Files:** Modify `src/config.rs`

**Step 1: Write the failing test**

In `#[cfg(test)] mod tests`, add:

```rust
#[test]
fn test_config_backend_field() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("config.json");
    std::fs::write(&path, r#"{"backend": "tmux"}"#).unwrap();
    let config = Config::load_from_path(&path).unwrap();
    assert_eq!(config.backend, "tmux");
}
```

**Step 2: Run test to verify failure**

```bash
cargo test config::tests::test_config_backend_field --
```

Expected: FAIL — `no field `backend` on type `Config``

**Step 3: Add backend field and Default**

Add to Config struct (after `per_repo_worktree_index`):

```rust
/// Runtime backend: "local" (PTY) or "tmux". Env PMUX_BACKEND overrides.
#[serde(default = "default_backend")]
pub backend: String,
```

Update `impl Default for Config` to include `backend: default_backend()`.

**Step 4: Run test to verify pass**

```bash
cargo test config::tests::test_config_backend_field --
```

Expected: PASS

**Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): add backend field to Config"
```

---

### Task 2: 实现 resolve_backend

**Files:** Modify `src/runtime/backends/mod.rs`

**Step 1: Write the failing tests**

Add `use crate::config::Config` at top. In `#[cfg(test)] mod tests`, add:

```rust
#[test]
fn test_resolve_backend_env_overrides_config() {
    std::env::set_var(PMUX_BACKEND_ENV, "tmux");
    let config = Config { backend: "local".into(), ..Config::default() };
    assert_eq!(resolve_backend(Some(&config)), "tmux");
    std::env::remove_var(PMUX_BACKEND_ENV);
}

#[test]
fn test_resolve_backend_config_overrides_default() {
    std::env::remove_var(PMUX_BACKEND_ENV);
    let config = Config { backend: "tmux".into(), ..Config::default() };
    assert_eq!(resolve_backend(Some(&config)), "tmux");
}

#[test]
fn test_resolve_backend_invalid_fallback() {
    std::env::remove_var(PMUX_BACKEND_ENV);
    let config = Config { backend: "docker".into(), ..Config::default() };
    assert_eq!(resolve_backend(Some(&config)), "local");
}
```

**Step 2: Run tests to verify failure**

```bash
cargo test runtime::backends::tests::test_resolve_backend --
```

Expected: FAIL — `cannot find function `resolve_backend` in this scope`

**Step 3: Implement resolve_backend**

After `DEFAULT_BACKEND`, before `session_name_for_workspace`, add:

```rust
/// Resolve backend: PMUX_BACKEND env > config.backend > "local".
/// Invalid values (non-local/tmux) fall back to "local".
pub fn resolve_backend(config: Option<&Config>) -> String {
    const VALID: [&str; 2] = ["local", "tmux"];
    let from_env = std::env::var(PMUX_BACKEND_ENV).ok();
    let from_config = config.map(|c| c.backend.as_str());
    let raw = from_env.as_deref().or(from_config).unwrap_or(DEFAULT_BACKEND);
    if VALID.contains(&raw) { raw.to_string() } else { DEFAULT_BACKEND.to_string() }
}
```

**Step 4: Run tests to verify pass**

```bash
cargo test runtime::backends::tests::test_resolve_backend --
```

Expected: PASS (3 tests)

**Step 5: Commit**

```bash
git add src/runtime/backends/mod.rs
git commit -m "feat(backends): add resolve_backend for config/env/default"
```

---

### Task 3: Config 加载时校验 backend

**文件**: `src/config.rs`

**步骤**:

1. 在 `load_from_path` 中，`serde_json::from_str` 之后、`migrate_from_legacy` 之前添加：

```rust
// Validate backend; log warning and fallback if invalid
const VALID_BACKENDS: [&str; 2] = ["local", "tmux"];
if !VALID_BACKENDS.contains(&config.backend.as_str()) {
    eprintln!(
        "pmux: invalid backend '{}' in config, using 'local'. Valid: local, tmux",
        config.backend
    );
    config.backend = "local".to_string();
}
```

2. 添加 test：`test_config_load_invalid_backend_fallback` — 写入 `{"backend": "docker"}`，加载后 `config.backend == "local"`

3. 运行 `cargo test config::` 通过

---

### Task 4: create_runtime_from_env 使用 resolve_backend

**文件**: `src/runtime/backends/mod.rs`

**步骤**:

1. 添加 `use crate::config::Config`（在文件顶部）

2. 修改 `create_runtime_from_env` 签名，在末尾增加 `config: Option<&Config>`：

```rust
pub fn create_runtime_from_env(
    workspace_path: &Path,
    worktree_path: &Path,
    branch_name: &str,
    cols: u16,
    rows: u16,
    config: Option<&Config>,
) -> Result<Arc<dyn AgentRuntime>, RuntimeError> {
    let backend = resolve_backend(config);
    // 原 match backend.as_str() 逻辑，用 backend 变量替代 std::env::var
```

3. 更新 doc comment，说明优先级：env > config > default

4. 运行 `cargo test runtime::backends` 通过

---

### Task 5: AppRoot 传入 config 给 create_runtime_from_env

**文件**: `src/ui/app_root.rs`

**步骤**:

1. 找到所有 `create_runtime_from_env(worktree_path, 80, 24)` 调用

2. 改为：

```rust
let config = Config::load().ok();
let runtime = match create_runtime_from_env(&workspace_path, worktree_path, branch_name, 80, 24, config.as_ref()) {
```

调用点：`start_local_session`（约 L477）、`switch_to_worktree`（约 L873）

3. 运行 `cargo test` 通过；手动启动 `pmux` 验证 backend 选择正确

---

### Task 6: StatusBar 显示 backend

**文件**: `src/ui/status_bar.rs`

**步骤**:

1. 修改 `from_context` 签名，增加参数：

```rust
pub fn from_context(
    worktree_branch: Option<&str>,
    pane_count: usize,
    focused_pane: usize,
    status_counts: &StatusCounts,
    backend: Option<&str>,  // e.g. "local" or "tmux"
) -> Self
```

2. 在 `left` 构建中，若 `backend.is_some()`，在最前面添加：

```rust
if let Some(b) = backend {
    left.push(StatusBarItem {
        text: format!("backend: {}", b),
        title: Some("Runtime backend. Set via config.json or PMUX_BACKEND env. Priority: env > config > default".to_string()),
    });
}
```

**文件**: `src/ui/app_root.rs`

3. 在构建 StatusBar 的 closure 中，解析当前 backend 并传入。需在 app_root 顶部 `use crate::runtime::backends::resolve_backend`：

```rust
let backend = resolve_backend(Config::load().ok().as_ref());
StatusBar::from_context(
    worktree_branch.as_deref(),
    self.split_tree.pane_count(),
    self.focused_pane_index,
    &self.status_counts,
    Some(backend.as_str()),  // 显示当前 backend，悬停见 tooltip
)
```

4. 运行 `cargo test` 通过；启动 pmux 确认 StatusBar 左侧显示 `backend: local` 或 `backend: tmux`

---

### Task 7: 更新文档与注释

**步骤**:

1. `docs/design-gap-analysis.md`：将 § 二.2 的「待做」标为完成，或移至「已正确实现」

2. `docs/shell-integration.md` 或 README（若有）：补充 backend 配置说明，例如：

```markdown
## Backend 配置

- **config.json**：`"backend": "tmux"` 或 `"backend": "local"`（默认 local）
- **环境变量**：`PMUX_BACKEND=tmux` 或 `PMUX_BACKEND=local`，覆盖 config
- **优先级**：环境变量 > config > 默认 (local)
```

3. 运行 `cargo test` 全文验证

---

## 5. 验证清单

- [ ] `cargo test` 通过
- [ ] config.json 含 `"backend": "tmux"` 时，新建 session 使用 tmux
- [ ] `PMUX_BACKEND=tmux pmux` 覆盖 config 中的 local
- [ ] 无效 backend（如 `"docker"`）在 config 中触发 warning 并 fallback 到 local
- [ ] StatusBar 左侧显示 `backend: local` 或 `backend: tmux`
- [ ] 悬停 backend 项显示 tooltip
- [ ] recover 流程不受影响（仍用 WorktreeState.backend）

---

## 6. 边界情况

| 场景 | 处理 |
|------|------|
| config.json 不存在 | `Config::load()` 失败，`resolve_backend(None)` 用 env 或 default |
| config 有 backend 但 load 失败（如 JSON 损坏） | 传入 `None`，用 env 或 default |
| 同时设置 env 和 config | env 优先 |
| 新 worktree 与 recover 的 worktree 不同 backend | recover 用 state 中存的 backend；新建用 resolve_backend |

---

## Execution Handoff

Plan saved to `docs/plans/2026-02-28-p1-config-backend.md`.

**Two options:**

1. **Subagent-Driven** (this session) — Use `subagent-driven-development` skill; fresh subagent per task, review between tasks.
2. **Parallel Session** — Open new session in worktree `.worktrees/feature-config-backend`, batch execute with `executing-plans` skill.

Which approach?
