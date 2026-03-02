# pmux 回归测试报告

**测试时间:** Mon Mar  2 00:52:51 CST 2026
**Git Commit:** be3a5a5
**分支:** main

## 测试结果摘要


## tmux Backend E2E: keyboard control + screen recording

- ✅ **1a: Session created**: PASS
- ✅ **1b: Main window**: PASS
- ✅ **1c: Window valid**: PASS
- ✅ **1d: Terminal content**: PASS
- ✅ **2a: pwd**: PASS
- ✅ **2b: Echo marker OCR**: PASS
- ✅ **2c: ls output**: PASS
- ✅ **2d: Ctrl+L**: PASS
- ✅ **2e: Arrow keys**: PASS
- ✅ **3: No staircase**: PASS
- ✅ **4a: Worktree switch window**: PASS
- ✅ **4b: Worktree terminal content**: PASS
- ✅ **4c: Worktree input**: PASS
- ❌ **4d: Multiple windows in session**: FAIL
- ✅ **4e: Switch back to main**: PASS
- ✅ **4f: Main content preserved**: PASS
- ✅ **5a: Session persists**: PASS
- ✅ **5b: Capture content**: PASS
- ❌ **5c: Windows preserved**: FAIL
- ✅ **6a: Recovery window**: PASS
- ✅ **6b: Recovery content**: PASS
- ✅ **6c: Recovery marker**: PASS
- ✅ **6d: Recovery input**: PASS
- ✅ **6e: Session reuse**: PASS
- ✅ **7a: vim open**: PASS
- ✅ **7b: vim insert**: PASS
- ✅ **7c: vim exit**: PASS
- ✅ **8a: Session naming**: PASS
- ✅ **8b: Pane mapping**: PASS
- ✅ **8c: CC client**: PASS
- ✅ **9: Process alive**: PASS

## 测试统计

- ✅ 通过: 27
- ❌ 失败: 4
- ⚠️  跳过: 0
- **总计:** 31

