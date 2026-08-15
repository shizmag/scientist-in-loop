use super::*;

/// OpenReview search adapter. Forum hosting and acceptance are deliberately separate observations.
pub struct OpenReviewProvider {
    client: ProviderClient,
}

impl OpenReviewProvider {
    /// Create an OpenReview provider with injectable transport and retry policy.
    pub fn new(transport: Arc<dyn HttpTransport>, policy: ProviderPolicy) -> Self {
        Self {
            client: ProviderClient::new(transport, policy),
        }
    }
}

fn text(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.get("value").and_then(text))
}

fn evidence(kind: &str, value: &serde_json::Value, out: &mut Vec<ProviderEvidence>) {
    if let Some(value) = text(value) {
        if !value.trim().is_empty() {
            out.push(ProviderEvidence {
                kind: kind.into(),
                value,
            });
        }
    } else if value.is_array() || value.is_object() {
        out.push(ProviderEvidence {
            kind: kind.into(),
            value: value.to_string(),
        });
    }
}

fn record(note: &serde_json::Value, url: &str, status: u16) -> Option<RawRecord> {
    let id = note
        .get("id")
        .or_else(|| note.get("forum"))
        .and_then(|v| v.as_str())?
        .to_string();
    let content = note.get("content").unwrap_or(&serde_json::Value::Null);
    let mut facts = Vec::new();
    for (key, value) in [
        ("forum", note.get("forum")),
        ("note", Some(note)),
        (
            "invitation",
            note.get("invitation").or_else(|| note.get("invitations")),
        ),
        (
            "group",
            note.get("signatures").or_else(|| note.get("writers")),
        ),
        ("domain", note.get("domain").or_else(|| note.get("domain"))),
        ("content", Some(content)),
    ] {
        if let Some(value) = value {
            evidence(key, value, &mut facts);
        }
    }
    let mut decision_facts = Vec::new();
    for (key, value) in facts
        .iter()
        .map(|fact| (fact.kind.as_str(), fact.value.as_str()))
    {
        if matches!(key, "invitation" | "content" | "group" | "domain")
            && (value.to_ascii_lowercase().contains("accept")
                || value.to_ascii_lowercase().contains("reject")
                || value.to_ascii_lowercase().contains("withdraw"))
        {
            decision_facts.push(ProviderEvidence {
                kind: key.into(),
                value: value.into(),
            });
        }
    }
    let lower = decision_facts
        .iter()
        .map(|x| x.value.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let state = if lower.iter().any(|x| x.contains("withdraw")) {
        "withdrawn"
    } else if lower.iter().any(|x| x.contains("reject"))
        && lower.iter().any(|x| x.contains("accept"))
    {
        "ambiguous"
    } else if lower.iter().any(|x| x.contains("reject")) {
        "rejected"
    } else if lower.iter().any(|x| x.contains("accept")) {
        "accepted"
    } else {
        "unknown"
    };
    let raw = serde_json::to_string(note).ok()?;
    Some(RawRecord {
        provider_record_id: id.clone(),
        title: content.get("title").and_then(text),
        authors: Vec::new(),
        abstract_text: content.get("abstract").and_then(text),
        year: note.get("year").and_then(|v| v.as_i64()).map(|v| v as i32),
        raw_venue: content.get("venue").and_then(text),
        work_type: Some("openreview-note".into()),
        identifiers: BTreeMap::from([(
            "openreview_forum".into(),
            note.get("forum")
                .and_then(|v| v.as_str())
                .unwrap_or(&id)
                .into(),
        )]),
        citation_count: None,
        source_external_ids: BTreeMap::new(),
        raw_payload: raw.clone(),
        provenance: RecordProvenance {
            provider: "openreview".into(),
            request_url: url.into(),
            response_status: status,
            retrieved_at: now_seconds(),
            payload_sha256: payload_hash(&raw),
        },
        evidence: facts.clone(),
        acceptance: Some(AcceptanceEvidence {
            state: state.into(),
            evidence: decision_facts,
        }),
    })
}

impl DiscoveryProvider for OpenReviewProvider {
    fn name(&self) -> &'static str {
        "openreview"
    }

    fn discover_page(
        &self,
        request: &DiscoveryRequest,
        cancellation: &dyn Cancellation,
    ) -> Result<DiscoveryPage, PartialError> {
        let offset = request.cursor.as_deref().unwrap_or("0");
        let url = format!(
            "https://api2.openreview.net/notes?content.title={}&limit={}&offset={}",
            url_encode(&request.query),
            request.page_size,
            url_encode(offset)
        );
        let response = self.client.send(
            HttpRequest {
                method: "GET".into(),
                url: url.clone(),
                headers: base_headers(),
            },
            "openreview",
            cancellation,
        )?;
        let json: serde_json::Value = serde_json::from_str(&response.body)
            .map_err(|e| parse_error("openreview", &url, e.to_string()))?;
        let notes = json
            .get("notes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| parse_error("openreview", &url, "missing notes"))?;
        let records = notes
            .iter()
            .filter_map(|note| record(note, &url, response.status))
            .collect::<Vec<_>>();
        let next = (notes.len() >= request.page_size)
            .then(|| (offset.parse::<usize>().unwrap_or(0) + notes.len()).to_string());
        Ok(DiscoveryPage {
            provider: "openreview".into(),
            records,
            next_cursor: next,
            request: request.clone(),
        })
    }
}
