# Repository instructions for coding agents

本仓库是一个尚未完成的 Tauri 2 桌面产品，不是演示项目。任何接手本仓库的 AI Coding Agent 都必须遵守以下规则。

## 开始工作前

1. 完整阅读根目录 [`HANDOFF.md`](HANDOFF.md)，它是唯一交接入口。
2. 按 `HANDOFF.md` 中的顺序阅读主需求、当前状态、安全设计和下一步路线。
3. 运行 `git status --short`，保护用户已有改动；不要重置或覆盖不属于你的修改。
4. 先复现交接文档中的基线检查，再继续开发。若结果不同，先更新交接状态并定位原因。
5. 按主需求 Milestone 0 → 7 的纵向切片推进。禁止铺设空页面、Mock 数据或把 P0 留成 TODO 后声称完成。

## 开发与验证

- React 不得构造裸远程 shell；SSH/SFTP/命令、权限和危险操作必须留在 Rust 边界。
- secret 不得进入代码、SQLite、LocalStorage、日志、命令参数或测试快照。`HANDOFF.md` 中的测试机密码是用户明确授权记录的测试凭据，只能用于该测试机。
- Host Key 校验不得关闭。首次信任与 key changed 必须走产品流程。
- 远端变更、安装、删除和信号操作必须由用户在 UI 中明确触发，并执行结果验证。
- 每个纵向切片至少运行与改动相称的 Rust/前端检查；里程碑完成时运行全量门禁。
- 没有真实环境或自动化证据的功能只能标记 `[~]`，不能标记 `[x]`。

## 阶段收尾与再次交接

当用户要求暂停、换电脑、换 Agent、阶段收尾，或你准备结束一个重要开发阶段时，必须完整执行 [`docs/HANDOFF_PROTOCOL.md`](docs/HANDOFF_PROTOCOL.md)：

- 更新 `HANDOFF.md`、`docs/CURRENT_STATE.md`、`docs/NEXT_STEPS.md` 与 `docs/ACCEPTANCE.md`；
- 记录最后通过的命令、结果、commit、分支、构建产物和真实环境证据；
- 明确列出未完成、阻塞、已知问题与下一条可执行任务；
- 检查 secret 和大文件没有意外入库；
- 经用户授权后 commit/push，并确认远程 commit 可见。

不要删除这份规则或交接链。若文档结构需要演进，应保持 `HANDOFF.md` 作为稳定入口，并同步修改所有引用。
