// Minimum app version check against the notification server.

use std::time::Instant;

use super::AppCore;

/// Skip re-checking if the last check was less than this long ago.
const CHECK_COOLDOWN: std::time::Duration = std::time::Duration::from_secs(5 * 60);

/// Compare two semver-style version strings (e.g. "0.2.9" < "0.3.0").
/// Returns true if `current` is strictly less than `minimum`.
/// Non-numeric or missing components are treated as 0.
fn version_less_than(current: &str, minimum: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let cur = parse(current);
    let min = parse(minimum);
    let max_len = cur.len().max(min.len());
    for i in 0..max_len {
        let c = cur.get(i).copied().unwrap_or(0);
        let m = min.get(i).copied().unwrap_or(0);
        if c < m {
            return true;
        }
        if c > m {
            return false;
        }
    }
    false // equal
}

impl AppCore {
    pub(super) fn check_min_version(&mut self) {
        // Throttle: skip if we checked recently.
        if let Some(last) = self.last_min_version_check {
            if last.elapsed() < CHECK_COOLDOWN {
                return;
            }
        }
        self.last_min_version_check = Some(Instant::now());

        let url = format!("{}/min-version", self.notification_url());
        let client = self.http_client.clone();
        let app_version = self.app_version.clone();
        let tx = self.core_sender.clone();

        self.runtime.spawn(async move {
            let resp = match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                client.get(&url).send(),
            )
            .await
            {
                Ok(Ok(resp)) if resp.status().is_success() => resp,
                _ => return, // Server down or error — silently ignore
            };

            let body: serde_json::Value =
                match tokio::time::timeout(std::time::Duration::from_secs(5), resp.json()).await {
                    Ok(Ok(v)) => v,
                    _ => return,
                };

            let Some(min_version) = body.get("min_version").and_then(|v| v.as_str()) else {
                return;
            };

            let update_required = version_less_than(&app_version, min_version);
            let _ = tx.send(crate::updates::CoreMsg::Internal(Box::new(
                crate::updates::InternalEvent::MinVersionChecked { update_required },
            )));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::version_less_than;

    #[test]
    fn same_version_is_not_less() {
        assert!(!version_less_than("0.2.9", "0.2.9"));
    }

    #[test]
    fn older_patch() {
        assert!(version_less_than("0.2.8", "0.2.9"));
    }

    #[test]
    fn newer_patch() {
        assert!(!version_less_than("0.2.10", "0.2.9"));
    }

    #[test]
    fn older_minor() {
        assert!(version_less_than("0.1.9", "0.2.0"));
    }

    #[test]
    fn older_major() {
        assert!(version_less_than("0.9.9", "1.0.0"));
    }

    #[test]
    fn newer_major() {
        assert!(!version_less_than("1.0.0", "0.9.9"));
    }

    #[test]
    fn missing_components_treated_as_zero() {
        assert!(version_less_than("0.2", "0.2.1"));
        assert!(!version_less_than("0.2.1", "0.2"));
    }

    #[test]
    fn empty_version_is_less_than_any() {
        assert!(version_less_than("", "0.0.1"));
        assert!(!version_less_than("0.0.1", ""));
    }
}
