import { invoke } from "@tauri-apps/api/core";

type PowerPlan = { guid: string; name: string; is_active: boolean };
type BackupItem = {
  guid: string;
  name: string;
  file_path: string;
  ok: boolean;
  error?: string | null;
};
type BackupResult = { backup_dir: string; exported: BackupItem[] };
type ServiceActionResult = {
  display_name: string;
  service_name?: string | null;
  ok: boolean;
  error?: string | null;
};
type OptimizeResult = {
  scheme_guid: string;
  updated_settings: number;
  failed_settings: number;
  skipped_sleep_settings: number;
  its?: ServiceActionResult | null;
  messages: string[];
};
type ResetResult = {
  scheme_guid: string;
  its?: ServiceActionResult | null;
  messages: string[];
};
type TaskItemResult = {
  name: string;
  ok: boolean;
  error?: string | null;
};
type TaskInstallResult = {
  script_path: string;
  tasks: TaskItemResult[];
};
type TaskRemoveResult = {
  tasks: TaskItemResult[];
};

type Lang = "zh" | "en" | "ja";

const DISCLAIMER_ACCEPT_KEY = "disclaimerAccepted.v2";

const BUILTIN_PLANS: Record<
  string,
  {
    zh: { title: string; desc: string };
    en: { title: string; desc: string };
    ja: { title: string; desc: string };
  }
> = {
  "381b4222-f694-41f0-9685-ff5bb260df2e": {
    zh: { title: "平衡", desc: "默认方案：兼顾性能与续航的折中。" },
    en: { title: "Balanced", desc: "Default plan: balances performance and efficiency." },
    ja: { title: "バランス", desc: "既定：性能と省電力のバランス。" },
  },
  "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c": {
    zh: { title: "高性能", desc: "优先响应与性能，功耗更高。" },
    en: { title: "High performance", desc: "Prioritizes performance and responsiveness." },
    ja: { title: "高パフォーマンス", desc: "性能と応答性を優先。" },
  },
  "a1841308-3541-4fab-bc81-f71556f20b4a": {
    zh: { title: "节能", desc: "尽量降低功耗以延长续航。" },
    en: { title: "Power saver", desc: "Reduces power usage to extend battery life." },
    ja: { title: "省電力", desc: "消費電力を抑えてバッテリーを延長。" },
  },
  "e9a42b02-d5df-448d-aa00-03f14749eb61": {
    zh: { title: "卓越性能", desc: "最激进的性能策略（部分设备可见）。" },
    en: { title: "Ultimate performance", desc: "Most aggressive performance policy (if available)." },
    ja: { title: "究極のパフォーマンス", desc: "最も積極的な性能ポリシー（利用可能な場合）。" },
  },
};

const STRINGS: Record<
  Lang,
  Record<string, string>
> = {
  zh: {
    title: "ThinkPad X1 Carbon 2024 电源计划工具",
    subtitle: "",
    actions: "操作",
    output: "输出",
    clear: "清空",
    listTitle: "查看电源计划",
    listDesc: "列出所有计划并标记当前活动方案。",
    backupTitle: "备份电源计划",
    backupDesc: "导出 .pow 文件到桌面，便于恢复。",
    optTitle: "同步插电=电池（推荐）",
    optDesc: "将当前计划的 AC 值对齐 DC 值，插电调度更一致。",
    optItsTitle: "同步 + 禁用 Lenovo ITS（高级）",
    optItsDesc: "在同步基础上禁用 ITS 服务，降低被写回的概率。",
    autoTaskInstallTitle: "安装自动回写任务",
    autoTaskInstallDesc: "在登录/唤醒/电源切换后自动重写 AC=DC。",
    autoTaskRemoveTitle: "移除自动回写任务",
    autoTaskRemoveDesc: "删除本工具创建的计划任务。",
    resetTitle: "恢复默认电源计划",
    resetDesc: "还原 Windows 默认方案并尝试恢复 ITS。",
    entryPlans: "电源计划列表",
    colActive: "状态",
    colPlan: "计划（解释）",
    colSystem: "系统名称",
    colGuid: "GUID",
    active: "活动",
    inactive: "—",
    yes: "是",
    no: "否",
    backupDir: "备份目录",
    entryBackup: "备份结果",
    entryOptimize: "优化结果",
    entryReset: "重置结果",
    entryAutoTaskInstall: "安装自动回写任务",
    entryAutoTaskRemove: "移除自动回写任务",
    scriptPath: "脚本路径",
    taskList: "任务列表",
    taskName: "任务",
    taskResult: "结果",
    updated: "已更新",
    failed: "失败",
    skippedSleep: "跳过睡眠设置",
    its: "ITS 服务",
    ok: "成功",
    fail: "失败",
    adminNeeded: "部分操作需要管理员权限（备份 / 优化 / 重置）。",
    relaunchAdmin: "⚠️ 以管理员身份重新启动",
    disclaimerTitle: "免责声明",
    disclaimerBody:
      "本软件为个人实验性质工具，可能修改系统电源计划与相关服务配置。使用本软件所产生的任何风险与后果（包括但不限于数据丢失、系统异常、硬件损坏或其他损失）均由使用者自行承担，作者不对此承担责任。建议在操作前先备份电源计划，并确认已了解相关命令含义。",
    disclaimerAccept: "我已了解并继续",
    disclaimerExit: "退出",
  },
  en: {
    title: "ThinkPad X1 Carbon 2024 Power Plan Tool",
    subtitle: "",
    actions: "Actions",
    output: "Output",
    clear: "Clear",
    listTitle: "View power plans",
    listDesc: "List all plans and mark the active one.",
    backupTitle: "Backup power plans",
    backupDesc: "Export .pow files to Desktop for restore.",
    optTitle: "Sync AC to DC (recommended)",
    optDesc: "Align AC values to DC for smoother plugged-in behavior.",
    optItsTitle: "Sync + disable Lenovo ITS (advanced)",
    optItsDesc: "Disable ITS after syncing to reduce overrides.",
    autoTaskInstallTitle: "Install auto reapply tasks",
    autoTaskInstallDesc: "Reapply AC=DC on logon/resume/power change.",
    autoTaskRemoveTitle: "Remove auto reapply tasks",
    autoTaskRemoveDesc: "Delete tasks created by this tool.",
    resetTitle: "Restore default power plans",
    resetDesc: "Restore Windows defaults and try to re-enable ITS.",
    entryPlans: "Power plan list",
    colActive: "Status",
    colPlan: "Plan (explained)",
    colSystem: "System name",
    colGuid: "GUID",
    active: "Active",
    inactive: "—",
    yes: "Yes",
    no: "No",
    backupDir: "Backup folder",
    entryBackup: "Backup result",
    entryOptimize: "Optimize result",
    entryReset: "Reset result",
    entryAutoTaskInstall: "Install auto reapply tasks",
    entryAutoTaskRemove: "Remove auto reapply tasks",
    scriptPath: "Script path",
    taskList: "Task list",
    taskName: "Task",
    taskResult: "Result",
    updated: "Updated",
    failed: "Failed",
    skippedSleep: "Skipped sleep settings",
    its: "ITS service",
    ok: "OK",
    fail: "FAIL",
    adminNeeded: "Some actions require Administrator privileges (backup / optimize / reset).",
    relaunchAdmin: "⚠️ Relaunch as Administrator",
    disclaimerTitle: "Disclaimer",
    disclaimerBody:
      "This is an experimental personal tool and may change Windows power plans and related service settings. You assume all risks and consequences from using this software (including but not limited to data loss, system instability, hardware damage, or any other losses). The author is not liable for any damages. It is recommended to back up your power plans before proceeding.",
    disclaimerAccept: "I Understand & Continue",
    disclaimerExit: "Exit",
  },
  ja: {
    title: "ThinkPad X1 Carbon 2024 電源プラン ツール",
    subtitle: "",
    actions: "操作",
    output: "出力",
    clear: "クリア",
    listTitle: "電源プランを表示",
    listDesc: "すべてのプランを一覧し、現在のプランを表示します。",
    backupTitle: "電源プランをバックアップ",
    backupDesc: "Desktop に .pow を保存して復元に備えます。",
    optTitle: "AC=DC に同期（推奨）",
    optDesc: "AC 値を DC に合わせ、接続時の挙動を統一します。",
    optItsTitle: "同期 + Lenovo ITS 無効化（上級）",
    optItsDesc: "同期後に ITS を無効化して上書きを減らします。",
    autoTaskInstallTitle: "自動再適用タスクを作成",
    autoTaskInstallDesc: "ログオン/復帰/電源切替後に AC=DC を再適用。",
    autoTaskRemoveTitle: "自動再適用タスクを削除",
    autoTaskRemoveDesc: "本ツールが作成したタスクを削除します。",
    resetTitle: "既定プランを復元",
    resetDesc: "Windows 既定に戻し ITS を復元します。",
    entryPlans: "電源プラン一覧",
    colActive: "状態",
    colPlan: "プラン（説明）",
    colSystem: "システム名",
    colGuid: "GUID",
    active: "有効",
    inactive: "—",
    yes: "はい",
    no: "いいえ",
    backupDir: "バックアップ先",
    entryBackup: "バックアップ結果",
    entryOptimize: "最適化結果",
    entryReset: "リセット結果",
    entryAutoTaskInstall: "自動再適用タスクを作成",
    entryAutoTaskRemove: "自動再適用タスクを削除",
    scriptPath: "スクリプトパス",
    taskList: "タスク一覧",
    taskName: "タスク",
    taskResult: "結果",
    updated: "更新",
    failed: "失敗",
    skippedSleep: "スリープ設定をスキップ",
    its: "ITS サービス",
    ok: "成功",
    fail: "失敗",
    adminNeeded: "一部の操作は管理者権限が必要です（バックアップ / 最適化 / リセット）。",
    relaunchAdmin: "⚠️ 管理者として再起動",
    disclaimerTitle: "免責事項",
    disclaimerBody:
      "本ソフトウェアは個人の実験的ツールであり、Windows の電源プランや関連サービス設定を変更する可能性があります。本ソフトウェアの利用に伴うあらゆるリスクおよび結果（データ損失、システム不安定、ハードウェア損傷、その他の損失を含みます）は利用者が負担し、作者は責任を負いません。事前に電源プランをバックアップすることを推奨します。",
    disclaimerAccept: "理解しました（続行）",
    disclaimerExit: "終了",
  },
};

function normalizeGuid(guid: string): string {
  return guid.trim().toLowerCase();
}

function nowTime(): string {
  const d = new Date();
  return d.toLocaleString();
}

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Record<string, string> = {},
  children: (Node | string)[] = [],
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  for (const c of children) node.append(c);
  return node;
}

function planExplain(lang: Lang, guid: string): { title: string; desc: string } | null {
  const key = normalizeGuid(guid);
  const v = BUILTIN_PLANS[key];
  if (!v) return null;
  return v[lang];
}

window.addEventListener("DOMContentLoaded", () => {
  const outputEl = document.querySelector<HTMLDivElement>("#output");
  const btnClear = document.querySelector<HTMLButtonElement>("#btn-clear");
  const btnList = document.querySelector<HTMLButtonElement>("#btn-list");
  const btnBackup = document.querySelector<HTMLButtonElement>("#btn-backup");
  const btnOptimize = document.querySelector<HTMLButtonElement>("#btn-optimize");
  const btnOptimizeIts =
    document.querySelector<HTMLButtonElement>("#btn-optimize-its");
  const btnAutoTaskInstall =
    document.querySelector<HTMLButtonElement>("#btn-autotask-install");
  const btnAutoTaskRemove =
    document.querySelector<HTMLButtonElement>("#btn-autotask-remove");
  const btnReset = document.querySelector<HTMLButtonElement>("#btn-reset");

  const btnLangZh = document.querySelector<HTMLButtonElement>("#lang-zh");
  const btnLangEn = document.querySelector<HTMLButtonElement>("#lang-en");
  const btnLangJa = document.querySelector<HTMLButtonElement>("#lang-ja");

  const titleEl = document.querySelector<HTMLElement>("#title");
  const subtitleEl = document.querySelector<HTMLElement>("#subtitle");
  const secActionsTitleEl = document.querySelector<HTMLElement>("#sec-actions-title");
  const secOutputTitleEl = document.querySelector<HTMLElement>("#sec-output-title");

  const setText = (id: string, value: string) => {
    const node = document.querySelector<HTMLElement>(`#${id}`);
    if (node) node.textContent = value;
  };

  let lang: Lang = (localStorage.getItem("lang") as Lang) || "zh";

  const t = (key: string) => STRINGS[lang][key] ?? key;

  const applyLang = (next: Lang) => {
    lang = next;
    localStorage.setItem("lang", next);
    btnLangZh?.classList.toggle("active", next === "zh");
    btnLangEn?.classList.toggle("active", next === "en");
    btnLangJa?.classList.toggle("active", next === "ja");

    if (titleEl) titleEl.textContent = t("title");
    if (subtitleEl) subtitleEl.textContent = t("subtitle");
    if (secActionsTitleEl) secActionsTitleEl.textContent = t("actions");
    if (secOutputTitleEl) secOutputTitleEl.textContent = t("output");
    if (btnClear) btnClear.textContent = t("clear");

    setText("btn-list-title", t("listTitle"));
    setText("btn-list-desc", t("listDesc"));
    setText("btn-backup-title", t("backupTitle"));
    setText("btn-backup-desc", t("backupDesc"));
    setText("btn-optimize-title", t("optTitle"));
    setText("btn-optimize-desc", t("optDesc"));
    setText("btn-optimize-its-title", t("optItsTitle"));
    setText("btn-optimize-its-desc", t("optItsDesc"));
    setText("btn-autotask-install-title", t("autoTaskInstallTitle"));
    setText("btn-autotask-install-desc", t("autoTaskInstallDesc"));
    setText("btn-autotask-remove-title", t("autoTaskRemoveTitle"));
    setText("btn-autotask-remove-desc", t("autoTaskRemoveDesc"));
    setText("btn-reset-title", t("resetTitle"));
    setText("btn-reset-desc", t("resetDesc"));

    syncDisclaimerText();
  };

  btnLangZh?.addEventListener("click", () => applyLang("zh"));
  btnLangEn?.addEventListener("click", () => applyLang("en"));
  btnLangJa?.addEventListener("click", () => applyLang("ja"));

  const clear = () => {
    if (!outputEl) return;
    outputEl.innerHTML = "";
  };

  const appendEntry = (entryTitle: string, body: Node) => {
    if (!outputEl) return;
    const entry = el("section", { class: "entry" }, [
      el("div", { class: "entry-meta" }, [
        el("div", { class: "entry-title" }, [entryTitle]),
        el("div", {}, [nowTime()]),
      ]),
      body,
    ]);
    outputEl.prepend(entry);
  };

  const renderTaskTable = (tasks: TaskItemResult[]) => {
    const table = el("table", { class: "table" }, []);
    table.append(
      el("thead", {}, [
        el("tr", {}, [el("th", {}, [t("taskName")]), el("th", {}, [t("taskResult")])]),
      ]),
    );
    const tbody = el("tbody", {}, []);
    for (const item of tasks) {
      const status = item.ok
        ? el("span", { class: "ok" }, [t("ok")])
        : el("span", { class: "fail" }, [t("fail")]);
      const detail = item.ok
        ? el("div", {}, [status])
        : el("div", {}, [
            status,
            el("div", { class: "muted small" }, [item.error ?? "unknown error"]),
          ]);
      tbody.append(
        el("tr", {}, [
          el("td", {}, [el("span", { class: "mono" }, [item.name])]),
          el("td", {}, [detail]),
        ]),
      );
    }
    table.append(tbody);
    return table;
  };

  let isAdmin = false;

  const renderAdminBanner = () => {
    const host = document.querySelector<HTMLDivElement>("#admin-banner");
    if (!host) return;
    host.innerHTML = "";
    if (isAdmin) {
      host.classList.add("hidden");
      return;
    }
    const btn = el("button", { class: "btn-primary", type: "button" }, [t("relaunchAdmin")]);
    btn.addEventListener("click", async () => {
      try {
        await invoke("relaunch_as_admin");
      } catch (e) {
        appendEntry(t("relaunchAdmin"), el("div", { class: "fail" }, [String(e)]));
      }
    });
    host.append(
      el("div", { class: "banner" }, [
        el("div", { class: "muted" }, [t("adminNeeded")]),
        btn,
      ]),
    );
    host.classList.remove("hidden");
  };

  const requireAdminOrExplain = (actionTitle: string): boolean => {
    if (isAdmin) return true;
    const btn = el("button", { class: "btn-primary", type: "button" }, [t("relaunchAdmin")]);
    btn.addEventListener("click", async () => {
      try {
        await invoke("relaunch_as_admin");
      } catch (e) {
        appendEntry(t("relaunchAdmin"), el("div", { class: "fail" }, [String(e)]));
      }
    });
    appendEntry(
      actionTitle,
      el("div", {}, [
        el("div", { class: "fail" }, [t("adminNeeded")]),
        el("div", { style: "margin-top:10px;" }, [btn]),
      ]),
    );
    return false;
  };

  const syncDisclaimerText = () => {
    const modal = document.querySelector<HTMLDivElement>("#disclaimer");
    if (!modal) return;
    const accepted = localStorage.getItem(DISCLAIMER_ACCEPT_KEY) === "1";
    if (accepted) return;
    const title = modal.querySelector<HTMLElement>("#disclaimer-title");
    const body = modal.querySelector<HTMLElement>("#disclaimer-body");
    const btnAccept = modal.querySelector<HTMLButtonElement>("#disclaimer-accept");
    const btnExit = modal.querySelector<HTMLButtonElement>("#disclaimer-exit");
    if (title) title.textContent = t("disclaimerTitle");
    if (body) body.textContent = t("disclaimerBody");
    if (btnAccept) btnAccept.textContent = t("disclaimerAccept");
    if (btnExit) btnExit.textContent = t("disclaimerExit");
  };

  const showDisclaimerIfNeeded = (onContinue: () => void) => {
    const accepted = localStorage.getItem(DISCLAIMER_ACCEPT_KEY) === "1";
    const modal = document.querySelector<HTMLDivElement>("#disclaimer");
    if (!modal || accepted) {
      onContinue();
      return;
    }

    syncDisclaimerText();

    const btnAccept = modal.querySelector<HTMLButtonElement>("#disclaimer-accept");
    const btnExit = modal.querySelector<HTMLButtonElement>("#disclaimer-exit");

    if (btnAccept) {
      btnAccept.onclick = () => {
        localStorage.setItem(DISCLAIMER_ACCEPT_KEY, "1");
        modal.classList.add("hidden");
        onContinue();
      };
    }
    if (btnExit) {
      btnExit.onclick = () => {
        window.close();
      };
    }

    modal.classList.remove("hidden");
  };

  const setBusy = (busy: boolean) => {
    const buttons = [
      btnList,
      btnBackup,
      btnOptimize,
      btnOptimizeIts,
      btnAutoTaskInstall,
      btnAutoTaskRemove,
      btnReset,
    ];
    for (const b of buttons) {
      if (b) b.disabled = busy;
    }
  };

  btnClear?.addEventListener("click", () => clear());

  btnList?.addEventListener("click", async () => {
    setBusy(true);
    try {
      const plans = await invoke<PowerPlan[]>("list_power_plans");
      const table = el("table", { class: "table" }, []);
      const thead = el("thead", {}, [
        el("tr", {}, [
          el("th", {}, [t("colActive")]),
          el("th", {}, [t("colPlan")]),
          el("th", {}, [t("colSystem")]),
          el("th", {}, [t("colGuid")]),
        ]),
      ]);
      const tbody = el("tbody", {}, []);

      for (const p of plans) {
        const explain = planExplain(lang, p.guid);
        const prettyTitle = explain ? explain.title : p.name;
        const prettyDesc = explain ? explain.desc : "";
        const planCell = el("div", {}, [
          el("div", {}, [prettyTitle]),
          prettyDesc ? el("div", { class: "muted small" }, [prettyDesc]) : el("span"),
        ]);

        const badge = el("span", { class: `badge ${p.is_active ? "ok" : ""}` }, [
          p.is_active ? t("active") : t("inactive"),
        ]);

        const row = el("tr", {}, [
          el("td", {}, [badge]),
          el("td", {}, [planCell]),
          el("td", {}, [p.name]),
          el("td", {}, [el("span", { class: "mono" }, [p.guid])]),
        ]);
        tbody.append(row);
      }

      table.append(thead, tbody);
      appendEntry(t("entryPlans"), table);
    } catch (e) {
      appendEntry(t("entryPlans"), el("div", { class: "fail" }, [String(e)]));
    } finally {
      setBusy(false);
    }
  });

  btnBackup?.addEventListener("click", async () => {
    if (!requireAdminOrExplain(t("entryBackup"))) return;
    setBusy(true);
    try {
      const res = await invoke<BackupResult>("backup_power_plans_to_desktop");
      const wrap = el("div", {}, [
        el("div", { style: "margin-bottom:10px;" }, [
          el("span", { class: "badge" }, [`${t("backupDir")}: `]),
          el("span", { class: "mono" }, [res.backup_dir]),
        ]),
      ]);

      const table = el("table", { class: "table" }, []);
      table.append(
        el("thead", {}, [
          el("tr", {}, [
            el("th", {}, [t("colPlan")]),
            el("th", {}, [t("colGuid")]),
            el("th", {}, [t("entryBackup")]),
          ]),
        ]),
      );
      const tbody = el("tbody", {}, []);
      for (const item of res.exported) {
        const explain = planExplain(lang, item.guid);
        const planName = explain ? `${explain.title} — ${explain.desc}` : item.name;
        const status = item.ok ? el("span", { class: "ok" }, [t("ok")]) : el("span", { class: "fail" }, [t("fail")]);
        const detail = item.ok
          ? el("div", {}, [status, el("div", { class: "mono", style: "margin-top:6px;" }, [item.file_path])])
          : el("div", {}, [
              status,
              el("div", { class: "muted small" }, [item.error ?? "unknown error"]),
            ]);
        tbody.append(
          el("tr", {}, [
            el("td", {}, [planName]),
            el("td", {}, [el("span", { class: "mono" }, [item.guid])]),
            el("td", {}, [detail]),
          ]),
        );
      }
      table.append(tbody);
      wrap.append(table);
      appendEntry(t("entryBackup"), wrap);
    } catch (e) {
      appendEntry(t("entryBackup"), el("div", { class: "fail" }, [String(e)]));
    } finally {
      setBusy(false);
    }
  });

  btnOptimize?.addEventListener("click", async () => {
    if (!requireAdminOrExplain(t("entryOptimize"))) return;
    setBusy(true);
    try {
      const res = await invoke<OptimizeResult>("optimize_active_power_plan", {
        disableIts: false,
      });
      const explain = planExplain(lang, res.scheme_guid);
      const head = el("div", {}, [
        el("div", {}, [
          el("span", { class: "badge" }, [explain ? explain.title : res.scheme_guid]),
          el("span", { class: "mono", style: "margin-left:10px;" }, [res.scheme_guid]),
        ]),
        explain ? el("div", { class: "muted small" }, [explain.desc]) : el("span"),
      ]);

      const stats = el("div", { class: "row-wrap" }, [
        el("span", { class: "badge ok" }, [`${t("updated")}: ${res.updated_settings}`]),
        el("span", { class: `badge ${res.failed_settings === 0 ? "ok" : "fail"}` }, [`${t("failed")}: ${res.failed_settings}`]),
        el("span", { class: "badge" }, [`${t("skippedSleep")}: ${res.skipped_sleep_settings}`]),
      ]);

      const msg =
        res.messages.length > 0
          ? el("div", { class: "details" }, [
              el("div", { class: "details-title" }, ["Details"]),
              el("div", { class: "mono" }, [res.messages.join("\n")]),
            ])
          : el("span");

      appendEntry(t("entryOptimize"), el("div", {}, [head, stats, msg]));
    } catch (e) {
      appendEntry(t("entryOptimize"), el("div", { class: "fail" }, [String(e)]));
    } finally {
      setBusy(false);
    }
  });

  btnOptimizeIts?.addEventListener("click", async () => {
    if (!requireAdminOrExplain(t("entryOptimize"))) return;
    setBusy(true);
    try {
      const res = await invoke<OptimizeResult>("optimize_active_power_plan", {
        disableIts: true,
      });
      const explain = planExplain(lang, res.scheme_guid);
      const head = el("div", {}, [
        el("div", {}, [
          el("span", { class: "badge" }, [explain ? explain.title : res.scheme_guid]),
          el("span", { class: "mono", style: "margin-left:10px;" }, [res.scheme_guid]),
        ]),
        explain ? el("div", { class: "muted small" }, [explain.desc]) : el("span"),
      ]);

      const stats = el("div", { class: "row-wrap" }, [
        el("span", { class: "badge ok" }, [`${t("updated")}: ${res.updated_settings}`]),
        el("span", { class: `badge ${res.failed_settings === 0 ? "ok" : "fail"}` }, [`${t("failed")}: ${res.failed_settings}`]),
        el("span", { class: "badge" }, [`${t("skippedSleep")}: ${res.skipped_sleep_settings}`]),
      ]);

      const its =
        res.its
          ? el("div", { class: "stack" }, [
              el("span", { class: `badge ${res.its.ok ? "ok" : "fail"}` }, [
                `${t("its")}: ${res.its.ok ? t("ok") : t("fail")}`,
              ]),
              el("div", { class: "mono", style: "margin-top:6px;" }, [res.its.service_name ?? ""]),
              res.its.error
                ? el("div", { class: "muted small" }, [
                    res.its.error,
                  ])
                : el("span"),
            ])
          : el("span");

      const msg =
        res.messages.length > 0
          ? el("div", { class: "details" }, [
              el("div", { class: "details-title" }, ["Details"]),
              el("div", { class: "mono" }, [res.messages.join("\n")]),
            ])
          : el("span");

      appendEntry(t("entryOptimize"), el("div", {}, [head, stats, its, msg]));
    } catch (e) {
      appendEntry(t("entryOptimize"), el("div", { class: "fail" }, [String(e)]));
    } finally {
      setBusy(false);
    }
  });

  btnAutoTaskInstall?.addEventListener("click", async () => {
    if (!requireAdminOrExplain(t("entryAutoTaskInstall"))) return;
    setBusy(true);
    try {
      const res = await invoke<TaskInstallResult>("install_auto_apply_tasks");
      const wrap = el("div", {}, [
        el("div", { style: "margin-bottom:10px;" }, [
          el("span", { class: "badge" }, [`${t("scriptPath")}: `]),
          el("span", { class: "mono" }, [res.script_path]),
        ]),
        renderTaskTable(res.tasks),
      ]);
      appendEntry(t("entryAutoTaskInstall"), wrap);
    } catch (e) {
      appendEntry(t("entryAutoTaskInstall"), el("div", { class: "fail" }, [String(e)]));
    } finally {
      setBusy(false);
    }
  });

  btnAutoTaskRemove?.addEventListener("click", async () => {
    if (!requireAdminOrExplain(t("entryAutoTaskRemove"))) return;
    setBusy(true);
    try {
      const res = await invoke<TaskRemoveResult>("remove_auto_apply_tasks");
      appendEntry(t("entryAutoTaskRemove"), renderTaskTable(res.tasks));
    } catch (e) {
      appendEntry(t("entryAutoTaskRemove"), el("div", { class: "fail" }, [String(e)]));
    } finally {
      setBusy(false);
    }
  });

  btnReset?.addEventListener("click", async () => {
    if (!requireAdminOrExplain(t("entryReset"))) return;
    setBusy(true);
    try {
      const res = await invoke<ResetResult>("reset_power_plans_and_restore_its");
      const explain = planExplain(lang, res.scheme_guid);
      const messages = el("div", { class: "mono" }, [res.messages.join("\n")]);

      const its =
        res.its
          ? el("div", { class: "stack" }, [
              el("span", { class: `badge ${res.its.ok ? "ok" : "fail"}` }, [
                `${t("its")}: ${res.its.ok ? t("ok") : t("fail")}`,
              ]),
              el("div", { class: "mono", style: "margin-top:6px;" }, [res.its.service_name ?? ""]),
              res.its.error
                ? el("div", { class: "muted small" }, [
                    res.its.error,
                  ])
                : el("span"),
            ])
          : el("span");

      const scheme = el("div", {}, [
        el("div", {}, [
          el("span", { class: "badge" }, [explain ? explain.title : res.scheme_guid]),
          el("span", { class: "mono", style: "margin-left:10px;" }, [res.scheme_guid]),
        ]),
        explain ? el("div", { class: "muted small" }, [explain.desc]) : el("span"),
      ]);

      appendEntry(t("entryReset"), el("div", {}, [scheme, el("div", { class: "stack" }, [messages]), its]));
    } catch (e) {
      appendEntry(t("entryReset"), el("div", { class: "fail" }, [String(e)]));
    } finally {
      setBusy(false);
    }
  });

  applyLang(lang);

  showDisclaimerIfNeeded(() => {
    setTimeout(async () => {
      try {
        isAdmin = await invoke<boolean>("get_privilege_status");
      } catch {
        isAdmin = false;
      }
      renderAdminBanner();
    }, 0);
  });
});
