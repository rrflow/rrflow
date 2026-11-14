//! Validation of DNS names, quantities, images and digests.

pub(super) fn validate_dns_name(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 63
        || !value.is_ascii()
        || value.starts_with('-')
        || value.ends_with('-')
        || value
            .bytes()
            .any(|byte| !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'))
    {
        return Err(format!("{field} must be one lowercase DNS label"));
    }
    Ok(())
}

pub(super) fn validate_quantity(value: &str) -> Result<(), String> {
    let number = ["Ki", "Mi", "Gi", "Ti"]
        .into_iter()
        .find_map(|suffix| value.strip_suffix(suffix));
    if number.is_none_or(|number| {
        number.is_empty()
            || number.starts_with('0')
            || !number.bytes().all(|byte| byte.is_ascii_digit())
            || number.parse::<u64>().ok().is_none_or(|value| value == 0)
    }) {
        return Err("storage size must be a positive binary Kubernetes quantity".into());
    }
    Ok(())
}

pub(super) fn validate_image(image: &str) -> Result<(), String> {
    let Some((repository, digest)) = image.rsplit_once("@sha256:") else {
        return Err("RRD image must be pinned by sha256 digest".into());
    };
    if repository.is_empty()
        || repository.len() > 512
        || digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("RRD image repository or sha256 digest is invalid".into());
    }
    Ok(())
}

pub(super) fn validate_sha256(value: &str, field: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(format!("{field} must be lowercase SHA-256 hex"));
    }
    Ok(())
}

pub(super) const fn default_true() -> bool {
    true
}
