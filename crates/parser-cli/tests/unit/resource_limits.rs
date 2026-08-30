use super::*;

#[test]
fn pretty_json_transport_accepts_exact_bytes_and_stops_at_one_over_including_newline() {
    let value = serde_json::json!({"value": [1, 2, 3]});
    let expected = format!("{}\n", serde_json::to_string_pretty(&value).unwrap()).into_bytes();
    let exact = encode_pretty_bounded(expected.len(), OutputTarget::RawDocument, |writer| {
        serde_json::to_writer_pretty(writer, &value)
    })
    .unwrap();
    assert_eq!(exact, expected);

    let failure = encode_pretty_bounded(expected.len() - 1, OutputTarget::RawDocument, |writer| {
        serde_json::to_writer_pretty(writer, &value)
    })
    .unwrap_err();
    assert_eq!(
        failure.kind,
        FailureKind::ResourceLimit {
            resource: parser_core::ResourceLimitKind::ResponseBytes,
            limit: (expected.len() - 1) as u64,
            actual: expected.len() as u64,
        }
    );
}
