use crate::error::{Error, Result};
use embedded_svc::ota::OtaUpdate;
use esp_idf_hal::io::Write;
use esp_idf_svc::{
    ota::{EspOta, EspOtaUpdate, SlotState},
    timer::{EspTimerService, Task},
};
use futures::{SinkExt, StreamExt};
use log::info;
use std::{str::FromStr, time::Duration};
use stowage_proto::{
    consts::P9_NOFID, Decodable, FileMode, Message, MessageCodec, OpenMode, QidType, Stat,
    TaggedMessage, Tattach, Tauth, Tclunk, Tcreate, Topen, Tread, Tstat, Tversion, Twalk, Twrite,
    Twstat,
};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{Decoder, Framed};

type Connection = Framed<TcpStream, MessageCodec>;

// OTA update handler over 9p protocol
pub struct OtaHandler {
    addr: String,
    path: String,
    timer: EspTimerService<Task>,
}

impl OtaHandler {
    pub async fn new(addr: String, path: String, timer: EspTimerService<Task>) -> Result<Self> {
        Ok(Self { addr, path, timer })
    }

    /// Launches a new task that continually checks for firmware updates.
    /// Only returns in case of an error.
    /// # Errors
    /// - IO failure
    pub async fn run(&mut self) -> Result<()> {
        let mut timer = self.timer.timer_async()?;

        let mut ota = EspOta::new()?;
        loop {
            timer.after(Duration::from_secs(16)).await?;

            match self.check_update(&mut ota).await {
                Ok(Some(version)) => self.perform_update(&mut ota, &version).await?,
                Ok(None) => {
                    info!("firmware already up to date");
                    false
                }
                Err(e) => {
                    info!("update check failed: {e:?}");
                    false
                }
            };
        }
    }

    pub async fn check_update(&mut self, ota: &mut EspOta) -> Result<Option<String>> {
        let mut version_buf = Vec::new();
        let upstream_version = cat_file(
            &self.addr,
            &format!("{}/version", self.path),
            &mut version_buf,
        )
        .await?;
        let upstream_version = String::from_utf8_lossy(&version_buf);
        let upstream_version = upstream_version.trim();

        let current_version = get_running_version(&ota)?;
        let current_version = current_version.to_string();

        match current_version.eq(upstream_version) {
            true => Ok(None),
            false => {
                info!("update {current_version} -> {upstream_version}");
                Ok(Some(upstream_version.to_string()))
            }
        }
    }

    pub async fn perform_update(&mut self, ota: &mut EspOta, version: &str) -> Result<bool> {
        info!("initiating update");
        let mut update = ota.initiate_update()?;

        match self.download_update(&mut update, version).await {
            Ok(_) => {
                info!("update complete, rebooting");
                update.complete()?;
                esp_idf_svc::hal::reset::restart();
                Ok(true)
            }
            Err(err) => {
                info!("update aborted: {err:?}");
                update.abort()?;
                Ok(false)
            }
        }
    }

    pub async fn download_update(
        &mut self,
        update: &mut EspOtaUpdate<'_>,
        version: &str,
    ) -> Result<bool> {
        let firmware_path = format!("{}/files/{}", self.path, version);
        info!("downloading {firmware_path}");
        cat_file(&self.addr, &firmware_path, update).await?;
        Ok(true)
    }
}

fn get_running_version(ota: &EspOta) -> Result<heapless::String<24>> {
    Ok(ota
        .get_running_slot()?
        .firmware
        .ok_or(Error::FirmwareInfoMissing)?
        .version)
}

async fn cat_file<W: Write>(addr: &str, path: &str, writer: &mut W) -> Result<()> {
    let stream = TcpStream::connect(addr).await?;
    let mut conn = Framed::new(stream, MessageCodec::new());
    let tag: u16 = 1;
    let msize = perform_handshake(&mut conn, tag).await?;

    let mut root_fid = 2;
    let components = parse_path_components(&path);

    if components.is_empty() {
        return Err(Error::Other(format!("cat: {}: Is a directory", path)));
    }

    let walk_success = walk_to_path(&mut conn, tag, root_fid, root_fid + 1, &path).await?;
    if !walk_success {
        return Err(Error::Other(format!("file not found: {}", path)));
    }
    root_fid += 1;

    // Open file for reading
    let open_msg = Topen {
        fid: root_fid,
        mode: OpenMode::Read.into(),
    };
    send_message(
        &mut conn,
        TaggedMessage {
            message: Message::Topen(open_msg),
            tag,
        },
    )
    .await?;

    let response = receive_message(&mut conn).await?;
    match response.message {
        Message::Ropen(ropen) => {
            if ropen.qid.qtype.contains(QidType::Dir) {
                return Err(Error::Other(format!("cat: {}: Is a directory", path)));
            }
        }
        Message::Rerror(err) => {
            return Err(Error::Other(format!("failed to open file: {}", err.ename)));
        }
        _ => return Err(Error::Other("unexpected response to Topen".into())),
    }

    read_file(&mut conn, tag, root_fid, msize, writer).await?;

    cleanup_fid(&mut conn, tag, root_fid).await?;

    Ok(())
}

fn parse_path_components(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
        .collect()
}

async fn read_file<W: Write>(
    conn: &mut Connection,
    tag: u16,
    fid: u32,
    msize: u32,
    writer: &mut W,
) -> Result<()> {
    let protocol_overhead = 100;
    let max_count = if msize > protocol_overhead {
        msize - protocol_overhead
    } else {
        4096
    };

    let mut offset: u64 = 0;

    loop {
        let tread = TaggedMessage::new(
            tag,
            Message::Tread(Tread {
                fid,
                offset,
                count: max_count,
            }),
        );

        send_message(conn, tread).await?;
        let response = receive_message(conn).await?;

        match response.message {
            Message::Rread(rread) => {
                if rread.data.is_empty() {
                    break; // end of file
                }

                let l = rread.data.len() as u64;
                writer
                    .write_all(&rread.data)
                    .map_err(|_| Error::EspUpdateError)?;
                offset += l;
            }
            Message::Rerror(err) => {
                return Err(Error::Other(format!("Failed to read file: {}", err.ename)));
            }
            _ => return Err(Error::Other("Unexpected response to Tread".into())),
        }
    }

    Ok(())
}

async fn cleanup_fid(conn: &mut Connection, tag: u16, fid: u32) -> Result<()> {
    let clunk_msg = Tclunk { fid };
    let tagged = TaggedMessage {
        message: Message::Tclunk(clunk_msg),
        tag,
    };

    send_message(conn, tagged).await?;

    // handle response but don't fail on clunk errors
    if let Ok(response) = receive_message(conn).await {
        match response.message {
            Message::Rclunk(_) => {}
            Message::Rerror(err) => {
                eprintln!("warning: Failed to clunk fid {}: {}", fid, err.ename);
            }
            _ => {
                eprintln!("warning: Unexpected response to Tclunk for fid {fid}");
            }
        }
    }

    Ok(())
}

async fn perform_handshake(conn: &mut Connection, tag: u16) -> Result<u32> {
    let msize = perform_version_negotiation(conn).await?;
    perform_authentication(conn, tag).await?;
    attach_to_filesystem(conn, tag).await?;
    Ok(msize)
}

async fn perform_version_negotiation(conn: &mut Connection) -> Result<u32> {
    let version_tag = 0xFFFF;
    let mut msize = 8192;

    let version_msg = Message::Tversion(Tversion {
        msize,
        version: String::from("9P2000"),
    });
    let tagged = TaggedMessage {
        message: version_msg,
        tag: version_tag,
    };

    send_message(conn, tagged).await?;

    let response = receive_message(conn).await?;
    match response.message {
        Message::Rversion(rversion) => {
            if rversion.version != "9P2000" {
                return Err(Error::Other(format!(
                    "server doesn't support 9P2000, got {}",
                    rversion.version
                )));
            }
            msize = std::cmp::min(msize, rversion.msize);
            println!(
                "negotiated version: {} with msize: {}",
                rversion.version, msize
            );
            Ok(msize)
        }
        Message::Rerror(err) => Err(Error::Other(format!(
            "version negotiation failed: {}",
            err.ename
        ))),
        _ => Err(Error::Other("unexpected response to Tversion".into())),
    }
}

async fn perform_authentication(conn: &mut Connection, tag: u16) -> Result<()> {
    let afid = 1;
    let auth_msg = Tauth {
        afid,
        uname: String::from("nobody"),
        aname: String::new(),
    };
    let tagged = TaggedMessage {
        message: Message::Tauth(auth_msg),
        tag,
    };

    send_message(conn, tagged).await?;

    let response = receive_message(conn).await?;
    match response.message {
        Message::Rauth(_) => Err(Error::Other(
            "authentication required but not supported by this client".into(),
        )),
        Message::Rerror(_) => {
            // expected when auth is not required
            Ok(())
        }
        _ => Err(Error::Other("unexpected response to Tauth".into())),
    }
}

async fn attach_to_filesystem(conn: &mut Connection, tag: u16) -> Result<()> {
    let root_fid = 2;
    let attach_msg = Tattach {
        fid: root_fid,
        afid: P9_NOFID,
        uname: String::from("nobody"),
        aname: String::new(),
    };
    let tagged = TaggedMessage {
        message: Message::Tattach(attach_msg),
        tag,
    };

    send_message(conn, tagged).await?;

    let response = receive_message(conn).await?;
    match response.message {
        Message::Rattach(_) => Ok(()),
        Message::Rerror(err) => Err(Error::Other(format!(
            "Failed to attach to filesystem: {}",
            err.ename
        ))),
        _ => Err(Error::Other("Unexpected response to Tattach".into())),
    }
}

async fn send_message(conn: &mut Connection, message: TaggedMessage) -> Result<()> {
    conn.send(message).await.map_err(Error::from)
}

async fn receive_message(conn: &mut Connection) -> Result<TaggedMessage> {
    match conn.next().await {
        Some(Ok(msg)) => Ok(msg),
        Some(Err(e)) => Err(Error::from(e)),
        None => Err(Error::Other("Connection closed".into())),
    }
}

async fn walk_to_path(
    conn: &mut Connection,
    tag: u16,
    base_fid: u32,
    new_fid: u32,
    path: &str,
) -> Result<bool> {
    let components = parse_path_components(path);

    if components.is_empty() {
        return Ok(true); // root path
    }

    let walk_msg = Twalk {
        fid: base_fid,
        newfid: new_fid,
        wnames: components.clone(),
    };
    let tagged = TaggedMessage {
        message: Message::Twalk(walk_msg),
        tag,
    };

    send_message(conn, tagged).await?;

    let response = receive_message(conn).await?;
    match response.message {
        Message::Rwalk(rwalk) => Ok(rwalk.wqids.len() == components.len()),
        Message::Rerror(_) => Ok(false),
        _ => Err(Error::Other("Unexpected response to Twalk".into())),
    }
}
