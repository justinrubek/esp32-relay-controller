use crate::error::{Error, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use bytes::{Bytes, BytesMut};
use ext::BytesMutWriteExt;
use flagset::{flags, FlagSet};
use std::io::Cursor;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

pub mod consts;
pub mod error;
mod ext;
mod fmt;

pub trait Encodable {
    /// Encode self to writer and return the number of bytes written
    /// # Errors
    /// - implementation specific
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize>;
}

pub trait Decodable: Sized {
    /// Decode self from reader
    /// # Errors
    /// - implementation specific
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self>;
}

pub trait Protocol: Encodable + Decodable {}
impl<T: Encodable + Decodable> Protocol for T {}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Tversion = 100,
    Rversion = 101,
    Tauth = 102,
    Rauth = 103,
    Tattach = 104,
    Rattach = 105,
    Rerror = 107,
    Tflush = 108,
    Rflush = 109,
    Twalk = 110,
    Rwalk = 111,
    Topen = 112,
    Ropen = 113,
    Tcreate = 114,
    Rcreate = 115,
    Tread = 116,
    Rread = 117,
    Twrite = 118,
    Rwrite = 119,
    Tclunk = 120,
    Rclunk = 121,
    Tremove = 122,
    Rremove = 123,
    Tstat = 124,
    Rstat = 125,
    Twstat = 126,
    Rwstat = 127,
}

impl MessageType {
    /// # Errors
    /// - the provided `value` is not a valid 9p message type
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            100 => Ok(MessageType::Tversion),
            101 => Ok(MessageType::Rversion),
            102 => Ok(MessageType::Tauth),
            103 => Ok(MessageType::Rauth),
            104 => Ok(MessageType::Tattach),
            105 => Ok(MessageType::Rattach),
            107 => Ok(MessageType::Rerror),
            108 => Ok(MessageType::Tflush),
            109 => Ok(MessageType::Rflush),
            110 => Ok(MessageType::Twalk),
            111 => Ok(MessageType::Rwalk),
            112 => Ok(MessageType::Topen),
            113 => Ok(MessageType::Ropen),
            114 => Ok(MessageType::Tcreate),
            115 => Ok(MessageType::Rcreate),
            116 => Ok(MessageType::Tread),
            117 => Ok(MessageType::Rread),
            118 => Ok(MessageType::Twrite),
            119 => Ok(MessageType::Rwrite),
            120 => Ok(MessageType::Tclunk),
            121 => Ok(MessageType::Rclunk),
            122 => Ok(MessageType::Tremove),
            123 => Ok(MessageType::Rremove),
            124 => Ok(MessageType::Tstat),
            125 => Ok(MessageType::Rstat),
            126 => Ok(MessageType::Twstat),
            127 => Ok(MessageType::Rwstat),
            _ => Err(Error::InvalidMessageType(value)),
        }
    }

    #[must_use]
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

flags! {
    #[repr(u32)]
    pub enum FileMode: u32 {
        Dir = 0x8000_0000,
        AppendOnly = 0x4000_0000,
        ExclAccess = 0x2000_0000,
        Mounted = 0x1000_0000,
        Auth = 0x0800_0000,

        Temporary = 0x0400_0000,
        OwnerRead = 0x0000_0100,
        OwnerWrite = 0x0000_0080,
        OwnerExec = 0x0000_0040,
        GroupRead = 0x0000_0020,
        GroupWrite = 0x0000_0010,
        GroupExec = 0x0000_0008,
        OtherRead = 0x0000_0004,
        OtherWrite = 0x0000_0002,
        OtherExec = 0x0000_0001,
        DontTouch = !0,
    }

    #[repr(u8)]
    pub enum OpenMode: u8 {
        Read = 0,
        Write = 1,
        ReadWrite = 2,
        Exec = 3,
        Trunc = 0x10, // 16
        RClose = 0x40, // 64
    }

    #[repr(u8)]
    pub enum QidType: u8 {
        Dir = 0x80,
        Append = 0x40,
        Exclusive = 0x20,
        Mount = 0x10,
        Auth = 0x08,
        Tmp = 0x04,
        File = 0x00,
        DontTouch = !0,
    }
}

impl FileMode {
    #[must_use]
    pub fn from_unix_perm(mode: u32, is_dir: bool) -> FlagSet<FileMode> {
        let mut flags = FlagSet::empty();

        if is_dir {
            flags |= FileMode::Dir;
        }

        if (mode & 0o400) != 0 {
            flags |= FileMode::OwnerRead;
        }
        if (mode & 0o200) != 0 {
            flags |= FileMode::OwnerWrite;
        }
        if (mode & 0o100) != 0 {
            flags |= FileMode::OwnerExec;
        }

        if (mode & 0o040) != 0 {
            flags |= FileMode::GroupRead;
        }
        if (mode & 0o020) != 0 {
            flags |= FileMode::GroupWrite;
        }
        if (mode & 0o010) != 0 {
            flags |= FileMode::GroupExec;
        }

        if (mode & 0o004) != 0 {
            flags |= FileMode::OtherRead;
        }
        if (mode & 0o002) != 0 {
            flags |= FileMode::OtherWrite;
        }
        if (mode & 0o001) != 0 {
            flags |= FileMode::OtherExec;
        }

        flags
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Qid {
    pub qtype: FlagSet<QidType>,
    pub version: u32,
    pub path: u64,
}

// `Message` wrapper that includes a tag
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedMessage {
    pub tag: u16,
    pub message: Message,
}

impl TaggedMessage {
    pub fn new(tag: u16, message: Message) -> Self {
        Self { tag, message }
    }

    pub fn message_type(&self) -> MessageType {
        self.message.message_type()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Tversion(Tversion),
    Rversion(Rversion),
    Tauth(Tauth),
    Rauth(Rauth),
    Tattach(Tattach),
    Rattach(Rattach),
    Rerror(Rerror),
    Tflush(Tflush),
    Rflush(Rflush),
    Twalk(Twalk),
    Rwalk(Rwalk),
    Topen(Topen),
    Ropen(Ropen),
    Tcreate(Tcreate),
    Rcreate(Rcreate),
    Tread(Tread),
    Rread(Rread),
    Twrite(Twrite),
    Rwrite(Rwrite),
    Tclunk(Tclunk),
    Rclunk(Rclunk),
    Tremove(Tremove),
    Rremove(Rremove),
    Tstat(Tstat),
    Rstat(Rstat),
    Twstat(Twstat),
    Rwstat(Rwstat),
}

impl Message {
    #[must_use]
    pub fn error(ename: String) -> Message {
        Message::Rerror(Rerror { ename })
    }

    pub fn message_type(&self) -> MessageType {
        match self {
            Message::Tversion(_) => MessageType::Tversion,
            Message::Rversion(_) => MessageType::Rversion,
            Message::Tauth(_) => MessageType::Tauth,
            Message::Rauth(_) => MessageType::Rauth,
            Message::Tattach(_) => MessageType::Tattach,
            Message::Rattach(_) => MessageType::Rattach,
            Message::Rerror(_) => MessageType::Rerror,
            Message::Tflush(_) => MessageType::Tflush,
            Message::Rflush(_) => MessageType::Rflush,
            Message::Twalk(_) => MessageType::Twalk,
            Message::Rwalk(_) => MessageType::Rwalk,
            Message::Topen(_) => MessageType::Topen,
            Message::Ropen(_) => MessageType::Ropen,
            Message::Tcreate(_) => MessageType::Tcreate,
            Message::Rcreate(_) => MessageType::Rcreate,
            Message::Tread(_) => MessageType::Tread,
            Message::Rread(_) => MessageType::Rread,
            Message::Twrite(_) => MessageType::Twrite,
            Message::Rwrite(_) => MessageType::Rwrite,
            Message::Tclunk(_) => MessageType::Tclunk,
            Message::Rclunk(_) => MessageType::Rclunk,
            Message::Tremove(_) => MessageType::Tremove,
            Message::Rremove(_) => MessageType::Rremove,
            Message::Tstat(_) => MessageType::Tstat,
            Message::Rstat(_) => MessageType::Rstat,
            Message::Twstat(_) => MessageType::Twstat,
            Message::Rwstat(_) => MessageType::Rwstat,
        }
    }

    pub fn to_tagged(self, tag: u16) -> TaggedMessage {
        TaggedMessage { tag, message: self }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tversion {
    pub msize: u32,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rversion {
    pub msize: u32,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tauth {
    pub afid: u32,
    pub uname: String,
    pub aname: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rauth {
    pub aqid: Qid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tattach {
    pub fid: u32,
    pub afid: u32,
    pub uname: String,
    pub aname: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rattach {
    pub qid: Qid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rerror {
    pub ename: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tflush {
    pub oldtag: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rflush;

#[derive(Debug, Clone, PartialEq)]
pub struct Twalk {
    pub fid: u32,
    pub newfid: u32,
    pub wnames: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rwalk {
    pub wqids: Vec<Qid>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Topen {
    pub fid: u32,
    pub mode: FlagSet<OpenMode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ropen {
    pub qid: Qid,
    pub iounit: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tcreate {
    pub fid: u32,
    pub name: String,
    pub perm: FlagSet<FileMode>,
    pub mode: FlagSet<OpenMode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rcreate {
    pub qid: Qid,
    pub iounit: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tread {
    pub fid: u32,
    pub offset: u64,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rread {
    pub data: Bytes,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Twrite {
    pub fid: u32,
    pub offset: u64,
    pub data: Bytes,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rwrite {
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tclunk {
    pub fid: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rclunk;

#[derive(Debug, Clone, PartialEq)]
pub struct Tremove {
    pub fid: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rremove;

#[derive(Debug, Clone, PartialEq)]
pub struct Tstat {
    pub fid: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rstat {
    pub stat: Stat,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Twstat {
    pub fid: u32,
    pub stat: Stat,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rwstat;

impl Encodable for u8 {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        w.write_u8(*self)?;
        Ok(1)
    }
}

impl Decodable for u8 {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(r.read_u8()?)
    }
}

impl Encodable for u16 {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        w.write_u16::<LittleEndian>(*self)?;
        Ok(2)
    }
}

impl Decodable for u16 {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(r.read_u16::<LittleEndian>()?)
    }
}

impl Encodable for u32 {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        w.write_u32::<LittleEndian>(*self)?;
        Ok(4)
    }
}

impl Decodable for u32 {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(r.read_u32::<LittleEndian>()?)
    }
}

impl Encodable for u64 {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        w.write_u64::<LittleEndian>(*self)?;
        Ok(8)
    }
}

impl Decodable for u64 {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(r.read_u64::<LittleEndian>()?)
    }
}

impl Encodable for String {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let bytes = self.as_bytes();
        if bytes.len() > u16::MAX as usize {
            return Err(Error::StringTooLong(bytes.len()));
        }

        let len = u16::try_from(bytes.len()).unwrap(); // safe due to check above
        w.write_u16::<LittleEndian>(len)?;
        w.write_all(bytes)?;

        Ok(2 + bytes.len())
    }
}

impl Decodable for String {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let len = r.read_u16::<LittleEndian>()? as usize;

        let mut string_bytes = vec![0u8; len];
        r.read_exact(&mut string_bytes)?;

        Ok(String::from_utf8(string_bytes)?)
    }
}

impl Encodable for Bytes {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let len = u32::try_from(self.len()).map_err(|_| Error::BytesTooLong(self.len()))?;

        w.write_u32::<LittleEndian>(len)?;
        w.write_all(self)?;

        Ok(4 + self.len())
    }
}

impl Decodable for Bytes {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let len = r.read_u32::<LittleEndian>()? as usize;

        let mut data = vec![0u8; len];
        r.read_exact(&mut data)?;

        Ok(Bytes::from(data))
    }
}

impl Encodable for FlagSet<OpenMode> {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.bits().encode(w)
    }
}

impl Decodable for FlagSet<OpenMode> {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let val = u8::decode(r)?;
        let f = FlagSet::<OpenMode>::new(val)?;
        Ok(f)
    }
}

impl Encodable for FlagSet<FileMode> {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.bits().encode(w)
    }
}

impl Decodable for FlagSet<FileMode> {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let val = u32::decode(r)?;
        let f = FlagSet::<FileMode>::new(val)?;
        Ok(f)
    }
}

impl Encodable for FlagSet<QidType> {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.bits().encode(w)
    }
}

impl Decodable for FlagSet<QidType> {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let val = u8::decode(r)?;
        let f = FlagSet::<QidType>::new(val)?;
        Ok(f)
    }
}

impl Encodable for Qid {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;

        bytes_written += self.qtype.encode(w)?;
        bytes_written += self.version.encode(w)?;
        bytes_written += self.path.encode(w)?;

        Ok(bytes_written)
    }
}

impl Decodable for Qid {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Qid {
            qtype: FlagSet::<QidType>::decode(r)?,
            version: u32::decode(r)?,
            path: u64::decode(r)?,
        })
    }
}

impl<T: Encodable> Encodable for Vec<T> {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        if self.len() > u16::MAX as usize {
            return Err(Error::VectorTooLong(self.len()));
        }

        let len = u16::try_from(self.len()).unwrap(); // safe due to check above
        let mut bytes_written = len.encode(w)?;

        for item in self {
            bytes_written += item.encode(w)?;
        }

        Ok(bytes_written)
    }
}

impl<T: Decodable> Decodable for Vec<T> {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let len = u16::decode(r)? as usize;
        let mut vec = Vec::with_capacity(len);

        for _ in 0..len {
            vec.push(T::decode(r)?);
        }

        Ok(vec)
    }
}

impl Encodable for () {
    fn encode<W: WriteBytesExt>(&self, _w: &mut W) -> Result<usize> {
        Ok(0)
    }
}

impl Decodable for () {
    fn decode<R: ReadBytesExt>(_r: &mut R) -> Result<Self> {
        Ok(())
    }
}

impl Encodable for Tversion {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;
        bytes_written += self.msize.encode(w)?;
        bytes_written += self.version.encode(w)?;
        Ok(bytes_written)
    }
}

impl Decodable for Tversion {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Tversion {
            msize: u32::decode(r)?,
            version: String::decode(r)?,
        })
    }
}

impl Encodable for Rversion {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;
        bytes_written += self.msize.encode(w)?;
        bytes_written += self.version.encode(w)?;
        Ok(bytes_written)
    }
}

impl Decodable for Rversion {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Rversion {
            msize: u32::decode(r)?,
            version: String::decode(r)?,
        })
    }
}

impl Encodable for Tauth {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;
        bytes_written += self.afid.encode(w)?;
        bytes_written += self.uname.encode(w)?;
        bytes_written += self.aname.encode(w)?;
        Ok(bytes_written)
    }
}

impl Decodable for Tauth {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Tauth {
            afid: u32::decode(r)?,
            uname: String::decode(r)?,
            aname: String::decode(r)?,
        })
    }
}

impl Encodable for Rauth {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.aqid.encode(w)
    }
}

impl Decodable for Rauth {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Rauth {
            aqid: Qid::decode(r)?,
        })
    }
}

impl Encodable for Tattach {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;
        bytes_written += self.fid.encode(w)?;
        bytes_written += self.afid.encode(w)?;
        bytes_written += self.uname.encode(w)?;
        bytes_written += self.aname.encode(w)?;
        Ok(bytes_written)
    }
}

impl Decodable for Tattach {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Tattach {
            fid: u32::decode(r)?,
            afid: u32::decode(r)?,
            uname: String::decode(r)?,
            aname: String::decode(r)?,
        })
    }
}

impl Encodable for Rattach {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.qid.encode(w)
    }
}

impl Decodable for Rattach {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Rattach {
            qid: Qid::decode(r)?,
        })
    }
}

impl Encodable for Rerror {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.ename.encode(w)
    }
}

impl Decodable for Rerror {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Rerror {
            ename: String::decode(r)?,
        })
    }
}

impl Encodable for Tflush {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.oldtag.encode(w)
    }
}

impl Decodable for Tflush {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Tflush {
            oldtag: u16::decode(r)?,
        })
    }
}

impl Encodable for Rflush {
    fn encode<W: WriteBytesExt>(&self, _w: &mut W) -> Result<usize> {
        Ok(0)
    }
}

impl Decodable for Rflush {
    fn decode<R: ReadBytesExt>(_r: &mut R) -> Result<Self> {
        Ok(Rflush)
    }
}

impl Encodable for Twalk {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;
        bytes_written += self.fid.encode(w)?;
        bytes_written += self.newfid.encode(w)?;
        bytes_written += self.wnames.encode(w)?;
        Ok(bytes_written)
    }
}

impl Decodable for Twalk {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Twalk {
            fid: u32::decode(r)?,
            newfid: u32::decode(r)?,
            wnames: Vec::<String>::decode(r)?,
        })
    }
}

impl Encodable for Rwalk {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.wqids.encode(w)
    }
}

impl Decodable for Rwalk {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Rwalk {
            wqids: Vec::<Qid>::decode(r)?,
        })
    }
}

impl Encodable for Topen {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;
        bytes_written += self.fid.encode(w)?;
        bytes_written += self.mode.encode(w)?;
        Ok(bytes_written)
    }
}

impl Decodable for Topen {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Topen {
            fid: u32::decode(r)?,
            mode: FlagSet::<OpenMode>::decode(r)?,
        })
    }
}

impl Encodable for Ropen {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;
        bytes_written += self.qid.encode(w)?;
        bytes_written += self.iounit.encode(w)?;
        Ok(bytes_written)
    }
}

impl Decodable for Ropen {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Ropen {
            qid: Qid::decode(r)?,
            iounit: u32::decode(r)?,
        })
    }
}

impl Encodable for Tcreate {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;
        bytes_written += self.fid.encode(w)?;
        bytes_written += self.name.encode(w)?;
        bytes_written += self.perm.encode(w)?;
        bytes_written += self.mode.encode(w)?;
        Ok(bytes_written)
    }
}

impl Decodable for Tcreate {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Tcreate {
            fid: u32::decode(r)?,
            name: String::decode(r)?,
            perm: FlagSet::<FileMode>::decode(r)?,
            mode: FlagSet::<OpenMode>::decode(r)?,
        })
    }
}

impl Encodable for Rcreate {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;
        bytes_written += self.qid.encode(w)?;
        bytes_written += self.iounit.encode(w)?;
        Ok(bytes_written)
    }
}

impl Decodable for Rcreate {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Rcreate {
            qid: Qid::decode(r)?,
            iounit: u32::decode(r)?,
        })
    }
}

impl Encodable for Tread {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;
        bytes_written += self.fid.encode(w)?;
        bytes_written += self.offset.encode(w)?;
        bytes_written += self.count.encode(w)?;
        Ok(bytes_written)
    }
}

impl Decodable for Tread {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Tread {
            fid: u32::decode(r)?,
            offset: u64::decode(r)?,
            count: u32::decode(r)?,
        })
    }
}

impl Encodable for Rread {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.data.encode(w)
    }
}

impl Decodable for Rread {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Rread {
            data: Bytes::decode(r)?,
        })
    }
}

impl Encodable for Twrite {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;
        bytes_written += self.fid.encode(w)?;
        bytes_written += self.offset.encode(w)?;
        bytes_written += self.data.encode(w)?;
        Ok(bytes_written)
    }
}

impl Decodable for Twrite {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Twrite {
            fid: u32::decode(r)?,
            offset: u64::decode(r)?,
            data: Bytes::decode(r)?,
        })
    }
}

impl Encodable for Rwrite {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.count.encode(w)
    }
}

impl Decodable for Rwrite {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Rwrite {
            count: u32::decode(r)?,
        })
    }
}

impl Encodable for Tclunk {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.fid.encode(w)
    }
}

impl Decodable for Tclunk {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Tclunk {
            fid: u32::decode(r)?,
        })
    }
}

impl Encodable for Rclunk {
    fn encode<W: WriteBytesExt>(&self, _w: &mut W) -> Result<usize> {
        Ok(0)
    }
}

impl Decodable for Rclunk {
    fn decode<R: ReadBytesExt>(_r: &mut R) -> Result<Self> {
        Ok(Rclunk)
    }
}

impl Encodable for Tremove {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.fid.encode(w)
    }
}

impl Decodable for Tremove {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Tremove {
            fid: u32::decode(r)?,
        })
    }
}

impl Encodable for Rremove {
    fn encode<W: WriteBytesExt>(&self, _w: &mut W) -> Result<usize> {
        Ok(0)
    }
}

impl Decodable for Rremove {
    fn decode<R: ReadBytesExt>(_r: &mut R) -> Result<Self> {
        Ok(Rremove)
    }
}

impl Encodable for Tstat {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        self.fid.encode(w)
    }
}

impl Decodable for Tstat {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        Ok(Tstat {
            fid: u32::decode(r)?,
        })
    }
}

impl Encodable for Rstat {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        // Calculate the stat size and encode it
        let mut temp_buf = Vec::new();
        let mut temp_writer = Cursor::new(&mut temp_buf);
        let stat_size = self.stat.encode(&mut temp_writer)?;

        let stat_size = u16::try_from(stat_size).map_err(|_| Error::StringTooLong(stat_size))?;

        let mut bytes_written = stat_size.encode(w)?;
        bytes_written += self.stat.encode(w)?;

        Ok(bytes_written)
    }
}

impl Decodable for Rstat {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let _stat_size = u16::decode(r)?;
        let stat = Stat::decode(r)?;
        Ok(Rstat { stat })
    }
}

impl Encodable for Twstat {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;
        bytes_written += self.fid.encode(w)?;

        let mut temp_buf = Vec::new();
        let mut temp_writer = Cursor::new(&mut temp_buf);
        let stat_size = self.stat.encode(&mut temp_writer)?;

        let stat_size = u16::try_from(stat_size).map_err(|_| Error::StringTooLong(stat_size))?;

        bytes_written += stat_size.encode(w)?;
        bytes_written += self.stat.encode(w)?;

        Ok(bytes_written)
    }
}

impl Decodable for Twstat {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let fid = u32::decode(r)?;
        let _stat_size = u16::decode(r)?;
        let stat = Stat::decode(r)?;
        Ok(Twstat { fid, stat })
    }
}

impl Encodable for Rwstat {
    fn encode<W: WriteBytesExt>(&self, _w: &mut W) -> Result<usize> {
        Ok(0)
    }
}

impl Decodable for Rwstat {
    fn decode<R: ReadBytesExt>(_r: &mut R) -> Result<Self> {
        Ok(Rwstat)
    }
}

impl Encodable for Message {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        match self {
            Message::Tversion(msg) => msg.encode(w),
            Message::Rversion(msg) => msg.encode(w),
            Message::Tauth(msg) => msg.encode(w),
            Message::Rauth(msg) => msg.encode(w),
            Message::Tattach(msg) => msg.encode(w),
            Message::Rattach(msg) => msg.encode(w),
            Message::Rerror(msg) => msg.encode(w),
            Message::Tflush(msg) => msg.encode(w),
            Message::Rflush(msg) => msg.encode(w),
            Message::Twalk(msg) => msg.encode(w),
            Message::Rwalk(msg) => msg.encode(w),
            Message::Topen(msg) => msg.encode(w),
            Message::Ropen(msg) => msg.encode(w),
            Message::Tcreate(msg) => msg.encode(w),
            Message::Rcreate(msg) => msg.encode(w),
            Message::Tread(msg) => msg.encode(w),
            Message::Rread(msg) => msg.encode(w),
            Message::Twrite(msg) => msg.encode(w),
            Message::Rwrite(msg) => msg.encode(w),
            Message::Tclunk(msg) => msg.encode(w),
            Message::Rclunk(msg) => msg.encode(w),
            Message::Tremove(msg) => msg.encode(w),
            Message::Rremove(msg) => msg.encode(w),
            Message::Tstat(msg) => msg.encode(w),
            Message::Rstat(msg) => msg.encode(w),
            Message::Twstat(msg) => msg.encode(w),
            Message::Rwstat(msg) => msg.encode(w),
        }
    }
}

impl Decodable for Message {
    fn decode<R: ReadBytesExt>(_r: &mut R) -> Result<Self> {
        Err(Error::Protocol(
            "Message::decode called directly".to_string(),
        ))
    }
}

impl Encodable for TaggedMessage {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        let mut bytes_written = 0;

        bytes_written += self.message.message_type().to_u8().encode(w)?;
        bytes_written += self.tag.encode(w)?;
        bytes_written += self.message.encode(w)?;

        Ok(bytes_written)
    }
}

impl Decodable for TaggedMessage {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let message_type = MessageType::from_u8(u8::decode(r)?)?;
        let tag = u16::decode(r)?;

        let message = match message_type {
            MessageType::Tversion => Message::Tversion(Tversion::decode(r)?),
            MessageType::Rversion => Message::Rversion(Rversion::decode(r)?),
            MessageType::Tauth => Message::Tauth(Tauth::decode(r)?),
            MessageType::Rauth => Message::Rauth(Rauth::decode(r)?),
            MessageType::Tattach => Message::Tattach(Tattach::decode(r)?),
            MessageType::Rattach => Message::Rattach(Rattach::decode(r)?),
            MessageType::Rerror => Message::Rerror(Rerror::decode(r)?),
            MessageType::Tflush => Message::Tflush(Tflush::decode(r)?),
            MessageType::Rflush => Message::Rflush(Rflush::decode(r)?),
            MessageType::Twalk => Message::Twalk(Twalk::decode(r)?),
            MessageType::Rwalk => Message::Rwalk(Rwalk::decode(r)?),
            MessageType::Topen => Message::Topen(Topen::decode(r)?),
            MessageType::Ropen => Message::Ropen(Ropen::decode(r)?),
            MessageType::Tcreate => Message::Tcreate(Tcreate::decode(r)?),
            MessageType::Rcreate => Message::Rcreate(Rcreate::decode(r)?),
            MessageType::Tread => Message::Tread(Tread::decode(r)?),
            MessageType::Rread => Message::Rread(Rread::decode(r)?),
            MessageType::Twrite => Message::Twrite(Twrite::decode(r)?),
            MessageType::Rwrite => Message::Rwrite(Rwrite::decode(r)?),
            MessageType::Tclunk => Message::Tclunk(Tclunk::decode(r)?),
            MessageType::Rclunk => Message::Rclunk(Rclunk::decode(r)?),
            MessageType::Tremove => Message::Tremove(Tremove::decode(r)?),
            MessageType::Rremove => Message::Rremove(Rremove::decode(r)?),
            MessageType::Tstat => Message::Tstat(Tstat::decode(r)?),
            MessageType::Rstat => Message::Rstat(Rstat::decode(r)?),
            MessageType::Twstat => Message::Twstat(Twstat::decode(r)?),
            MessageType::Rwstat => Message::Rwstat(Rwstat::decode(r)?),
        };

        Ok(TaggedMessage { tag, message })
    }
}

pub struct MessageCodec {
    length_codec: LengthDelimitedCodec,
}

impl MessageCodec {
    #[must_use]
    pub fn new() -> Self {
        Self {
            length_codec: LengthDelimitedCodec::builder()
                .little_endian()
                .length_field_length(4)
                .length_adjustment(-4) // don't include length field in payload
                .new_codec(),
        }
    }
}

impl Default for MessageCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for MessageCodec {
    type Item = TaggedMessage;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if let Some(frame) = self.length_codec.decode(src).map_err(Error::Io)? {
            let mut cursor = Cursor::new(frame.as_ref());
            let message = TaggedMessage::decode(&mut cursor)?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<TaggedMessage> for MessageCodec {
    type Error = Error;

    fn encode(&mut self, item: TaggedMessage, dst: &mut BytesMut) -> Result<()> {
        let mut payload = BytesMut::new();
        item.encode(&mut payload.write_adapter())?;
        self.length_codec
            .encode(payload.freeze(), dst)
            .map_err(Error::Io)?;
        Ok(())
    }
}

/// Represents a 9P stat structure as defined in the protocol
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stat {
    pub r#type: u16,
    pub dev: u32,
    pub qid: Qid,
    pub mode: FlagSet<FileMode>,
    pub atime: u32,
    pub mtime: u32,
    pub length: u64,
    pub name: String,
    pub uid: String,
    pub gid: String,
    pub muid: String,
}

impl Stat {
    /// Create a new Stat with "don't touch" values for all fields
    /// This is useful for creating wstat messages that only modify specific fields
    #[must_use]
    pub fn new_dont_touch() -> Self {
        Stat {
            r#type: u16::MAX,
            dev: u32::MAX,
            qid: Qid {
                qtype: QidType::DontTouch.into(),
                version: u32::MAX,
                path: u64::MAX,
            },
            mode: FileMode::DontTouch.into(),
            atime: u32::MAX,
            mtime: u32::MAX,
            length: u64::MAX,
            name: String::new(), // empty string = don't touch
            uid: String::new(),
            gid: String::new(),
            muid: String::new(),
        }
    }

    /// Check if a field has a "don't touch" value
    #[must_use]
    pub fn is_dont_touch_u16(val: u16) -> bool {
        val == u16::MAX
    }

    #[must_use]
    pub fn is_dont_touch_u32(val: u32) -> bool {
        val == u32::MAX
    }

    #[must_use]
    pub fn is_dont_touch_u64(val: u64) -> bool {
        val == u64::MAX
    }

    #[must_use]
    pub fn is_dont_touch_string(val: &str) -> bool {
        val.is_empty()
    }
}

impl Encodable for Stat {
    fn encode<W: WriteBytesExt>(&self, w: &mut W) -> Result<usize> {
        // using a temporary buffer to calculate the size
        let mut temp_buf = Vec::new();
        let mut temp_writer = Cursor::new(&mut temp_buf);

        // encode all fields except the size to the temporary buffer
        self.r#type.encode(&mut temp_writer)?;
        self.dev.encode(&mut temp_writer)?;
        self.qid.encode(&mut temp_writer)?;
        self.mode.encode(&mut temp_writer)?;
        self.atime.encode(&mut temp_writer)?;
        self.mtime.encode(&mut temp_writer)?;
        self.length.encode(&mut temp_writer)?;
        self.name.encode(&mut temp_writer)?;
        self.uid.encode(&mut temp_writer)?;
        self.gid.encode(&mut temp_writer)?;
        self.muid.encode(&mut temp_writer)?;

        let total_size =
            u16::try_from(temp_buf.len()).map_err(|_| Error::StringTooLong(temp_buf.len()))?;

        let mut bytes_written = total_size.encode(w)?;
        w.write_all(&temp_buf)?;
        bytes_written += temp_buf.len();

        Ok(bytes_written)
    }
}

impl Decodable for Stat {
    fn decode<R: ReadBytesExt>(r: &mut R) -> Result<Self> {
        let stat_size = u16::decode(r)? as usize;

        let mut stat_data = vec![0u8; stat_size];
        r.read_exact(&mut stat_data)?;
        let mut stat_cursor = Cursor::new(&stat_data[..]);

        let r#type = u16::decode(&mut stat_cursor)?;
        let dev = u32::decode(&mut stat_cursor)?;
        let qid = Qid::decode(&mut stat_cursor)?;
        let mode = FlagSet::<FileMode>::decode(&mut stat_cursor)?;
        let atime = u32::decode(&mut stat_cursor)?;
        let mtime = u32::decode(&mut stat_cursor)?;
        let length = u64::decode(&mut stat_cursor)?;

        // String fields
        let name = String::decode(&mut stat_cursor)?;
        let uid = String::decode(&mut stat_cursor)?;
        let gid = String::decode(&mut stat_cursor)?;
        let muid = String::decode(&mut stat_cursor)?;

        Ok(Stat {
            r#type,
            dev,
            qid,
            mode,
            atime,
            mtime,
            length,
            name,
            uid,
            gid,
            muid,
        })
    }
}

// #[cfg(test)]
// mod tests;
