use super::*;

/// Crossref REST discovery adapter, including journal and proceedings records.
pub struct CrossrefProvider {
    client: ProviderClient,
}

impl CrossrefProvider {
    /// Create a Crossref provider with injectable transport and independent policy.
    pub fn new(transport: Arc<dyn HttpTransport>, policy: ProviderPolicy) -> Self {
        Self {
            client: ProviderClient::new(transport, policy),
        }
    }
}

fn year(item: &serde_json::Value) -> Option<i32> {
    [
        "published-print",
        "published-online",
        "published",
        "issued",
        "created",
    ]
    .iter()
    .find_map(|key| {
        item.get(*key)?
            .get("date-parts")?
            .as_array()?
            .first()?
            .as_array()?
            .first()?
            .as_i64()
    })
    .map(|y| y as i32)
}

fn record(item: &serde_json::Value, url: &str, status: u16) -> Option<RawRecord> {
    let id = item.get("DOI").and_then(|v| v.as_str())?.trim();
    if id.is_empty() {
        return None;
    }
    let title = item
        .get("title")
        .and_then(|v| v.as_array())
        .and_then(|v| v.first())
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let authors = item
        .get("author")
        .and_then(|v| v.as_array())
        .map(|xs| {
            xs.iter()
                .filter_map(|a| {
                    let name = a.get("name").and_then(|v| v.as_str()).map(str::to_string);
                    name.or_else(|| {
                        let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
                        let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
                        (!given.is_empty() || !family.is_empty())
                            .then(|| format!("{given} {family}").trim().to_string())
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let raw = serde_json::to_string(item).ok()?;
    let mut identifiers = BTreeMap::new();
    identifiers.insert("doi".into(), id.into());
    Some(RawRecord {
        provider_record_id: id.into(),
        title,
        authors,
        abstract_text: item
            .get("abstract")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        year: year(item),
        raw_venue: item
            .get("container-title")
            .and_then(|v| v.as_array())
            .and_then(|v| v.first())
            .and_then(|v| v.as_str())
            .map(str::to_string),
        work_type: item
            .get("type")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        identifiers,
        citation_count: item.get("is-referenced-by-count").and_then(|v| v.as_u64()),
        source_external_ids: BTreeMap::new(),
        raw_payload: raw.clone(),
        provenance: RecordProvenance {
            provider: "crossref".into(),
            request_url: url.into(),
            response_status: status,
            retrieved_at: now_seconds(),
            payload_sha256: payload_hash(&raw),
        },
        evidence: Vec::new(),
        acceptance: None,
    })
}

impl DiscoveryProvider for CrossrefProvider {
    fn name(&self) -> &'static str {
        "crossref"
    }
    fn discover_page(
        &self,
        request: &DiscoveryRequest,
        cancellation: &dyn Cancellation,
    ) -> Result<DiscoveryPage, PartialError> {
        let mut params: Vec<(String, String)> = vec![
            ("query".into(), url_encode(&request.query)),
            ("rows".into(), request.page_size.to_string()),
            (
                "cursor".into(),
                url_encode(request.cursor.as_deref().unwrap_or("*")),
            ),
        ];
        if let Some(venue) = &request.venue {
            params.push(("query.container-title".into(), url_encode(venue)));
        }
        if !request.filters.is_empty() {
            params.push(("filter".into(), url_encode(&request.filters.join(","))));
        }
        let query = params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!("https://api.crossref.org/works?{query}");
        let response = self.client.send(
            HttpRequest {
                method: "GET".into(),
                url: url.clone(),
                headers: base_headers(),
            },
            "crossref",
            cancellation,
        )?;
        let json: serde_json::Value = serde_json::from_str(&response.body)
            .map_err(|e| parse_error("crossref", &url, e.to_string()))?;
        let message = json
            .get("message")
            .ok_or_else(|| parse_error("crossref", &url, "missing message"))?;
        let items = message
            .get("items")
            .and_then(|v| v.as_array())
            .ok_or_else(|| parse_error("crossref", &url, "missing message.items"))?;
        let records = items
            .iter()
            .filter_map(|item| record(item, &url, response.status))
            .collect();
        let next = message
            .get("next-cursor")
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty() && items.len() >= request.page_size)
            .map(str::to_string);
        Ok(DiscoveryPage {
            provider: "crossref".into(),
            records,
            next_cursor: next,
            request: request.clone(),
        })
    }
}
