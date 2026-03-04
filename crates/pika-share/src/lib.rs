use base64::engine::general_purpose::{
    STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD as BASE64_URL_SAFE,
};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

uniffi::setup_scaffolding!();

const QUEUE_DIR_NAME: &str = "share_queue";
const PENDING_DIR_NAME: &str = "pending";
const INFLIGHT_DIR_NAME: &str = "inflight";
const RESULTS_DIR_NAME: &str = "results";
const INDEXES_DIR_NAME: &str = "indexes";
const MEDIA_DIR_NAME: &str = "media";

const DEFAULT_LEASE_MS: u64 = 60_000;
const DEFAULT_RETRY_BACKOFF_MS: u64 = 2_000;
const DEFAULT_QUEUE_TTL_MS: u64 = 7 * 24 * 60 * 60 * 1000;
const DEFAULT_RESULT_TTL_MS: u64 = 14 * 24 * 60 * 60 * 1000;

#[derive(uniffi::Enum, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharePayloadKind {
    Text,
    Url,
    Image,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ShareEnqueueRequest {
    pub chat_id: String,
    pub compose_text: String,
    pub payload_kind: SharePayloadKind,
    pub payload_text: Option<String>,
    pub media_relative_path: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_filename: Option<String>,
    pub client_request_id: String,
    pub created_at_ms: u64,
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShareQueueStatus {
    Queued,
    Duplicate,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ShareQueueReceipt {
    pub item_id: String,
    pub queued_at_ms: u64,
    pub status: ShareQueueStatus,
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShareDispatchKind {
    Message {
        content: String,
    },
    Media {
        caption: String,
        mime_type: String,
        filename: String,
        data_base64: String,
    },
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ShareDispatchJob {
    pub item_id: String,
    pub chat_id: String,
    pub kind: ShareDispatchKind,
    pub attempt_count: u32,
}

#[derive(uniffi::Enum, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShareAckStatus {
    AcceptedByCore,
    RetryableFailure,
    PermanentFailure,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ShareDispatchAck {
    pub item_id: String,
    pub status: ShareAckStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug, Serialize, Deserialize)]
pub struct ShareResult {
    pub item_id: String,
    pub client_request_id: String,
    pub chat_id: String,
    pub status: ShareAckStatus,
    pub updated_at_ms: u64,
    pub is_terminal: bool,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug, Default, Serialize, Deserialize)]
pub struct ShareGcStats {
    pub requeued_inflight: u32,
    pub removed_pending: u32,
    pub removed_inflight: u32,
    pub removed_results: u32,
    pub removed_indexes: u32,
    pub removed_orphan_media: u32,
}

#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum ShareError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("not found: {0}")]
    NotFound(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredQueueItem {
    item_id: String,
    chat_id: String,
    payload_kind: SharePayloadKind,
    payload_text: Option<String>,
    compose_text: String,
    media_relative_path: Option<String>,
    media_mime_type: Option<String>,
    media_filename: Option<String>,
    client_request_id: String,
    created_at_ms: u64,
    queued_at_ms: u64,
    updated_at_ms: u64,
    next_attempt_at_ms: u64,
    lease_expires_at_ms: Option<u64>,
    attempt_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredRequestIndex {
    client_request_id: String,
    item_id: String,
    first_queued_at_ms: u64,
    last_status: Option<ShareAckStatus>,
    updated_at_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredResult {
    item_id: String,
    client_request_id: String,
    chat_id: String,
    status: ShareAckStatus,
    updated_at_ms: u64,
    is_terminal: bool,
    error_code: Option<String>,
    error_message: Option<String>,
}

#[derive(Clone, Debug)]
struct QueueLayout {
    root_dir: PathBuf,
    queue_dir: PathBuf,
    pending_dir: PathBuf,
    inflight_dir: PathBuf,
    results_dir: PathBuf,
    indexes_dir: PathBuf,
    media_dir: PathBuf,
}

#[uniffi::export]
pub fn share_enqueue(
    root_dir: String,
    request: ShareEnqueueRequest,
) -> Result<ShareQueueReceipt, ShareError> {
    validate_enqueue_request(&request)?;
    let layout = QueueLayout::new(&root_dir)?;
    ensure_layout(&layout)?;

    let now = now_ms();
    let normalized = normalize_request(request, now);

    if let Some(media_relative_path) = &normalized.media_relative_path {
        let absolute_media_path = resolve_relative_path(&layout.root_dir, media_relative_path)?;
        if !absolute_media_path.is_file() {
            return Err(ShareError::InvalidRequest(format!(
                "media file does not exist: {media_relative_path}"
            )));
        }
    }

    let index_path = layout.request_index_path(&normalized.client_request_id);
    if index_path.exists() {
        let existing: StoredRequestIndex = read_json(&index_path)?;
        return Ok(ShareQueueReceipt {
            item_id: existing.item_id,
            queued_at_ms: existing.first_queued_at_ms,
            status: ShareQueueStatus::Duplicate,
        });
    }

    let item_id = Uuid::new_v4().to_string();
    let item = StoredQueueItem {
        item_id: item_id.clone(),
        chat_id: normalized.chat_id,
        payload_kind: normalized.payload_kind,
        payload_text: normalized.payload_text,
        compose_text: normalized.compose_text,
        media_relative_path: normalized.media_relative_path,
        media_mime_type: normalized.media_mime_type,
        media_filename: normalized.media_filename,
        client_request_id: normalized.client_request_id.clone(),
        created_at_ms: normalized.created_at_ms,
        queued_at_ms: normalized.created_at_ms,
        updated_at_ms: now,
        next_attempt_at_ms: normalized.created_at_ms,
        lease_expires_at_ms: None,
        attempt_count: 0,
    };

    let pending_path = layout.pending_item_path(&item_id);
    let index = StoredRequestIndex {
        client_request_id: normalized.client_request_id,
        item_id: item_id.clone(),
        first_queued_at_ms: item.created_at_ms,
        last_status: None,
        updated_at_ms: now,
    };

    write_json_atomic(&pending_path, &item)?;
    if let Err(err) = write_json_atomic(&index_path, &index) {
        let _ = fs::remove_file(&pending_path);
        return Err(err);
    }

    Ok(ShareQueueReceipt {
        item_id,
        queued_at_ms: item.created_at_ms,
        status: ShareQueueStatus::Queued,
    })
}

#[uniffi::export]
pub fn share_dequeue_batch(
    root_dir: String,
    now_ms_override: u64,
    limit: u32,
) -> Result<Vec<ShareDispatchJob>, ShareError> {
    if limit == 0 {
        return Ok(vec![]);
    }

    let now = resolve_now(now_ms_override);
    let layout = QueueLayout::new(&root_dir)?;
    ensure_layout(&layout)?;
    reclaim_expired_inflight(&layout, now)?;

    let mut pending = load_pending_items(&layout)?;
    pending.sort_by(|left, right| {
        left.item
            .created_at_ms
            .cmp(&right.item.created_at_ms)
            .then_with(|| left.item.item_id.cmp(&right.item.item_id))
    });

    let mut jobs = Vec::new();
    for pending_entry in pending {
        if jobs.len() >= limit as usize {
            break;
        }
        if pending_entry.item.next_attempt_at_ms > now {
            continue;
        }

        let inflight_path = layout.inflight_item_path(&pending_entry.item.item_id);
        match fs::rename(&pending_entry.path, &inflight_path) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(ShareError::Io(err.to_string())),
        }

        let mut claimed: StoredQueueItem = read_json(&inflight_path)?;
        claimed.updated_at_ms = now;
        claimed.lease_expires_at_ms = Some(now.saturating_add(DEFAULT_LEASE_MS));
        claimed.attempt_count = claimed.attempt_count.saturating_add(1);
        write_json_atomic(&inflight_path, &claimed)?;

        match build_dispatch_job(&layout, &claimed) {
            Ok(job) => jobs.push(job),
            Err(err) => {
                finalize_item(
                    &layout,
                    &claimed,
                    ShareAckStatus::PermanentFailure,
                    Some("dequeue_build_failed".to_string()),
                    Some(err.to_string()),
                    now,
                )?;
            }
        }
    }

    Ok(jobs)
}

#[uniffi::export]
pub fn share_ack(root_dir: String, ack: ShareDispatchAck) -> Result<(), ShareError> {
    let item_id = ack.item_id.trim().to_string();
    if item_id.is_empty() {
        return Err(ShareError::InvalidRequest("missing item_id".to_string()));
    }

    let now = now_ms();
    let layout = QueueLayout::new(&root_dir)?;
    ensure_layout(&layout)?;

    let inflight_path = layout.inflight_item_path(&item_id);
    if !inflight_path.exists() {
        return Err(ShareError::NotFound(format!(
            "item not inflight: {item_id}"
        )));
    }
    let mut item: StoredQueueItem = read_json(&inflight_path)?;

    match ack.status {
        ShareAckStatus::RetryableFailure => {
            item.updated_at_ms = now;
            item.lease_expires_at_ms = None;
            item.next_attempt_at_ms = now.saturating_add(DEFAULT_RETRY_BACKOFF_MS);
            write_json_atomic(&inflight_path, &item)?;
            let pending_path = layout.pending_item_path(&item.item_id);
            fs::rename(&inflight_path, pending_path)
                .map_err(|err| ShareError::Io(err.to_string()))?;
            write_result(
                &layout,
                &item,
                ShareAckStatus::RetryableFailure,
                ack.error_code,
                ack.error_message,
                false,
                now,
            )?;
            update_index_status(
                &layout,
                &item.client_request_id,
                ShareAckStatus::RetryableFailure,
                now,
            )?;
            Ok(())
        }
        ShareAckStatus::AcceptedByCore | ShareAckStatus::PermanentFailure => finalize_item(
            &layout,
            &item,
            ack.status,
            ack.error_code,
            ack.error_message,
            now,
        ),
    }
}

#[uniffi::export]
pub fn share_list_recent_results(
    root_dir: String,
    limit: u32,
) -> Result<Vec<ShareResult>, ShareError> {
    let layout = QueueLayout::new(&root_dir)?;
    ensure_layout(&layout)?;

    let mut results = load_results(&layout)?;
    results.sort_by(|left, right| {
        right
            .updated_at_ms
            .cmp(&left.updated_at_ms)
            .then_with(|| right.item_id.cmp(&left.item_id))
    });

    if limit > 0 && results.len() > limit as usize {
        results.truncate(limit as usize);
    }

    Ok(results
        .into_iter()
        .map(|stored| ShareResult {
            item_id: stored.item_id,
            client_request_id: stored.client_request_id,
            chat_id: stored.chat_id,
            status: stored.status,
            updated_at_ms: stored.updated_at_ms,
            is_terminal: stored.is_terminal,
            error_code: stored.error_code,
            error_message: stored.error_message,
        })
        .collect())
}

#[uniffi::export]
pub fn share_gc(root_dir: String, now_ms_override: u64) -> Result<ShareGcStats, ShareError> {
    let now = resolve_now(now_ms_override);
    let layout = QueueLayout::new(&root_dir)?;
    ensure_layout(&layout)?;

    let mut stats = ShareGcStats {
        requeued_inflight: reclaim_expired_inflight(&layout, now)?,
        ..ShareGcStats::default()
    };

    for pending in load_pending_items(&layout)? {
        if now.saturating_sub(pending.item.created_at_ms) <= DEFAULT_QUEUE_TTL_MS {
            continue;
        }
        remove_pending_item(&layout, &pending.item)?;
        update_index_status(
            &layout,
            &pending.item.client_request_id,
            ShareAckStatus::PermanentFailure,
            now,
        )?;
        stats.removed_pending = stats.removed_pending.saturating_add(1);
    }

    for inflight in load_inflight_items(&layout)? {
        if now.saturating_sub(inflight.item.created_at_ms) <= DEFAULT_QUEUE_TTL_MS {
            continue;
        }
        let _ = fs::remove_file(&inflight.path);
        remove_media_for_item(&layout, &inflight.item)?;
        update_index_status(
            &layout,
            &inflight.item.client_request_id,
            ShareAckStatus::PermanentFailure,
            now,
        )?;
        stats.removed_inflight = stats.removed_inflight.saturating_add(1);
    }

    for entry in read_json_files(&layout.results_dir)? {
        let stored: StoredResult = read_json(&entry)?;
        if now.saturating_sub(stored.updated_at_ms) > DEFAULT_RESULT_TTL_MS {
            let _ = fs::remove_file(&entry);
            stats.removed_results = stats.removed_results.saturating_add(1);
        }
    }

    for entry in read_json_files(&layout.indexes_dir)? {
        let index: StoredRequestIndex = read_json(&entry)?;
        let pending_exists = layout.pending_item_path(&index.item_id).exists();
        let inflight_exists = layout.inflight_item_path(&index.item_id).exists();
        let result_path = layout.result_item_path(&index.item_id);
        let result_exists = result_path.exists();
        let keep = pending_exists || inflight_exists || result_exists;
        if !keep {
            let _ = fs::remove_file(entry);
            stats.removed_indexes = stats.removed_indexes.saturating_add(1);
        }
    }

    stats.removed_orphan_media = remove_orphan_media(&layout)?;

    Ok(stats)
}

#[derive(Clone, Debug)]
struct PendingEntry {
    path: PathBuf,
    item: StoredQueueItem,
}

#[derive(Clone, Debug)]
struct NormalizedRequest {
    chat_id: String,
    compose_text: String,
    payload_kind: SharePayloadKind,
    payload_text: Option<String>,
    media_relative_path: Option<String>,
    media_mime_type: Option<String>,
    media_filename: Option<String>,
    client_request_id: String,
    created_at_ms: u64,
}

impl QueueLayout {
    fn new(root_dir: &str) -> Result<Self, ShareError> {
        let trimmed = root_dir.trim();
        if trimmed.is_empty() {
            return Err(ShareError::InvalidRequest("missing root_dir".to_string()));
        }

        let root_dir = PathBuf::from(trimmed);
        let queue_dir = root_dir.join(QUEUE_DIR_NAME);
        let pending_dir = queue_dir.join(PENDING_DIR_NAME);
        let inflight_dir = queue_dir.join(INFLIGHT_DIR_NAME);
        let results_dir = queue_dir.join(RESULTS_DIR_NAME);
        let indexes_dir = queue_dir.join(INDEXES_DIR_NAME);
        let media_dir = queue_dir.join(MEDIA_DIR_NAME);

        Ok(Self {
            root_dir,
            queue_dir,
            pending_dir,
            inflight_dir,
            results_dir,
            indexes_dir,
            media_dir,
        })
    }

    fn pending_item_path(&self, item_id: &str) -> PathBuf {
        self.pending_dir.join(format!("{item_id}.json"))
    }

    fn inflight_item_path(&self, item_id: &str) -> PathBuf {
        self.inflight_dir.join(format!("{item_id}.json"))
    }

    fn result_item_path(&self, item_id: &str) -> PathBuf {
        self.results_dir.join(format!("{item_id}.json"))
    }

    fn request_index_path(&self, client_request_id: &str) -> PathBuf {
        let key = BASE64_URL_SAFE.encode(client_request_id.as_bytes());
        self.indexes_dir.join(format!("{key}.json"))
    }
}

fn ensure_layout(layout: &QueueLayout) -> Result<(), ShareError> {
    fs::create_dir_all(&layout.queue_dir).map_err(|err| ShareError::Io(err.to_string()))?;
    fs::create_dir_all(&layout.pending_dir).map_err(|err| ShareError::Io(err.to_string()))?;
    fs::create_dir_all(&layout.inflight_dir).map_err(|err| ShareError::Io(err.to_string()))?;
    fs::create_dir_all(&layout.results_dir).map_err(|err| ShareError::Io(err.to_string()))?;
    fs::create_dir_all(&layout.indexes_dir).map_err(|err| ShareError::Io(err.to_string()))?;
    fs::create_dir_all(&layout.media_dir).map_err(|err| ShareError::Io(err.to_string()))?;
    Ok(())
}

fn validate_enqueue_request(request: &ShareEnqueueRequest) -> Result<(), ShareError> {
    if request.chat_id.trim().is_empty() {
        return Err(ShareError::InvalidRequest("missing chat_id".to_string()));
    }
    let request_id = request.client_request_id.trim();
    if request_id.is_empty() {
        return Err(ShareError::InvalidRequest(
            "missing client_request_id".to_string(),
        ));
    }
    if request_id.len() > 256 {
        return Err(ShareError::InvalidRequest(
            "client_request_id too long".to_string(),
        ));
    }

    match request.payload_kind {
        SharePayloadKind::Text | SharePayloadKind::Url => {
            let payload_text = request
                .payload_text
                .as_ref()
                .map(|value| value.trim())
                .unwrap_or_default();
            let compose_text = request.compose_text.trim();
            if payload_text.is_empty() && compose_text.is_empty() {
                return Err(ShareError::InvalidRequest(
                    "text/url share requires payload_text or compose_text".to_string(),
                ));
            }
        }
        SharePayloadKind::Image => {
            let media_relative_path = request
                .media_relative_path
                .as_ref()
                .map(|value| value.trim())
                .unwrap_or_default();
            if media_relative_path.is_empty() {
                return Err(ShareError::InvalidRequest(
                    "image share requires media_relative_path".to_string(),
                ));
            }
            if !is_safe_relative_path(media_relative_path) {
                return Err(ShareError::InvalidRequest(
                    "media_relative_path must be a safe relative path".to_string(),
                ));
            }
        }
    }

    Ok(())
}

fn normalize_request(request: ShareEnqueueRequest, now: u64) -> NormalizedRequest {
    let chat_id = request.chat_id.trim().to_string();
    let compose_text = request.compose_text.trim().to_string();
    let payload_text = request.payload_text.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    let media_relative_path = request.media_relative_path.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    let media_mime_type = request.media_mime_type.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });
    let media_filename = request.media_filename.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    NormalizedRequest {
        chat_id,
        compose_text,
        payload_kind: request.payload_kind,
        payload_text,
        media_relative_path,
        media_mime_type,
        media_filename,
        client_request_id: request.client_request_id.trim().to_string(),
        created_at_ms: if request.created_at_ms == 0 {
            now
        } else {
            request.created_at_ms
        },
    }
}

fn build_dispatch_job(
    layout: &QueueLayout,
    item: &StoredQueueItem,
) -> Result<ShareDispatchJob, ShareError> {
    let kind = match item.payload_kind {
        SharePayloadKind::Text | SharePayloadKind::Url => {
            let content = merge_message_content(&item.compose_text, item.payload_text.as_deref());
            if content.is_empty() {
                return Err(ShareError::InvalidRequest(
                    "empty content after merge".to_string(),
                ));
            }
            ShareDispatchKind::Message { content }
        }
        SharePayloadKind::Image => {
            let media_relative_path = item.media_relative_path.as_deref().ok_or_else(|| {
                ShareError::InvalidRequest("missing media_relative_path".to_string())
            })?;
            let media_absolute_path = resolve_relative_path(&layout.root_dir, media_relative_path)?;
            let data =
                fs::read(&media_absolute_path).map_err(|err| ShareError::Io(err.to_string()))?;
            if data.is_empty() {
                return Err(ShareError::InvalidRequest(
                    "media file is empty".to_string(),
                ));
            }

            let mime_type = item
                .media_mime_type
                .clone()
                .unwrap_or_else(|| "image/jpeg".to_string());
            let filename = item
                .media_filename
                .clone()
                .unwrap_or_else(|| "shared-image.jpg".to_string());
            let caption = item.compose_text.clone();
            ShareDispatchKind::Media {
                caption,
                mime_type,
                filename,
                data_base64: BASE64_STANDARD.encode(data),
            }
        }
    };

    Ok(ShareDispatchJob {
        item_id: item.item_id.clone(),
        chat_id: item.chat_id.clone(),
        kind,
        attempt_count: item.attempt_count,
    })
}

fn merge_message_content(compose_text: &str, payload_text: Option<&str>) -> String {
    let prefix = compose_text.trim();
    let payload = payload_text.unwrap_or_default().trim();
    if prefix.is_empty() {
        return payload.to_string();
    }
    if payload.is_empty() {
        return prefix.to_string();
    }
    format!("{prefix}\n{payload}")
}

fn finalize_item(
    layout: &QueueLayout,
    item: &StoredQueueItem,
    status: ShareAckStatus,
    error_code: Option<String>,
    error_message: Option<String>,
    now: u64,
) -> Result<(), ShareError> {
    let is_terminal = matches!(
        status,
        ShareAckStatus::AcceptedByCore | ShareAckStatus::PermanentFailure
    );

    write_result(
        layout,
        item,
        status.clone(),
        error_code,
        error_message,
        is_terminal,
        now,
    )?;
    update_index_status(layout, &item.client_request_id, status, now)?;
    let inflight_path = layout.inflight_item_path(&item.item_id);
    let _ = fs::remove_file(inflight_path);
    if is_terminal {
        remove_media_for_item(layout, item)?;
    }
    Ok(())
}

fn write_result(
    layout: &QueueLayout,
    item: &StoredQueueItem,
    status: ShareAckStatus,
    error_code: Option<String>,
    error_message: Option<String>,
    is_terminal: bool,
    now: u64,
) -> Result<(), ShareError> {
    let result = StoredResult {
        item_id: item.item_id.clone(),
        client_request_id: item.client_request_id.clone(),
        chat_id: item.chat_id.clone(),
        status,
        updated_at_ms: now,
        is_terminal,
        error_code,
        error_message,
    };
    let result_path = layout.result_item_path(&item.item_id);
    write_json_atomic(&result_path, &result)
}

fn update_index_status(
    layout: &QueueLayout,
    client_request_id: &str,
    status: ShareAckStatus,
    now: u64,
) -> Result<(), ShareError> {
    let index_path = layout.request_index_path(client_request_id);
    if !index_path.exists() {
        return Ok(());
    }
    let mut index: StoredRequestIndex = read_json(&index_path)?;
    index.last_status = Some(status);
    index.updated_at_ms = now;
    write_json_atomic(&index_path, &index)
}

fn reclaim_expired_inflight(layout: &QueueLayout, now: u64) -> Result<u32, ShareError> {
    let mut requeued = 0u32;
    for path in read_json_files(&layout.inflight_dir)? {
        let mut item: StoredQueueItem = read_json(&path)?;
        let Some(lease_expires_at_ms) = item.lease_expires_at_ms else {
            continue;
        };
        if lease_expires_at_ms > now {
            continue;
        }

        item.lease_expires_at_ms = None;
        item.updated_at_ms = now;
        write_json_atomic(&path, &item)?;

        let pending_path = layout.pending_item_path(&item.item_id);
        fs::rename(&path, &pending_path).map_err(|err| ShareError::Io(err.to_string()))?;
        requeued = requeued.saturating_add(1);
    }
    Ok(requeued)
}

fn remove_pending_item(layout: &QueueLayout, item: &StoredQueueItem) -> Result<(), ShareError> {
    let pending_path = layout.pending_item_path(&item.item_id);
    let _ = fs::remove_file(pending_path);
    remove_media_for_item(layout, item)?;
    Ok(())
}

fn remove_media_for_item(layout: &QueueLayout, item: &StoredQueueItem) -> Result<(), ShareError> {
    let Some(media_relative_path) = &item.media_relative_path else {
        return Ok(());
    };
    let media_absolute_path = resolve_relative_path(&layout.root_dir, media_relative_path)?;
    let _ = fs::remove_file(media_absolute_path);
    Ok(())
}

fn remove_orphan_media(layout: &QueueLayout) -> Result<u32, ShareError> {
    let mut referenced = std::collections::HashSet::new();
    for item in load_pending_items(layout)?
        .into_iter()
        .chain(load_inflight_items(layout)?.into_iter())
        .map(|entry| entry.item)
    {
        if let Some(relative_path) = item.media_relative_path {
            let abs = resolve_relative_path(&layout.root_dir, &relative_path)?;
            referenced.insert(abs);
        }
    }

    let mut removed = 0u32;
    for entry in fs::read_dir(&layout.media_dir).map_err(|err| ShareError::Io(err.to_string()))? {
        let path = entry.map_err(|err| ShareError::Io(err.to_string()))?.path();
        if !path.is_file() {
            continue;
        }
        if referenced.contains(&path) {
            continue;
        }
        let _ = fs::remove_file(path);
        removed = removed.saturating_add(1);
    }

    Ok(removed)
}

fn load_pending_items(layout: &QueueLayout) -> Result<Vec<PendingEntry>, ShareError> {
    let mut entries = Vec::new();
    for path in read_json_files(&layout.pending_dir)? {
        let item: StoredQueueItem = read_json(&path)?;
        entries.push(PendingEntry { path, item });
    }
    Ok(entries)
}

fn load_inflight_items(layout: &QueueLayout) -> Result<Vec<PendingEntry>, ShareError> {
    let mut entries = Vec::new();
    for path in read_json_files(&layout.inflight_dir)? {
        let item: StoredQueueItem = read_json(&path)?;
        entries.push(PendingEntry { path, item });
    }
    Ok(entries)
}

fn load_results(layout: &QueueLayout) -> Result<Vec<StoredResult>, ShareError> {
    let mut results = Vec::new();
    for path in read_json_files(&layout.results_dir)? {
        let result: StoredResult = read_json(&path)?;
        results.push(result);
    }
    Ok(results)
}

fn resolve_relative_path(root_dir: &Path, relative: &str) -> Result<PathBuf, ShareError> {
    if !is_safe_relative_path(relative) {
        return Err(ShareError::InvalidRequest(
            "expected safe relative path".to_string(),
        ));
    }
    Ok(root_dir.join(relative))
}

fn is_safe_relative_path(path: &str) -> bool {
    let candidate = Path::new(path);
    if candidate.as_os_str().is_empty() || candidate.is_absolute() {
        return false;
    }
    for component in candidate.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return false;
        }
    }
    true
}

fn read_json_files(dir: &Path) -> Result<Vec<PathBuf>, ShareError> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).map_err(|err| ShareError::Io(err.to_string()))? {
        let path = entry.map_err(|err| ShareError::Io(err.to_string()))?.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), ShareError> {
    let parent = path.parent().ok_or_else(|| {
        ShareError::Io(format!("missing parent directory for {}", path.display()))
    })?;
    fs::create_dir_all(parent).map_err(|err| ShareError::Io(err.to_string()))?;
    let tmp_name = format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("tmp"),
        Uuid::new_v4()
    );
    let tmp_path = parent.join(tmp_name);
    let bytes =
        serde_json::to_vec(value).map_err(|err| ShareError::Serialization(err.to_string()))?;
    fs::write(&tmp_path, bytes).map_err(|err| ShareError::Io(err.to_string()))?;
    fs::rename(&tmp_path, path).map_err(|err| ShareError::Io(err.to_string()))?;
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ShareError> {
    let bytes = fs::read(path).map_err(|err| ShareError::Io(err.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|err| ShareError::Serialization(err.to_string()))
}

fn now_ms() -> u64 {
    resolve_now(0)
}

fn resolve_now(now_ms_override: u64) -> u64 {
    if now_ms_override > 0 {
        return now_ms_override;
    }
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn enqueue_then_dequeue_then_ack_text() {
        let harness = Harness::new();
        let request = harness.text_request("req-text-1", "chat-a", "hello");

        let receipt = share_enqueue(harness.root_dir(), request).expect("enqueue should work");
        assert_eq!(receipt.status, ShareQueueStatus::Queued);

        let jobs = share_dequeue_batch(harness.root_dir(), 1_000, 10).expect("dequeue should work");
        assert_eq!(jobs.len(), 1);
        let job = &jobs[0];
        assert_eq!(job.item_id, receipt.item_id);
        assert_eq!(job.chat_id, "chat-a");
        match &job.kind {
            ShareDispatchKind::Message { content } => assert_eq!(content, "hello"),
            _ => panic!("expected message dispatch kind"),
        }
        assert_eq!(job.attempt_count, 1);

        share_ack(
            harness.root_dir(),
            ShareDispatchAck {
                item_id: receipt.item_id.clone(),
                status: ShareAckStatus::AcceptedByCore,
                error_code: None,
                error_message: None,
            },
        )
        .expect("ack should succeed");

        let results = share_list_recent_results(harness.root_dir(), 10).expect("results");
        assert_eq!(results.len(), 1);
        let result = &results[0];
        assert_eq!(result.item_id, receipt.item_id);
        assert_eq!(result.status, ShareAckStatus::AcceptedByCore);
        assert!(result.is_terminal);
    }

    #[test]
    fn duplicate_client_request_id_returns_duplicate_receipt() {
        let harness = Harness::new();
        let request = harness.text_request("req-dupe-1", "chat-a", "hello");
        let first = share_enqueue(harness.root_dir(), request.clone()).expect("first enqueue");
        let second = share_enqueue(harness.root_dir(), request).expect("second enqueue");
        assert_eq!(second.status, ShareQueueStatus::Duplicate);
        assert_eq!(second.item_id, first.item_id);
    }

    #[test]
    fn retryable_ack_requeues_item_after_backoff() {
        let harness = Harness::new();
        let request = harness.text_request("req-retry-1", "chat-a", "hello");
        let receipt = share_enqueue(harness.root_dir(), request).expect("enqueue");

        let jobs = share_dequeue_batch(harness.root_dir(), 10_000, 10).expect("first dequeue");
        assert_eq!(jobs.len(), 1);

        share_ack(
            harness.root_dir(),
            ShareDispatchAck {
                item_id: receipt.item_id.clone(),
                status: ShareAckStatus::RetryableFailure,
                error_code: Some("network_down".to_string()),
                error_message: Some("temporary failure".to_string()),
            },
        )
        .expect("retry ack");

        let too_early =
            share_dequeue_batch(harness.root_dir(), 10_001, 10).expect("too early dequeue");
        assert!(too_early.is_empty());

        let retried =
            share_dequeue_batch(harness.root_dir(), u64::MAX - 1, 10).expect("retry dequeue");
        assert_eq!(retried.len(), 1);
        assert_eq!(retried[0].item_id, receipt.item_id);
        assert_eq!(retried[0].attempt_count, 2);
    }

    #[test]
    fn expired_inflight_item_is_reclaimed_on_next_dequeue() {
        let harness = Harness::new();
        let request = harness.text_request("req-lease-1", "chat-a", "hello");
        let receipt = share_enqueue(harness.root_dir(), request).expect("enqueue");

        let first = share_dequeue_batch(harness.root_dir(), 5_000, 10).expect("first dequeue");
        assert_eq!(first.len(), 1);

        let reclaimed = share_dequeue_batch(harness.root_dir(), 5_000 + DEFAULT_LEASE_MS + 1, 10)
            .expect("second dequeue");
        assert_eq!(reclaimed.len(), 1);
        assert_eq!(reclaimed[0].item_id, receipt.item_id);
        assert_eq!(reclaimed[0].attempt_count, 2);
    }

    #[test]
    fn image_dispatch_reads_media_bytes() {
        let harness = Harness::new();
        let relative_media = "share_queue/media/payload.jpg";
        harness.write_file(relative_media, &[1, 2, 3, 4]);

        let request = ShareEnqueueRequest {
            chat_id: "chat-image".to_string(),
            compose_text: "caption".to_string(),
            payload_kind: SharePayloadKind::Image,
            payload_text: None,
            media_relative_path: Some(relative_media.to_string()),
            media_mime_type: Some("image/jpeg".to_string()),
            media_filename: Some("payload.jpg".to_string()),
            client_request_id: "req-image-1".to_string(),
            created_at_ms: 1_000,
        };

        share_enqueue(harness.root_dir(), request).expect("enqueue image");
        let jobs = share_dequeue_batch(harness.root_dir(), 1_001, 10).expect("dequeue image");
        assert_eq!(jobs.len(), 1);
        match &jobs[0].kind {
            ShareDispatchKind::Media {
                caption,
                mime_type,
                filename,
                data_base64,
            } => {
                assert_eq!(caption, "caption");
                assert_eq!(mime_type, "image/jpeg");
                assert_eq!(filename, "payload.jpg");
                assert_eq!(data_base64, &BASE64_STANDARD.encode([1, 2, 3, 4]));
            }
            _ => panic!("expected media dispatch"),
        }
    }

    #[test]
    fn gc_removes_stale_pending_and_results() {
        let harness = Harness::new();
        let stale_created = 1_000;
        let gc_now = stale_created + DEFAULT_QUEUE_TTL_MS + 1;

        let request = ShareEnqueueRequest {
            chat_id: "chat-stale".to_string(),
            compose_text: "".to_string(),
            payload_kind: SharePayloadKind::Text,
            payload_text: Some("stale".to_string()),
            media_relative_path: None,
            media_mime_type: None,
            media_filename: None,
            client_request_id: "req-stale-1".to_string(),
            created_at_ms: stale_created,
        };

        let receipt = share_enqueue(harness.root_dir(), request).expect("enqueue stale");
        let stats = share_gc(harness.root_dir(), gc_now).expect("gc");
        assert_eq!(stats.removed_pending, 1);

        let jobs = share_dequeue_batch(harness.root_dir(), gc_now + 1, 10).expect("dequeue");
        assert!(jobs.is_empty());

        let list = share_list_recent_results(harness.root_dir(), 10).expect("results");
        assert!(list.is_empty());

        // Write a result then verify result GC purges it.
        let request2 = harness.text_request("req-result-gc-1", "chat-a", "hello");
        let receipt2 = share_enqueue(harness.root_dir(), request2).expect("enqueue 2");
        let jobs2 = share_dequeue_batch(harness.root_dir(), gc_now + 10, 10).expect("dequeue 2");
        assert_eq!(jobs2.len(), 1);
        share_ack(
            harness.root_dir(),
            ShareDispatchAck {
                item_id: receipt2.item_id.clone(),
                status: ShareAckStatus::AcceptedByCore,
                error_code: None,
                error_message: None,
            },
        )
        .expect("ack 2");

        let stats2 = share_gc(harness.root_dir(), u64::MAX - 1).expect("gc 2");
        assert!(stats2.removed_results >= 1);
        assert!(stats2.removed_indexes >= 1);

        let _ = receipt;
    }

    #[derive(Debug)]
    struct Harness {
        temp_dir: TempDir,
    }

    impl Harness {
        fn new() -> Self {
            Self {
                temp_dir: tempfile::tempdir().expect("tempdir"),
            }
        }

        fn root_dir(&self) -> String {
            self.temp_dir.path().to_string_lossy().to_string()
        }

        fn text_request(&self, request_id: &str, chat_id: &str, text: &str) -> ShareEnqueueRequest {
            ShareEnqueueRequest {
                chat_id: chat_id.to_string(),
                compose_text: "".to_string(),
                payload_kind: SharePayloadKind::Text,
                payload_text: Some(text.to_string()),
                media_relative_path: None,
                media_mime_type: None,
                media_filename: None,
                client_request_id: request_id.to_string(),
                created_at_ms: 1_000,
            }
        }

        fn write_file(&self, relative_path: &str, bytes: &[u8]) {
            let absolute = self.temp_dir.path().join(relative_path);
            if let Some(parent) = absolute.parent() {
                fs::create_dir_all(parent).expect("create parent dirs");
            }
            fs::write(absolute, bytes).expect("write file");
        }
    }
}
