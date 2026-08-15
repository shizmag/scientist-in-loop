use super::*;
use quick_xml::Reader;
use quick_xml::events::Event;

/// DBLP search adapter for proceedings, streams, editions, and workshops.
pub struct DblpProvider {
    client: ProviderClient,
}

impl DblpProvider {
    /// Create a DBLP provider with injectable transport and retry policy.
    pub fn new(transport: Arc<dyn HttpTransport>, policy: ProviderPolicy) -> Self {
        Self {
            client: ProviderClient::new(transport, policy),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn make_record(
    id: String,
    title: Option<String>,
    authors: Vec<String>,
    year: Option<i32>,
    venue: Option<String>,
    proceedings: Option<String>,
    raw: String,
    url: &str,
    status: u16,
) -> RawRecord {
    let mut ids = BTreeMap::new();
    ids.insert("dblp_key".into(), id.clone());
    let mut external = BTreeMap::new();
    let mut stream = id.split('/');
    let stream_id = match (stream.next(), stream.next()) {
        (Some(prefix), Some(name)) => format!("{prefix}/{name}"),
        _ => id.clone(),
    };
    external.insert("dblp_stream".into(), stream_id);
    RawRecord {
        provider_record_id: id,
        title,
        authors,
        abstract_text: None,
        year,
        raw_venue: venue,
        work_type: Some("dblp-proceedings-record".into()),
        identifiers: ids,
        citation_count: None,
        source_external_ids: external,
        raw_payload: raw.clone(),
        provenance: RecordProvenance {
            provider: "dblp".into(),
            request_url: url.into(),
            response_status: status,
            retrieved_at: now_seconds(),
            payload_sha256: payload_hash(&raw),
        },
        evidence: proceedings
            .into_iter()
            .map(|value| ProviderEvidence {
                kind: "proceedings".into(),
                value,
            })
            .collect(),
        acceptance: None,
    }
}

fn json_records(
    json: &serde_json::Value,
    raw: &str,
    url: &str,
    status: u16,
) -> Result<Vec<RawRecord>, PartialError> {
    let hits = json
        .get("result")
        .and_then(|v| v.get("hits"))
        .and_then(|v| v.get("hit"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| parse_error("dblp", url, "missing result.hits.hit"))?;
    Ok(hits
        .iter()
        .filter_map(|hit| {
            let info = hit.get("info")?;
            let id = info.get("key").and_then(|v| v.as_str())?.to_string();
            let authors = info
                .get("authors")
                .and_then(|v| v.get("author"))
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.get("text").and_then(|v| v.as_str()).map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            Some(make_record(
                id,
                info.get("title")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                authors,
                info.get("year").and_then(|v| v.as_i64()).map(|v| v as i32),
                info.get("venue")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                info.get("booktitle")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                raw.into(),
                url,
                status,
            ))
        })
        .collect())
}

fn xml_records(raw: &str, url: &str, status: u16) -> Result<Vec<RawRecord>, PartialError> {
    let mut reader = Reader::from_str(raw);
    let mut buf = Vec::new();
    let mut current = None;
    let mut field = String::new();
    let mut value = String::new();
    let mut out = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if name == "hit" {
                    current = Some((String::new(), None, Vec::new(), None, None, None));
                } else {
                    field = name;
                    value.clear();
                }
            }
            Ok(Event::Text(e)) => value.push_str(&String::from_utf8_lossy(e.as_ref())),
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if name == "hit" {
                    if let Some((id, title, authors, year, venue, booktitle)) = current.take()
                        && !id.is_empty()
                    {
                        out.push(make_record(
                            id,
                            title,
                            authors,
                            year,
                            venue,
                            booktitle,
                            raw.into(),
                            url,
                            status,
                        ));
                    }
                } else if let Some(item) = current.as_mut() {
                    match field.as_str() {
                        "key" => item.0 = value.trim().into(),
                        "title" => item.1 = Some(value.trim().into()),
                        "author" => item.2.push(value.trim().into()),
                        "year" => item.3 = value.trim().parse().ok(),
                        "venue" => item.4 = Some(value.trim().into()),
                        "booktitle" => item.5 = Some(value.trim().into()),
                        _ => {}
                    }
                    field.clear();
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(parse_error("dblp", url, e.to_string())),
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

impl DiscoveryProvider for DblpProvider {
    fn name(&self) -> &'static str {
        "dblp"
    }
    fn discover_page(
        &self,
        request: &DiscoveryRequest,
        cancellation: &dyn Cancellation,
    ) -> Result<DiscoveryPage, PartialError> {
        let first = request.cursor.as_deref().unwrap_or("0");
        let url = format!(
            "https://dblp.org/search/publ/api?q={}&h={}&f={}&format=json",
            url_encode(&request.query),
            request.page_size,
            url_encode(first)
        );
        let response = self.client.send(
            HttpRequest {
                method: "GET".into(),
                url: url.clone(),
                headers: base_headers(),
            },
            "dblp",
            cancellation,
        )?;
        let records = if response.body.trim_start().starts_with('<') {
            xml_records(&response.body, &url, response.status)?
        } else {
            let json = serde_json::from_str(&response.body)
                .map_err(|e| parse_error("dblp", &url, e.to_string()))?;
            json_records(&json, &response.body, &url, response.status)?
        };
        let next = (records.len() >= request.page_size)
            .then(|| (first.parse::<usize>().unwrap_or(0) + records.len()).to_string());
        Ok(DiscoveryPage {
            provider: "dblp".into(),
            records,
            next_cursor: next,
            request: request.clone(),
        })
    }
}
