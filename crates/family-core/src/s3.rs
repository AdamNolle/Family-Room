//! S3-compatible encrypted object transport. Credentials remain device-local.
//! SigV4 is checked against AWS's published signing vector.
use crate::{
    error::{invalid, CoreError, Result},
    model::{StorageSource, TransferJob},
    providers::{checked, client, network, request_error},
};
use hmac::{Hmac, Mac};
use reqwest::{
    blocking::{RequestBuilder, Response},
    Method, Url,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
use zeroize::{Zeroize, Zeroizing};

const PART: u64 = 8 * 1024 * 1024;
const MAX_XML: u64 = 8 * 1024 * 1024;
#[derive(Deserialize)]
struct Credentials {
    access_key_id: String,
    secret_access_key: String,
    region: String,
    #[serde(default)]
    session_token: Option<String>,
}
impl Drop for Credentials {
    fn drop(&mut self) {
        self.access_key_id.zeroize();
        self.secret_access_key.zeroize();
        if let Some(token) = &mut self.session_token {
            token.zeroize();
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Upload {
    id: String,
    etags: Vec<String>,
}
fn credentials(source: &StorageSource) -> Result<Credentials> {
    let c: Credentials = serde_json::from_str(&source.token).map_err(|_| {
        invalid("Enter S3 credentials as access_key_id, secret_access_key and region JSON")
    })?;
    if c.access_key_id.is_empty()
        || c.secret_access_key.is_empty()
        || c.region.is_empty()
        || !c
            .region
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || [&c.access_key_id, &c.secret_access_key]
            .iter()
            .any(|s| !s.is_ascii() || s.bytes().any(|b| b.is_ascii_whitespace()))
        || c.session_token
            .as_ref()
            .is_some_and(|s| !s.is_ascii() || s.bytes().any(|b| b.is_ascii_control()))
    {
        return Err(invalid("Invalid S3 credentials"));
    }
    Ok(c)
}
fn endpoint(s: &StorageSource) -> Result<Url> {
    let url = Url::parse(&s.endpoint).map_err(|_| invalid("Use the S3 bucket and prefix URL"))?;
    if !(url.scheme() == "https"
        || (url.scheme() == "http"
            && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))))
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path().split('/').any(|s| s == "." || s == "..")
        || !url
            .path()
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/-._~".contains(&b))
    {
        return Err(invalid("Use HTTPS with an unescaped S3 bucket/prefix path; credentials belong in the credential field"));
    }
    Ok(url)
}
pub(crate) fn validate(s: &StorageSource) -> Result<()> {
    endpoint(s)?;
    credentials(s)?;
    Ok(())
}
fn encoded(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            result.push(b as char);
        } else {
            result.push_str(&format!("%{b:02X}"));
        }
    }
    result
}
fn hmac(key: &[u8], value: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).map_err(|_| CoreError::Authentication)?;
    mac.update(value);
    Ok(Zeroizing::new(mac.finalize().into_bytes().to_vec()))
}
fn authorization(
    c: &Credentials,
    method: &str,
    url: &Url,
    query: &str,
    headers: &BTreeMap<String, String>,
    digest: &str,
    date: &str,
) -> Result<String> {
    let canonical_headers = headers
        .iter()
        .map(|(key, value)| {
            format!(
                "{key}:{}\n",
                value.split_whitespace().collect::<Vec<_>>().join(" ")
            )
        })
        .collect::<String>();
    let signed = headers.keys().cloned().collect::<Vec<_>>().join(";");
    let canonical = format!(
        "{method}\n{}\n{query}\n{canonical_headers}\n{signed}\n{digest}",
        url.path()
    );
    let scope = format!("{}/{}/s3/aws4_request", &date[..8], c.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{date}\n{scope}\n{}",
        hex::encode(Sha256::digest(canonical))
    );
    let initial = Zeroizing::new(format!("AWS4{}", c.secret_access_key));
    let day = hmac(initial.as_bytes(), &date.as_bytes()[..8])?;
    let region = hmac(&day, c.region.as_bytes())?;
    let service = hmac(&region, b"s3")?;
    let key = hmac(&service, b"aws4_request")?;
    let signature = hex::encode(&*hmac(&key, string_to_sign.as_bytes())?);
    Ok(format!(
        "AWS4-HMAC-SHA256 Credential={}/{scope},SignedHeaders={signed},Signature={signature}",
        c.access_key_id
    ))
}
fn request(
    s: &StorageSource,
    method: &str,
    mut url: Url,
    query: &[(&str, String)],
    digest: &str,
) -> Result<RequestBuilder> {
    let c = credentials(s)?;
    let mut parameters: Vec<_> = query
        .iter()
        .map(|(k, v)| (encoded(k), encoded(v)))
        .collect();
    parameters.sort();
    let query = parameters
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    url.set_query(if query.is_empty() { None } else { Some(&query) });
    let host = format!(
        "{}{}",
        url.host_str().ok_or_else(|| invalid("Missing S3 host"))?,
        url.port().map(|p| format!(":{p}")).unwrap_or_default()
    );
    let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let mut headers = BTreeMap::from([
        ("host".into(), host),
        ("x-amz-content-sha256".into(), digest.into()),
        ("x-amz-date".into(), date.clone()),
    ]);
    if let Some(token) = &c.session_token {
        headers.insert("x-amz-security-token".into(), token.clone());
    }
    let signature = authorization(&c, method, &url, &query, &headers, digest, &date)?;
    let mut builder = client()?
        .request(
            Method::from_bytes(method.as_bytes()).map_err(|_| invalid("Invalid S3 method"))?,
            url,
        )
        .header("Authorization", signature);
    for (key, value) in headers {
        builder = builder.header(key, value);
    }
    Ok(builder)
}
fn object(s: &StorageSource, name: &str) -> Result<Url> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-.".contains(&b))
    {
        return Err(invalid("Invalid S3 object name"));
    }
    let mut base = endpoint(s)?;
    base.set_path(&format!("{}/{}", base.path().trim_end_matches('/'), name));
    Ok(base)
}
fn empty() -> String {
    hex::encode(Sha256::digest([]))
}
fn xml(response: Response) -> Result<String> {
    let mut result = String::new();
    checked(response)?
        .take(MAX_XML + 1)
        .read_to_string(&mut result)?;
    if result.len() as u64 > MAX_XML {
        return Err(network("S3 response was too large"));
    }
    let document =
        roxmltree::Document::parse(&result).map_err(|_| network("Invalid S3 response"))?;
    // CompleteMultipartUpload can return an Error inside HTTP 200.
    if document
        .descendants()
        .any(|n| n.tag_name().name() == "Error")
    {
        return Err(network(
            "S3 could not complete this upload; retry or reconnect",
        ));
    }
    Ok(result)
}
fn field(document: &roxmltree::Document<'_>, name: &str) -> Option<String> {
    document
        .descendants()
        .find(|n| n.tag_name().name() == name)
        .and_then(|n| n.text())
        .map(str::to_owned)
}
pub(crate) fn list(s: &StorageSource) -> Result<Vec<String>> {
    let mut base = endpoint(s)?;
    // Path-style endpoint: /bucket/prefix. Virtual-hosted endpoint: /prefix
    // is selected explicitly in credential JSON through virtual_hosted=true.
    let configuration: serde_json::Value = serde_json::from_str(&s.token)?;
    let virtual_hosted = configuration["virtual_hosted"].as_bool().unwrap_or(false);
    let segments = base
        .path()
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if !virtual_hosted && segments.is_empty() {
        return Err(invalid("Include the bucket in the S3 URL"));
    }
    let bucket_path = if virtual_hosted {
        "/".into()
    } else {
        format!("/{}", segments[0])
    };
    let prefix = segments[usize::from(!virtual_hosted)..].join("/");
    let prefix = if prefix.is_empty() {
        prefix
    } else {
        format!("{prefix}/")
    };
    base.set_path(&bucket_path);
    let mut names = vec![];
    let mut continuation = String::new();
    let mut seen = BTreeSet::new();
    loop {
        let mut query = vec![
            ("list-type", "2".into()),
            ("prefix", prefix.clone()),
            ("max-keys", "1000".into()),
        ];
        if !continuation.is_empty() {
            query.push(("continuation-token", continuation.clone()));
        }
        let body = xml(request(s, "GET", base.clone(), &query, &empty())?
            .send()
            .map_err(request_error)?)?;
        let document =
            roxmltree::Document::parse(&body).map_err(|_| network("Invalid S3 listing"))?;
        names.extend(
            document
                .descendants()
                .filter(|n| n.tag_name().name() == "Key")
                .filter_map(|n| n.text())
                .filter_map(|key| key.strip_prefix(&prefix))
                .filter(|key| {
                    key.starts_with(&s.room_id)
                        && (key.ends_with(".frblob")
                            || key.ends_with(".frindex")
                            || key.ends_with(".frpolicy"))
                        && !key.contains('/')
                })
                .map(str::to_owned),
        );
        if field(&document, "IsTruncated").as_deref() != Some("true") {
            break;
        }
        continuation = field(&document, "NextContinuationToken")
            .ok_or_else(|| network("Missing S3 next page"))?;
        if !seen.insert(continuation.clone()) || seen.len() > 10000 {
            return Err(network("Invalid S3 listing pagination"));
        }
    }
    Ok(names)
}
pub(crate) fn get(s: &StorageSource, name: &str) -> Result<Response> {
    request(s, "GET", object(s, name)?, &[], &empty())?
        .send()
        .map_err(request_error)
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub(crate) fn upload_step(
    s: &StorageSource,
    name: &str,
    input: &Path,
    job: &mut TransferJob,
) -> Result<()> {
    let total = fs::metadata(input)?.len();
    if total != job.total || total > PART * 10000 {
        return Err(invalid("S3 upload exceeds the supported multipart size"));
    }
    if total <= PART {
        let digest = crate::crypto::digest(input)?.0;
        checked(
            request(s, "PUT", object(s, name)?, &[], &digest)?
                .header("Content-Length", total)
                .body(fs::File::open(input)?)
                .send()
                .map_err(request_error)?,
        )?;
        job.transferred = total;
        return Ok(());
    }
    let mut upload: Upload = match &job.session {
        Some(session) => {
            serde_json::from_str(session).map_err(|_| network("Invalid S3 upload checkpoint"))?
        }
        None => {
            let body = xml(request(
                s,
                "POST",
                object(s, name)?,
                &[("uploads", "".into())],
                &empty(),
            )?
            .body("")
            .send()
            .map_err(request_error)?)?;
            let doc = roxmltree::Document::parse(&body)
                .map_err(|_| network("Invalid S3 upload session"))?;
            Upload {
                id: field(&doc, "UploadId").ok_or_else(|| network("Missing S3 upload session"))?,
                etags: vec![],
            }
        }
    };
    job.session = Some(serde_json::to_string(&upload)?);
    if upload.etags.len() > 10000 || upload.id.len() > 4096 {
        return Err(network("Invalid S3 upload checkpoint"));
    }
    let offset = (upload.etags.len() as u64 * PART).min(total);
    if offset < total {
        let mut file = fs::File::open(input)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut part = vec![0; (total - offset).min(PART) as usize];
        file.read_exact(&mut part)?;
        let digest = hex::encode(Sha256::digest(&part));
        let number = (upload.etags.len() + 1).to_string();
        let response = request(
            s,
            "PUT",
            object(s, name)?,
            &[("partNumber", number), ("uploadId", upload.id.clone())],
            &digest,
        )?
        .body(part)
        .send()
        .map_err(request_error)?;
        // A completed upload may have lost its final local checkpoint. Let the
        // core verify the existing encrypted object rather than upload it again.
        if response.status().as_u16() == 404 {
            let head = request(s, "HEAD", object(s, name)?, &[], &empty())?
                .send()
                .map_err(request_error)?;
            if head.status().is_success()
                && head
                    .headers()
                    .get("content-length")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    == Some(total)
            {
                job.transferred = total;
                return Ok(());
            }
        }
        let response = checked(response)?;
        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.is_empty() && v.len() <= 512)
            .ok_or_else(|| network("Missing S3 part verification"))?
            .to_string();
        upload.etags.push(etag);
    }
    job.session = Some(serde_json::to_string(&upload)?);
    let uploaded = (upload.etags.len() as u64 * PART).min(total);
    if uploaded == total {
        let parts = upload
            .etags
            .iter()
            .enumerate()
            .map(|(i, etag)| {
                format!(
                    "<Part><PartNumber>{}</PartNumber><ETag>{}</ETag></Part>",
                    i + 1,
                    escape(etag)
                )
            })
            .collect::<String>();
        let body = format!("<CompleteMultipartUpload>{parts}</CompleteMultipartUpload>");
        let digest = hex::encode(Sha256::digest(body.as_bytes()));
        xml(request(
            s,
            "POST",
            object(s, name)?,
            &[("uploadId", upload.id)],
            &digest,
        )?
        .header("Content-Type", "application/xml")
        .body(body)
        .send()
        .map_err(request_error)?)?;
    }
    job.transferred = uploaded;
    Ok(())
}
pub(crate) fn abort(s: &StorageSource, name: &str, session: &str) -> Result<()> {
    let upload: Upload =
        serde_json::from_str(session).map_err(|_| network("Invalid S3 upload checkpoint"))?;
    let response = request(
        s,
        "DELETE",
        object(s, name)?,
        &[("uploadId", upload.id)],
        &empty(),
    )?
    .send()
    .map_err(request_error)?;
    if response.status().as_u16() != 404 {
        checked(response)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "gateway")]
    #[test]
    fn multipart_restart_verifies_ciphertext_and_cancellation_releases_parts() {
        use axum::{
            body::Bytes,
            extract::{DefaultBodyLimit, State},
            http::{HeaderMap, Method, StatusCode, Uri},
            response::{IntoResponse, Response as HttpResponse},
            Router,
        };
        use serde_json::json;
        use std::sync::{Arc, Mutex};
        #[derive(Default)]
        struct Server {
            parts: BTreeMap<u32, Vec<u8>>,
            objects: BTreeMap<String, Vec<u8>>,
            fail_completion: bool,
            aborts: usize,
        }
        async fn handler(
            State(server): State<Arc<Mutex<Server>>>,
            method: Method,
            uri: Uri,
            headers: HeaderMap,
            bytes: Bytes,
        ) -> HttpResponse {
            let digest = hex::encode(Sha256::digest(&bytes));
            if headers
                .get("x-amz-content-sha256")
                .and_then(|v| v.to_str().ok())
                != Some(&digest)
                || !headers
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.starts_with("AWS4-HMAC-SHA256 Credential=EXAMPLE/"))
            {
                return StatusCode::FORBIDDEN.into_response();
            }
            let address = Url::parse(&format!("http://localhost{uri}")).unwrap();
            let query: BTreeMap<_, _> = address
                .query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            let mut server = server.lock().unwrap();
            let name = uri.path().rsplit('/').next().unwrap().to_string();
            if method == Method::GET && query.contains_key("list-type") {
                let keys = server
                    .objects
                    .keys()
                    .map(|key| format!("<Contents><Key>family/{key}</Key></Contents>"))
                    .collect::<String>();
                return format!(
                    "<ListBucketResult><IsTruncated>false</IsTruncated>{keys}</ListBucketResult>"
                )
                .into_response();
            }
            if method == Method::POST && query.contains_key("uploads") {
                server.parts.clear();
                return "<InitiateMultipartUploadResult><UploadId>id=+ with space</UploadId></InitiateMultipartUploadResult>".into_response();
            }
            if query.contains_key("uploadId") {
                assert_eq!(query["uploadId"], "id=+ with space");
                if method == Method::PUT {
                    let number: u32 = query["partNumber"].parse().unwrap();
                    server.parts.insert(number, bytes.to_vec());
                    return (
                        [(axum::http::header::ETAG, format!("\"part&{number}\""))],
                        "",
                    )
                        .into_response();
                }
                if method == Method::DELETE {
                    server.parts.clear();
                    server.aborts += 1;
                    return StatusCode::NO_CONTENT.into_response();
                }
                if method == Method::POST {
                    if server.fail_completion {
                        server.fail_completion = false;
                        return "<Error><Code>InternalError</Code></Error>".into_response();
                    }
                    let xml = std::str::from_utf8(&bytes).unwrap();
                    let document = roxmltree::Document::parse(xml).unwrap();
                    let tags: Vec<_> = document
                        .descendants()
                        .filter(|n| n.tag_name().name() == "ETag")
                        .filter_map(|n| n.text())
                        .collect();
                    assert_eq!(tags, vec!["\"part&1\"", "\"part&2\""]);
                    let original = server.parts.values().flatten().copied().collect();
                    server.objects.insert(name, original);
                    server.parts.clear();
                    return "<CompleteMultipartUploadResult><ETag>verified</ETag></CompleteMultipartUploadResult>".into_response();
                }
            }
            if method == Method::PUT {
                server.objects.insert(name, bytes.to_vec());
                return StatusCode::OK.into_response();
            }
            if let Some(data) = server.objects.get(&name) {
                if method == Method::HEAD {
                    return (
                        [(axum::http::header::CONTENT_LENGTH, data.len().to_string())],
                        "",
                    )
                        .into_response();
                }
                return data.clone().into_response();
            }
            StatusCode::NOT_FOUND.into_response()
        }
        let server = Arc::new(Mutex::new(Server {
            fail_completion: true,
            ..Default::default()
        }));
        let app = Router::new()
            .fallback(handler)
            .layer(DefaultBodyLimit::max(9 * 1024 * 1024))
            .with_state(server.clone());
        let (sender, receiver) = std::sync::mpsc::channel();
        let (shutdown, stop) = tokio::sync::oneshot::channel::<()>();
        let thread = std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async move {
                    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                    sender.send(listener.local_addr().unwrap()).unwrap();
                    axum::serve(listener, app)
                        .with_graceful_shutdown(async {
                            let _ = stop.await;
                        })
                        .await
                        .unwrap();
                });
        });
        let dir = tempfile::tempdir().unwrap();
        let mut core = crate::engine::Engine::open(dir.path(), [9; 32]).unwrap();
        let room = core
            .execute(json!({"action":"create_room","name":"S3 family"}))
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let input = dir.path().join("large.mp4");
        let original = vec![37; 10 * 1024 * 1024];
        fs::write(&input, &original).unwrap();
        let asset = core
            .execute(json!({"action":"import","room_id":room,"path":input}))
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let source = core.execute(json!({"action":"add_source","room_id":room,"name":"S3 fixture","kind":"s3",
            "endpoint":format!("http://{}/bucket/family",receiver.recv().unwrap()),
            "token":json!({"access_key_id":"EXAMPLE","secret_access_key":"SECRET","region":"test-1","session_token":"temporary"}).to_string()
        })).unwrap()["id"].as_str().unwrap().to_string();
        let first = core
            .execute(json!({"action":"replicate_step","source_id":source,"asset_id":asset}))
            .unwrap();
        assert_eq!(first["transferred"], PART);
        assert_eq!(server.lock().unwrap().parts.len(), 1);
        drop(core);
        let mut core = crate::engine::Engine::open(dir.path(), [9; 32]).unwrap();
        // HTTP 200 with an embedded Error must never claim a verified copy.
        let failed = core
            .execute(json!({"action":"replicate_step","source_id":source,"asset_id":asset}))
            .unwrap();
        assert_eq!(failed["state"], "waiting");
        assert!(!core.state.replicas.iter().any(|r| r.verified));
        let resumed = core
            .execute(json!({"action":"retry_transfer","id":first["id"]}))
            .unwrap();
        assert_eq!(resumed["state"], "complete");
        assert_eq!(
            server.lock().unwrap().objects.values().next().unwrap(),
            &fs::read(core.blob(&room, &asset).unwrap()).unwrap()
        );
        fs::remove_file(core.blob(&room, &asset).unwrap()).unwrap();
        core.execute(json!({"action":"fetch","room_id":room,"asset_id":asset}))
            .unwrap();
        let output = dir.path().join("restored.mp4");
        core.execute(
            json!({"action":"materialize","room_id":room,"asset_id":asset,"destination":output}),
        )
        .unwrap();
        assert_eq!(fs::read(output).unwrap(), original);
        // A different original starts a separate multipart job.
        fs::write(&input, vec![91; 10 * 1024 * 1024]).unwrap();
        let other = core
            .execute(json!({"action":"import","room_id":room,"path":input}))
            .unwrap()["id"]
            .clone();
        let partial = core
            .execute(json!({"action":"replicate_step","source_id":source,"asset_id":other}))
            .unwrap();
        assert_eq!(server.lock().unwrap().parts.len(), 1);
        core.execute(json!({"action":"cancel_transfer","id":partial["id"]}))
            .unwrap();
        assert!(server.lock().unwrap().parts.is_empty());
        assert_eq!(server.lock().unwrap().aborts, 1);
        let source_configuration = core.state.sources[0].clone();
        let listed = list(&source_configuration).unwrap();
        assert_eq!(listed, vec![format!("{room}-{asset}.frblob")]);
        assert!(!core.snapshot().unwrap().to_string().contains("SECRET"));
        shutdown.send(()).unwrap();
        thread.join().unwrap();
    }
    #[test]
    fn aws_published_sigv4_vector() {
        // AWS public test credentials, never production secrets.
        let c = Credentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
            region: "us-east-1".into(),
            session_token: None,
        };
        let headers = BTreeMap::from([
            ("host".into(), "examplebucket.s3.amazonaws.com".into()),
            ("range".into(), "bytes=0-9".into()),
            ("x-amz-content-sha256".into(), empty()),
            ("x-amz-date".into(), "20130524T000000Z".into()),
        ]);
        let value = authorization(
            &c,
            "GET",
            &Url::parse("https://examplebucket.s3.amazonaws.com/test.txt").unwrap(),
            "",
            &headers,
            &empty(),
            "20130524T000000Z",
        )
        .unwrap();
        assert!(value.ends_with(
            "Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        ));
        assert_eq!(encoded("a b+/="), "a%20b%2B%2F%3D");
    }
}
