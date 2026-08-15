use super::*;

/// OpenAlex broad works discovery adapter with citation and source identifiers.
pub struct OpenAlexProvider {
    client: ProviderClient,
}

impl OpenAlexProvider {
    /// Create an OpenAlex provider with injectable transport and independent policy.
    pub fn new(transport: Arc<dyn HttpTransport>, policy: ProviderPolicy) -> Self {
        Self {
            client: ProviderClient::new(transport, policy),
        }
    }
}

fn record(item: &serde_json::Value, url: &str, status: u16) -> Option<RawRecord> {
    let id = item.get("id").and_then(|v| v.as_str())?.to_string();
    let mut identifiers = BTreeMap::new();
    if let Some(doi) = item.get("doi").and_then(|v| v.as_str()) {
        identifiers.insert(
            "doi".into(),
            doi.trim_start_matches("https://doi.org/").into(),
        );
    }
    let mut sources = BTreeMap::new();
    if let Some(source) = item.get("primary_location").and_then(|v| v.get("source")) {
        if let Some(source_id) = source.get("id").and_then(|v| v.as_str()) {
            sources.insert("openalex_source".into(), source_id.into());
        }
        if let Some(issn) = source.get("issn_l").and_then(|v| v.as_str()) {
            sources.insert("issn_l".into(), issn.into());
        }
    }
    let raw = serde_json::to_string(item).ok()?;
    let authors = item
        .get("authorships")
        .and_then(|v| v.as_array())
        .map(|xs| {
            xs.iter()
                .filter_map(|a| {
                    a.get("author")
                        .and_then(|v| v.get("display_name"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default();
    Some(RawRecord {
        provider_record_id: id,
        title: item
            .get("title")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        authors,
        abstract_text: None,
        year: item
            .get("publication_year")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        raw_venue: item
            .get("primary_location")
            .and_then(|v| v.get("source"))
            .and_then(|v| v.get("display_name"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        work_type: item
            .get("type")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        identifiers,
        citation_count: item.get("cited_by_count").and_then(|v| v.as_u64()),
        source_external_ids: sources,
        raw_payload: raw.clone(),
        provenance: RecordProvenance {
            provider: "openalex".into(),
            request_url: url.into(),
            response_status: status,
            retrieved_at: now_seconds(),
            payload_sha256: payload_hash(&raw),
        },
        evidence: Vec::new(),
        acceptance: None,
    })
}

impl DiscoveryProvider for OpenAlexProvider {
    fn name(&self) -> &'static str {
        "openalex"
    }
    fn discover_page(
        &self,
        request: &DiscoveryRequest,
        cancellation: &dyn Cancellation,
    ) -> Result<DiscoveryPage, PartialError> {
        let mut params: Vec<(String, String)> = vec![
            ("search".into(), url_encode(&request.query)),
            ("per-page".into(), request.page_size.to_string()),
        ];
        if let Some(cursor) = &request.cursor {
            params.push(("cursor".into(), url_encode(cursor)));
        }
        if let Some(venue) = &request.venue {
            params.push((
                "filter".into(),
                url_encode(&format!(
                    "primary_location.source.display_name.search:{venue}"
                )),
            ));
        }
        if !request.filters.is_empty() {
            params.push(("filter".into(), url_encode(&request.filters.join(","))));
        }
        let query = params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!("https://api.openalex.org/works?{query}");
        let response = self.client.send(
            HttpRequest {
                method: "GET".into(),
                url: url.clone(),
                headers: base_headers(),
            },
            "openalex",
            cancellation,
        )?;
        let json: serde_json::Value = serde_json::from_str(&response.body)
            .map_err(|e| parse_error("openalex", &url, e.to_string()))?;
        let items = json
            .get("results")
            .and_then(|v| v.as_array())
            .ok_or_else(|| parse_error("openalex", &url, "missing results"))?;
        let records = items
            .iter()
            .filter_map(|item| record(item, &url, response.status))
            .collect();
        let next = json
            .get("meta")
            .and_then(|v| v.get("next_cursor"))
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty() && items.len() >= request.page_size)
            .map(str::to_string);
        Ok(DiscoveryPage {
            provider: "openalex".into(),
            records,
            next_cursor: next,
            request: request.clone(),
        })
    }
}
