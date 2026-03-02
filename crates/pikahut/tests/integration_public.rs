use anyhow::Result;
use std::sync::{Mutex, OnceLock};

use pikahut::testing::scenarios::public::{self, PublicUiPlatform};
use pikahut::testing::{Requirement, emit_skip, skip_if_missing_env, skip_if_missing_requirements};

const ENV_PIKA_TEST_NSEC: &str = "PIKA_TEST_NSEC";
const ENV_PIKA_UI_E2E_BOT_NPUB: &str = "PIKA_UI_E2E_BOT_NPUB";
const ENV_PIKA_UI_E2E_RELAYS: &str = "PIKA_UI_E2E_RELAYS";
const ENV_PIKA_UI_E2E_KP_RELAYS: &str = "PIKA_UI_E2E_KP_RELAYS";
const ENV_PIKA_UI_E2E_NSEC: &str = "PIKA_UI_E2E_NSEC";

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn required_env(name: &'static str) -> bool {
    public::optional_env(name).is_some()
}

fn skip_if_missing(requirements: &[Requirement], env_vars: &[&'static str]) -> bool {
    if skip_if_missing_requirements(&workspace_root(), requirements) {
        return true;
    }

    if skip_if_missing_env(env_vars, public::optional_env) {
        return true;
    }

    if !(required_env(ENV_PIKA_UI_E2E_NSEC) || required_env(ENV_PIKA_TEST_NSEC)) {
        emit_skip(&format!(
            "required env missing: {ENV_PIKA_UI_E2E_NSEC} or {ENV_PIKA_TEST_NSEC}"
        ));
        return true;
    }

    false
}

fn public_lane_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
#[ignore = "nondeterministic public relay flow"]
fn ui_e2e_public_android() -> Result<()> {
    let _guard = public_lane_lock()
        .lock()
        .expect("public selector lock poisoned");

    if skip_if_missing(
        &[
            Requirement::PublicNetwork,
            Requirement::AndroidTools,
            Requirement::AndroidEmulatorAvd,
        ],
        &[
            ENV_PIKA_UI_E2E_BOT_NPUB,
            ENV_PIKA_UI_E2E_RELAYS,
            ENV_PIKA_UI_E2E_KP_RELAYS,
        ],
    ) {
        return Ok(());
    }

    public::run_public_ui_e2e(PublicUiPlatform::Android).map(|_| ())
}

#[test]
#[ignore = "nondeterministic public relay flow"]
fn ui_e2e_public_ios() -> Result<()> {
    let _guard = public_lane_lock()
        .lock()
        .expect("public selector lock poisoned");

    if skip_if_missing(
        &[
            Requirement::PublicNetwork,
            Requirement::HostMacOs,
            Requirement::Xcode,
        ],
        &[
            ENV_PIKA_UI_E2E_BOT_NPUB,
            ENV_PIKA_UI_E2E_RELAYS,
            ENV_PIKA_UI_E2E_KP_RELAYS,
        ],
    ) {
        return Ok(());
    }

    public::run_public_ui_e2e(PublicUiPlatform::Ios).map(|_| ())
}

#[test]
#[ignore = "nondeterministic public relay flow"]
fn ui_e2e_public_all() -> Result<()> {
    let _guard = public_lane_lock()
        .lock()
        .expect("public selector lock poisoned");

    if skip_if_missing(
        &[
            Requirement::PublicNetwork,
            Requirement::HostMacOs,
            Requirement::Xcode,
            Requirement::AndroidTools,
            Requirement::AndroidEmulatorAvd,
        ],
        &[
            ENV_PIKA_UI_E2E_BOT_NPUB,
            ENV_PIKA_UI_E2E_RELAYS,
            ENV_PIKA_UI_E2E_KP_RELAYS,
        ],
    ) {
        return Ok(());
    }

    public::run_public_ui_e2e(PublicUiPlatform::All).map(|_| ())
}

#[test]
#[ignore = "nondeterministic deployed bot flow"]
fn deployed_bot_call_flow() -> Result<()> {
    let _guard = public_lane_lock()
        .lock()
        .expect("public selector lock poisoned");

    if skip_if_missing(&[Requirement::PublicNetwork], &[]) {
        return Ok(());
    }

    public::run_deployed_bot_call_flow().map(|_| ())
}
