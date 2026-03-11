# 账号页刷新心智统一设计

## 背景

当前账号页把多种不同动作都混在“刷新”这个词下：

- 顶部按钮实际触发的是 usage / rate-limit refresh job
- 行级“刷新登录”实际触发的是 OAuth refresh job
- 页面切回焦点时会重新拉取列表
- `seen_ok` 会在后台 best-effort 刷新 in-use 账号的 wham 使用量

技术上这些路径都成立，但在产品心智上不统一，用户很难判断：

- 这次刷新到底刷了什么
- 是已经完成，还是只是把任务丢进队列
- 为什么列表更新了，详情还是旧的

## 目标

把用户可见的刷新动作收敛成两类，并统一反馈语义：

1. 顶部 `刷新`
   - 面向 usage / rate-limit 快照
   - 保持 job 完成后再给成功/失败反馈
2. 行级 `刷新登录`
   - 面向 OAuth 登录资料与凭据刷新
   - 改成和顶部按钮一致：等待 job 终态后再给成功/失败反馈

同时保证账号详情页和列表页看到的是同一套“最新状态”。

## 非目标

- 不改动后端 `seen_ok` 自动刷新机制
- 不把后台隐式刷新暴露成新的用户动作
- 不在本轮解决“老账号缺失的 profile 字段必须重新 OAuth 才能拿全”的上游限制

## 交互规则

### 顶部刷新

- 文案统一为 `刷新`
- 行为仍然是创建 usage / rate-limit refresh job
- 前端继续轮询 job，直到 `completed / failed / cancelled`
- 成功与失败通知要明确指向“用量/限额刷新”

### 行级刷新登录

- 文案统一为 `刷新登录`
- 创建 OAuth refresh job 后，不再立即提示“成功”
- 前端轮询 job summary 到终态，再提示：
  - 成功：登录刷新完成
  - 失败：登录刷新失败
  - 超时/取消：给出明确状态

### 详情/列表一致性

- 详情页单账号 query 也开启 `refetchOnWindowFocus: 'always'`
- 全局账号刷新失效时，连同 `oauthStatusDetail` 一起失效
- 如果详情弹窗开着，在顶部刷新或登录刷新完成后，主动 refetch 详情 query
- 详情页展示状态时优先使用“较新”的那份 OAuth status，避免详情旧 query 压住列表中的新状态

## 风险与取舍

- 批量“刷新登录”如果仍只入队不等待完成，会继续制造语义不一致
- 因此前端需要把批量刷新登录也调整成“等待所有 job 到终态后再反馈”
- 这会让批量操作等待时间更长，但心智更一致

## 验证

- `npm run i18n:check`
- `npm run i18n:hardcode -- --no-baseline`
- `node scripts/i18n/check-missing-runtime-keys.mjs`
- `npm run lint`
- `npm run build`
