use crate::{
    error::{invalid, CoreError, Result},
    model::{StorageSource, TransferJob},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use reqwest::{
    blocking::{Client, Response},
    redirect::Policy,
    Url,
};
use serde_json::{json, Value};
use sha2::Digest;
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::Path,
    time::Duration,
};

const PART: usize = 1280 * 1024; // Multiple of both Google 256 KiB and OneDrive 320 KiB.
pub(crate) fn network(message: &str) -> CoreError {
    CoreError::Storage {
        reason: message.into(),
    }
}
pub(crate) fn client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(90))
        .redirect(Policy::none())
        .build()
        .map_err(|_| network("Could not start a storage connection"))
}
pub(crate) fn checked(response: Response) -> Result<Response> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(network(match response.status().as_u16() {
            401 | 403 => "Reconnect this storage account",
            404 => "The storage object is unavailable",
            429 => "The provider is busy; retry later",
            507 => "Upload paused: storage full",
            _ => "The storage provider rejected the request",
        }))
    }
}
pub(crate) fn request_error(_: reqwest::Error) -> CoreError {
    network("Storage connection interrupted; retry when the source is reachable")
}

fn gateway_request(
    s: &StorageSource,
    method: &str,
    path: &str,
    digest: &str,
) -> Result<reqwest::blocking::RequestBuilder> {
    let value: Value = serde_json::from_str(&s.token)?;
    let owner = value["owner_id"]
        .as_str()
        .ok_or(CoreError::Authentication)?;
    let grant: crate::model::Grant = serde_json::from_value(value["grant"].clone())?;
    let seed = value["signing_seed"]
        .as_str()
        .ok_or(CoreError::Authentication)?;
    let time = crate::engine::now().to_string();
    let proof = crate::signing::sign_message(
        seed,
        format!("{method}\n{path}\n{digest}\n{time}").as_bytes(),
    )?;
    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|_| invalid("Invalid HTTP method"))?;
    Ok(client()?
        .request(
            method,
            format!("{}{}", s.endpoint.trim_end_matches('/'), path),
        )
        .header("x-family-owner", owner)
        .header(
            "x-family-grant",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&grant)?),
        )
        .header(
            "x-family-access-revision",
            value["access_revision"].as_u64().unwrap_or(0).to_string(),
        )
        .header("x-family-time", time)
        .header("x-family-sha256", digest)
        .header("x-family-proof", proof))
}

pub(crate) fn read_access(s: &StorageSource) -> Result<Vec<crate::model::AccessChange>> {
    if s.kind == "gateway" {
        let path = format!("/v1/rooms/{}/access", s.room_id);
        let response = json_response(
            gateway_request(s, "GET", &path, &hex::encode(sha2::Sha256::digest([])))?
                .send()
                .map_err(request_error)?,
        )?;
        return Ok(serde_json::from_value(response["changes"].clone())?);
    }
    let name = format!("{}-access.frpolicy", s.room_id);
    if !list(s)?.contains(&name) {
        return Ok(vec![]);
    }
    let temp = tempfile::NamedTempFile::new()?;
    download(s, &name, temp.path())?;
    let file = fs::File::open(temp.path())?;
    if file.metadata()?.len() > 32 * 1024 * 1024 {
        return Err(network("Access history is too large"));
    }
    Ok(serde_json::from_reader(file)?)
}
pub(crate) fn publish_access(
    s: &StorageSource,
    changes: &[crate::model::AccessChange],
) -> Result<()> {
    let bytes = serde_json::to_vec(changes)?;
    if s.kind == "gateway" {
        let path = format!("/v1/rooms/{}/access", s.room_id);
        let digest = hex::encode(sha2::Sha256::digest(&bytes));
        checked(
            gateway_request(s, "PUT", &path, &digest)?
                .body(bytes)
                .send()
                .map_err(request_error)?,
        )?;
        return Ok(());
    }
    let mut temp = tempfile::NamedTempFile::new()?;
    use std::io::Write;
    temp.write_all(&bytes)?;
    let total = bytes.len() as u64;
    let mut job = TransferJob {
        id: crate::engine::id(),
        room_id: s.room_id.clone(),
        asset_id: String::new(),
        source_id: s.id.clone(),
        state: "transferring".into(),
        transferred: 0,
        total,
        error: None,
        session: None,
    };
    let name = format!("{}-access.frpolicy", s.room_id);
    while job.transferred < total {
        upload_step(s, &name, temp.path(), &mut job)?;
    }
    Ok(())
}

pub fn register_gateway(s: &StorageSource) -> Result<()> {
    let path = format!("/v1/rooms/{}", s.room_id);
    checked(
        gateway_request(s, "PUT", &path, &hex::encode(sha2::Sha256::digest([])))?
            .body("")
            .send()
            .map_err(request_error)?,
    )?;
    Ok(())
}
fn json_response(response: Response) -> Result<Value> {
    checked(response)?
        .json()
        .map_err(|_| network("Invalid storage provider response"))
}
pub fn capabilities(kind: &str) -> Value {
    json!({"implemented":(["local","webdav","google_drive","onedrive","gateway","s3"].contains(&kind)),"quota":(["local","google_drive","onedrive"].contains(&kind)),"resumable":(["local","google_drive","onedrive","s3"].contains(&kind)),"delegated_access":(["google_drive","onedrive","gateway"].contains(&kind)),"background_available":kind!="local","shared_access_requires_configuration":true})
}
pub fn validate(s: &StorageSource) -> Result<()> {
    if s.kind == "s3" {
        return crate::s3::validate(s);
    }
    if s.kind == "local" {
        if !Path::new(&s.endpoint).is_dir() {
            return Err(invalid("Choose an existing storage folder"));
        }
        return Ok(());
    }
    if s.kind == "webdav" || s.kind == "gateway" {
        let url = Url::parse(&s.endpoint).map_err(|_| invalid("Use an HTTPS WebDAV URL"))?;
        validate_url(&url)?;
        if url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(invalid("Keep credentials outside the WebDAV URL"));
        }
    }
    if s.kind == "google_drive"
        && (s.endpoint.is_empty()
            || !s
                .endpoint
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'))
    {
        return Err(invalid("Enter the app-owned Google Drive folder ID"));
    }
    if s.kind != "local" && s.token.trim().is_empty() {
        return Err(invalid("Authorize this storage source first"));
    }
    Ok(())
}
fn validate_url(url: &Url) -> Result<()> {
    if url.scheme() == "https"
        || (url.scheme() == "http"
            && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")))
    {
        Ok(())
    } else {
        Err(invalid(
            "Use HTTPS; HTTP is allowed only for localhost development",
        ))
    }
}
fn url(s: &StorageSource, name: &str) -> Result<String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'.')
    {
        return Err(invalid("Invalid storage object name"));
    }
    Ok(format!("{}/{name}", s.endpoint.trim_end_matches('/')))
}
fn drive_id(s: &StorageSource, name: &str) -> Result<Option<String>> {
    let query = format!(
        "'{}' in parents and name = '{}' and trashed = false",
        s.endpoint, name
    );
    let value = json_response(
        client()?
            .get("https://www.googleapis.com/drive/v3/files")
            .bearer_auth(&s.token)
            .query(&[
                ("q", query.as_str()),
                ("fields", "files(id)"),
                ("pageSize", "100"),
            ])
            .send()
            .map_err(request_error)?,
    )?;
    Ok(value["files"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|x| x["id"].as_str())
        .map(str::to_owned))
}
pub fn quota(s: &StorageSource) -> Result<Value> {
    match s.kind.as_str() {
        "local" => Ok(
            json!({"remaining":fs2::available_space(&s.endpoint)?,"total":fs2::total_space(&s.endpoint)?}),
        ),
        "google_drive" => {
            let v = json_response(
                client()?
                    .get("https://www.googleapis.com/drive/v3/about")
                    .bearer_auth(&s.token)
                    .query(&[("fields", "storageQuota")])
                    .send()
                    .map_err(request_error)?,
            )?;
            let limit = v["storageQuota"]["limit"]
                .as_str()
                .and_then(|x| x.parse::<u64>().ok());
            let used = v["storageQuota"]["usage"]
                .as_str()
                .and_then(|x| x.parse::<u64>().ok());
            Ok(
                json!({"total":limit,"used":used,"remaining":limit.zip(used).map(|(l,u)|l.saturating_sub(u))}),
            )
        }
        "onedrive" => {
            let v = json_response(
                client()?
                    .get("https://graph.microsoft.com/v1.0/me/drive")
                    .bearer_auth(&s.token)
                    .send()
                    .map_err(request_error)?,
            )?;
            Ok(
                json!({"remaining":v["quota"]["remaining"],"total":v["quota"]["total"],"used":v["quota"]["used"]}),
            )
        }
        "webdav" | "gateway" | "s3" => Ok(json!({"remaining":null,"total":null,"unknown":true})),
        _ => Err(invalid("Provider is not implemented")),
    }
}
pub fn list(s: &StorageSource) -> Result<Vec<String>> {
    if s.kind == "s3" {
        return crate::s3::list(s);
    }
    match s.kind.as_str() {
        "gateway" => {
            let path = format!("/v1/rooms/{}/objects", s.room_id);
            let result = json_response(
                gateway_request(s, "GET", &path, &hex::encode(sha2::Sha256::digest([])))?
                    .send()
                    .map_err(request_error)?,
            )?;
            Ok(serde_json::from_value(result["objects"].clone())?)
        }
        "local" => {
            let mut names = vec![];
            for e in fs::read_dir(&s.endpoint)? {
                let e = e?;
                if e.file_type()?.is_file() {
                    if let Some(name) = e.file_name().to_str() {
                        names.push(name.into());
                    }
                }
            }
            Ok(names)
        }
        "google_drive" => {
            let mut names = vec![];
            let mut page = String::new();
            loop {
                let q = format!("'{}' in parents and trashed = false", s.endpoint);
                let v = json_response(
                    client()?
                        .get("https://www.googleapis.com/drive/v3/files")
                        .bearer_auth(&s.token)
                        .query(&[
                            ("q", q.as_str()),
                            ("fields", "nextPageToken,files(name)"),
                            ("pageSize", "1000"),
                            ("pageToken", page.as_str()),
                        ])
                        .send()
                        .map_err(request_error)?,
                )?;
                if let Some(files) = v["files"].as_array() {
                    names.extend(
                        files
                            .iter()
                            .filter_map(|f| f["name"].as_str())
                            .map(str::to_owned),
                    );
                }
                match v["nextPageToken"].as_str() {
                    Some(p) => page = p.into(),
                    None => break,
                }
            }
            Ok(names)
        }
        "onedrive" => {
            let mut next="https://graph.microsoft.com/v1.0/me/drive/special/approot/children?$select=name&$top=1000".to_string();
            let mut names = vec![];
            loop {
                let parsed = Url::parse(&next).map_err(|_| invalid("Invalid provider page"))?;
                if parsed.scheme() != "https" || parsed.host_str() != Some("graph.microsoft.com") {
                    return Err(CoreError::Authentication);
                }
                let v = json_response(
                    client()?
                        .get(&next)
                        .bearer_auth(&s.token)
                        .send()
                        .map_err(request_error)?,
                )?;
                if let Some(files) = v["value"].as_array() {
                    names.extend(
                        files
                            .iter()
                            .filter_map(|f| f["name"].as_str())
                            .map(str::to_owned),
                    );
                }
                match v["@odata.nextLink"].as_str() {
                    Some(n) => next = n.into(),
                    None => break,
                }
            }
            Ok(names)
        }
        "webdav" => {
            let response=checked(client()?.request(reqwest::Method::from_bytes(b"PROPFIND").map_err(|_|invalid("Invalid HTTP method"))?,&s.endpoint).bearer_auth(&s.token).header("Depth","1").header("Content-Type","application/xml").body("<?xml version=\"1.0\"?><d:propfind xmlns:d=\"DAV:\"><d:prop><d:resourcetype/></d:prop></d:propfind>").send().map_err(request_error)?)?;
            let xml = response.text().map_err(request_error)?;
            let document = roxmltree::Document::parse(&xml)
                .map_err(|_| network("Invalid WebDAV directory listing"))?;
            Ok(document
                .descendants()
                .filter(|n| n.tag_name().name() == "href")
                .filter_map(|n| n.text())
                .filter_map(|href| href.trim_end_matches('/').rsplit('/').next())
                .filter(|name| {
                    name.ends_with(".frindex")
                        || name.ends_with(".frblob")
                        || name.ends_with(".frpolicy")
                })
                .map(str::to_owned)
                .collect())
        }
        _ => Err(invalid("Provider is not implemented")),
    }
}
pub fn upload_step(
    s: &StorageSource,
    name: &str,
    input: &Path,
    job: &mut TransferJob,
) -> Result<()> {
    if s.kind == "s3" {
        return crate::s3::upload_step(s, name, input, job);
    }
    match s.kind.as_str() {
        "gateway" => {
            let path = format!("/v1/rooms/{}/objects/{name}", s.room_id);
            let hash = crate::crypto::digest(input)?.0;
            checked(
                gateway_request(s, "PUT", &path, &hash)?
                    .body(fs::File::open(input)?)
                    .send()
                    .map_err(request_error)?,
            )?;
            job.transferred = job.total;
            Ok(())
        }
        "local" => {
            let dest = Path::new(&s.endpoint).join(name);
            let staging = Path::new(&s.endpoint).join(format!(".{name}.{}.partial", job.id));
            let mut output = fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&staging)?;
            // Resume from durable bytes, rather than trusting a potentially stale checkpoint.
            let offset = output.metadata()?.len().min(job.total);
            output.set_len(offset)?;
            output.seek(SeekFrom::Start(offset))?;
            let mut source = fs::File::open(input)?;
            source.seek(SeekFrom::Start(offset))?;
            let mut bytes = vec![0; (job.total - offset).min(PART as u64) as usize];
            source.read_exact(&mut bytes)?;
            use std::io::Write;
            output.write_all(&bytes)?;
            output.sync_all()?;
            job.transferred = offset + bytes.len() as u64;
            if job.transferred == job.total {
                fs::rename(staging, dest)?;
            }
            Ok(())
        }
        "webdav" => {
            let file = fs::File::open(input)?;
            checked(
                client()?
                    .put(url(s, name)?)
                    .bearer_auth(&s.token)
                    .header("Content-Type", "application/octet-stream")
                    .body(file)
                    .send()
                    .map_err(request_error)?,
            )?;
            job.transferred = job.total;
            Ok(())
        }
        "google_drive" | "onedrive" => {
            if job.session.is_none() {
                let session = if s.kind == "google_drive" {
                    let existing = drive_id(s, name)?;
                    let base = existing
                        .as_ref()
                        .map(|id| format!("https://www.googleapis.com/upload/drive/v3/files/{id}"))
                        .unwrap_or_else(|| {
                            "https://www.googleapis.com/upload/drive/v3/files".into()
                        });
                    let body = if existing.is_some() {
                        json!({"name":name})
                    } else {
                        json!({"name":name,"parents":[s.endpoint]})
                    };
                    let response = checked(
                        client()?
                            .request(
                                if existing.is_some() {
                                    reqwest::Method::PATCH
                                } else {
                                    reqwest::Method::POST
                                },
                                base,
                            )
                            .query(&[("uploadType", "resumable")])
                            .bearer_auth(&s.token)
                            .header("X-Upload-Content-Length", job.total)
                            .header("X-Upload-Content-Type", "application/octet-stream")
                            .json(&body)
                            .send()
                            .map_err(request_error)?,
                    )?;
                    response
                        .headers()
                        .get("Location")
                        .and_then(|h| h.to_str().ok())
                        .ok_or_else(|| network("Missing upload session"))?
                        .to_owned()
                } else {
                    let v=json_response(client()?.post(format!("https://graph.microsoft.com/v1.0/me/drive/special/approot:/{name}:/createUploadSession")).bearer_auth(&s.token).json(&json!({"item":{"@microsoft.graph.conflictBehavior":"replace","name":name}})).send().map_err(request_error)?)?;
                    v["uploadUrl"]
                        .as_str()
                        .ok_or_else(|| network("Missing upload session"))?
                        .to_owned()
                };
                job.session = Some(session);
                job.transferred = 0;
            }
            let session = job
                .session
                .as_deref()
                .ok_or_else(|| network("Missing upload session"))?;
            let parsed = Url::parse(session).map_err(|_| network("Invalid upload session"))?;
            if parsed.scheme() != "https" {
                return Err(CoreError::Authentication);
            }
            // Query the provider after every restart/interruption; the server's offset is authoritative.
            let status = if s.kind == "google_drive" {
                client()?
                    .put(session)
                    .header("Content-Length", 0)
                    .header("Content-Range", format!("bytes */{}", job.total))
                    .send()
            } else {
                client()?.get(session).send()
            }
            .map_err(request_error)?;
            if status.status().as_u16() == 404 || status.status().as_u16() == 410 {
                job.session = None;
                job.transferred = 0;
                return Ok(());
            }
            if s.kind == "google_drive" {
                if status.status().is_success() {
                    job.transferred = job.total;
                    return Ok(());
                }
                if status.status().as_u16() != 308 {
                    checked(status)?;
                    return Err(network("Invalid upload checkpoint"));
                }
                job.transferred = status
                    .headers()
                    .get("Range")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|r| r.rsplit('-').next())
                    .and_then(|n| n.parse::<u64>().ok())
                    .map(|n| n + 1)
                    .unwrap_or(0);
            } else {
                let v = json_response(status)?;
                job.transferred = v["nextExpectedRanges"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(Value::as_str)
                    .and_then(|s| s.split('-').next())
                    .and_then(|n| n.parse::<u64>().ok())
                    .ok_or_else(|| network("Invalid upload checkpoint"))?;
            }
            if job.transferred >= job.total {
                return Err(network("Invalid upload checkpoint"));
            }
            let n = (job.total - job.transferred).min(PART as u64) as usize;
            let mut file = fs::File::open(input)?;
            file.seek(SeekFrom::Start(job.transferred))?;
            let mut bytes = vec![0; n];
            file.read_exact(&mut bytes)?;
            let response = client()?
                .put(session)
                .header("Content-Length", n)
                .header(
                    "Content-Range",
                    format!(
                        "bytes {}-{}/{}",
                        job.transferred,
                        job.transferred + n as u64 - 1,
                        job.total
                    ),
                )
                .body(bytes)
                .send()
                .map_err(request_error)?;
            if response.status().as_u16() != 308 && !response.status().is_success() {
                checked(response)?;
                return Err(network("Upload interrupted"));
            }
            job.transferred += n as u64;
            Ok(())
        }
        _ => Err(invalid("Provider is not implemented")),
    }
}
pub fn download(s: &StorageSource, name: &str, target: &Path) -> Result<()> {
    if s.kind == "local" {
        let file = Path::new(&s.endpoint).join(name);
        let source = fs::canonicalize(&file)?;
        let root = fs::canonicalize(&s.endpoint)?;
        if !source.starts_with(root) {
            return Err(CoreError::AccessDenied);
        }
        fs::copy(source, target)?;
        return Ok(());
    }
    let response = match s.kind.as_str() {
        "s3" => crate::s3::get(s, name)?,
        "gateway" => gateway_request(
            s,
            "GET",
            &format!("/v1/rooms/{}/objects/{name}", s.room_id),
            &hex::encode(sha2::Sha256::digest([])),
        )?
        .send()
        .map_err(request_error)?,
        "webdav" => client()?
            .get(url(s, name)?)
            .bearer_auth(&s.token)
            .send()
            .map_err(request_error)?,
        "google_drive" => {
            let object = drive_id(s, name)?
                .ok_or_else(|| network("The original is not in this storage source"))?;
            client()?
                .get(format!(
                    "https://www.googleapis.com/drive/v3/files/{object}"
                ))
                .query(&[("alt", "media")])
                .bearer_auth(&s.token)
                .send()
                .map_err(request_error)?
        }
        "onedrive" => {
            let metadata = json_response(
                client()?
                    .get(format!(
                        "https://graph.microsoft.com/v1.0/me/drive/special/approot:/{name}"
                    ))
                    .bearer_auth(&s.token)
                    .send()
                    .map_err(request_error)?,
            )?;
            let url = metadata["@microsoft.graph.downloadUrl"]
                .as_str()
                .ok_or_else(|| network("Missing download location"))?;
            let parsed = Url::parse(url).map_err(|_| network("Invalid download location"))?;
            if parsed.scheme() != "https" {
                return Err(CoreError::Authentication);
            }
            client()?.get(url).send().map_err(request_error)?
        }
        _ => return Err(invalid("Provider is not implemented")),
    };
    let mut response = checked(response)?;
    let parent = target
        .parent()
        .ok_or_else(|| invalid("Missing destination directory"))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    response.copy_to(&mut temp).map_err(request_error)?;
    temp.as_file().sync_all()?;
    temp.persist(target).map_err(|e| CoreError::from(e.error))?;
    Ok(())
}
