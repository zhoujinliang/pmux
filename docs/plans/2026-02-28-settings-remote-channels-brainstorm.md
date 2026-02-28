# Settings 界面：Remote Channels 配置 — Brainstorm

## 目标

在 pmux 中提供 Settings 界面，可配置 Discord、KOOK、飞书三个远程通知通道，并带有 step-by-step 引导，帮助用户在各平台完成对应配置。

---

## 1. 入口与整体结构

### 1.1 入口

- **已有**：Settings 菜单 → Preferences…（`OpenSettings` action），目前为 TODO
- **实现**：`open_settings` 调用 `cx.focus_window()` 并向窗口发送「打开 Settings」信号，AppRoot 响应后展示 Settings  overlay/modal

### 1.2 展示形式

| 方案 | 优点 | 缺点 |
|------|------|------|
| **A. 独立 Settings 窗口** | 可多窗口、可 resize | 需管理多窗口生命周期，实现复杂 |
| **B. 主窗口内 overlay（推荐）** | 与 NewBranch/DeleteWorktree 一致，实现简单 | 单窗口 |
| **C. 侧边栏展开** | 可保留主内容可见 | 空间有限，step 引导较挤 |

**建议**：B — 与现有 modal 风格统一，全屏半透明遮罩 + 居中面板。

---

## 2. Settings 主界面结构

```
┌─────────────────────────────────────────────────────────┐
│  Settings                                    [×] Close  │
├─────────────────────────────────────────────────────────┤
│  Tabs / Sections:                                       │
│  ┌─────────────┬──────────────────────────────────────┐ │
│  │ General     │                                      │ │
│  │ Remote      │  [当前选中 section 内容]               │ │
│  │ Channels    │                                      │ │
│  └─────────────┴──────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

- **Phase 1**：仅做 Remote Channels，不引入 General 等，避免过度设计
- 主界面 = 三个 channel 卡片：Discord、KOOK、飞书
- 每个卡片：开关、状态、配置入口、引导入口

---

## 3. Channel 卡片设计（每个平台）

### 3.1 卡片内容

```
┌─────────────────────────────────────────────────────────────┐
│ [Discord Logo] Discord                          [Toggle ●]   │
│                                                             │
│ 状态: ✓ 已配置，可推送    或  ○ 未配置 / 配置错误             │
│                                                             │
│ [配置 channel_id]  [📖 查看配置指南]                         │
└─────────────────────────────────────────────────────────────┘
```

- **Toggle**：enabled/disabled，对应 config
- **状态**：根据 secrets + config 推断（有 token + channel_id 且非空 = 已配置）
- **配置入口**：内联输入或「编辑」打开 step 引导

---

## 4. Step-by-Step 引导（核心）

### 4.1 引导形式

- **方案 A**：侧边/底部折叠式「帮助面板」，与主表单并排
- **方案 B**：向导式（Wizard）— 点击「配置指南」后进入多步骤 flow，每步一屏
- **方案 C**：Inline 展开 — 在卡片下方展开步骤说明 + 输入框

**建议**：**B（Wizard）**，适合「在 Discord 创建 Bot → 复制 Token → 获取 Channel ID」这类跨平台操作，用户注意力集中。

### 4.2 Discord 引导步骤

| Step | 标题 | 内容 |
|------|------|------|
| 1 | 创建 Discord 应用 | 1. 打开 [Discord Developer Portal](https://discord.com/developers/applications) 2. 点击 New Application，命名（如 pmux） 3. 左侧 Bot → Add Bot |
| 2 | 获取 Bot Token | 1. Bot 页 → Reset Token / Copy 2. 保存到 secrets.json 的 `discord.bot_token`，或在此处粘贴（仅内存，保存时写入 secrets） |
| 3 | 邀请 Bot 到服务器 | 1. OAuth2 → URL Generator 2. Scopes: `bot` 3. Bot Permissions: Send Messages, Read Message History 4. 复制生成的 URL，在浏览器打开，选择服务器 |
| 4 | 获取 Channel ID | 1. Discord 设置 → 高级 → 开发者模式 开启 2. 右键目标频道 → 复制频道 ID 3. 粘贴到 config 的 `channel_id` |

### 4.3 KOOK 引导步骤

| Step | 标题 | 内容 |
|------|------|------|
| 1 | 创建 KOOK 机器人 | 1. 打开 [KOOK 开发者中心](https://developer.kookapp.cn/) 2. 创建应用，添加机器人 3. 获取 Bot Token |
| 2 | 获取 Bot Token | 1. 应用详情 → 机器人 → 复制 Token 2. 格式如 `1/NDUxMjE=/xxx...` 3. 保存到 secrets 的 `kook.bot_token` |
| 3 | 邀请机器人到服务器 | 1. 应用详情 → 邀请链接 2. 格式：`https://www.kookapp.cn/app/oauth2/authorize?id={BOT_ID}&permissions=1006628864&client_id={CLIENT_ID}&redirect_uri=&scope=bot` 3. 在浏览器打开，选择服务器完成邀请 |
| 4 | 获取 Channel ID | 1. 在目标频道发送任意消息 2. 通过 [频道列表 API](https://developer.kookapp.cn/doc/http/channel#获取频道列表) 或 KOOK 客户端开发者模式右键频道复制 ID |

### 4.4 飞书引导步骤

| Step | 标题 | 内容 |
|------|------|------|
| 1 | 创建飞书应用 | 1. 打开 [飞书开放平台](https://open.feishu.cn/) 2. 创建企业自建应用 3. 记录 App ID、App Secret |
| 2 | 配置凭证 | 将 App ID、App Secret 填入 secrets 的 `feishu.app_id`、`feishu.app_secret` |
| 3 | 开通权限 | 1. 应用权限 → 搜索「获取与发送群消息」「以应用身份读取通讯录」等 2. 申请并开通 |
| 4 | 获取群聊 Chat ID | 1. 将机器人加入目标群聊 2. 通过 [获取群列表 API](https://open.feishu.cn/document/uAjLw4CM/ukTMukTMukTM/reference/im-v1/chat/list) 或 群设置 → 群机器人 查看 chat_id（格式 `oc_xxx`） |
| 5 | 填入配置 | 将 chat_id 填入 config 的 `feishu.chat_id` |

---

## 5. 敏感信息处理

- **config.json**：只存 `enabled`、`channel_id` / `chat_id`（channel_id 不算高敏，可放 config）
- **secrets.json**：存 `bot_token`、`app_id`、`app_secret`
- **UI 行为**：
  - Token / Secret 输入框：type=password，不 echo
  - 已有值时显示 `••••••••`，可「重新填写」清空再输入
  - 保存时写入 secrets.json，确保目录存在

---

## 6. 数据流与持久化

```
User 编辑
    → 内存中的 "draft" 状态（Config + Secrets 的编辑副本）
    → 点击 [Save] → 写入 config.json + secrets.json
    → 可选：重启 Publisher / 重载 channels（或下次启动生效）
```

- 建议：Save 后立即重载 RemoteChannelPublisher，无需重启应用

---

## 7. 实现拆分（建议顺序）

| Phase | 内容 |
|-------|------|
| **P0** | Settings overlay 骨架：菜单触发、遮罩、关闭，空面板 |
| **P1** | Remote Channels 主界面：三个 channel 卡片，enabled toggle，状态展示 |
| **P2** | Discord 配置：内联或简单表单编辑 channel_id；secrets 中 bot_token 的编辑与保存 |
| **P3** | Discord Step-by-Step 向导：独立 Wizard 组件，4 步引导 |
| **P4** | KOOK、飞书：复用 P2/P3 模式，各自步骤内容 |
| **P5** | 保存后重载 Publisher，连接测试（可选） |

---

## 8. 技术要点

- **GPUI**：`div`、`label`、`input`、`button`，与 NewBranchDialogUi 类似
- **状态**：`SettingsState { show: bool, active_section, discord_draft, ... }`
- **Actions**：`OpenSettings` 需能通知到 AppRoot（通过 `cx.dispatch_action` 或 `window.update` 更新 AppRoot 的 `show_settings`）
- **布局**：flex，channel 卡片垂直排列，固定最大宽度（如 560px）居中

---

## 9. 待决策

1. **Wizard 与主表单关系**：点击「配置」是弹出一层 Wizard overlay，还是直接进入 Wizard 替代主 Settings 内容？
2. **连接测试**：是否提供「发送测试消息」按钮，用于验证配置？
3. **文档链接**：步骤中是否内嵌 `open::that(url)` 打开浏览器到官方文档？

---

## 10. 总结

- **入口**：Settings → Preferences…（已有菜单项）
- **形式**：主窗口 overlay，与现有 modal 一致
- **内容**：Remote Channels 三个卡片 + 每平台 Step-by-Step 向导
- **敏感信息**：token/secret 只写 secrets.json，UI 用 password 输入
