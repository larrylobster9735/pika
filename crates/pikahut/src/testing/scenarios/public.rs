use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use pika_core::{AppAction, AuthState, CallStatus, FfiApp};
use pika_relay_profiles::{app_default_key_package_relays, app_default_message_relays};

use crate::config;
use crate::testing::{ArtifactPolicy, CommandRunner, CommandSpec, TestContext};

use super::artifacts::{self, CommandOutcomeRecord};
use super::types::ScenarioRunOutput;

const ENV_PIKA_TEST_NSEC: &str = "PIKA_TEST_NSEC";
const ENV_PIKA_UI_E2E_BOT_NPUB: &str = "PIKA_UI_E2E_BOT_NPUB";
const ENV_PIKA_UI_E2E_RELAYS: &str = "PIKA_UI_E2E_RELAYS";
const ENV_PIKA_UI_E2E_KP_RELAYS: &str = "PIKA_UI_E2E_KP_RELAYS";
const ENV_PIKA_UI_E2E_NSEC: &str = "PIKA_UI_E2E_NSEC";
const DEFAULT_BOT_NPUB: &str = "npub1z6ujr8rad5zp9sr9w22rkxm0truulf2jntrks6rlwskhdmqsawpqmnjlcp";
const DEFAULT_MOQ_URL: &str = "https://us-east.moq.logos.surf/anon";

static DOTENV_DEFAULTS: OnceLock<HashMap<String, String>> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
pub enum PublicUiPlatform {
    Android,
    Ios,
    All,
}

pub fn optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| dotenv_defaults().get(name).cloned())
}

pub fn run_public_ui_e2e(platform: PublicUiPlatform) -> Result<ScenarioRunOutput> {
    let run_name = match platform {
        PublicUiPlatform::Android => "ui-e2e-public-android",
        PublicUiPlatform::Ios => "ui-e2e-public-ios",
        PublicUiPlatform::All => "ui-e2e-public-all",
    };

    let mut context = TestContext::builder(run_name)
        .artifact_policy(ArtifactPolicy::PreserveOnFailure)
        .build()?;
    let root = config::find_workspace_root()?;
    let runner = CommandRunner::new(&context);
    let mut outcomes = Vec::new();

    match platform {
        PublicUiPlatform::Android => {
            outcomes.extend(run_public_android(&runner, &root)?);
        }
        PublicUiPlatform::Ios => {
            outcomes.extend(run_public_ios(&runner, &root)?);
        }
        PublicUiPlatform::All => {
            outcomes.extend(run_public_ios(&runner, &root)?);
            outcomes.extend(run_public_android(&runner, &root)?);
        }
    }

    let mut result = ScenarioRunOutput::completed(context.state_dir().to_path_buf())
        .with_metadata("platform", format!("{:?}", platform));
    let summary = artifacts::write_standard_summary(
        &context,
        "public::ui_e2e",
        &result,
        outcomes,
        BTreeMap::new(),
    )?;
    result = result.with_summary(summary);

    context.mark_success();
    Ok(result)
}

pub fn run_deployed_bot_call_flow() -> Result<ScenarioRunOutput> {
    pika_core::init_rustls_crypto_provider();

    let mut context = TestContext::builder("deployed-bot-call-flow")
        .artifact_policy(ArtifactPolicy::PreserveOnFailure)
        .build()?;

    let nsec = optional_env(ENV_PIKA_TEST_NSEC)
        .or_else(|| optional_env(ENV_PIKA_UI_E2E_NSEC))
        .ok_or_else(|| anyhow!("missing {ENV_PIKA_TEST_NSEC} and {ENV_PIKA_UI_E2E_NSEC}"))?;
    let bot_npub = env_or_default("PIKA_BOT_NPUB", DEFAULT_BOT_NPUB);
    let relays = env_csv_or_default_fn("PIKA_E2E_RELAYS", app_default_message_relays);
    let kp_relays = env_csv_or_default_fn("PIKA_E2E_KP_RELAYS", app_default_key_package_relays);
    let moq_url = env_or_default("PIKA_CALL_MOQ_URL", DEFAULT_MOQ_URL);

    let app_state_dir = context.state_dir().join("deployed-bot-app");
    fs::create_dir_all(&app_state_dir)?;
    let config_path = write_config_multi(&app_state_dir, &relays, &kp_relays, &moq_url)?;

    let app = FfiApp::new(app_state_dir.to_string_lossy().to_string(), String::new());

    app.dispatch(AppAction::Login { nsec });
    wait_until_true("logged in", Duration::from_secs(20), || {
        matches!(app.state().auth, AuthState::LoggedIn { .. })
    })?;

    app.dispatch(AppAction::CreateChat {
        peer_npub: bot_npub.clone(),
    });
    wait_until_true("chat opened", Duration::from_secs(120), || {
        app.state().current_chat.is_some()
    })?;

    let chat_id = app
        .state()
        .current_chat
        .as_ref()
        .map(|chat| chat.chat_id.clone())
        .ok_or_else(|| anyhow!("chat was not available after creation"))?;

    let nonce = format!(
        "{}-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
        std::process::id()
    );
    let ping = format!("ping:{nonce}");
    let pong = format!("pong:{nonce}");

    app.dispatch(AppAction::SendMessage {
        chat_id: chat_id.clone(),
        content: ping,
        kind: None,
        reply_to_message_id: None,
    });
    wait_until_true("bot pong", Duration::from_secs(30), || {
        app.state()
            .current_chat
            .as_ref()
            .map(|chat| chat.messages.iter().any(|message| message.content == pong))
            .unwrap_or(false)
    })?;

    app.dispatch(AppAction::StartCall {
        chat_id: chat_id.clone(),
    });
    wait_until_true("call active", Duration::from_secs(60), || {
        app.state()
            .active_call
            .as_ref()
            .map(|call| matches!(call.status, CallStatus::Active))
            .unwrap_or(false)
    })?;

    wait_until_true("tx frames flowing", Duration::from_secs(10), || {
        app.state()
            .active_call
            .as_ref()
            .and_then(|call| call.debug.as_ref())
            .map(|debug| debug.tx_frames > 10)
            .unwrap_or(false)
    })?;

    let media_window = Duration::from_secs(20);
    let media_start = Instant::now();
    let mut max_rx: u64 = 0;
    while media_start.elapsed() < media_window {
        if let Some(debug) = app
            .state()
            .active_call
            .as_ref()
            .and_then(|call| call.debug.as_ref())
        {
            max_rx = max_rx.max(debug.rx_frames);
            if max_rx >= 10 {
                break;
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    if max_rx < 5 {
        bail!("expected at least 5 rx frames from deployed bot (got {max_rx})");
    }

    app.dispatch(AppAction::EndCall);
    wait_until_true("call ended", Duration::from_secs(15), || {
        app.state()
            .active_call
            .as_ref()
            .map(|call| matches!(call.status, CallStatus::Ended { .. }))
            .unwrap_or(false)
    })?;

    let (tx_frames, rx_frames, rx_dropped, jitter_ms) = app
        .state()
        .active_call
        .as_ref()
        .and_then(|call| call.debug.as_ref())
        .map(|debug| {
            (
                debug.tx_frames.to_string(),
                debug.rx_frames.to_string(),
                debug.rx_dropped.to_string(),
                debug.jitter_buffer_ms.to_string(),
            )
        })
        .unwrap_or_else(|| {
            (
                "unknown".to_string(),
                "unknown".to_string(),
                "unknown".to_string(),
                "unknown".to_string(),
            )
        });

    let config_snapshot = fs::read_to_string(&config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    let config_artifact =
        context.write_artifact("public/deployed-bot-config.json", config_snapshot)?;
    let diag_artifact = context.write_artifact(
        "public/deployed-bot-diagnostics.txt",
        format!(
            "bot_npub={bot_npub}\nrelays={}\nkey_package_relays={}\nmoq_url={moq_url}\nchat_id={chat_id}\nmax_rx_frames={max_rx}\nfinal_tx_frames={tx_frames}\nfinal_rx_frames={rx_frames}\nfinal_rx_dropped={rx_dropped}\nfinal_jitter_buffer_ms={jitter_ms}\n",
            relays.join(","),
            kp_relays.join(","),
        ),
    )?;

    let mut result = ScenarioRunOutput::completed(context.state_dir().to_path_buf())
        .with_artifact(config_artifact)
        .with_artifact(diag_artifact)
        .with_metadata("bot_npub", bot_npub)
        .with_metadata("relay_count", relays.len().to_string())
        .with_metadata("key_package_relay_count", kp_relays.len().to_string())
        .with_metadata("moq_url", moq_url)
        .with_metadata("chat_id", chat_id)
        .with_metadata("max_rx_frames", max_rx.to_string());
    let summary = artifacts::write_standard_summary(
        &context,
        "public::deployed_bot_call",
        &result,
        Vec::new(),
        BTreeMap::new(),
    )?;
    result = result.with_summary(summary);

    context.mark_success();
    Ok(result)
}

fn run_public_android(
    runner: &CommandRunner<'_>,
    root: &Path,
) -> Result<Vec<CommandOutcomeRecord>> {
    let peer = required_env(ENV_PIKA_UI_E2E_BOT_NPUB)?;
    let relays = required_env(ENV_PIKA_UI_E2E_RELAYS)?;
    let kp_relays = required_env(ENV_PIKA_UI_E2E_KP_RELAYS)?;
    let nsec = optional_env(ENV_PIKA_UI_E2E_NSEC)
        .or_else(|| optional_env(ENV_PIKA_TEST_NSEC))
        .ok_or_else(|| anyhow!("missing {ENV_PIKA_UI_E2E_NSEC} and {ENV_PIKA_TEST_NSEC}"))?;

    let test_suffix = optional_env("PIKA_ANDROID_TEST_APPLICATION_ID_SUFFIX")
        .unwrap_or_else(|| ".test".to_string());
    let test_app_id = format!("org.pikachat.pika{test_suffix}");

    let mut outcomes = Vec::new();

    if optional_env("PIKA_ANDROID_SERIAL").is_none() {
        let emulator = runner.run(
            &CommandSpec::new("./tools/android-emulator-ensure")
                .cwd(root)
                .capture_name("android-emulator-ensure"),
        )?;
        outcomes.push(CommandOutcomeRecord::from_output(
            "android-emulator-ensure",
            &emulator,
        ));
    }

    let prepare = runner.run(
        &CommandSpec::new("just")
            .cwd(root)
            .args(["gen-kotlin", "android-rust", "android-local-properties"])
            .capture_name("android-prepare-build"),
    )?;
    outcomes.push(CommandOutcomeRecord::from_output(
        "android-prepare-build",
        &prepare,
    ));

    let installable = runner.run(
        &CommandSpec::new("./tools/android-ensure-debug-installable")
            .cwd(root)
            .env("PIKA_ANDROID_APP_ID", &test_app_id)
            .capture_name("android-ensure-installable"),
    )?;
    outcomes.push(CommandOutcomeRecord::from_output(
        "android-ensure-installable",
        &installable,
    ));

    let serial_output = runner.run(
        &CommandSpec::new("./tools/android-pick-serial")
            .cwd(root)
            .capture_name("android-pick-serial"),
    )?;
    outcomes.push(CommandOutcomeRecord::from_output(
        "android-pick-serial",
        &serial_output,
    ));

    let serial = String::from_utf8_lossy(&serial_output.stdout)
        .trim()
        .to_string();
    if serial.is_empty() {
        bail!("android serial output was empty");
    }

    if !serial.starts_with("emulator-") {
        let unlock = runner.run(
            &CommandSpec::new("./tools/android-ensure-unlocked")
                .cwd(root)
                .arg(serial.clone())
                .capture_name("android-ensure-unlocked"),
        )?;
        outcomes.push(CommandOutcomeRecord::from_output(
            "android-ensure-unlocked",
            &unlock,
        ));
    }

    let ui = runner.run(
        &CommandSpec::gradlew()
            .cwd(root.join("android"))
            .env("ANDROID_SERIAL", serial)
            .arg(":app:connectedDebugAndroidTest")
            .arg(format!("-PPIKA_ANDROID_APPLICATION_ID_SUFFIX={test_suffix}"))
            .arg("-Pandroid.testInstrumentationRunnerArguments.class=com.pika.app.PikaE2eUiTest")
            .arg("-Pandroid.testInstrumentationRunnerArguments.pika_e2e=1")
            .arg("-Pandroid.testInstrumentationRunnerArguments.pika_disable_network=false")
            .arg("-Pandroid.testInstrumentationRunnerArguments.pika_reset=1")
            .arg(format!("-Pandroid.testInstrumentationRunnerArguments.pika_peer_npub={peer}"))
            .arg(format!("-Pandroid.testInstrumentationRunnerArguments.pika_relay_urls={relays}"))
            .arg(format!("-Pandroid.testInstrumentationRunnerArguments.pika_key_package_relay_urls={kp_relays}"))
            .arg(format!("-Pandroid.testInstrumentationRunnerArguments.pika_nsec={nsec}"))
            .capture_name("android-ui-e2e-public"),
    )?;
    outcomes.push(CommandOutcomeRecord::from_output(
        "android-ui-e2e-public",
        &ui,
    ));

    Ok(outcomes)
}

fn run_public_ios(runner: &CommandRunner<'_>, root: &Path) -> Result<Vec<CommandOutcomeRecord>> {
    let peer = required_env(ENV_PIKA_UI_E2E_BOT_NPUB)?;
    let relays = required_env(ENV_PIKA_UI_E2E_RELAYS)?;
    let kp_relays = required_env(ENV_PIKA_UI_E2E_KP_RELAYS)?;
    let nsec = optional_env(ENV_PIKA_UI_E2E_NSEC)
        .or_else(|| optional_env(ENV_PIKA_TEST_NSEC))
        .ok_or_else(|| anyhow!("missing {ENV_PIKA_UI_E2E_NSEC} and {ENV_PIKA_TEST_NSEC}"))?;

    let mut outcomes = Vec::new();

    let prepare = runner.run(
        &CommandSpec::new("just")
            .cwd(root)
            .args(["ios-xcframework", "ios-xcodeproj"])
            .capture_name("ios-prepare-build"),
    )?;
    outcomes.push(CommandOutcomeRecord::from_output(
        "ios-prepare-build",
        &prepare,
    ));

    let sim_output = runner.run(
        &CommandSpec::new("./tools/ios-sim-ensure")
            .cwd(root)
            .env(ENV_PIKA_UI_E2E_NSEC, &nsec)
            .capture_name("ios-sim-ensure-public"),
    )?;
    outcomes.push(CommandOutcomeRecord::from_output(
        "ios-sim-ensure-public",
        &sim_output,
    ));
    let sim_stdout = String::from_utf8_lossy(&sim_output.stdout);
    let udid = super::common::extract_udid(&sim_stdout)
        .ok_or_else(|| anyhow!("could not determine simulator udid from ios-sim-ensure"))?;

    let ios_ui = runner.run(
        &CommandSpec::new("./tools/xcode-run")
            .cwd(root)
            .env("PIKA_UI_E2E", "1")
            .arg("xcodebuild")
            .args(["-project", "ios/Pika.xcodeproj", "-scheme", "Pika"])
            .arg("-destination")
            .arg(format!("id={udid}"))
            .arg("test")
            .arg("CODE_SIGNING_ALLOWED=NO")
            .arg(format!("PIKA_UI_E2E_BOT_NPUB={peer}"))
            .arg(format!("PIKA_UI_E2E_RELAYS={relays}"))
            .arg(format!("PIKA_UI_E2E_KP_RELAYS={kp_relays}"))
            .arg(format!("PIKA_UI_E2E_NSEC={nsec}"))
            .arg(format!(
                "PIKA_APP_BUNDLE_ID={}",
                optional_env("PIKA_IOS_BUNDLE_ID")
                    .unwrap_or_else(|| "org.pikachat.pika.dev".to_string())
            ))
            .arg("-only-testing:PikaUITests/PikaUITests/testE2E_deployedRustBot_pingPong")
            .capture_name("ios-ui-e2e-public"),
    )?;
    outcomes.push(CommandOutcomeRecord::from_output(
        "ios-ui-e2e-public",
        &ios_ui,
    ));

    Ok(outcomes)
}

fn env_or_default(key: &str, default: &str) -> String {
    optional_env(key).unwrap_or_else(|| default.to_string())
}

fn env_csv_or_default_fn(key: &str, defaults: impl FnOnce() -> Vec<String>) -> Vec<String> {
    if let Some(raw) = optional_env(key) {
        let parsed: Vec<String> = raw
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_string)
            .collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }
    defaults()
}

fn write_config_multi(
    data_dir: &Path,
    relays: &[String],
    kp_relays: &[String],
    moq_url: &str,
) -> Result<PathBuf> {
    let config_path = data_dir.join("pika_config.json");
    let config = serde_json::json!({
        "disable_network": false,
        "relay_urls": relays,
        "key_package_relay_urls": kp_relays,
        "call_moq_url": moq_url,
        "call_broadcast_prefix": "pika/calls",
        "call_audio_backend": "synthetic",
    });
    fs::write(&config_path, serde_json::to_vec(&config)?)
        .with_context(|| format!("write {}", config_path.display()))?;
    Ok(config_path)
}

fn wait_until_true(
    what: &str,
    timeout: Duration,
    mut condition: impl FnMut() -> bool,
) -> Result<()> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if condition() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    bail!("{what}: condition not met within {timeout:?}")
}

fn required_env(name: &str) -> Result<String> {
    if let Some(value) = optional_env(name) {
        return Ok(value);
    }
    bail!("missing required env: {name}");
}

fn dotenv_defaults() -> &'static HashMap<String, String> {
    DOTENV_DEFAULTS.get_or_init(|| {
        load_dotenv_defaults(&config::find_workspace_root().unwrap_or_else(|_| PathBuf::from(".")))
            .unwrap_or_default()
    })
}

fn load_dotenv_defaults(root: &Path) -> Result<HashMap<String, String>> {
    let mut defaults = HashMap::new();

    for file_name in [".env", ".env.local"] {
        let path = root.join(file_name);
        if !path.is_file() {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some((raw_key, raw_value)) = trimmed.split_once('=') else {
                continue;
            };

            let raw_key = raw_key.trim();
            let key = raw_key
                .strip_prefix("export")
                .map(str::trim_start)
                .unwrap_or(raw_key)
                .trim();
            if key.is_empty() || std::env::var_os(key).is_some() {
                continue;
            }

            let value = parse_dotenv_value(raw_value.trim());
            if value.is_empty() {
                continue;
            }

            defaults.insert(key.to_string(), value);
        }
    }

    Ok(defaults)
}

fn parse_dotenv_value(raw: &str) -> String {
    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        return raw[1..raw.len() - 1].to_string();
    }
    if raw.len() >= 2 && raw.starts_with('\'') && raw.ends_with('\'') {
        return raw[1..raw.len() - 1].to_string();
    }
    raw.to_string()
}
