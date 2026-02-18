# 发布流程（tag 驱动）

本仓库当前使用 `cargo release` + GitHub Actions 的 tag 驱动流程发布 crates。

## 关键事实

- 版本号来源：workspace 根 `Cargo.toml` 的 `[workspace.package].version`
- 发布触发：推送 tag `vX.Y.Z`（也兼容 `rust-genai-vX.Y.Z`）会触发 `.github/workflows/release.yml`
- 当前 `release.yml` 为手写发布流，不是 `cargo dist init` 生成（仓库内也没有 `dist-workspace.toml`）

## 推荐操作（minor 为例）

```bash
# 1) 先预览（不落盘）
cargo release minor --workspace --no-publish

# 2) 执行版本更新 + 提交 + 打 tag + 推送
cargo release minor --workspace --no-publish --execute
```

## 发布工作流会做什么

`release.yml` 会按顺序执行：

1. `cargo package/publish -p rust-genai-types`
2. `cargo package/publish -p rust-genai`
3. `cargo package/publish -p rust-genai-macros`
4. 生成/更新对应 GitHub Release Notes

## 发版前检查

- `main` 分支 CI 全绿
- `CRATES_IO_TOKEN` 已配置到 GitHub 仓库 Secret
- 文档中的依赖示例版本与将发布的版本一致

## 文档版本自动同步

仓库根目录的 `release.toml` 已配置 `pre-release-replacements`，在 `cargo release` 的 `replace` 步骤会自动更新以下文档中的 `rust-genai` 版本号：

- `README.md`（中英文安装片段）
- `docs/getting-started.md`
- `docs/mcp.md`
- `docs/best-practices.md`

建议每次新增“安装/依赖”示例时，同步在 `release.toml` 增加对应替换规则，避免文档版本漂移。
