use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantNamespace {
    tenant_slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantNamespaceError {
    details: String,
}

impl TenantNamespaceError {
    fn new(details: impl Into<String>) -> Self {
        Self {
            details: details.into(),
        }
    }
}

impl fmt::Display for TenantNamespaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for TenantNamespaceError {}

impl TenantNamespace {
    pub fn new(tenant_id: impl AsRef<str>) -> Result<Self, TenantNamespaceError> {
        let slug = sanitize_segment(tenant_id.as_ref());
        if slug.is_empty() {
            return Err(TenantNamespaceError::new(
                "tenant identifier is empty after sanitization",
            ));
        }
        Ok(Self { tenant_slug: slug })
    }

    pub fn tenant_slug(&self) -> &str {
        &self.tenant_slug
    }

    pub fn relay_namespace(&self, channel: impl AsRef<str>) -> String {
        format!(
            "tenant/{}/relay/{}",
            self.tenant_slug,
            sanitize_or_default(channel.as_ref(), "default-channel")
        )
    }

    pub fn moq_namespace(&self, topic: impl AsRef<str>) -> String {
        format!(
            "tenant/{}/moq/{}",
            self.tenant_slug,
            sanitize_or_default(topic.as_ref(), "default-topic")
        )
    }
}

fn sanitize_or_default(input: &str, fallback: &str) -> String {
    let sanitized = sanitize_segment(input);
    if sanitized.is_empty() {
        fallback.to_string()
    } else {
        sanitized
    }
}

fn sanitize_segment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('-');
        }
    }

    out.trim_matches(['-', '_', '.']).to_string()
}

#[cfg(test)]
mod tests {
    use super::TenantNamespace;

    #[test]
    fn namespace_helpers_produce_canonical_paths() {
        let tenant = TenantNamespace::new("Team Alpha / 01").unwrap();
        assert_eq!(tenant.tenant_slug(), "team-alpha---01");
        assert_eq!(
            tenant.relay_namespace("inbox/events"),
            "tenant/team-alpha---01/relay/inbox-events"
        );
        assert_eq!(
            tenant.moq_namespace("rooms/main"),
            "tenant/team-alpha---01/moq/rooms-main"
        );
    }

    #[test]
    fn namespace_helpers_apply_safe_fallback_segments() {
        let tenant = TenantNamespace::new("tenant-a").unwrap();
        assert_eq!(
            tenant.relay_namespace("///"),
            "tenant/tenant-a/relay/default-channel"
        );
        assert_eq!(
            tenant.moq_namespace(""),
            "tenant/tenant-a/moq/default-topic"
        );
    }
}
