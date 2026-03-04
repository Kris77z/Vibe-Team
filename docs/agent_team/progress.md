# Agent Team 推进记录与关键节点

## 1. 记录规则

本文件只记录：

1. 阶段目标变化
2. 关键里程碑
3. 阻塞与修复结论
4. 下一步决策

不记录逐条调试噪音日志。详细命令与排障细节仍放 runbook。

## 2. 已达成里程碑

### 2026-03-03 ~ 2026-03-04

1. 远程使用链路打通：
   - 开发机通过 Tailscale + SSH 访问部署机 Spacebot UI
2. `Spacebot -> Antfarm` 真实触发链路可用：
   - 可从 Spacebot UI 发起 run
3. 权限分离模型落地：
   - workflow worker 与 `__cron` helper 分离
4. unattended 运行达到 terminal：
   - `#17` 到 terminal (`completed`)
   - `#18` 再次到 terminal（中途暴露并修复 polling-only cleanup 触发点）

## 3. 当前状态判断

当前状态不是“能不能跑通”，而是“稳定性是否足够可重复”。

已确认：

1. 主链可跑通
2. 至少两轮 unattended 可到 terminal

仍需收敛：

1. 降低 `kickstart` 恢复依赖
2. 降低 `runningAtMs` 残留导致的卡顿概率
3. 收敛 terminal 结构化输出，减少 best-effort 解析

## 4. 下一步决策

当前优先级：

1. 继续 1-2 轮 unattended soak，验证可重复性
2. 达标后进入输出契约收敛
3. 然后推进“4 库 + 测试”多仓闭环 workflow

不优先：

1. UI 细化
2. Dashboard 嵌入
3. 过早切到复杂并行 workflow
