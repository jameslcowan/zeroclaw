<p align="center">
  <img src="zeroclaw.png" alt="ZeroClaw" width="200" />
</p>

<h1 align="center">ZeroClaw 🦀</h1>

<p align="center">
  <strong>零开销。零妥协。100% Rust。100% 无绑定。</strong>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" /></a>
</p>

快速、小巧、完全自治的 AI 助手基础设施 —— 随处部署，随意替换。

```
~3.4MB 二进制 · <10ms 启动 · 1,017 项测试 · 22+ 提供商 · 8 个 trait · 全面可插拔
```

### 为什么团队选择 ZeroClaw

- **默认精简：** 小巧 Rust 二进制、启动快、内存占用低。
- **安全优先：** 配对机制、严格沙箱、显式白名单、工作区作用域。
- **完全可替换：** 核心系统均为 trait（providers、channels、tools、memory、tunnels）。
- **无厂商锁定：** 支持 OpenAI 兼容 provider + 可插拔自定义端点。

## 基准快照（ZeroClaw vs OpenClaw）

本机快速基准（macOS arm64，2026 年 2 月），同一主机各运行 3 次。

| 指标 | ZeroClaw（Rust release 二进制） | OpenClaw（Node + 构建后的 `dist`） |
|---|---:|---:|
| 构建产物大小 | `target/release/zeroclaw`: **3.4 MB** | `dist/`: **28 MB** |
| `--help` 启动（冷/热） | **0.38s / ~0.00s** | **3.31s / ~1.11s** |
| `status` 命令耗时（3 次取最好） | **~0.00s** | **5.98s** |
| `--help` 观测到的最大 RSS | **~7.3 MB** | **~394 MB** |
| `status` 观测到的最大 RSS | **~7.8 MB** | **~1.52 GB** |

> 说明：使用 `/usr/bin/time -l` 测量；首次运行包含冷启动影响。OpenClaw 结果在执行 `pnpm install` + `pnpm build` 后测得。

在本地复现 ZeroClaw 数据：

```bash
cargo build --release
ls -lh target/release/zeroclaw

/usr/bin/time -l target/release/zeroclaw --help
/usr/bin/time -l target/release/zeroclaw status
```

## 快速开始

```bash
git clone https://github.com/theonlyhennygod/zeroclaw.git
cd zeroclaw
cargo build --release
cargo install --path . --force

# 快速设置（无提示）
zeroclaw onboard --api-key sk-... --provider openrouter

# 或使用交互式向导
zeroclaw onboard --interactive

# 或仅快速修复 channels/allowlists
zeroclaw onboard --channels-only

# 聊天
zeroclaw agent -m "Hello, ZeroClaw!"

# 交互模式
zeroclaw agent

# 启动 gateway（webhook 服务器）
zeroclaw gateway                # 默认：127.0.0.1:8080
zeroclaw gateway --port 0       # 随机端口（安全加固）

# 启动完整自治运行时
zeroclaw daemon

# 查看状态
zeroclaw status

# 运行系统诊断
zeroclaw doctor

# 检查 channel 健康状态
zeroclaw channel doctor

# 获取集成配置详情
zeroclaw integrations info Telegram

# 管理后台服务
zeroclaw service install
zeroclaw service status

# 从 OpenClaw 迁移记忆（先安全预览）
zeroclaw migrate openclaw --dry-run
zeroclaw migrate openclaw
```

> **开发回退（不做全局安装）：** 命令前加 `cargo run --release --`（例如：`cargo run --release -- status`）。

## 架构

每个子系统都是一个 **trait** —— 通过配置变更即可替换实现，无需改代码。

<p align="center">
  <img src="docs/architecture.svg" alt="ZeroClaw Architecture" width="900" />
</p>

| 子系统 | Trait | 内置内容 | 扩展 |
|-----------|-------|------------|--------|
| **AI 模型** | `Provider` | 22+ providers（OpenRouter、Anthropic、OpenAI、Ollama、Venice、Groq、Mistral、xAI、DeepSeek、Together、Fireworks、Perplexity、Cohere、Bedrock 等） | `custom:https://your-api.com` —— 任意 OpenAI 兼容 API |
| **通道** | `Channel` | CLI、Telegram、Discord、Slack、iMessage、Matrix、WhatsApp、Webhook | 任意消息 API |
| **记忆** | `Memory` | SQLite 混合检索（FTS5 + 向量余弦相似度）、Markdown | 任意持久化后端 |
| **工具** | `Tool` | shell、file_read、file_write、memory_store、memory_recall、memory_forget、browser_open（Brave + allowlist）、composio（可选） | 任意能力 |
| **可观测性** | `Observer` | Noop、Log、Multi | Prometheus、OTel |
| **运行时** | `RuntimeAdapter` | Native（Mac/Linux/Pi） | Docker、WASM（计划中；不支持的 kind 会快速失败） |
| **安全** | `SecurityPolicy` | Gateway 配对、沙箱、白名单、速率限制、文件系统作用域、加密密钥 | — |
| **身份** | `IdentityConfig` | OpenClaw（markdown）、AIEOS v1.1（JSON） | 任意身份格式 |
| **隧道** | `Tunnel` | None、Cloudflare、Tailscale、ngrok、Custom | 任意隧道二进制 |
| **心跳** | Engine | HEARTBEAT.md 周期任务 | — |
| **技能** | Loader | TOML 清单 + SKILL.md 说明 | 社区技能包 |
| **集成** | Registry | 9 大类 50+ 集成 | 插件系统 |

### 运行时支持（当前）

- ✅ 当前已支持：`runtime.kind = "native"`
- 🚧 已规划但尚未实现：Docker / WASM / 边缘运行时

当配置了不支持的 `runtime.kind` 时，ZeroClaw 会以明确错误退出，而不是静默回退到 native。

### 记忆系统（全栈搜索引擎）

全部自研、零外部依赖 —— 无 Pinecone、无 Elasticsearch、无 LangChain：

| 层 | 实现 |
|-------|---------------|
| **向量数据库** | Embeddings 以 BLOB 形式存储在 SQLite 中，使用余弦相似度检索 |
| **关键词检索** | FTS5 虚拟表 + BM25 评分 |
| **混合合并** | 自定义加权合并函数（`vector.rs`） |
| **Embeddings** | `EmbeddingProvider` trait —— OpenAI、自定义 URL 或 noop |
| **分块** | 基于行的 markdown 分块器，保留标题结构 |
| **缓存** | SQLite `embedding_cache` 表 + LRU 淘汰 |
| **安全重建索引** | 原子重建 FTS5 + 重新嵌入缺失向量 |

Agent 会通过工具自动召回、保存并管理记忆。

```toml
[memory]
backend = "sqlite"          # "sqlite", "markdown", "none"
auto_save = true
embedding_provider = "openai"
vector_weight = 0.7
keyword_weight = 0.3
```

## 安全

ZeroClaw 在 **每一层** 都执行安全策略 —— 不仅仅是沙箱。它通过了社区安全检查清单中的所有项。

### 安全检查清单

| # | 项目 | 状态 | 实现方式 |
|---|------|--------|-----|
| 1 | **Gateway 不对公网暴露** | ✅ | 默认绑定 `127.0.0.1`。没有隧道或未显式设置 `allow_public_bind = true` 时拒绝 `0.0.0.0`。 |
| 2 | **必须配对** | ✅ | 启动时生成 6 位一次性验证码。通过 `POST /pair` 换取 bearer token。所有 `/webhook` 请求都需要 `Authorization: Bearer <token>`。 |
| 3 | **文件系统有作用域（非 /）** | ✅ | 默认 `workspace_only = true`。屏蔽 14 个系统目录 + 4 个敏感 dotfiles。阻止空字节注入。通过规范化 + 解析后路径工作区检查检测符号链接逃逸。 |
| 4 | **仅通过隧道访问** | ✅ | 无活跃隧道时 Gateway 拒绝公网绑定。支持 Tailscale、Cloudflare、ngrok 或任意自定义隧道。 |

> **你可以自行执行 nmap：** `nmap -p 1-65535 <your-host>` —— ZeroClaw 仅绑定 localhost，除非你显式配置隧道，否则不会暴露端口。

### 通道白名单（Telegram / Discord / Slack）

入站发送者策略现已统一：

- 白名单为空 = **拒绝所有入站消息**
- `"*"` = **允许全部**（显式选择加入）
- 其他情况 = 精确匹配白名单

这样可以在默认情况下尽量降低误暴露风险。

推荐低摩擦配置（安全 + 高效）：

- **Telegram：** 白名单填写你自己的 `@username`（不含 `@`）和/或 Telegram 数字用户 ID。
- **Discord：** 白名单填写你自己的 Discord 用户 ID。
- **Slack：** 白名单填写你自己的 Slack 成员 ID（通常以 `U` 开头）。
- `"*"` 仅用于临时开放测试。

如果不确定该用哪个身份值：

1. 启动 channels 并向你的机器人发送一条消息。
2. 查看 warning 日志中的精确发送者身份。
3. 将该值加入白名单，然后重新执行 channels-only 设置。

如果日志中出现授权警告（例如：`ignoring message from unauthorized user`），
仅重新运行 channel 设置：

```bash
zeroclaw onboard --channels-only
```

### WhatsApp Business Cloud API 设置

WhatsApp 使用 Meta Cloud API + Webhook（推送式，而非轮询）：

1. **创建 Meta Business App：**
   - 打开 [developers.facebook.com](https://developers.facebook.com)
   - 新建应用 → 选择 "Business" 类型
   - 添加 "WhatsApp" 产品

2. **获取你的凭据：**
   - **Access Token：** 在 WhatsApp → API Setup 中生成 token（或创建 System User 以获取长期 token）
   - **Phone Number ID：** 在 WhatsApp → API Setup 中获取 Phone number ID
   - **Verify Token：** 由你自定义（任意随机字符串）—— Meta 会在 webhook 验证时回传它

3. **配置 ZeroClaw：**
   ```toml
   [channels_config.whatsapp]
   access_token = "EAABx..."
   phone_number_id = "123456789012345"
   verify_token = "my-secret-verify-token"
   allowed_numbers = ["+1234567890"]  # E.164 格式，或 ["*"] 允许全部
   ```

4. **通过隧道启动 gateway：**
   ```bash
   zeroclaw gateway --port 8080
   ```
   WhatsApp 需要 HTTPS，因此请使用隧道（ngrok、Cloudflare、Tailscale Funnel）。

5. **配置 Meta webhook：**
   - 在 Meta Developer Console → WhatsApp → Configuration → Webhook
   - **Callback URL：** `https://your-tunnel-url/whatsapp`
   - **Verify Token：** 与配置中的 `verify_token` 保持一致
   - 订阅 `messages` 字段

6. **测试：** 向你的 WhatsApp Business 号码发送消息 —— ZeroClaw 会通过 LLM 回复。

## 配置

配置文件：`~/.zeroclaw/config.toml`（由 `onboard` 创建）

```toml
api_key = "sk-..."
default_provider = "openrouter"
default_model = "anthropic/claude-sonnet-4-20250514"
default_temperature = 0.7

[memory]
backend = "sqlite"              # "sqlite", "markdown", "none"
auto_save = true
embedding_provider = "openai"   # "openai", "noop"
vector_weight = 0.7
keyword_weight = 0.3

[gateway]
require_pairing = true          # 首次连接要求配对码
allow_public_bind = false       # 无隧道时拒绝 0.0.0.0

[autonomy]
level = "supervised"            # "readonly", "supervised", "full"（默认：supervised）
workspace_only = true           # 默认：true —— 作用域限制在工作区
allowed_commands = ["git", "npm", "cargo", "ls", "cat", "grep"]
forbidden_paths = ["/etc", "/root", "/proc", "/sys", "~/.ssh", "~/.gnupg", "~/.aws"]

[runtime]
kind = "native"                # 当前仅支持此值；不支持的 kind 会快速失败

[heartbeat]
enabled = false
interval_minutes = 30

[tunnel]
provider = "none"               # "none", "cloudflare", "tailscale", "ngrok", "custom"

[secrets]
encrypt = true                  # API key 使用本地密钥文件加密

[browser]
enabled = false                 # 选择启用 browser_open 工具
allowed_domains = ["docs.rs"]  # 启用 browser 时必填

[composio]
enabled = false                 # 选择启用：通过 composio.dev 使用 1000+ OAuth 应用

[identity]
format = "openclaw"             # "openclaw"（默认，markdown 文件）或 "aieos"（JSON）
# aieos_path = "identity.json"  # AIEOS JSON 文件路径（相对工作区或绝对路径）
# aieos_inline = '{"identity":{"names":{"first":"Nova"}}}'  # 内联 AIEOS JSON
```

## 身份系统（AIEOS 支持）

ZeroClaw 通过两种格式支持 **身份无关** 的 AI 人格：

### OpenClaw（默认）

工作区中的传统 markdown 文件：
- `IDENTITY.md` —— Agent 是谁
- `SOUL.md` —— 核心人格与价值观
- `USER.md` —— Agent 在帮助谁
- `AGENTS.md` —— 行为指南

### AIEOS（AI Entity Object Specification）

[AIEOS](https://aieos.org) 是面向可移植 AI 身份的标准化框架。ZeroClaw 支持 AIEOS v1.1 JSON 载荷，使你可以：

- **导入身份**（来自 AIEOS 生态）
- **导出身份**（到其他兼容 AIEOS 的系统）
- **在不同 AI 模型间保持行为一致性**

#### 启用 AIEOS

```toml
[identity]
format = "aieos"
aieos_path = "identity.json"  # 相对工作区或绝对路径
```

或使用内联 JSON：

```toml
[identity]
format = "aieos"
aieos_inline = '''
{
  "identity": {
    "names": { "first": "Nova", "nickname": "N" }
  },
  "psychology": {
    "neural_matrix": { "creativity": 0.9, "logic": 0.8 },
    "traits": { "mbti": "ENTP" },
    "moral_compass": { "alignment": "Chaotic Good" }
  },
  "linguistics": {
    "text_style": { "formality_level": 0.2, "slang_usage": true }
  },
  "motivations": {
    "core_drive": "Push boundaries and explore possibilities"
  }
}
'''
```

#### AIEOS Schema 分区

| 分区 | 描述 |
|---------|-------------|
| `identity` | 姓名、简介、起源、居住地 |
| `psychology` | 神经矩阵（认知权重）、MBTI、OCEAN、道德罗盘 |
| `linguistics` | 文本风格、正式度、口头禅、禁用词 |
| `motivations` | 核心驱动力、短期/长期目标、恐惧 |
| `capabilities` | Agent 可访问的技能与工具 |
| `physicality` | 用于图像生成的视觉描述符 |
| `history` | 起源故事、教育、职业 |
| `interests` | 爱好、偏好、生活方式 |

完整 Schema 与实时示例请参见 [aieos.org](https://aieos.org)。

## Gateway API

| 端点 | 方法 | 认证 | 描述 |
|----------|--------|------|-------------|
| `/health` | GET | 无 | 健康检查（始终公开，不泄露敏感信息） |
| `/pair` | POST | `X-Pairing-Code` header | 用一次性验证码换取 bearer token |
| `/webhook` | POST | `Authorization: Bearer <token>` | 发送消息：`{"message": "your prompt"}` |
| `/whatsapp` | GET | Query params | Meta webhook 验证（hub.mode、hub.verify_token、hub.challenge） |
| `/whatsapp` | POST | 无（Meta signature） | WhatsApp 入站消息 webhook |

## 命令

| 命令 | 描述 |
|---------|-------------|
| `onboard` | 快速设置（默认） |
| `onboard --interactive` | 完整交互式 7 步向导 |
| `onboard --channels-only` | 仅重配 channels/allowlists（快速修复流程） |
| `agent -m "..."` | 单消息模式 |
| `agent` | 交互聊天模式 |
| `gateway` | 启动 webhook 服务器（默认：`127.0.0.1:8080`） |
| `gateway --port 0` | 随机端口模式 |
| `daemon` | 启动长运行自治运行时 |
| `service install/start/stop/status/uninstall` | 管理用户级后台服务 |
| `doctor` | 诊断 daemon/scheduler/channel 新鲜度 |
| `status` | 显示完整系统状态 |
| `channel doctor` | 对已配置 channels 执行健康检查 |
| `integrations info <name>` | 显示单个集成的设置/状态详情 |

## 开发

```bash
cargo build              # 开发构建
cargo build --release    # 发布构建（~3.4MB）
cargo test               # 1,017 项测试
cargo clippy             # Lint（0 warnings）
cargo fmt                # 格式化

# 运行 SQLite 与 Markdown 基准测试
cargo test --test memory_comparison -- --nocapture
```

### Pre-push hook

git hook 会在每次 push 前运行 `cargo fmt --check`、`cargo clippy -- -D warnings` 和 `cargo test`。一次启用：

```bash
git config core.hooksPath .githooks
```

如果你在开发时需要快速 push，可跳过 hook：

```bash
git push --no-verify
```

## 许可证

MIT —— 见 [LICENSE](LICENSE)

## 贡献

见 [CONTRIBUTING.md](CONTRIBUTING.md)。实现一个 trait，提交一个 PR：
- 新 `Provider` → `src/providers/`
- 新 `Channel` → `src/channels/`
- 新 `Observer` → `src/observability/`
- 新 `Tool` → `src/tools/`
- 新 `Memory` → `src/memory/`
- 新 `Tunnel` → `src/tunnel/`
- 新 `Skill` → `~/.zeroclaw/workspace/skills/<name>/`

---

**ZeroClaw** —— 零开销。零妥协。随处部署。随意替换。🦀
