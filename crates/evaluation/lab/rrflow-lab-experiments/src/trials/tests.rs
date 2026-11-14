use super::metrics;

#[test]
fn unreported_provider_usage_and_tool_count_are_unknown() {
    assert_eq!(
        metrics("claude", r#"{"usage":{},"num_turns":4}"#),
        (None, None, None)
    );
    assert_eq!(metrics("codex", ""), (None, None, None));
    assert_eq!(
        metrics(
            "claude",
            r#"{"usage":{"input_tokens":10,"cache_read_input_tokens":7,"output_tokens":3}}"#
        ),
        (Some(10), Some(3), None)
    );
}
