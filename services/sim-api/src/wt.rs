use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use crc32fast::Hasher;
use serde::{Deserialize, Serialize};
use sim_mirror::{
    ControlError, MirrorControl, MirrorControlStatus, MirrorHandle, StateDelta, StateDigest, StateSnapshot,
};
use tokio::{select, sync::mpsc, task};
use tracing::{error, info, warn};
use wtransport::{
    Endpoint, Identity, ServerConfig,
    connection::Connection,
    endpoint::IncomingSession,
    error::{ConnectionError, StreamReadExactError},
    stream::{RecvStream, SendStream},
    tls::Sha256DigestFmt,
};

const DEFAULT_PORT: u16 = 4433;
const CERT_DIR: &str = ".cert";
const CERT_FILE: &str = "cert.pem";
const KEY_FILE: &str = "key.pem";

pub fn spawn_server(mirror: MirrorHandle, control: Arc<dyn MirrorControl + Send + Sync>) -> task::JoinHandle<()> {
    task::spawn(async move {
        if let Err(err) = run_server(mirror, control).await {
            error!(error = %err, "webtransport server terminated");
        }
    })
}

async fn run_server(mirror: MirrorHandle, control: Arc<dyn MirrorControl + Send + Sync>) -> Result<()> {
    let (cert_path, key_path) = resolve_certificate_paths()?;
    let identity = load_or_generate_identity(&cert_path, &key_path).await?;

    let port = env::var("SIM_WT_PORT").ok().and_then(|value| value.parse::<u16>().ok()).unwrap_or(DEFAULT_PORT);

    let certificate_hash = identity.certificate_chain().as_slice()[0].hash().fmt(Sha256DigestFmt::BytesArray);

    let config = ServerConfig::builder().with_bind_default(port).with_identity(identity).build();

    let server = Endpoint::server(config)?;

    info!(
        port,
        %certificate_hash,
        "webtransport server listening"
    );

    loop {
        let incoming: IncomingSession = server.accept().await;
        let mirror_clone = mirror.clone();
        let control_clone = control.clone();

        task::spawn(async move {
            match incoming.await {
                Ok(request) => {
                    let path = request.path().to_string();
                    match request.accept().await {
                        Ok(connection) => {
                            info!(%path, "accepted webtransport session");
                            if let Err(err) = handle_connection(connection, mirror_clone, control_clone).await {
                                warn!(error = %err, %path, "webtransport session closed");
                            }
                        }
                        Err(err) => {
                            warn!(error = %err, %path, "failed to accept webtransport connection");
                        }
                    }
                }
                Err(err) => {
                    warn!(error = %err, "failed to await incoming session");
                }
            }
        });
    }
}

async fn handle_connection(
    connection: Connection, mirror: MirrorHandle, control: Arc<dyn MirrorControl + Send + Sync>,
) -> Result<()> {
    let connection = Arc::new(connection);

    let mut updates_task = task::spawn(pump_updates(connection.clone(), mirror.clone()));
    let mut request_task = task::spawn(handle_incoming_streams(connection.clone(), mirror, control));

    select! {
        result = &mut updates_task => {
            request_task.abort();
            match result {
                Ok(inner) => inner?,
                Err(err) => return Err(anyhow!(err)),
            }
        }
        result = &mut request_task => {
            updates_task.abort();
            match result {
                Ok(inner) => inner?,
                Err(err) => return Err(anyhow!(err)),
            }
        }
    }

    Ok(())
}

async fn pump_updates(connection: Arc<Connection>, mirror: MirrorHandle) -> Result<()> {
    let mut send = connection.open_uni().await?.await?;
    let mut snapshots = subscribe_snapshots(mirror);
    let mut first = true;

    while let Some(snapshot) = snapshots.recv().await {
        let digest = snapshot.digest.as_ref();
        let heartbeat = heartbeat_frame(digest);

        if first {
            first = false;
            send_chunked_snapshot(&mut send, digest).await?;
            send_frame(&mut send, &heartbeat).await?;
            continue;
        }

        if let Some(delta) = snapshot.delta.as_ref() {
            send_frame(&mut send, &TransportFrame::Delta(delta.clone())).await?;
            send_frame(&mut send, &heartbeat).await?;
        } else {
            send_frame(&mut send, &heartbeat).await?;
        }
    }

    let _ = send.finish().await;
    Ok(())
}

async fn handle_incoming_streams(
    connection: Arc<Connection>, mirror: MirrorHandle, control: Arc<dyn MirrorControl + Send + Sync>,
) -> Result<()> {
    loop {
        match connection.accept_bi().await {
            Ok((send, recv)) => {
                let mirror_clone = mirror.clone();
                let control_clone = control.clone();
                task::spawn(async move {
                    if let Err(err) = handle_request_stream(send, recv, mirror_clone, control_clone).await {
                        warn!(error = %err, "webtransport request stream error");
                    }
                });
            }
            Err(err)
                if matches!(
                    err,
                    ConnectionError::LocallyClosed
                        | ConnectionError::TimedOut
                        | ConnectionError::ApplicationClosed(_)
                        | ConnectionError::ConnectionClosed(_)
                ) =>
            {
                return Ok(());
            }
            Err(err) => return Err(anyhow!(err)),
        }
    }
}

async fn handle_request_stream(
    mut send: SendStream, mut recv: RecvStream, mirror: MirrorHandle, control: Arc<dyn MirrorControl + Send + Sync>,
) -> Result<()> {
    while let Some(payload) = read_frame(&mut recv).await? {
        if payload.is_empty() {
            continue;
        }

        match rmp_serde::from_slice::<ClientRequest>(&payload) {
            Ok(ClientRequest::Ping { id }) => {
                let frame = TransportFrame::Ack { message: id.unwrap_or_else(|| "pong".into()) };
                send_frame(&mut send, &frame).await?;
            }
            Ok(ClientRequest::Snapshot) => {
                let snapshot = mirror.latest();
                send_chunked_snapshot(&mut send, snapshot.digest.as_ref()).await?;
                let heartbeat = heartbeat_frame(snapshot.digest.as_ref());
                send_frame(&mut send, &heartbeat).await?;
            }
            Ok(ClientRequest::Subscribe { topics }) => {
                let _ = topics;
                let frame = TransportFrame::Ack { message: "subscription acknowledged".into() };
                send_frame(&mut send, &frame).await?;
            }
            Ok(ClientRequest::Pause) => {
                let result = control.pause();
                respond_control_action(&mut send, &control, "pause", result).await?;
            }
            Ok(ClientRequest::Resume) => {
                let result = control.resume();
                respond_control_action(&mut send, &control, "resume", result).await?;
            }
            Ok(ClientRequest::Step) => {
                let result = control.step();
                respond_control_action(&mut send, &control, "step", result).await?;
            }
            Ok(ClientRequest::Reset) => {
                let result = control.reset();
                respond_control_action(&mut send, &control, "reset", result).await?;
            }
            Ok(ClientRequest::SetInterval { millis }) => match millis {
                Some(value) if value > 0 => {
                    let duration = Duration::from_millis(value);
                    let result = control.set_interval(duration);
                    respond_control_action(&mut send, &control, "set_interval", result).await?;
                }
                _ => {
                    let frame =
                        TransportFrame::Error { message: "set_interval requires a positive millis value".into() };
                    send_frame(&mut send, &frame).await?;
                }
            },
            Ok(ClientRequest::Status) => {
                let status = control.status();
                let frame = TransportFrame::ControlStatus(status);
                send_frame(&mut send, &frame).await?;
            }
            Ok(ClientRequest::Unknown) => {
                let frame = TransportFrame::Error { message: "unknown request".into() };
                send_frame(&mut send, &frame).await?;
            }
            Err(err) => {
                let frame = TransportFrame::Error { message: format!("decode error: {err}") };
                send_frame(&mut send, &frame).await?;
            }
        }
    }

    Ok(())
}

async fn respond_control_action(
    send: &mut SendStream, control: &Arc<dyn MirrorControl + Send + Sync>, action: &str,
    result: Result<(), ControlError>,
) -> Result<()> {
    match result {
        Ok(()) => {
            let status = control.status();
            let ack = TransportFrame::Ack { message: format!("{action} ok") };
            send_frame(send, &ack).await?;
            let status_frame = TransportFrame::ControlStatus(status);
            send_frame(send, &status_frame).await?;
        }
        Err(err) => {
            let frame = TransportFrame::Error { message: format!("{action} failed: {err}") };
            send_frame(send, &frame).await?;
        }
    }

    Ok(())
}

fn heartbeat_frame(digest: &StateDigest) -> TransportFrame {
    TransportFrame::Heartbeat { tick: digest.tick, generated_at_ms: digest.timings.generated_at.timestamp_millis() }
}

async fn send_chunked_snapshot(send: &mut SendStream, digest: &StateDigest) -> Result<()> {
    const CHUNK: usize = 64 * 1024;
    let bytes = rmp_serde::to_vec_named(digest)?;

    if bytes.len() <= CHUNK {
        let frame = TransportFrame::Snapshot(digest.clone());
        send_frame(send, &frame).await?;
        return Ok(());
    }

    let chunk_count = (bytes.len() + CHUNK - 1) / CHUNK;
    if chunk_count > u16::MAX as usize {
        return Err(anyhow!("snapshot exceeds maximum chunk count"));
    }

    let id = digest.tick;
    let total = chunk_count as u16;

    for (seq, slice) in bytes.chunks(CHUNK).enumerate() {
        let mut hasher = Hasher::new();
        hasher.update(slice);
        let crc32 = hasher.finalize();

        let frame = TransportFrame::SnapshotChunk { id, seq: seq as u16, total, crc32, bytes: slice.to_vec() };

        send_frame(send, &frame).await?;
    }

    Ok(())
}

async fn send_frame(stream: &mut SendStream, frame: &TransportFrame) -> Result<()> {
    let payload = rmp_serde::to_vec_named(frame)?;
    let len = (payload.len() as u32).to_be_bytes();
    stream.write_all(&len).await.map_err(|err| anyhow!(err))?;
    stream.write_all(&payload).await.map_err(|err| anyhow!(err))?;
    Ok(())
}

async fn read_frame(stream: &mut RecvStream) -> Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    match stream.read_exact(&mut len_buf).await {
        Ok(()) => {}
        Err(StreamReadExactError::FinishedEarly(_)) => return Ok(None),
        Err(StreamReadExactError::Read(err)) => {
            return match err {
                wtransport::error::StreamReadError::NotConnected => Ok(None),
                wtransport::error::StreamReadError::Reset(_) => Ok(None),
                wtransport::error::StreamReadError::QuicProto => Err(anyhow!(err)),
            };
        }
    }

    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 {
        return Ok(Some(Vec::new()));
    }

    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).await.map_err(|err| match err {
        StreamReadExactError::FinishedEarly(_) => anyhow!("stream finished early"),
        StreamReadExactError::Read(inner) => anyhow!(inner),
    })?;

    Ok(Some(payload))
}

fn subscribe_snapshots(mirror: MirrorHandle) -> mpsc::UnboundedReceiver<Arc<StateSnapshot>> {
    let rx = mirror.subscribe();
    let (tx, rx_async) = mpsc::unbounded_channel();

    task::spawn_blocking(move || {
        for snapshot in rx.iter() {
            if tx.send(snapshot).is_err() {
                break;
            }
        }
    });

    rx_async
}

fn resolve_certificate_paths() -> Result<(PathBuf, PathBuf)> {
    if let (Ok(cert), Ok(key)) = (env::var("SIM_WT_CERT_PATH"), env::var("SIM_WT_KEY_PATH")) {
        return Ok((PathBuf::from(cert), PathBuf::from(key)));
    }

    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CERT_DIR);
    fs::create_dir_all(&base)?;
    Ok((base.join(CERT_FILE), base.join(KEY_FILE)))
}

async fn load_or_generate_identity(cert_path: &Path, key_path: &Path) -> Result<Identity> {
    if cert_path.exists() && key_path.exists() {
        Identity::load_pemfiles(cert_path, key_path).await.context("failed to load WebTransport identity from disk")
    } else {
        let identity = Identity::self_signed(["localhost", "127.0.0.1", "::1"])?;
        identity
            .certificate_chain()
            .store_pemfile(cert_path)
            .await
            .context("failed to write self-signed WebTransport certificate")?;
        identity
            .private_key()
            .store_secret_pemfile(key_path)
            .await
            .context("failed to write self-signed WebTransport key")?;
        Ok(identity)
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum TransportFrame {
    Snapshot(StateDigest),
    SnapshotChunk { id: u32, seq: u16, total: u16, crc32: u32, bytes: Vec<u8> },
    Delta(StateDelta),
    ControlStatus(MirrorControlStatus),
    Ack { message: String },
    Error { message: String },
    Heartbeat { tick: u32, generated_at_ms: i64 },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientRequest {
    Ping {
        id: Option<String>,
    },
    Snapshot,
    Subscribe {
        topics: Option<Vec<String>>,
    },
    Pause,
    Resume,
    Step,
    Reset,
    SetInterval {
        millis: Option<u64>,
    },
    Status,
    #[serde(other)]
    Unknown,
}
