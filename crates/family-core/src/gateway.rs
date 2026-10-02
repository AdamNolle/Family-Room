//! Optional encrypted transport service. It never receives a Room decryption key.
use crate::{
    access,
    model::{AccessChange, Grant, Role, StorageSource},
    providers, signing,
};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::{get, put},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use futures_util::StreamExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

type HttpError = (StatusCode, &'static str);
#[derive(Clone)]
struct Service {
    root: PathBuf,
    authorities: Arc<Mutex<BTreeMap<String, String>>>,
    access: Arc<Mutex<BTreeMap<String, Vec<AccessChange>>>>,
    sources: Arc<BTreeMap<String, StorageSource>>,
}
fn failure() -> HttpError {
    (StatusCode::FORBIDDEN, "Invalid Room authorization")
}
fn io_error(_: impl std::fmt::Display) -> HttpError {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "Storage source unavailable",
    )
}
fn safe(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
        && s != "."
        && s != ".."
}
fn header<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, HttpError> {
    headers
        .get(name)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(failure)
}
fn authorize(
    service: &Service,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    room: &str,
    write: bool,
    register: bool,
) -> Result<(Grant, String), HttpError> {
    let owner = header(headers, "x-family-owner")?;
    let bytes = URL_SAFE_NO_PAD
        .decode(header(headers, "x-family-grant")?)
        .map_err(|_| failure())?;
    let grant: Grant = serde_json::from_slice(&bytes).map_err(|_| failure())?;
    if grant.room_id != room || signing::verify_grant(&grant, owner).is_err() {
        return Err(failure());
    }
    if grant.role == Role::Owner && grant.device_id != owner {
        return Err(failure());
    }
    if write && grant.role == Role::Viewer {
        return Err(failure());
    }
    let time = header(headers, "x-family-time")?
        .parse::<u64>()
        .map_err(|_| failure())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io_error)?
        .as_secs();
    if now.abs_diff(time) > 300 {
        return Err(failure());
    }
    let digest = header(headers, "x-family-sha256")?;
    if digest.len() != 64 {
        return Err(failure());
    }
    let message = format!("{method}\n{path}\n{digest}\n{time}");
    signing::verify(
        &grant.device_id,
        message.as_bytes(),
        header(headers, "x-family-proof")?,
    )
    .map_err(|_| failure())?;
    let authorities = service.authorities.lock().map_err(io_error)?;
    match authorities.get(room) {
        Some(known) if known == owner => {}
        None if register && grant.role == Role::Owner => {}
        _ => return Err(failure()),
    }
    check_access(
        service,
        headers,
        &grant,
        path.ends_with("/access") || register,
    )?;
    Ok((grant, owner.into()))
}

fn check_access(
    service: &Service,
    headers: &HeaderMap,
    grant: &Grant,
    policy_route: bool,
) -> Result<(), HttpError> {
    if policy_route {
        return Ok(());
    }
    let access = service.access.lock().map_err(io_error)?;
    if let Some(policy) = access.get(&grant.room_id).and_then(|chain| chain.last()) {
        let revision = headers
            .get("x-family-access-revision")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        if revision != policy.revision
            || !policy.members.get(&grant.device_id).is_some_and(|current| {
                current.role == grant.role && current.signature == grant.signature
            })
        {
            return Err(failure());
        }
    }
    Ok(())
}
async fn access_read(
    State(service): State<Service>,
    Path(room): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, HttpError> {
    if !safe(&room) {
        return Err(failure());
    }
    let path = format!("/v1/rooms/{room}/access");
    authorize(&service, &headers, "GET", &path, &room, false, false)?;
    // Removed devices may retrieve their owner-signed removal without gaining
    // access to any media or catalog ciphertext.
    let policies = service.access.lock().map_err(io_error)?;
    Ok(Json(
        json!({"changes":policies.get(&room).cloned().unwrap_or_default()}),
    ))
}
async fn access_update(
    State(service): State<Service>,
    Path(room): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<Value>, HttpError> {
    if !safe(&room) {
        return Err(failure());
    }
    let path = format!("/v1/rooms/{room}/access");
    let (grant, owner) = authorize(&service, &headers, "PUT", &path, &room, true, false)?;
    if grant.role != Role::Owner
        || grant.device_id != owner
        || hex::encode(Sha256::digest(&body)) != header(&headers, "x-family-sha256")?
    {
        return Err(failure());
    }
    let incoming: Vec<AccessChange> = serde_json::from_slice(&body).map_err(|_| failure())?;
    let root = service.root.clone();
    let policies = service.access.clone();
    tokio::task::spawn_blocking(move || {
        let mut policies = policies.lock().map_err(io_error)?;
        let existing = policies.get(&room).cloned().unwrap_or_default();
        if !access::verify_extension(&existing, &incoming, &room, &owner).map_err(|_| failure())? {
            if incoming.len() < existing.len() {
                return Err(failure());
            }
            return Ok::<_, HttpError>(());
        }
        // Persist first. A failed disk write must not claim server revocation.
        let mut updated = policies.clone();
        updated.insert(room, incoming);
        crate::engine::atomic(
            &root.join("access.json"),
            &serde_json::to_vec(&updated).map_err(io_error)?,
        )
        .map_err(io_error)?;
        *policies = updated;
        Ok(())
    })
    .await
    .map_err(io_error)??;
    Ok(Json(json!({"updated":true})))
}

async fn register(
    State(service): State<Service>,
    Path(room): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, HttpError> {
    if !safe(&room) {
        return Err(failure());
    }
    let path = format!("/v1/rooms/{room}");
    let (_, owner) = authorize(&service, &headers, "PUT", &path, &room, true, true)?;
    let root = service.root.clone();
    let registry = service.authorities.clone();
    tokio::task::spawn_blocking(move || {
        let mut authorities = registry.lock().map_err(io_error)?;
        if let Some(existing) = authorities.get(&room) {
            if existing != &owner {
                return Err(failure());
            }
        }
        authorities.insert(room, owner);
        let bytes = serde_json::to_vec(&*authorities).map_err(io_error)?;
        crate::engine::atomic(&root.join("authorities.json"), &bytes).map_err(io_error)
    })
    .await
    .map_err(io_error)??;
    Ok(Json(json!({"registered":true})))
}
async fn list(
    State(service): State<Service>,
    Path(room): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, HttpError> {
    let path = format!("/v1/rooms/{room}/objects");
    authorize(&service, &headers, "GET", &path, &room, false, false)?;
    let room_id = room.clone();
    let names = tokio::task::spawn_blocking(move || {
        if let Some(source) = service.sources.get(&room) {
            return providers::list(source).map_err(io_error);
        }
        let path = service.root.join("objects").join(&room);
        std::fs::create_dir_all(&path).map_err(io_error)?;
        let mut names = vec![];
        for entry in std::fs::read_dir(path).map_err(io_error)? {
            let e = entry.map_err(io_error)?;
            if e.file_type().map_err(io_error)?.is_file() {
                if let Some(name) = e.file_name().to_str() {
                    if safe(name) {
                        names.push(name.into());
                    }
                }
            }
        }
        Ok(names)
    })
    .await
    .map_err(io_error)??;
    Ok(Json(json!({"room_id":room_id,"objects":names})))
}
async fn upload(
    State(service): State<Service>,
    Path((room, name)): Path<(String, String)>,
    headers: HeaderMap,
    body: Body,
) -> Result<Json<Value>, HttpError> {
    if !safe(&room)
        || !safe(&name)
        || !name.starts_with(&format!("{room}-"))
        || !(name.ends_with(".frblob") || name.ends_with(".frindex"))
    {
        return Err(failure());
    }
    let path = format!("/v1/rooms/{room}/objects/{name}");
    let (grant, _) = authorize(
        &service,
        &headers,
        "PUT",
        &path,
        &room,
        name.ends_with(".frblob"),
        false,
    )?;
    if name.ends_with(".frindex") && name != format!("{room}-{}.frindex", grant.device_id) {
        return Err(failure());
    }
    let expected = header(&headers, "x-family-sha256")?.to_string();
    let directory = service.root.join("objects").join(&room);
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(io_error)?;
    let temporary = tempfile::NamedTempFile::new_in(&directory).map_err(io_error)?;
    let mut file = tokio::fs::File::create(temporary.path())
        .await
        .map_err(io_error)?;
    let mut stream = body.into_data_stream();
    let mut hash = Sha256::new();
    let mut count = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(io_error)?;
        count += chunk.len() as u64;
        if count > 64 * 1024 * 1024 * 1024 {
            return Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                "Object exceeds gateway limit",
            ));
        }
        hash.update(&chunk);
        file.write_all(&chunk).await.map_err(io_error)?;
    }
    file.sync_all().await.map_err(io_error)?;
    drop(file);
    if hex::encode(hash.finalize()) != expected {
        return Err((StatusCode::BAD_REQUEST, "Ciphertext checksum mismatch"));
    }
    check_access(&service, &headers, &grant, false)?;
    if let Some(source) = service.sources.get(&room).cloned() {
        tokio::task::spawn_blocking(move || {
            let mut job = crate::model::TransferJob {
                id: crate::engine::id(),
                room_id: room,
                asset_id: String::new(),
                source_id: source.id.clone(),
                state: "transferring".into(),
                transferred: 0,
                total: count,
                error: None,
                session: None,
            };
            while job.transferred < count {
                providers::upload_step(&source, &name, temporary.path(), &mut job)
                    .map_err(io_error)?;
            }
            Ok::<(), HttpError>(())
        })
        .await
        .map_err(io_error)??;
    } else {
        temporary.persist(directory.join(name)).map_err(io_error)?;
    }
    Ok(Json(json!({"stored":true,"bytes":count})))
}
async fn download(
    State(service): State<Service>,
    Path((room, name)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, HttpError> {
    if !safe(&room) || !safe(&name) || !name.starts_with(&format!("{room}-")) {
        return Err(failure());
    }
    let path = format!("/v1/rooms/{room}/objects/{name}");
    authorize(&service, &headers, "GET", &path, &room, false, false)?;
    let local = service.root.join("objects").join(&room).join(&name);
    if let Some(source) = service.sources.get(&room).cloned() {
        let local = local.clone();
        tokio::task::spawn_blocking(move || {
            if let Some(parent) = local.parent() {
                std::fs::create_dir_all(parent).map_err(io_error)?;
            }
            providers::download(&source, &name, &local).map_err(io_error)
        })
        .await
        .map_err(io_error)??;
    }
    let file = tokio::fs::File::open(local)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Object unavailable"))?;
    Response::builder()
        .header("Content-Type", "application/octet-stream")
        .body(Body::from_stream(ReaderStream::new(file)))
        .map_err(io_error)
}
/// Constructs a loopback/reverse-proxy service. Provider config is optional and contains
/// provider credentials only, never client catalog or Room decryption keys.
pub fn router(root: PathBuf, provider_config: Option<PathBuf>) -> Result<Router, crate::CoreError> {
    std::fs::create_dir_all(&root)?;
    let authorities: BTreeMap<String, String> = match std::fs::read(root.join("authorities.json")) {
        Ok(data) => serde_json::from_slice(&data)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
        Err(e) => return Err(e.into()),
    };
    let policies: BTreeMap<String, Vec<AccessChange>> =
        match std::fs::read(root.join("access.json")) {
            Ok(data) => serde_json::from_slice(&data)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(e) => return Err(e.into()),
        };
    for (room, chain) in &policies {
        let owner = authorities
            .get(room)
            .ok_or(crate::CoreError::Authentication)?;
        access::verify_chain(chain, room, owner)?;
    }
    let sources = if let Some(path) = provider_config {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if std::fs::metadata(&path)?.permissions().mode() & 0o077 != 0 {
                return Err(crate::error::invalid(
                    "Provider configuration must be readable only by its owner (chmod 600)",
                ));
            }
        }
        let sources: BTreeMap<String, StorageSource> =
            serde_json::from_slice(&std::fs::read(path)?)?;
        for (room, source) in &sources {
            if room != &source.room_id || source.kind == "gateway" {
                return Err(crate::error::invalid("Invalid gateway provider mapping"));
            }
            providers::validate(source)?;
        }
        sources
    } else {
        BTreeMap::new()
    };
    let service = Service {
        root,
        authorities: Arc::new(Mutex::new(authorities)),
        access: Arc::new(Mutex::new(policies)),
        sources: Arc::new(sources),
    };
    Ok(Router::new()
        .route(
            "/health",
            get(|| async { "Family Room encrypted gateway v1" }),
        )
        .route("/v1/rooms/{room}", put(register))
        .route(
            "/v1/rooms/{room}/access",
            get(access_read).put(access_update),
        )
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024))
        .route("/v1/rooms/{room}/objects", get(list))
        .route("/v1/rooms/{room}/objects/{name}", put(upload).get(download))
        .with_state(service))
}
