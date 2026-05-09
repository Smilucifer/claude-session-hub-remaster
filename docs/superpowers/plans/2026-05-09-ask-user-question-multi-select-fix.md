# AskUserQuestion 多题多选修复 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 App 内 AskUserQuestion 在多题且题目声明 `multiSelect: true` 时只能单选的问题，并保持现有 done-state 展示兼容。

**Architecture:** 仅修改 `InlineToolCard.svelte` 的多题 permission_prompt 分支。组件内部按题目维护多选状态，提交出口继续序列化为现有的逗号分隔字符串；通过新增组件交互测试先复现 bug，再以最小实现修复。

**Tech Stack:** Svelte 5, Vitest, @testing-library/svelte

---

## File Map

- Create: `src/lib/components/InlineToolCard.ask-user-question.test.ts`
- Modify: `src/lib/components/InlineToolCard.svelte`
- Verify: `src/lib/components/InlineToolCard.layout.test.ts`

---

## Task 1: 写失败测试复现多题多选 bug

**Files:**
- Create: `src/lib/components/InlineToolCard.ask-user-question.test.ts`

- [ ] **Step 1: 写组件测试，覆盖“每题可多选”的 permission_prompt 场景**

```ts
import { render, screen, fireEvent } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import InlineToolCard from "./InlineToolCard.svelte";

it("submits multiple selected options for each multi-select question", async () => {
  const onPermissionRespond = vi.fn();
  render(InlineToolCard, {
    tool: {
      tool_use_id: "ask-multi",
      tool_name: "AskUserQuestion",
      status: "permission_prompt",
      permission_request_id: "req-ask-multi",
      input: {
        questions: [
          {
            header: "审查方1",
            question: "选择参与方案讨论的 provider（可多选，第 1 组）？",
            multiSelect: true,
            options: [{ label: "Claude" }, { label: "DeepSeek" }, { label: "GLM" }],
          },
          {
            header: "审查方2",
            question: "选择参与方案讨论的 provider（可多选，第 2 组）？",
            multiSelect: true,
            options: [{ label: "QWEN" }, { label: "KIMI" }, { label: "Packy" }],
          },
        ],
      },
    },
    onPermissionRespond,
  });

  await fireEvent.click(screen.getByRole("button", { name: /Claude/ }));
  await fireEvent.click(screen.getByRole("button", { name: /DeepSeek/ }));
  await fireEvent.click(screen.getByRole("button", { name: /QWEN/ }));
  await fireEvent.click(screen.getByRole("button", { name: /KIMI/ }));
  await fireEvent.click(screen.getByRole("button", { name: "提交 (2/2)" }));

  expect(onPermissionRespond).toHaveBeenCalledWith(
    "req-ask-multi",
    "allow",
    undefined,
    expect.objectContaining({
      answers: {
        "选择参与方案讨论的 provider（可多选，第 1 组）？": "Claude, DeepSeek",
        "选择参与方案讨论的 provider（可多选，第 2 组）？": "QWEN, KIMI",
      },
    }),
  );
});
```

- [ ] **Step 2: 运行单测并确认先失败**

Run: `npm test -- src/lib/components/InlineToolCard.ask-user-question.test.ts`
Expected: FAIL，且失败原因是当前多题分支只保留每题最后一次选择。

## Task 2: 实现最小修复

**Files:**
- Modify: `src/lib/components/InlineToolCard.svelte`

- [ ] **Step 3: 为多题 multiSelect 增加按题目的选中集合状态**

```ts
let questionMultiChecked: Record<string, Record<string, boolean>> = $state({});

function toggleQuestionMulti(questionText: string, option: string) {
  const current = questionMultiChecked[questionText] ?? {};
  questionMultiChecked = {
    ...questionMultiChecked,
    [questionText]: {
      ...current,
      [option]: !current[option],
    },
  };
}
```

- [ ] **Step 4: 更新多题 permission_prompt 渲染逻辑**

```ts
function getQuestionMultiSelected(questionText: string): string[] {
  return Object.keys(questionMultiChecked[questionText] ?? {}).filter(
    (option) => questionMultiChecked[questionText]?.[option],
  );
}
```

对 `pq.multiSelect === true` 的题目使用独立按钮高亮与切换；单选题继续保留 `questionAnswers` 流程。

- [ ] **Step 5: 在统一提交出口序列化多选答案**

```ts
const answers: Record<string, string> = {};
for (const pq of parsedQuestions) {
  if (pq.multiSelect) {
    const selected = getQuestionMultiSelected(pq.question);
    const otherVal = otherActive[pq.question] ? otherText[pq.question]?.trim() : "";
    const merged = otherVal ? [...selected, otherVal] : selected;
    answers[pq.question] = merged.join(", ");
  } else {
    answers[pq.question] = questionAnswers[pq.question];
  }
}
```

- [ ] **Step 6: 保持 done-state 兼容，不扩散修改到 store / IPC**

确认 `askAnswersMap`、`askAnswerSet`、`split(", ")` 相关逻辑无需改协议。

## Task 3: 验证

**Files:**
- Verify: `src/lib/components/InlineToolCard.ask-user-question.test.ts`
- Verify: `src/lib/components/InlineToolCard.layout.test.ts`

- [ ] **Step 7: 重跑新测试，确认转绿**

Run: `npm test -- src/lib/components/InlineToolCard.ask-user-question.test.ts`
Expected: PASS

- [ ] **Step 8: 重跑现有布局测试，确认未回归**

Run: `npm test -- src/lib/components/InlineToolCard.layout.test.ts`
Expected: PASS
