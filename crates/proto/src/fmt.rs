use flagset::FlagSet;

use crate::{FileMode, OpenMode, QidType};

use super::{
    Message, Qid, Rattach, Rauth, Rclunk, Rcreate, Rerror, Rflush, Ropen, Rread, Rremove, Rstat,
    Rversion, Rwalk, Rwrite, Rwstat, Stat, TaggedMessage, Tattach, Tauth, Tclunk, Tcreate, Tflush,
    Topen, Tread, Tremove, Tstat, Tversion, Twalk, Twrite, Twstat,
};
use std::fmt;

impl fmt::Display for Qid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut chars = String::new();

        let flagset = self.qtype;
        if flagset.contains(QidType::Dir) {
            chars.push('d');
        }
        if flagset.contains(QidType::Append) {
            chars.push('a');
        }
        if flagset.contains(QidType::Exclusive) {
            chars.push('l');
        }
        if flagset.contains(QidType::Mount) {
            chars.push('m');
        }
        if flagset.contains(QidType::Auth) {
            chars.push('A');
        }
        if flagset.contains(QidType::Tmp) {
            chars.push('t');
        }

        // If no flags are set, show a space for regular file
        if chars.is_empty() {
            chars.push(' ');
        }
        write!(f, "{:016x} {} {chars}", self.path, self.version)
    }
}

fn format_data(data: &[u8]) -> String {
    if data.is_empty() {
        return "''".to_string();
    }

    let is_text = data.iter().all(|&b| {
        b.is_ascii()
            && (b.is_ascii_graphic()
                || b.is_ascii_whitespace()
                || b == b'\n'
                || b == b'\r'
                || b == b'\t')
    });

    if is_text {
        // format as quoted string
        format!("'{}'", String::from_utf8_lossy(data))
    } else if data.len() <= 64 {
        // short binary data as hex
        let hex_bytes: Vec<String> = data.iter().map(|b| format!("{b:02x}")).collect();

        let mut result = String::new();
        for (i, chunk) in hex_bytes.chunks(8).enumerate() {
            if i > 0 {
                result.push(' ');
            }
            result.push_str(&chunk.join(""));
        }
        result
    } else {
        // long binary data, first 64 bytes
        let preview_data = &data[..64];
        let hex_bytes: Vec<String> = preview_data.iter().map(|b| format!("{b:02x}")).collect();

        let mut result = String::new();
        for (i, chunk) in hex_bytes.chunks(8).enumerate() {
            if i > 0 {
                result.push(' ');
            }
            result.push_str(&chunk.join(""));
        }
        format!("{} [+{} more bytes]", result, data.len() - 64)
    }
}

fn format_fid(fid: u32) -> String {
    if fid == 0xFFFF_FFFF {
        "-1".to_string()
    } else {
        fid.to_string()
    }
}

fn format_perm(perm: FlagSet<FileMode>) -> String {
    let mut result = String::new();

    if perm.contains(FileMode::Dir) {
        result.push('d');
    } else {
        result.push('-');
    }

    result.push(if perm.contains(FileMode::OwnerRead) {
        'r'
    } else {
        '-'
    });
    result.push(if perm.contains(FileMode::OwnerWrite) {
        'w'
    } else {
        '-'
    });
    result.push(if perm.contains(FileMode::OwnerExec) {
        'x'
    } else {
        '-'
    });

    result.push(if perm.contains(FileMode::GroupRead) {
        'r'
    } else {
        '-'
    });
    result.push(if perm.contains(FileMode::GroupWrite) {
        'w'
    } else {
        '-'
    });
    result.push(if perm.contains(FileMode::GroupExec) {
        'x'
    } else {
        '-'
    });

    result.push(if perm.contains(FileMode::OtherRead) {
        'r'
    } else {
        '-'
    });
    result.push(if perm.contains(FileMode::OtherWrite) {
        'w'
    } else {
        '-'
    });
    result.push(if perm.contains(FileMode::OtherExec) {
        'x'
    } else {
        '-'
    });

    result
}

fn format_mode(mode: FlagSet<OpenMode>) -> String {
    if mode.is_empty() {
        return "0".to_string();
    }

    if mode == OpenMode::Read {
        return "0".to_string();
    } else if mode == OpenMode::Write {
        return "1".to_string();
    } else if mode == OpenMode::ReadWrite {
        return "2".to_string();
    } else if mode == OpenMode::Exec {
        return "3".to_string();
    } else if mode == OpenMode::Trunc {
        return "16".to_string();
    } else if mode == OpenMode::RClose {
        return "64".to_string();
    } else if mode == (OpenMode::Trunc | OpenMode::Write) {
        return "17".to_string();
    }

    format!("{mode:?}")
}

impl fmt::Display for Tversion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "msize {} version '{}'", self.msize, self.version)
    }
}

impl fmt::Display for Rversion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "msize {} version '{}'", self.msize, self.version)
    }
}

impl fmt::Display for Tauth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "fid {} afid {} uname {} aname {}",
            self.afid,
            format_fid(self.afid),
            self.uname,
            self.aname
        )
    }
}

impl fmt::Display for Rauth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "qid ({})", self.aqid)
    }
}

impl fmt::Display for Tattach {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "fid {} afid {} uname {} aname {}",
            self.fid,
            format_fid(self.afid),
            self.uname,
            self.aname
        )
    }
}

impl fmt::Display for Rattach {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "qid ({})", self.qid)
    }
}

impl fmt::Display for Rerror {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ename {}", self.ename)
    }
}

impl fmt::Display for Tflush {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "oldtag {}", self.oldtag)
    }
}

impl fmt::Display for Rflush {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl fmt::Display for Twalk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "fid {} newfid {} nwname {}",
            self.fid,
            self.newfid,
            self.wnames.len()
        )?;

        // Add walk names if present
        for (i, name) in self.wnames.iter().enumerate() {
            write!(f, " {i}:{name}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Rwalk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "nwqid {}", self.wqids.len())?;

        // Add qids if present
        for (i, qid) in self.wqids.iter().enumerate() {
            write!(f, " {i}:({qid})")?;
        }
        Ok(())
    }
}

impl fmt::Display for Topen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fid {} mode {}", self.fid, format_mode(self.mode))
    }
}

impl fmt::Display for Ropen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "qid ({}) iounit {}", self.qid, self.iounit)
    }
}

impl fmt::Display for Tcreate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "fid {} name {} perm {} mode {}",
            self.fid,
            self.name,
            format_perm(self.perm),
            format_mode(self.mode)
        )
    }
}

impl fmt::Display for Rcreate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "qid ({}) iounit {}", self.qid, self.iounit)
    }
}

impl fmt::Display for Tread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "fid {} offset {} count {}",
            self.fid, self.offset, self.count
        )
    }
}

impl fmt::Display for Rread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "count {} {}", self.data.len(), format_data(&self.data))
    }
}

impl fmt::Display for Twrite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "fid {} offset {} count {} {}",
            self.fid,
            self.offset,
            self.data.len(),
            format_data(&self.data)
        )
    }
}

impl fmt::Display for Rwrite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "count {}", self.count)
    }
}

impl fmt::Display for Tclunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fid {}", self.fid)
    }
}

impl fmt::Display for Rclunk {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl fmt::Display for Tremove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fid {}", self.fid)
    }
}

impl fmt::Display for Rremove {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl fmt::Display for Tstat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fid {}", self.fid)
    }
}

impl fmt::Display for Rstat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, " {}", self.stat)
    }
}

impl fmt::Display for Twstat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fid {} {}", self.fid, self.stat)
    }
}

impl fmt::Display for Rwstat {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Tversion(msg) => write!(f, "Tversion {msg}"),
            Message::Rversion(msg) => write!(f, "Rversion {msg}"),
            Message::Tauth(msg) => write!(f, "Tauth {msg}"),
            Message::Rauth(msg) => write!(f, "Rauth {msg}"),
            Message::Tattach(msg) => write!(f, "Tattach {msg}"),
            Message::Rattach(msg) => write!(f, "Rattach {msg}"),
            Message::Rerror(msg) => write!(f, "Rerror {msg}"),
            Message::Tflush(msg) => write!(f, "Tflush {msg}"),
            Message::Rflush(msg) => write!(f, "Rflush {msg}"),
            Message::Twalk(msg) => write!(f, "Twalk {msg}"),
            Message::Rwalk(msg) => write!(f, "Rwalk {msg}"),
            Message::Topen(msg) => write!(f, "Topen {msg}"),
            Message::Ropen(msg) => write!(f, "Ropen {msg}"),
            Message::Tcreate(msg) => write!(f, "Tcreate {msg}"),
            Message::Rcreate(msg) => write!(f, "Rcreate {msg}"),
            Message::Tread(msg) => write!(f, "Tread {msg}"),
            Message::Rread(msg) => write!(f, "Rread {msg}"),
            Message::Twrite(msg) => write!(f, "Twrite {msg}"),
            Message::Rwrite(msg) => write!(f, "Rwrite {msg}"),
            Message::Tclunk(msg) => write!(f, "Tclunk {msg}"),
            Message::Rclunk(msg) => write!(f, "Rclunk {msg}"),
            Message::Tremove(msg) => write!(f, "Tremove {msg}"),
            Message::Rremove(msg) => write!(f, "Rremove {msg}"),
            Message::Tstat(msg) => write!(f, "Tstat {msg}"),
            Message::Rstat(msg) => write!(f, "Rstat {msg}"),
            Message::Twstat(msg) => write!(f, "Twstat {msg}"),
            Message::Rwstat(msg) => write!(f, "Rwstat {msg}"),
        }
    }
}

impl fmt::Display for TaggedMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.message.to_string();
        let mut parts = s.splitn(2, ' ');
        let msg_name = parts.next().unwrap_or("");
        let msg_details = parts.next().unwrap_or("");

        write!(f, "{} tag {}", msg_name, self.tag)?;

        if !msg_details.is_empty() {
            write!(f, " {msg_details}")?;
        }

        Ok(())
    }
}

impl fmt::Display for Stat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let type_str = if Stat::is_dont_touch_u16(self.r#type) {
            String::new()
        } else {
            format!("{}", self.r#type)
        };

        let dev_str = if Stat::is_dont_touch_u32(self.dev) {
            "-1".to_string()
        } else {
            format!("{}", self.dev)
        };

        let qid_str = if Stat::is_dont_touch_u32(self.qid.version)
            && Stat::is_dont_touch_u64(self.qid.path)
            && self.qid.qtype == QidType::DontTouch
        {
            "(ffffffffffffffff 18446744073709551615 dalmA)".to_string()
        } else {
            format!("({})", self.qid)
        };

        // handle mode - output in octal for readability and compatibility with u9fs
        let mode_str = format!("{:o}", self.mode.bits());

        // times
        let atime_str = if Stat::is_dont_touch_u32(self.atime) {
            String::new()
        } else {
            format!("{}", self.atime)
        };

        let mtime_str = if Stat::is_dont_touch_u32(self.mtime) {
            String::new()
        } else {
            format!("{}", self.mtime)
        };

        // length
        let length_str = if Stat::is_dont_touch_u64(self.length) {
            "-1".to_string()
        } else {
            format!("{}", self.length)
        };

        write!(
            f,
            "stat '{}' '{}' '{}' '{}' q {} m {} at {} mt {} l {} t {} d {}",
            self.name,
            self.uid,
            self.gid,
            self.muid,
            qid_str,
            mode_str,
            atime_str,
            mtime_str,
            length_str,
            type_str,
            dev_str
        )
    }
}
