# 发布流程（两阶段，tag 驱动）

本仓库当前使用 `cargo release` + GitHub Actions 的两阶段发布流程发布 crates。

## 关键事实

- 版本号来源：workspace 根 `Cargo.toml` 的 `[workspace.package].version`
- 候选门禁：手动运行 `.github/workflows/release-candidate.yml`
- 发布触发：推送 tag `vX.Y.Z`（也兼容 `rust-genai-vX.Y.Z`）会触发 `.github/workflows/release.yml`
- 当前 `release.yml` 为手写发布流，不是 `cargo dist init` 生成（仓库内也没有 `dist-workspace.toml`）

## 推荐操作（minor 为例）

```bash
# 1) 先预览版本提升
cargo release minor --workspace --no-publish --no-tag --no-push

# 2) 写入版本号、同步文档版本，并生成 release commit
cargo release minor --workspace --no-publish --no-tag --no-push --execute

# 3) 推送 main，让候选工作流有固定提交可检验
git push origin main
```

然后在 GitHub Actions 里手动运行 `Release Candidate`，等待 `ubuntu-latest`、`macos-latest`、`windows-latest` 三个平台全部通过。

候选通过后再推送正式 tag：

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

## 发布工作流会做什么

`release-candidate.yml` 会在三个平台上执行：

1. `cargo test --all-features --all-targets`
2. `cargo package -p rust-genai-types`
3. `cargo package -p rust-genai`
4. `cargo package -p rust-genai-macros`

`release.yml` 会在收到 tag 后按顺序执行：

1. `cargo package/publish -p rust-genai-types`
2. `cargo package/publish -p rust-genai`
3. `cargo package/publish -p rust-genai-macros`
4. 生成/更新对应 GitHub Release Notes

## 发版前检查

- `main` 分支 CI 全绿
- `Release Candidate` 三个平台全绿
- `CRATES_IO_TOKEN` 已配置到 GitHub 仓库 Secret
- 文档中的依赖示例版本与将发布的版本一致

## 文档版本自动同步

仓库根目录的 `release.toml` 已配置 `pre-release-replacements`，在 `cargo release` 的 `replace` 步骤会自动更新以下文档中的 `rust-genai` 版本号：

- `README.md`（中英文安装片段）
- `docs/getting-started.md`
- `docs/mcp.md`
- `docs/best-practices.md`

建议每次新增“安装/依赖”示例时，同步在 `release.toml` 增加对应替换规则，避免文档版本漂移。
