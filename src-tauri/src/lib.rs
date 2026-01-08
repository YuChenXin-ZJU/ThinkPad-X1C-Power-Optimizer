use encoding_rs::GB18030;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const DEFAULT_BALANCED_GUID: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
const SUB_SLEEP_GUID: &str = "238c9fa8-0aad-41ed-83f4-97be242c8f20";
const ITS_POWER_MODE_CONTROL_DISPLAY_NAME: &str = "Lenovo ITS Power Mode Control";
const AUTO_APPLY_TASK_LOGON: &str = "\\ThinkPadX1PowerOptimize\\ApplyOnLogon";
const AUTO_APPLY_TASK_RESUME: &str = "\\ThinkPadX1PowerOptimize\\ApplyOnResume";
const AUTO_APPLY_TASK_POWERSRC: &str = "\\ThinkPadX1PowerOptimize\\ApplyOnPowerSource";
const AUTO_APPLY_TASK_WATCHDOG: &str = "\\ThinkPadX1PowerOptimize\\ApplyWatchdog";
const AUTO_APPLY_SCRIPT_NAME: &str = "apply-ac-equals-dc.ps1";
const AUTO_APPLY_WATCH_SCRIPT_NAME: &str = "apply-ac-equals-dc-watch.ps1";
const AUTO_APPLY_BASELINE_NAME: &str = "baseline.json";
const AUTO_APPLY_DIR_NAME: &str = ".Thinkpad_Power";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PowerPlan {
    guid: String,
    name: String,
    is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupResult {
    backup_dir: String,
    exported: Vec<BackupItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupItem {
    guid: String,
    name: String,
    file_path: String,
    ok: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OptimizeResult {
    scheme_guid: String,
    updated_settings: u32,
    failed_settings: u32,
    skipped_sleep_settings: u32,
    its: Option<ServiceActionResult>,
    messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResetResult {
    scheme_guid: String,
    its: Option<ServiceActionResult>,
    messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceActionResult {
    display_name: String,
    service_name: Option<String>,
    ok: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceBackup {
    display_name: String,
    service_name: String,
    start_type: u32,
    was_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskItemResult {
    name: String,
    ok: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskInstallResult {
    script_path: String,
    tasks: Vec<TaskItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskRemoveResult {
    tasks: Vec<TaskItemResult>,
}

#[derive(Debug, Clone)]
enum TaskTrigger {
    Logon,
    Event { subscription: String },
}

#[derive(Debug, Clone)]
struct DcSetting {
    subgroup_guid: String,
    setting_guid: String,
    dc_value: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BaselineSetting {
    subgroup_guid: String,
    setting_guid: String,
    value: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BaselineFile {
    scheme_guid: String,
    settings: Vec<BaselineSetting>,
}

#[tauri::command]
fn list_power_plans() -> Result<Vec<PowerPlan>, String> {
    let output = run_capture("powercfg", &["/list"])?;
    Ok(parse_powercfg_list(&output))
}

#[tauri::command]
fn backup_power_plans_to_desktop() -> Result<BackupResult, String> {
    let plans = list_power_plans()?;
    let base_dir = ensure_app_desktop_dir()?;
    let ts = unix_timestamp();
    let backup_dir = base_dir.join(format!("power-plans-backup-{}", ts));
    fs::create_dir_all(&backup_dir).map_err(|e| format!("创建备份目录失败: {e}"))?;

    let mut exported = Vec::new();
    for plan in plans {
        let file_path = backup_dir.join(format!("{}-{}.pow", plan.name, plan.guid));
        let file_path = sanitize_windows_filename(&file_path);
        let mut item = BackupItem {
            guid: plan.guid.clone(),
            name: plan.name.clone(),
            file_path: file_path.to_string_lossy().to_string(),
            ok: false,
            error: None,
        };
        match run_capture(
            "powercfg",
            &[
                "/export",
                item.file_path.as_str(),
                item.guid.as_str(),
            ],
        ) {
            Ok(_) => {
                item.ok = true;
            }
            Err(e) => {
                item.error = Some(e);
            }
        }
        exported.push(item);
    }

    Ok(BackupResult {
        backup_dir: backup_dir.to_string_lossy().to_string(),
        exported,
    })
}

#[tauri::command]
fn optimize_active_power_plan(disable_its: bool) -> Result<OptimizeResult, String> {
    let scheme_guid = get_active_scheme_guid()?;
    let query_output = run_capture("powercfg", &["/query", scheme_guid.as_str()])?;
    let settings = parse_dc_settings_from_query(&query_output);

    let mut updated = 0u32;
    let mut failed = 0u32;
    let mut skipped_sleep = 0u32;
    let mut messages = Vec::new();

    if let Err(e) = write_baseline_file(&scheme_guid, &settings) {
        if messages.len() < 20 {
            messages.push(format!("保存基准失败: {}", e));
        }
    }

    for s in settings {
        if s.subgroup_guid.eq_ignore_ascii_case(SUB_SLEEP_GUID) {
            skipped_sleep += 1;
            continue;
        }
        let dc_value = s.dc_value.to_string();
        match run_capture(
            "powercfg",
            &[
                "/setacvalueindex",
                scheme_guid.as_str(),
                s.subgroup_guid.as_str(),
                s.setting_guid.as_str(),
                dc_value.as_str(),
            ],
        ) {
            Ok(_) => updated += 1,
            Err(e) => {
                failed += 1;
                if messages.len() < 20 {
                    messages.push(format!(
                        "设置失败: subgroup={} setting={} err={}",
                        s.subgroup_guid, s.setting_guid, e
                    ));
                }
            }
        }
    }

    let _ = run_capture("powercfg", &["/setactive", scheme_guid.as_str()]);

    let its = if disable_its {
        Some(disable_its_power_mode_control()?)
    } else {
        None
    };

    Ok(OptimizeResult {
        scheme_guid,
        updated_settings: updated,
        failed_settings: failed,
        skipped_sleep_settings: skipped_sleep,
        its,
        messages,
    })
}

#[tauri::command]
fn reset_power_plans_and_restore_its() -> Result<ResetResult, String> {
    let mut messages = Vec::new();
    run_capture("powercfg", &["/restoredefaultschemes"])?;
    let _ = run_capture("powercfg", &["/setactive", DEFAULT_BALANCED_GUID]);
    messages.push("已执行 powercfg /restoredefaultschemes".to_string());
    messages.push(format!("已尝试切换到平衡模式: {}", DEFAULT_BALANCED_GUID));

    let its = Some(restore_its_power_mode_control()?);

    Ok(ResetResult {
        scheme_guid: DEFAULT_BALANCED_GUID.to_string(),
        its,
        messages,
    })
}

#[tauri::command]
fn install_auto_apply_tasks() -> Result<TaskInstallResult, String> {
    let script_path = ensure_auto_apply_script()?;
    let watch_script_path = ensure_auto_apply_watch_script()?;
    if let Ok(path) = baseline_path() {
        if !path.exists() {
            if let Ok(scheme_guid) = get_active_scheme_guid() {
                if let Ok(query_output) = run_capture("powercfg", &["/query", scheme_guid.as_str()])
                {
                    let settings = parse_dc_settings_from_query(&query_output);
                    let _ = write_baseline_file(&scheme_guid, &settings);
                }
            }
        }
    }

    let mut tasks = Vec::new();
    let task_specs = vec![
        (
            AUTO_APPLY_TASK_LOGON,
            TaskTrigger::Logon,
            script_path.clone(),
            false,
        ),
        (
            AUTO_APPLY_TASK_RESUME,
            TaskTrigger::Event {
                subscription: "*[System[Provider[@Name='Microsoft-Windows-Power-Troubleshooter'] and (EventID=1)]]"
                    .to_string(),
            },
            script_path.clone(),
            false,
        ),
        (
            AUTO_APPLY_TASK_POWERSRC,
            TaskTrigger::Event {
                subscription: "*[System[Provider[@Name='Microsoft-Windows-Kernel-Power'] and (EventID=105)]]"
                    .to_string(),
            },
            script_path.clone(),
            false,
        ),
        (
            AUTO_APPLY_TASK_WATCHDOG,
            TaskTrigger::Logon,
            watch_script_path,
            true,
        ),
    ];

    for (name, trigger, path, run_forever) in task_specs {
        let result = register_task_with_powershell(name, &trigger, &path, run_forever)
            .and_then(|_| verify_task_exists(name));
        match result {
            Ok(_) => tasks.push(TaskItemResult {
                name: name.to_string(),
                ok: true,
                error: None,
            }),
            Err(e) => tasks.push(TaskItemResult {
                name: name.to_string(),
                ok: false,
                error: Some(e),
            }),
        }
    }

    let start_label = format!("{} (start)", AUTO_APPLY_TASK_WATCHDOG);
    match run_task_now(AUTO_APPLY_TASK_WATCHDOG) {
        Ok(_) => tasks.push(TaskItemResult {
            name: start_label,
            ok: true,
            error: None,
        }),
        Err(e) => tasks.push(TaskItemResult {
            name: start_label,
            ok: false,
            error: Some(e),
        }),
    }

    Ok(TaskInstallResult {
        script_path: script_path.to_string_lossy().to_string(),
        tasks,
    })
}

#[tauri::command]
fn remove_auto_apply_tasks() -> Result<TaskRemoveResult, String> {
    let mut tasks = Vec::new();
    let names = vec![
        AUTO_APPLY_TASK_LOGON,
        AUTO_APPLY_TASK_RESUME,
        AUTO_APPLY_TASK_POWERSRC,
        AUTO_APPLY_TASK_WATCHDOG,
    ];

    for name in names {
        match unregister_task_with_powershell(name) {
            Ok(_) => tasks.push(TaskItemResult {
                name: name.to_string(),
                ok: true,
                error: None,
            }),
            Err(e) => tasks.push(TaskItemResult {
                name: name.to_string(),
                ok: false,
                error: Some(e),
            }),
        }
    }

    Ok(TaskRemoveResult { tasks })
}

#[tauri::command]
fn get_privilege_status() -> Result<bool, String> {
    let mut cmd = Command::new("net");
    cmd.args(["session"]);
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("启动命令失败: net session: {e}"))?;
    Ok(output.status.success())
}

#[tauri::command]
fn relaunch_as_admin() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("无法获取程序路径: {e}"))?;
    let exe_str = exe.to_string_lossy().replace('\'', "''");
    let script = format!("Start-Process -FilePath '{}' -Verb RunAs", exe_str);
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-Command", script.as_str()]);
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let status = cmd
        .status()
        .map_err(|e| format!("启动提权失败: {e}"))?;

    if status.success() {
        std::process::exit(0);
    }

    Err(format!("启动提权失败，退出码: {:?}", status.code()))
}

fn run_capture(program: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("启动命令失败: {program} {args:?}: {e}"))?;

    let mut all = Vec::new();
    all.extend_from_slice(&output.stdout);
    all.extend_from_slice(&output.stderr);
    let text = decode_output(&all);

    if output.status.success() {
        Ok(text)
    } else {
        Err(format!(
            "命令退出码 {:?}: {} {} | 输出: {}",
            output.status.code(),
            program,
            args.join(" "),
            text.trim()
        ))
    }
}

fn run_capture_dynamic(program: &str, args: &[String]) -> Result<String, String> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("鍚姩鍛戒护澶辫触: {program} {args:?}: {e}"))?;

    let mut all = Vec::new();
    all.extend_from_slice(&output.stdout);
    all.extend_from_slice(&output.stderr);
    let text = decode_output(&all);

    if output.status.success() {
        Ok(text)
    } else {
        Err(format!(
            "鍛戒护閫€鍑虹爜 {:?}: {} {} | 杈撳嚭: {}",
            output.status.code(),
            program,
            args.join(" "),
            text.trim()
        ))
    }
}

fn decode_output(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) && bytes.len() >= 4 {
        let mut u16s = Vec::with_capacity((bytes.len() - 2) / 2);
        let mut i = 2;
        while i + 1 < bytes.len() {
            u16s.push(u16::from_le_bytes([bytes[i], bytes[i + 1]]));
            i += 2;
        }
        return String::from_utf16_lossy(&u16s);
    }

    let mut likely_utf16le = 0usize;
    for chunk in bytes.chunks_exact(2).take(2000) {
        if chunk[1] == 0 {
            likely_utf16le += 1;
        }
    }
    if likely_utf16le > 50 {
        let mut u16s = Vec::with_capacity(bytes.len() / 2);
        let mut i = 0;
        while i + 1 < bytes.len() {
            u16s.push(u16::from_le_bytes([bytes[i], bytes[i + 1]]));
            i += 2;
        }
        return String::from_utf16_lossy(&u16s);
    }

    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }

    let (cow, _, _) = GB18030.decode(bytes);
    cow.to_string()
}

fn parse_powercfg_list(text: &str) -> Vec<PowerPlan> {
    let guid_re = Regex::new(r"(?i)([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})")
        .unwrap();
    let name_re = Regex::new(r"\(([^)]+)\)").unwrap();

    let mut plans = Vec::new();
    for line in text.lines() {
        let Some(guid_cap) = guid_re.captures(line) else {
            continue;
        };
        let guid = guid_cap.get(1).unwrap().as_str().to_string();
        let name = name_re
            .captures(line)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let is_active = line.contains('*');
        plans.push(PowerPlan {
            guid,
            name,
            is_active,
        });
    }
    plans
}

fn get_active_scheme_guid() -> Result<String, String> {
    let output = run_capture("powercfg", &["/getactivescheme"])?;
    let guid_re = Regex::new(r"(?i)([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})")
        .unwrap();
    let guid = guid_re
        .captures(&output)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| format!("无法解析当前活动电源计划 GUID: {}", output.trim()))?;
    Ok(guid)
}

fn parse_dc_settings_from_query(text: &str) -> Vec<DcSetting> {
    let subgroup_re =
        Regex::new(r"(?i)(子组 GUID|Subgroup GUID):\s*([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})")
            .unwrap();
    let setting_re =
        Regex::new(r"(?i)(电源设置 GUID|Power Setting GUID):\s*([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})")
            .unwrap();
    let dc_re = Regex::new(r"(?i)(当前直流电源设置索引|Current DC Power Setting Index):\s*0x([0-9a-f]+)")
        .unwrap();

    let mut subgroup_guid: Option<String> = None;
    let mut setting_guid: Option<String> = None;
    let mut out = Vec::new();

    for line in text.lines() {
        if let Some(c) = subgroup_re.captures(line) {
            subgroup_guid = Some(c.get(2).unwrap().as_str().to_string());
            setting_guid = None;
            continue;
        }
        if let Some(c) = setting_re.captures(line) {
            setting_guid = Some(c.get(2).unwrap().as_str().to_string());
            continue;
        }
        if let Some(c) = dc_re.captures(line) {
            let Some(subgroup) = subgroup_guid.clone() else {
                continue;
            };
            let Some(setting) = setting_guid.clone() else {
                continue;
            };
            let dc_hex = c.get(2).unwrap().as_str();
            let Ok(dc_value) = u32::from_str_radix(dc_hex, 16) else {
                continue;
            };
            out.push(DcSetting {
                subgroup_guid: subgroup,
                setting_guid: setting,
                dc_value,
            });
        }
    }

    out
}

fn ensure_app_desktop_dir() -> Result<PathBuf, String> {
    let user_profile =
        env::var("USERPROFILE").map_err(|_| "无法读取 USERPROFILE 环境变量".to_string())?;
    let desktop = Path::new(&user_profile).join("Desktop");
    let app_dir = desktop.join("ThinkPadX1PowerOptimize");
    fs::create_dir_all(&app_dir).map_err(|e| format!("创建桌面应用目录失败: {e}"))?;
    Ok(app_dir)
}

fn ensure_auto_apply_script() -> Result<PathBuf, String> {
    write_auto_apply_script(AUTO_APPLY_SCRIPT_NAME, false)
}

fn ensure_auto_apply_watch_script() -> Result<PathBuf, String> {
    write_auto_apply_script(AUTO_APPLY_WATCH_SCRIPT_NAME, true)
}

fn write_auto_apply_script(name: &str, loop_mode: bool) -> Result<PathBuf, String> {
    let base_dir = ensure_auto_apply_dir()?;
    let script_path = base_dir.join(name);
    let script = build_auto_apply_script(loop_mode, &base_dir);
    fs::write(&script_path, script).map_err(|e| format!("Write script failed: {e}"))?;
    fs::metadata(&script_path).map_err(|e| format!("Verify script failed: {e}"))?;
    Ok(script_path)
}

fn build_auto_apply_script(loop_mode: bool, base_dir: &Path) -> String {
    let loop_block = if loop_mode {
        "while ($true) {\n  Invoke-Apply\n  Start-Sleep -Seconds 2\n}\n".to_string()
    } else {
        "Invoke-Apply\n".to_string()
    };

    let zh_subgroup = "\u{5b50}\u{7ec4} GUID";
    let zh_setting = "\u{7535}\u{6e90}\u{8bbe}\u{7f6e} GUID";
    let zh_dc = "\u{5f53}\u{524d}\u{76f4}\u{6d41}\u{7535}\u{6e90}\u{8bbe}\u{7f6e}\u{7d22}\u{5f15}";

    let subgroup_pattern = format!(
        r"(?i)(Subgroup GUID|{}):\s*([0-9a-f-]{{36}})",
        zh_subgroup
    );
    let setting_pattern = format!(
        r"(?i)(Power Setting GUID|{}):\s*([0-9a-f-]{{36}})",
        zh_setting
    );
    let dc_pattern = format!(
        r"(?i)(Current DC Power Setting Index|{}):\s*0x([0-9a-f]+)",
        zh_dc
    );

    let base_dir_literal = base_dir.to_string_lossy().replace('\'', "''");

    format!(
        r#"$ErrorActionPreference = "SilentlyContinue"
$ProgressPreference = "SilentlyContinue"

$subSleep = "{sub_sleep}"
$baseDir = '{base_dir}'
$baselinePath = Join-Path $baseDir 'baseline.json'

function Invoke-Apply {{
  $scheme = $null
  $schemeText = powercfg /getactivescheme
  if ($schemeText -match "(?i)([0-9a-f]{{8}}-[0-9a-f]{{4}}-[0-9a-f]{{4}}-[0-9a-f]{{4}}-[0-9a-f]{{12}})") {{
    $scheme = $Matches[1]
  }}

  $baseline = $null
  if (Test-Path $baselinePath) {{
    try {{
      $baseline = Get-Content $baselinePath -Raw | ConvertFrom-Json
    }} catch {{
      $baseline = $null
    }}
  }}
  if ($baseline -and $baseline.scheme_guid) {{
    $scheme = $baseline.scheme_guid
  }}
  if (-not $scheme) {{
    return
  }}

  $updates = New-Object System.Collections.Generic.List[Object]

  if ($baseline -and $baseline.settings) {{
    foreach ($item in $baseline.settings) {{
      $subgroup = $item.subgroup_guid
      $setting = $item.setting_guid
      if (-not $subgroup -or -not $setting) {{ continue }}
      if ($subgroup -ieq $subSleep) {{ continue }}
      $value = [Convert]::ToInt32($item.value)
      $updates.Add([pscustomobject]@{{ Subgroup=$subgroup; Setting=$setting; Value=$value; ApplyDc=$true }}) | Out-Null
    }}
  }} else {{
    $query = powercfg /query $scheme
    $subgroup = $null
    $setting = $null

    foreach ($line in ($query -split "`r?`n")) {{
      if ($line -match "{subgroup_pattern}") {{
        $subgroup = $Matches[2]
        $setting = $null
        continue
      }}
      if ($line -match "{setting_pattern}") {{
        $setting = $Matches[2]
        continue
      }}
      if ($line -match "{dc_pattern}") {{
        if (-not $subgroup -or -not $setting) {{ continue }}
        if ($subgroup -ieq $subSleep) {{ continue }}
        $dc = [Convert]::ToInt32($Matches[2], 16)
        $updates.Add([pscustomobject]@{{ Subgroup=$subgroup; Setting=$setting; Value=$dc; ApplyDc=$false }}) | Out-Null
      }}
    }}
  }}

  foreach ($u in $updates) {{
    powercfg /setacvalueindex $scheme $u.Subgroup $u.Setting $u.Value | Out-Null
    if ($u.ApplyDc) {{
      powercfg /setdcvalueindex $scheme $u.Subgroup $u.Setting $u.Value | Out-Null
    }}
  }}
  powercfg /setactive $scheme | Out-Null
}}

{loop_block}
"#,
        sub_sleep = SUB_SLEEP_GUID,
        subgroup_pattern = subgroup_pattern,
        setting_pattern = setting_pattern,
        dc_pattern = dc_pattern,
        base_dir = base_dir_literal,
        loop_block = loop_block,
    )
}

fn ensure_auto_apply_dir() -> Result<PathBuf, String> {
    let user_profile =
        env::var("USERPROFILE").map_err(|_| "鏃犳硶璇诲彇 USERPROFILE 鐜鍙橀噺".to_string())?;
    let dir = Path::new(&user_profile).join(AUTO_APPLY_DIR_NAME);
    fs::create_dir_all(&dir).map_err(|e| format!("鍒涘缓鑴氭湰鐩綍澶辫触: {e}"))?;
    Ok(dir)
}

fn baseline_path() -> Result<PathBuf, String> {
    let base_dir = ensure_auto_apply_dir()?;
    Ok(base_dir.join(AUTO_APPLY_BASELINE_NAME))
}

fn write_baseline_file(scheme_guid: &str, settings: &[DcSetting]) -> Result<PathBuf, String> {
    let mut out = Vec::new();
    for s in settings {
        if s.subgroup_guid.eq_ignore_ascii_case(SUB_SLEEP_GUID) {
            continue;
        }
        out.push(BaselineSetting {
            subgroup_guid: s.subgroup_guid.clone(),
            setting_guid: s.setting_guid.clone(),
            value: s.dc_value,
        });
    }
    let baseline = BaselineFile {
        scheme_guid: scheme_guid.to_string(),
        settings: out,
    };
    let json = serde_json::to_string_pretty(&baseline).map_err(|e| e.to_string())?;
    let path = baseline_path()?;
    fs::write(&path, json).map_err(|e| format!("写入基准配置失败: {e}"))?;
    Ok(path)
}

fn ps_quote(value: &str) -> String {
    value.replace('\'', "''")
}

fn split_task_path_name(full: &str) -> (String, String) {
    let trimmed = full.trim();
    if let Some(pos) = trimmed.rfind('\\') {
        let (path, name) = trimmed.split_at(pos + 1);
        let path = if path.is_empty() { "\\" } else { path };
        let name = name.trim_start_matches('\\');
        (path.to_string(), name.to_string())
    } else {
        ("\\".to_string(), trimmed.to_string())
    }
}

fn run_powershell_dynamic(script: &str) -> Result<String, String> {
    let args = vec![
        "-NoProfile".to_string(),
        "-ExecutionPolicy".to_string(),
        "Bypass".to_string(),
        "-Command".to_string(),
        script.to_string(),
    ];
    run_capture_dynamic("powershell.exe", &args)
}

fn register_task_with_powershell(
    full_name: &str,
    trigger: &TaskTrigger,
    script_path: &Path,
    run_forever: bool,
) -> Result<(), String> {
    match trigger {
        TaskTrigger::Event { subscription } => {
            register_event_task_with_schtasks(full_name, subscription, script_path)
        }
        TaskTrigger::Logon => register_logon_task_with_powershell(full_name, script_path, run_forever),
    }
}

fn register_event_task_with_schtasks(
    full_name: &str,
    subscription: &str,
    script_path: &Path,
) -> Result<(), String> {
    let task_cmd = format!(
        "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"{}\"",
        script_path.to_string_lossy()
    );
    let args = vec![
        "/Create".to_string(),
        "/F".to_string(),
        "/TN".to_string(),
        full_name.to_string(),
        "/SC".to_string(),
        "ONEVENT".to_string(),
        "/EC".to_string(),
        "System".to_string(),
        "/MO".to_string(),
        subscription.to_string(),
        "/TR".to_string(),
        task_cmd,
        "/RL".to_string(),
        "HIGHEST".to_string(),
        "/RU".to_string(),
        "SYSTEM".to_string(),
    ];
    run_capture_dynamic("schtasks", &args)?;
    update_task_power_settings(full_name, false)?;
    Ok(())
}

fn register_logon_task_with_powershell(
    full_name: &str,
    script_path: &Path,
    run_forever: bool,
) -> Result<(), String> {
    let (task_path, task_name) = split_task_path_name(full_name);
    let task_path_q = ps_quote(&task_path);
    let task_name_q = ps_quote(&task_name);
    let script_path_q = ps_quote(&script_path.to_string_lossy());
    let folder_path = task_path.trim_end_matches('\\');

    let folder_block = if folder_path.is_empty() || folder_path == "\\" {
        String::new()
    } else {
        format!(
            "try {{ New-ScheduledTaskFolder -Path '{}' -ErrorAction Stop | Out-Null }} catch {{ }}",
            ps_quote(folder_path)
        )
    };

    let settings_block = if run_forever {
        "$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew -Compatibility Win8"
    } else {
        "$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -Compatibility Win8"
    };

    let script = format!(
        r#"$ErrorActionPreference = "Stop"
$taskPath = '{task_path}'
$taskName = '{task_name}'
$scriptPath = '{script_path}'
{folder_block}
$action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$scriptPath`""
$trigger = New-ScheduledTaskTrigger -AtLogOn
{settings_block}
function Register-Task($principal) {{
  Register-ScheduledTask -TaskName $taskName -TaskPath $taskPath -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Force | Out-Null
}}
try {{
  $principal = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -LogonType ServiceAccount -RunLevel Highest
  Register-Task $principal
}} catch {{
  $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType InteractiveToken -RunLevel Highest
  Register-Task $principal
}}
"#,
        task_path = task_path_q,
        task_name = task_name_q,
        script_path = script_path_q,
        folder_block = folder_block,
        settings_block = settings_block,
    );

    run_powershell_dynamic(&script)?;
    update_task_power_settings(full_name, run_forever)?;
    Ok(())
}

fn update_task_power_settings(full_name: &str, run_forever: bool) -> Result<(), String> {
    let (task_path, task_name) = split_task_path_name(full_name);
    let settings_block = if run_forever {
        "$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew -Compatibility Win8"
    } else {
        "$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -Compatibility Win8"
    };
    let script = format!(
        "$ErrorActionPreference = 'Stop'; Import-Module ScheduledTasks; {settings_block}; Set-ScheduledTask -TaskName '{}' -TaskPath '{}' -Settings $settings | Out-Null",
        ps_quote(&task_name),
        ps_quote(&task_path),
        settings_block = settings_block,
    );
    run_powershell_dynamic(&script).map(|_| ())
}

fn run_task_now(full_name: &str) -> Result<(), String> {
    run_capture("schtasks", &["/Run", "/TN", full_name]).map(|_| ())
}

fn unregister_task_with_powershell(full_name: &str) -> Result<(), String> {
    let (task_path, task_name) = split_task_path_name(full_name);
    let script = format!(
        "$ErrorActionPreference = 'Stop'; Unregister-ScheduledTask -TaskName '{}' -TaskPath '{}' -Confirm:$false",
        ps_quote(&task_name),
        ps_quote(&task_path)
    );
    run_powershell_dynamic(&script).map(|_| ())
}

fn verify_task_exists(name: &str) -> Result<(), String> {
    let (task_path, task_name) = split_task_path_name(name);
    let script = format!(
        "$ErrorActionPreference = 'Stop'; Get-ScheduledTask -TaskName '{}' -TaskPath '{}' | Out-Null",
        ps_quote(&task_name),
        ps_quote(&task_path)
    );
    run_powershell_dynamic(&script).map(|_| ())
}


fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn sanitize_windows_filename(path: &Path) -> PathBuf {
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return path.to_path_buf();
    };
    let mut sanitized = String::with_capacity(file_name.len());
    for ch in file_name.chars() {
        let replaced = match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        };
        sanitized.push(replaced);
    }
    path.with_file_name(sanitized)
}

fn disable_its_power_mode_control() -> Result<ServiceActionResult, String> {
    let base_dir = ensure_app_desktop_dir()?;
    let state_path = base_dir.join("its-service-backup.json");

    let service_name = match get_service_name_by_display(ITS_POWER_MODE_CONTROL_DISPLAY_NAME) {
        Ok(v) => v,
        Err(e) => {
            return Ok(ServiceActionResult {
                display_name: ITS_POWER_MODE_CONTROL_DISPLAY_NAME.to_string(),
                service_name: None,
                ok: false,
                error: Some(e),
            })
        }
    };

    let (start_type, was_running) = match get_service_start_and_running(&service_name) {
        Ok(v) => v,
        Err(e) => {
            return Ok(ServiceActionResult {
                display_name: ITS_POWER_MODE_CONTROL_DISPLAY_NAME.to_string(),
                service_name: Some(service_name),
                ok: false,
                error: Some(e),
            })
        }
    };

    let backup = ServiceBackup {
        display_name: ITS_POWER_MODE_CONTROL_DISPLAY_NAME.to_string(),
        service_name: service_name.clone(),
        start_type,
        was_running,
    };
    let json = serde_json::to_string_pretty(&backup).map_err(|e| e.to_string())?;
    fs::write(&state_path, json).map_err(|e| format!("写入服务备份失败: {e}"))?;

    let _ = run_capture("sc", &["stop", service_name.as_str()]);
    match run_capture("sc", &["config", service_name.as_str(), "start=", "disabled"]) {
        Ok(_) => Ok(ServiceActionResult {
            display_name: ITS_POWER_MODE_CONTROL_DISPLAY_NAME.to_string(),
            service_name: Some(service_name),
            ok: true,
            error: None,
        }),
        Err(e) => Ok(ServiceActionResult {
            display_name: ITS_POWER_MODE_CONTROL_DISPLAY_NAME.to_string(),
            service_name: Some(service_name),
            ok: false,
            error: Some(e),
        }),
    }
}

fn restore_its_power_mode_control() -> Result<ServiceActionResult, String> {
    let base_dir = ensure_app_desktop_dir()?;
    let state_path = base_dir.join("its-service-backup.json");

    if let Ok(text) = fs::read_to_string(&state_path) {
        if let Ok(backup) = serde_json::from_str::<ServiceBackup>(&text) {
            let start_value = match backup.start_type {
                2 => "auto",
                3 => "demand",
                _ => "auto",
            };
            let _ = run_capture("sc", &["config", backup.service_name.as_str(), "start=", start_value]);
            if backup.was_running {
                let _ = run_capture("sc", &["start", backup.service_name.as_str()]);
            }
            return Ok(ServiceActionResult {
                display_name: backup.display_name,
                service_name: Some(backup.service_name),
                ok: true,
                error: None,
            });
        }
    }

    let service_name = match get_service_name_by_display(ITS_POWER_MODE_CONTROL_DISPLAY_NAME) {
        Ok(v) => v,
        Err(e) => {
            return Ok(ServiceActionResult {
                display_name: ITS_POWER_MODE_CONTROL_DISPLAY_NAME.to_string(),
                service_name: None,
                ok: false,
                error: Some(e),
            })
        }
    };

    let _ = run_capture("sc", &["config", service_name.as_str(), "start=", "auto"]);
    let _ = run_capture("sc", &["start", service_name.as_str()]);
    Ok(ServiceActionResult {
        display_name: ITS_POWER_MODE_CONTROL_DISPLAY_NAME.to_string(),
        service_name: Some(service_name),
        ok: true,
        error: None,
    })
}

fn get_service_name_by_display(display_name: &str) -> Result<String, String> {
    let out = run_capture("sc", &["getkeyname", display_name])?;
    let re = Regex::new(r"(?i)NAME:\s*([^\s\r\n]+)").unwrap();
    re.captures(&out)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| format!("无法找到服务: {}", display_name))
}

fn get_service_start_and_running(service_name: &str) -> Result<(u32, bool), String> {
    let qc = run_capture("sc", &["qc", service_name])?;
    let query = run_capture("sc", &["query", service_name])?;

    let start_type_re = Regex::new(r"(?i)START_TYPE\s*:\s*(\d+)").unwrap();
    let state_re = Regex::new(r"(?i)STATE\s*:\s*\d+\s+(\w+)").unwrap();

    let start_type = start_type_re
        .captures(&qc)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
        .unwrap_or(2);

    let running = state_re
        .captures(&query)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().eq_ignore_ascii_case("RUNNING"))
        .unwrap_or(false);

    Ok((start_type, running))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_power_plans,
            backup_power_plans_to_desktop,
            optimize_active_power_plan,
            reset_power_plans_and_restore_its,
            install_auto_apply_tasks,
            remove_auto_apply_tasks,
            get_privilege_status,
            relaunch_as_admin
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
