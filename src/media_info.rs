use std::env;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MediaInfoSource {
    #[default]
    BuiltIn,
    MediaInfoCli,
    Ffprobe,
}

impl MediaInfoSource {
    pub fn label(self) -> &'static str {
        match self {
            MediaInfoSource::BuiltIn => "Built-in",
            MediaInfoSource::MediaInfoCli => "MediaInfo CLI",
            MediaInfoSource::Ffprobe => "ffprobe",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VideoMediaInfo {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_secs: Option<f64>,
    pub fps: Option<f64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_channels: Option<u32>,
    pub bitrate_bps: Option<u64>,
    pub recorded_date: Option<String>,
    pub hdr: Option<String>,
    pub container: Option<String>,
    pub source: MediaInfoSource,
}

impl VideoMediaInfo {
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        match (self.width, self.height) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some((w, h)),
            _ => None,
        }
    }

    pub fn resolution_label(&self) -> Option<String> {
        let (w, h) = self.dimensions()?;
        let long_side = w.max(h);
        let short_side = w.min(h);
        let label = if long_side >= 7600 || short_side >= 4300 {
            "4320p".to_string()
        } else if long_side >= 3800 || short_side >= 2100 {
            "2160p".to_string()
        } else if long_side >= 2500 || short_side >= 1400 {
            "1440p".to_string()
        } else if long_side >= 1900 || short_side >= 1060 {
            "1080p".to_string()
        } else if long_side >= 1260 || short_side >= 700 {
            "720p".to_string()
        } else if short_side >= 560 {
            "576p".to_string()
        } else if short_side >= 460 {
            "480p".to_string()
        } else if short_side >= 340 {
            "360p".to_string()
        } else {
            format!("{}p", short_side)
        };
        Some(label)
    }

    pub fn fps_tag(&self) -> Option<String> {
        let fps = self.fps?;
        if !fps.is_finite() || !(1.0..=240.0).contains(&fps) {
            return None;
        }
        if (fps - fps.round()).abs() < 0.05 {
            Some(format!("{}fps", fps.round() as u32))
        } else {
            let mut formatted = format!("{:.2}", fps);
            while formatted.ends_with('0') {
                formatted.pop();
            }
            if formatted.ends_with('.') {
                formatted.pop();
            }
            Some(format!("{}fps", formatted))
        }
    }

    pub fn duration_tag(&self) -> Option<String> {
        let secs = self.duration_secs?;
        if !secs.is_finite() || secs < 0.5 {
            return None;
        }
        let total = secs.round() as u64;
        let h = total / 3600;
        let m = (total % 3600) / 60;
        let s = total % 60;
        if h > 0 {
            Some(format!("{:02}h{:02}m{:02}s", h, m, s))
        } else if m > 0 {
            Some(format!("{:02}m{:02}s", m, s))
        } else {
            Some(format!("{}s", s))
        }
    }

    pub fn duration_display(&self) -> Option<String> {
        let secs = self.duration_secs?;
        if !secs.is_finite() || secs < 0.5 {
            return None;
        }
        let total = secs.round() as u64;
        let h = total / 3600;
        let m = (total % 3600) / 60;
        let s = total % 60;
        if h > 0 {
            Some(format!("{:02}:{:02}:{:02}", h, m, s))
        } else {
            Some(format!("{:02}:{:02}", m, s))
        }
    }

    pub fn audio_channels_tag(&self) -> Option<String> {
        let ch = self.audio_channels?;
        match ch {
            0 => None,
            1 => Some("1.0".to_string()),
            2 => Some("2.0".to_string()),
            3 => Some("2.1".to_string()),
            4 => Some("4.0".to_string()),
            5 => Some("5.0".to_string()),
            6 => Some("5.1".to_string()),
            8 => Some("7.1".to_string()),
            n => Some(format!("{}ch", n)),
        }
    }

    pub fn bitrate_tag(&self) -> Option<String> {
        let bps = self.bitrate_bps?;
        if bps == 0 {
            return None;
        }
        if bps >= 10_000_000 {
            Some(format!("{}Mbps", (bps as f64 / 1_000_000.0).round() as u64))
        } else if bps >= 1_000_000 {
            let mbps = bps as f64 / 1_000_000.0;
            let s = format!("{:.1}", mbps);
            let trimmed = s.strip_suffix(".0").unwrap_or(&s);
            Some(format!("{}Mbps", trimmed))
        } else if bps >= 1_000 {
            Some(format!("{}kbps", (bps as f64 / 1_000.0).round() as u64))
        } else {
            Some(format!("{}bps", bps))
        }
    }

    pub fn has_any_metadata(&self) -> bool {
        self.dimensions().is_some()
            || self.duration_secs.is_some()
            || self.fps.is_some()
            || self.video_codec.is_some()
            || self.audio_codec.is_some()
            || self.bitrate_bps.is_some()
            || self.recorded_date.is_some()
    }

    pub fn is_core_complete(&self) -> bool {
        self.dimensions().is_some()
            && self.video_codec.is_some()
            && self.duration_secs.is_some()
    }

    fn merge_missing_from(&mut self, other: &VideoMediaInfo) {
        let mut used_other = false;
        if self.width.is_none() || self.height.is_none() {
            if other.dimensions().is_some() {
                self.width = other.width;
                self.height = other.height;
                used_other = true;
            }
        }
        if self.duration_secs.is_none() && other.duration_secs.is_some() {
            self.duration_secs = other.duration_secs;
            used_other = true;
        }
        if self.fps.is_none() && other.fps.is_some() {
            self.fps = other.fps;
            used_other = true;
        }
        if self.video_codec.is_none() && other.video_codec.is_some() {
            self.video_codec = other.video_codec.clone();
            used_other = true;
        }
        if self.audio_codec.is_none() && other.audio_codec.is_some() {
            self.audio_codec = other.audio_codec.clone();
            used_other = true;
        }
        if self.audio_channels.is_none() && other.audio_channels.is_some() {
            self.audio_channels = other.audio_channels;
            used_other = true;
        }
        if self.bitrate_bps.is_none() && other.bitrate_bps.is_some() {
            self.bitrate_bps = other.bitrate_bps;
            used_other = true;
        }
        if self.recorded_date.is_none() && other.recorded_date.is_some() {
            self.recorded_date = other.recorded_date.clone();
            used_other = true;
        }
        if self.hdr.is_none() && other.hdr.is_some() {
            self.hdr = other.hdr.clone();
            used_other = true;
        }
        if self.container.is_none() && other.container.is_some() {
            self.container = other.container.clone();
            used_other = true;
        }
        if used_other {
            self.source = other.source;
        }
    }
}

pub fn is_video_ext(ext: &str) -> bool {
    matches!(
        ext,
        "mp4"
            | "m4v"
            | "mov"
            | "mkv"
            | "webm"
            | "avi"
            | "3gp"
            | "3g2"
            | "flv"
            | "wmv"
            | "asf"
            | "ts"
            | "m2ts"
            | "mts"
            | "mpg"
            | "mpeg"
            | "vob"
            | "ogv"
            | "rmvb"
    )
}

pub fn normalize_video_codec(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let low = trimmed.to_lowercase();
    let mapped = if low.contains("avc1")
        || low.contains("avc3")
        || low == "avc"
        || low.contains("h264")
        || low.contains("h.264")
        || low.contains("x264")
        || low == "v_mpeg4/iso/avc"
    {
        "H264"
    } else if low.contains("hvc1")
        || low.contains("hev1")
        || low.contains("hevc")
        || low.contains("h265")
        || low.contains("h.265")
        || low.contains("x265")
        || low.contains("dvh1")
        || low.contains("dvhe")
        || low == "v_mpegh/iso/hevc"
    {
        "HEVC"
    } else if low.starts_with("av01") || low == "av1" || low == "v_av1" || low.contains("av1") {
        "AV1"
    } else if low.starts_with("vp09") || low == "vp90" || low.contains("vp9") || low == "v_vp9" {
        "VP9"
    } else if low.starts_with("vp08") || low == "vp80" || low.contains("vp8") || low == "v_vp8" {
        "VP8"
    } else if low.starts_with("apch")
        || low.starts_with("apcn")
        || low.starts_with("apcs")
        || low.starts_with("apco")
        || low.starts_with("ap4h")
        || low.starts_with("ap4x")
        || low.contains("prores")
        || low == "v_prores"
    {
        "ProRes"
    } else if low.contains("xvid") {
        "Xvid"
    } else if low.contains("divx") || low == "dx50" || low == "div3" {
        "DivX"
    } else if low.contains("mp4v")
        || low == "mpeg4"
        || low.contains("mpeg-4 visual")
        || low.starts_with("v_mpeg4/iso/sp")
        || low.starts_with("v_mpeg4/iso/asp")
        || low.starts_with("v_mpeg4/iso/ap")
    {
        "MPEG4"
    } else if low.contains("mpeg2")
        || low.contains("mpeg-2")
        || low == "mpeg video"
        || low == "v_mpeg2"
        || low == "mp2v"
    {
        "MPEG2"
    } else if low.contains("mjpeg") || low == "mjpg" || low == "mjpa" || low == "mjpb" {
        "MJPEG"
    } else if low.contains("vc-1") || low == "vc1" || low == "wvc1" {
        "VC1"
    } else if low.starts_with("wmv") {
        "WMV"
    } else if low.contains("theora") || low == "v_theora" {
        "Theora"
    } else if low.contains("vp6") {
        "VP6"
    } else if low == "h263" || low == "s263" {
        "H263"
    } else {
        let cleaned: String = trimmed
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '+')
            .collect();
        return if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        };
    };
    Some(mapped.to_string())
}

pub fn normalize_audio_codec(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let low = trimmed.to_lowercase();
    let mapped = if low.contains("ec-3")
        || low.contains("eac3")
        || low.contains("e-ac-3")
        || low.contains("a_eac3")
        || low == "ddp"
    {
        "EAC3"
    } else if low.contains("ac-3") || low.contains("ac3") || low == "a_ac3" {
        "AC3"
    } else if low.contains("mlpa")
        || low.contains("truehd")
        || low.contains("mlp fba")
        || low == "a_truehd"
    {
        "TrueHD"
    } else if low.contains("dts-hd")
        || low.contains("dtsh")
        || low.contains("dtsl")
        || low.contains("dts xll")
    {
        "DTS-HD"
    } else if low == "dts" || low.starts_with("dts") || low.starts_with("a_dts") {
        "DTS"
    } else if low == "mp4a" || low.contains("aac") || low.starts_with("a_aac") {
        "AAC"
    } else if low.contains("opus") || low == "a_opus" {
        "Opus"
    } else if low.contains("flac") || low == "a_flac" {
        "FLAC"
    } else if low.contains("alac") || low == "a_alac" {
        "ALAC"
    } else if low.contains("vorbis") || low == "a_vorbis" {
        "Vorbis"
    } else if low == "mp3"
        || low == ".mp3"
        || low == "a_mpeg/l3"
        || low == "mpeg audio"
        || low.contains("mp3")
    {
        "MP3"
    } else if low.starts_with("pcm")
        || low.starts_with("a_pcm")
        || low == "lpcm"
        || low == "sowt"
        || low == "twos"
        || low == "in24"
        || low == "in32"
        || low == "fl32"
    {
        "PCM"
    } else if low.starts_with("wma") {
        "WMA"
    } else {
        let cleaned: String = trimmed
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        return if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        };
    };
    Some(mapped.to_string())
}

fn container_from_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "mp4" => Some("MP4"),
        "m4v" => Some("M4V"),
        "mov" => Some("MOV"),
        "mkv" => Some("MKV"),
        "webm" => Some("WebM"),
        "avi" => Some("AVI"),
        "3gp" => Some("3GP"),
        "3g2" => Some("3G2"),
        "flv" => Some("FLV"),
        "wmv" => Some("WMV"),
        "asf" => Some("ASF"),
        "ts" => Some("TS"),
        "m2ts" | "mts" => Some("M2TS"),
        "mpg" | "mpeg" => Some("MPEG"),
        "vob" => Some("VOB"),
        "ogv" => Some("OGV"),
        "rmvb" => Some("RMVB"),
        _ => None,
    }
}

pub fn extract_video_info(
    path: &Path,
    file_size: u64,
    allow_external: bool,
) -> Option<VideoMediaInfo> {
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    let mut info = VideoMediaInfo {
        container: container_from_ext(&ext).map(|s| s.to_string()),
        source: MediaInfoSource::BuiltIn,
        ..Default::default()
    };

    if let Ok(mut file) = fs::File::open(path) {
        let mut magic = [0u8; 12];
        let n = file.read(&mut magic).unwrap_or(0);
        let _ = file.seek(SeekFrom::Start(0));

        if matches!(ext.as_str(), "mp4" | "m4v" | "mov" | "3gp" | "3g2")
            || (n >= 8 && &magic[4..8] == b"ftyp")
        {
            if let Some(parsed) = parse_mp4_reader(&mut file, file_size) {
                info.merge_missing_from(&parsed);
                info.source = MediaInfoSource::BuiltIn;
            }
        } else if matches!(ext.as_str(), "mkv" | "webm")
            || (n >= 4 && magic[0..4] == [0x1A, 0x45, 0xDF, 0xA3])
        {
            if let Some(parsed) = parse_mkv_reader(&mut file, file_size) {
                info.merge_missing_from(&parsed);
                info.source = MediaInfoSource::BuiltIn;
            }
        } else if ext == "avi" || (n >= 12 && &magic[0..4] == b"RIFF" && &magic[8..12] == b"AVI ") {
            if let Some(parsed) = parse_avi_reader(&mut file, file_size) {
                info.merge_missing_from(&parsed);
                info.source = MediaInfoSource::BuiltIn;
            }
        } else if ext == "flv" || (n >= 3 && &magic[0..3] == b"FLV") {
            if let Some(parsed) = parse_flv_reader(&mut file, file_size) {
                info.merge_missing_from(&parsed);
                info.source = MediaInfoSource::BuiltIn;
            }
        }
    }

    if allow_external && !info.is_core_complete() {
        if let Some(ext_info) = query_external_mediainfo(path) {
            info.merge_missing_from(&ext_info);
        }
    }

    if info.has_any_metadata() || info.container.is_some() {
        Some(info)
    } else {
        None
    }
}

// ============================================================================
// ISO BMFF (MP4 / MOV / M4V / 3GP) Parser
// ============================================================================

fn read_u32_be(b: &[u8], off: usize) -> Option<u32> {
    let slice = b.get(off..off + 4)?;
    Some(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u16_be(b: &[u8], off: usize) -> Option<u16> {
    let slice = b.get(off..off + 2)?;
    Some(u16::from_be_bytes([slice[0], slice[1]]))
}

fn read_u64_be(b: &[u8], off: usize) -> Option<u64> {
    let slice = b.get(off..off + 8)?;
    Some(u64::from_be_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

fn format_unix_date_checked(unix_secs: u64) -> Option<String> {
    let (y, m, d, _, _) = crate::decompose_timestamp(unix_secs);
    if (1980..=2100).contains(&y) {
        Some(format!("{:04}-{:02}-{:02}", y, m, d))
    } else {
        None
    }
}

fn next_mp4_box<'a>(data: &'a [u8], pos: &mut usize) -> Option<([u8; 4], &'a [u8])> {
    if *pos + 8 > data.len() {
        return None;
    }
    let size32 = u32::from_be_bytes(data[*pos..*pos + 4].try_into().ok()?);
    let kind: [u8; 4] = data[*pos + 4..*pos + 8].try_into().ok()?;
    let (header_len, total_size) = if size32 == 1 {
        if *pos + 16 > data.len() {
            return None;
        }
        let size64 = u64::from_be_bytes(data[*pos + 8..*pos + 16].try_into().ok()?);
        let sz = usize::try_from(size64).ok()?;
        (16usize, sz)
    } else if size32 == 0 {
        (8usize, data.len() - *pos)
    } else {
        (8usize, size32 as usize)
    };
    if total_size < header_len || *pos + total_size > data.len() {
        return None;
    }
    let payload = &data[*pos + header_len..*pos + total_size];
    *pos += total_size;
    Some((kind, payload))
}

pub fn parse_mp4_reader<R: Read + Seek>(reader: &mut R, file_size_hint: u64) -> Option<VideoMediaInfo> {
    let stream_len = reader.seek(SeekFrom::End(0)).ok()?;
    let file_len = if stream_len > 0 { stream_len } else { file_size_hint };
    if file_len < 8 {
        return None;
    }
    reader.seek(SeekFrom::Start(0)).ok()?;

    let mut info = VideoMediaInfo {
        source: MediaInfoSource::BuiltIn,
        ..Default::default()
    };

    let mut pos: u64 = 0;
    let mut box_count: usize = 0;
    while pos + 8 <= file_len && box_count < 256 {
        box_count += 1;
        reader.seek(SeekFrom::Start(pos)).ok()?;
        let mut hdr = [0u8; 8];
        if reader.read_exact(&mut hdr).is_err() {
            break;
        }
        let size32 = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        let kind: [u8; 4] = [hdr[4], hdr[5], hdr[6], hdr[7]];

        let (header_len, box_size) = if size32 == 1 {
            let mut ext_buf = [0u8; 8];
            if reader.read_exact(&mut ext_buf).is_err() {
                break;
            }
            (16u64, u64::from_be_bytes(ext_buf))
        } else if size32 == 0 {
            (8u64, file_len.saturating_sub(pos))
        } else {
            (8u64, size32 as u64)
        };

        if box_size < header_len || pos.saturating_add(box_size) > file_len {
            break;
        }

        let payload_len = box_size - header_len;
        if &kind == b"ftyp" && info.container.is_none() && (4..=1024).contains(&payload_len) {
            let mut ftyp = vec![0u8; payload_len as usize];
            if reader.read_exact(&mut ftyp).is_ok() && ftyp.len() >= 4 {
                info.container = match &ftyp[0..4] {
                    b"qt  " => Some("MOV".to_string()),
                    b"M4V " => Some("M4V".to_string()),
                    b"3gp4" | b"3gp5" | b"3gp6" => Some("3GP".to_string()),
                    b"3g2a" => Some("3G2".to_string()),
                    _ => Some("MP4".to_string()),
                };
            }
        } else if &kind == b"moov" {
            const MAX_MOOV: u64 = 16 * 1024 * 1024;
            if payload_len > MAX_MOOV {
                break;
            }
            let mut moov = vec![0u8; payload_len as usize];
            if reader.read_exact(&mut moov).is_ok() {
                parse_mp4_moov(&moov, &mut info);
            }
            break;
        }

        pos = pos.saturating_add(box_size);
    }

    if let Some(dur) = info.duration_secs {
        if info.bitrate_bps.is_none() && file_len > 0 && dur > 0.1 {
            info.bitrate_bps = Some(((file_len as f64 * 8.0) / dur) as u64);
        }
    }

    if info.has_any_metadata() {
        Some(info)
    } else {
        None
    }
}

fn parse_mp4_moov(moov: &[u8], info: &mut VideoMediaInfo) {
    const MP4_EPOCH_OFFSET: u64 = 2_082_844_800;
    let mut pos = 0;
    while let Some((kind, payload)) = next_mp4_box(moov, &mut pos) {
        if &kind == b"mvhd" && !payload.is_empty() {
            let ver = payload[0];
            let (creation_time, timescale, duration) = if ver == 0 && payload.len() >= 20 {
                (
                    read_u32_be(payload, 4).unwrap_or(0) as u64,
                    read_u32_be(payload, 12).unwrap_or(0),
                    read_u32_be(payload, 16).unwrap_or(0) as u64,
                )
            } else if ver == 1 && payload.len() >= 32 {
                (
                    read_u64_be(payload, 4).unwrap_or(0),
                    read_u32_be(payload, 20).unwrap_or(0),
                    read_u64_be(payload, 24).unwrap_or(0),
                )
            } else {
                (0, 0, 0)
            };

            if info.recorded_date.is_none() && creation_time > MP4_EPOCH_OFFSET {
                info.recorded_date = format_unix_date_checked(creation_time - MP4_EPOCH_OFFSET);
            }
            if info.duration_secs.is_none() && timescale > 0 && duration > 0 {
                info.duration_secs = Some(duration as f64 / timescale as f64);
            }
        } else if &kind == b"trak" {
            parse_mp4_trak(payload, info);
        }
    }
}

fn parse_mp4_trak(trak: &[u8], info: &mut VideoMediaInfo) {
    let mut tkhd_dims: Option<(u32, u32)> = None;
    let mut handler: Option<[u8; 4]> = None;
    let mut mdhd_timescale: u32 = 0;
    let mut mdhd_duration: u64 = 0;
    let mut stsd_payload: Option<&[u8]> = None;
    let mut stts_payload: Option<&[u8]> = None;

    let mut pos = 0;
    while let Some((kind, payload)) = next_mp4_box(trak, &mut pos) {
        if &kind == b"tkhd" && !payload.is_empty() {
            let ver = payload[0];
            let (w_raw, h_raw) = if ver == 0 && payload.len() >= 84 {
                (
                    read_u32_be(payload, 76).unwrap_or(0),
                    read_u32_be(payload, 80).unwrap_or(0),
                )
            } else if ver == 1 && payload.len() >= 96 {
                (
                    read_u32_be(payload, 88).unwrap_or(0),
                    read_u32_be(payload, 92).unwrap_or(0),
                )
            } else {
                (0, 0)
            };
            let w = w_raw >> 16;
            let h = h_raw >> 16;
            if w > 0 && h > 0 {
                tkhd_dims = Some((w, h));
            }
        } else if &kind == b"mdia" {
            let mut mpos = 0;
            while let Some((mkind, mpay)) = next_mp4_box(payload, &mut mpos) {
                if &mkind == b"mdhd" && !mpay.is_empty() {
                    let ver = mpay[0];
                    if ver == 0 && mpay.len() >= 20 {
                        mdhd_timescale = read_u32_be(mpay, 12).unwrap_or(0);
                        mdhd_duration = read_u32_be(mpay, 16).unwrap_or(0) as u64;
                    } else if ver == 1 && mpay.len() >= 32 {
                        mdhd_timescale = read_u32_be(mpay, 20).unwrap_or(0);
                        mdhd_duration = read_u64_be(mpay, 24).unwrap_or(0);
                    }
                } else if &mkind == b"hdlr" && mpay.len() >= 12 {
                    handler = mpay[8..12].try_into().ok();
                } else if &mkind == b"minf" {
                    let mut minf_pos = 0;
                    while let Some((k2, p2)) = next_mp4_box(mpay, &mut minf_pos) {
                        if &k2 == b"stbl" {
                            let mut stbl_pos = 0;
                            while let Some((k3, p3)) = next_mp4_box(p2, &mut stbl_pos) {
                                if &k3 == b"stsd" {
                                    stsd_payload = Some(p3);
                                } else if &k3 == b"stts" {
                                    stts_payload = Some(p3);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if handler == Some(*b"vide") && info.video_codec.is_none() {
        if info.duration_secs.is_none() && mdhd_timescale > 0 && mdhd_duration > 0 {
            info.duration_secs = Some(mdhd_duration as f64 / mdhd_timescale as f64);
        }

        if let Some(stsd) = stsd_payload {
            if stsd.len() > 8 {
                let mut spos = 8;
                if let Some((codec_bytes, entry)) = next_mp4_box(stsd, &mut spos) {
                    let fourcc = String::from_utf8_lossy(&codec_bytes);
                    info.video_codec = normalize_video_codec(&fourcc);
                    let f_low = fourcc.to_lowercase();
                    if f_low == "dvh1" || f_low == "dvhe" || f_low == "dva1" || f_low == "dvav" {
                        info.hdr = Some("DV".to_string());
                    }

                    if entry.len() >= 28 {
                        let w = read_u16_be(entry, 24).unwrap_or(0) as u32;
                        let h = read_u16_be(entry, 26).unwrap_or(0) as u32;
                        if w > 0 && h > 0 {
                            info.width = Some(w);
                            info.height = Some(h);
                        }
                    }

                    if entry.len() > 78 {
                        let mut sub_pos = 78;
                        while let Some((skind, spay)) = next_mp4_box(entry, &mut sub_pos) {
                            if &skind == b"colr" && spay.len() >= 10 {
                                let ctype = &spay[0..4];
                                if ctype == b"nclx" || ctype == b"nclc" {
                                    let transfer = read_u16_be(spay, 6).unwrap_or(0);
                                    if transfer == 16 && info.hdr.is_none() {
                                        info.hdr = Some("HDR10".to_string());
                                    } else if transfer == 18 && info.hdr.is_none() {
                                        info.hdr = Some("HLG".to_string());
                                    }
                                }
                            } else if &skind == b"dvcC" || &skind == b"dvvC" || &skind == b"dvwC" {
                                info.hdr = Some("DV".to_string());
                            }
                        }
                    }
                }
            }
        }

        if (info.width.is_none() || info.height.is_none()) && tkhd_dims.is_some() {
            if let Some((w, h)) = tkhd_dims {
                info.width = Some(w);
                info.height = Some(h);
            }
        }

        if let Some(stts) = stts_payload {
            if mdhd_timescale > 0 && stts.len() >= 8 {
                let entry_count = read_u32_be(stts, 4).unwrap_or(0) as usize;
                let max_entries = ((stts.len() - 8) / 8).min(entry_count).min(100_000);
                let mut total_samples: u64 = 0;
                let mut total_delta: u64 = 0;
                for i in 0..max_entries {
                    let off = 8 + i * 8;
                    let cnt = read_u32_be(stts, off).unwrap_or(0) as u64;
                    let delta = read_u32_be(stts, off + 4).unwrap_or(0) as u64;
                    total_samples = total_samples.saturating_add(cnt);
                    total_delta = total_delta.saturating_add(cnt.saturating_mul(delta));
                }
                if total_samples > 0 && total_delta > 0 {
                    let fps = (total_samples as f64 * mdhd_timescale as f64) / total_delta as f64;
                    if (1.0..=240.0).contains(&fps) {
                        info.fps = Some(fps);
                    }
                }
            }
        }
    } else if handler == Some(*b"soun") && info.audio_codec.is_none() {
        if let Some(stsd) = stsd_payload {
            if stsd.len() > 8 {
                let mut spos = 8;
                if let Some((codec_bytes, entry)) = next_mp4_box(stsd, &mut spos) {
                    let fourcc = String::from_utf8_lossy(&codec_bytes);
                    info.audio_codec = normalize_audio_codec(&fourcc);

                    let version = read_u16_be(entry, 8).unwrap_or(0);
                    let mut channels: Option<u32> = if version == 2 && entry.len() >= 44 {
                        read_u32_be(entry, 40)
                    } else if entry.len() >= 18 {
                        read_u16_be(entry, 16).map(|v| v as u32)
                    } else {
                        None
                    };

                    let sub_start = match version {
                        0 => 28,
                        1 => 44,
                        2 => 64,
                        _ => entry.len(),
                    };

                    if entry.len() > sub_start {
                        let mut sub_pos = sub_start;
                        while let Some((skind, spay)) = next_mp4_box(entry, &mut sub_pos) {
                            if &skind == b"dac3" && spay.len() >= 3 {
                                let val = ((spay[0] as u32) << 16)
                                    | ((spay[1] as u32) << 8)
                                    | (spay[2] as u32);
                                let acmod = (val >> 11) & 0x07;
                                let lfeon = (val >> 10) & 0x01;
                                channels = Some(ac3_channels(acmod, lfeon));
                            } else if &skind == b"dec3" && spay.len() >= 4 {
                                let acmod = ((spay[3] >> 1) & 0x07) as u32;
                                let lfeon = (spay[3] & 0x01) as u32;
                                channels = Some(ac3_channels(acmod, lfeon));
                            }
                        }
                    }

                    if let Some(ch) = channels {
                        if ch > 0 && ch <= 32 {
                            info.audio_channels = Some(ch);
                        }
                    }
                }
            }
        }
    }
}

fn ac3_channels(acmod: u32, lfeon: u32) -> u32 {
    let base = match acmod {
        0 => 2,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 3,
        5 => 4,
        6 => 4,
        7 => 5,
        _ => 2,
    };
    base + lfeon
}

// ============================================================================
// Matroska / WebM (MKV / WebM) EBML Parser
// ============================================================================

fn read_ebml_id_slice(data: &[u8], pos: &mut usize) -> Option<u64> {
    let b0 = *data.get(*pos)?;
    if b0 == 0 {
        return None;
    }
    let len = b0.leading_zeros() as usize + 1;
    if len > 4 || *pos + len > data.len() {
        return None;
    }
    let mut id: u64 = 0;
    for i in 0..len {
        id = (id << 8) | (data[*pos + i] as u64);
    }
    *pos += len;
    Some(id)
}

fn read_ebml_size_slice(data: &[u8], pos: &mut usize) -> Option<Option<u64>> {
    let b0 = *data.get(*pos)?;
    if b0 == 0 {
        return None;
    }
    let len = b0.leading_zeros() as usize + 1;
    if len > 8 || *pos + len > data.len() {
        return None;
    }
    let mask = (1u8 << (8 - len)) - 1;
    let mut val: u64 = (b0 & mask) as u64;
    for i in 1..len {
        val = (val << 8) | (data[*pos + i] as u64);
    }
    *pos += len;
    let all_ones = (1u64 << (7 * len)) - 1;
    if val == all_ones {
        Some(None)
    } else {
        Some(Some(val))
    }
}

fn next_ebml_elem<'a>(data: &'a [u8], pos: &mut usize) -> Option<(u64, &'a [u8])> {
    let id = read_ebml_id_slice(data, pos)?;
    let size = read_ebml_size_slice(data, pos)?? as usize;
    if *pos + size > data.len() {
        return None;
    }
    let payload = &data[*pos..*pos + size];
    *pos += size;
    Some((id, payload))
}

fn read_ebml_id_reader<R: Read>(r: &mut R) -> Option<u64> {
    let mut b0 = [0u8; 1];
    r.read_exact(&mut b0).ok()?;
    if b0[0] == 0 {
        return None;
    }
    let len = b0[0].leading_zeros() as usize + 1;
    if len > 4 {
        return None;
    }
    let mut id = b0[0] as u64;
    if len > 1 {
        let mut rest = [0u8; 3];
        r.read_exact(&mut rest[..len - 1]).ok()?;
        for &b in &rest[..len - 1] {
            id = (id << 8) | (b as u64);
        }
    }
    Some(id)
}

fn read_ebml_size_reader<R: Read>(r: &mut R) -> Option<Option<u64>> {
    let mut b0 = [0u8; 1];
    r.read_exact(&mut b0).ok()?;
    if b0[0] == 0 {
        return None;
    }
    let len = b0[0].leading_zeros() as usize + 1;
    if len > 8 {
        return None;
    }
    let mask = (1u8 << (8 - len)) - 1;
    let mut val = (b0[0] & mask) as u64;
    if len > 1 {
        let mut rest = [0u8; 7];
        r.read_exact(&mut rest[..len - 1]).ok()?;
        for &b in &rest[..len - 1] {
            val = (val << 8) | (b as u64);
        }
    }
    let all_ones = (1u64 << (7 * len)) - 1;
    if val == all_ones {
        Some(None)
    } else {
        Some(Some(val))
    }
}

fn ebml_uint(b: &[u8]) -> u64 {
    let mut v = 0u64;
    for &byte in b.iter().take(8) {
        v = (v << 8) | (byte as u64);
    }
    v
}

fn ebml_sint(b: &[u8]) -> Option<i64> {
    if b.is_empty() || b.len() > 8 {
        return None;
    }
    let sign_byte = if (b[0] & 0x80) != 0 { 0xFF } else { 0x00 };
    let mut buf = [sign_byte; 8];
    let start = 8 - b.len();
    buf[start..8].copy_from_slice(b);
    Some(i64::from_be_bytes(buf))
}

fn ebml_float(b: &[u8]) -> Option<f64> {
    match b.len() {
        4 => Some(f32::from_be_bytes([b[0], b[1], b[2], b[3]]) as f64),
        8 => Some(f64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ])),
        _ => None,
    }
}

pub fn parse_mkv_reader<R: Read + Seek>(reader: &mut R, file_size_hint: u64) -> Option<VideoMediaInfo> {
    let stream_len = reader.seek(SeekFrom::End(0)).ok()?;
    let file_len = if stream_len > 0 { stream_len } else { file_size_hint };
    if file_len < 16 {
        return None;
    }
    reader.seek(SeekFrom::Start(0)).ok()?;

    let mut info = VideoMediaInfo {
        source: MediaInfoSource::BuiltIn,
        ..Default::default()
    };

    let ebml_id = read_ebml_id_reader(reader)?;
    if ebml_id != 0x1A45DFA3 {
        return None;
    }
    let ebml_sz = read_ebml_size_reader(reader)??;
    if ebml_sz <= 1024 {
        let mut hdr_buf = vec![0u8; ebml_sz as usize];
        if reader.read_exact(&mut hdr_buf).is_ok() {
            let mut hpos = 0;
            while let Some((hid, hpay)) = next_ebml_elem(&hdr_buf, &mut hpos) {
                if hid == 0x4282 {
                    let doc = String::from_utf8_lossy(hpay).to_lowercase();
                    if doc.contains("webm") {
                        info.container = Some("WebM".to_string());
                    } else if doc.contains("matroska") && info.container.is_none() {
                        info.container = Some("MKV".to_string());
                    }
                }
            }
        }
    } else {
        reader.seek(SeekFrom::Current(ebml_sz as i64)).ok()?;
    }

    let seg_id = read_ebml_id_reader(reader)?;
    if seg_id != 0x18538067 {
        return None;
    }
    let seg_sz_opt = read_ebml_size_reader(reader)?;
    let seg_data_start = reader.stream_position().ok()?;
    let seg_end = match seg_sz_opt {
        Some(sz) => seg_data_start.saturating_add(sz).min(file_len),
        None => file_len,
    };

    let mut got_info = false;
    let mut got_tracks = false;
    let mut seek_info_pos: Option<u64> = None;
    let mut seek_tracks_pos: Option<u64> = None;

    let mut iter_count = 0usize;
    while iter_count < 128 {
        iter_count += 1;
        let cur = reader.stream_position().ok()?;
        if cur >= seg_end {
            break;
        }
        let Some(id) = read_ebml_id_reader(reader) else {
            break;
        };
        if id == 0x1F43B675 {
            // Cluster reached
            break;
        }
        let Some(Some(sz)) = read_ebml_size_reader(reader) else {
            break;
        };
        let elem_data_start = reader.stream_position().ok()?;
        let elem_end = elem_data_start.saturating_add(sz);
        if elem_end > seg_end {
            break;
        }

        if id == 0x114D9B74 && sz <= 256 * 1024 {
            let mut buf = vec![0u8; sz as usize];
            if reader.read_exact(&mut buf).is_ok() {
                parse_mkv_seekhead(&buf, &mut seek_info_pos, &mut seek_tracks_pos);
            }
        } else if id == 0x1549A966 && sz <= 1024 * 1024 {
            let mut buf = vec![0u8; sz as usize];
            if reader.read_exact(&mut buf).is_ok() {
                parse_mkv_info(&buf, &mut info);
                got_info = true;
            }
        } else if id == 0x1654AE6B && sz <= 4 * 1024 * 1024 {
            let mut buf = vec![0u8; sz as usize];
            if reader.read_exact(&mut buf).is_ok() {
                parse_mkv_tracks(&buf, &mut info);
                got_tracks = true;
            }
        }

        if got_info && got_tracks {
            break;
        }
        if reader.seek(SeekFrom::Start(elem_end)).is_err() {
            break;
        }
    }

    if !got_info {
        if let Some(rel) = seek_info_pos {
            let target = seg_data_start.saturating_add(rel);
            if target < seg_end && reader.seek(SeekFrom::Start(target)).is_ok() {
                if read_ebml_id_reader(reader) == Some(0x1549A966) {
                    if let Some(Some(sz)) = read_ebml_size_reader(reader) {
                        if sz <= 1024 * 1024 {
                            let mut buf = vec![0u8; sz as usize];
                            if reader.read_exact(&mut buf).is_ok() {
                                parse_mkv_info(&buf, &mut info);
                            }
                        }
                    }
                }
            }
        }
    }

    if !got_tracks {
        if let Some(rel) = seek_tracks_pos {
            let target = seg_data_start.saturating_add(rel);
            if target < seg_end && reader.seek(SeekFrom::Start(target)).is_ok() {
                if read_ebml_id_reader(reader) == Some(0x1654AE6B) {
                    if let Some(Some(sz)) = read_ebml_size_reader(reader) {
                        if sz <= 4 * 1024 * 1024 {
                            let mut buf = vec![0u8; sz as usize];
                            if reader.read_exact(&mut buf).is_ok() {
                                parse_mkv_tracks(&buf, &mut info);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(dur) = info.duration_secs {
        if info.bitrate_bps.is_none() && file_len > 0 && dur > 0.1 {
            info.bitrate_bps = Some(((file_len as f64 * 8.0) / dur) as u64);
        }
    }

    if info.has_any_metadata() {
        Some(info)
    } else {
        None
    }
}

fn parse_mkv_seekhead(
    data: &[u8],
    seek_info_pos: &mut Option<u64>,
    seek_tracks_pos: &mut Option<u64>,
) {
    let mut pos = 0;
    while let Some((id, payload)) = next_ebml_elem(data, &mut pos) {
        if id == 0x4DBB {
            let mut spos = 0;
            let mut target_id: Option<u64> = None;
            let mut target_pos: Option<u64> = None;
            while let Some((sid, spay)) = next_ebml_elem(payload, &mut spos) {
                if sid == 0x53AB {
                    target_id = Some(ebml_uint(spay));
                } else if sid == 0x53AC {
                    target_pos = Some(ebml_uint(spay));
                }
            }
            if let (Some(tid), Some(tpos)) = (target_id, target_pos) {
                if tid == 0x1549A966 && seek_info_pos.is_none() {
                    *seek_info_pos = Some(tpos);
                } else if tid == 0x1654AE6B && seek_tracks_pos.is_none() {
                    *seek_tracks_pos = Some(tpos);
                }
            }
        }
    }
}

fn parse_mkv_info(data: &[u8], info: &mut VideoMediaInfo) {
    const MKV_EPOCH_UNIX_SECS: i64 = 978_307_200;
    let mut timecode_scale: u64 = 1_000_000;
    let mut raw_duration: Option<f64> = None;

    let mut pos = 0;
    while let Some((id, payload)) = next_ebml_elem(data, &mut pos) {
        match id {
            0x2AD7B1 => {
                let ts = ebml_uint(payload);
                if ts > 0 {
                    timecode_scale = ts;
                }
            }
            0x4489 => {
                raw_duration = ebml_float(payload);
            }
            0x4461 => {
                if info.recorded_date.is_none() {
                    if let Some(ns) = ebml_sint(payload) {
                        let unix_i64 = MKV_EPOCH_UNIX_SECS + (ns / 1_000_000_000);
                        if unix_i64 > 0 {
                            info.recorded_date = format_unix_date_checked(unix_i64 as u64);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(dur) = raw_duration {
        let secs = (dur * timecode_scale as f64) / 1_000_000_000.0;
        if secs > 0.0 && secs.is_finite() {
            info.duration_secs = Some(secs);
        }
    }
}

fn parse_mkv_tracks(data: &[u8], info: &mut VideoMediaInfo) {
    let mut pos = 0;
    while let Some((id, payload)) = next_ebml_elem(data, &mut pos) {
        if id != 0xAE {
            continue;
        }
        let mut track_type: u64 = 0;
        let mut codec_id: Option<String> = None;
        let mut codec_private: Option<&[u8]> = None;
        let mut default_dur_ns: Option<u64> = None;
        let mut vid_w: Option<u32> = None;
        let mut vid_h: Option<u32> = None;
        let mut vid_hdr: Option<String> = None;
        let mut aud_ch: Option<u32> = None;

        let mut tpos = 0;
        while let Some((tid, tpay)) = next_ebml_elem(payload, &mut tpos) {
            match tid {
                0x83 => track_type = ebml_uint(tpay),
                0x86 => {
                    codec_id = Some(
                        String::from_utf8_lossy(tpay)
                            .trim_matches(char::from(0))
                            .to_string(),
                    );
                }
                0x63A2 => codec_private = Some(tpay),
                0x23E383 => {
                    let v = ebml_uint(tpay);
                    if v > 0 {
                        default_dur_ns = Some(v);
                    }
                }
                0xE0 => {
                    let mut vpos = 0;
                    while let Some((vid, vpay)) = next_ebml_elem(tpay, &mut vpos) {
                        match vid {
                            0xB0 => vid_w = Some(ebml_uint(vpay) as u32),
                            0xBA => vid_h = Some(ebml_uint(vpay) as u32),
                            0x55B0 => {
                                let mut cpos = 0;
                                while let Some((cid, cpay)) = next_ebml_elem(vpay, &mut cpos) {
                                    if cid == 0x55BA {
                                        let tc = ebml_uint(cpay);
                                        if tc == 16 {
                                            vid_hdr = Some("HDR10".to_string());
                                        } else if tc == 18 {
                                            vid_hdr = Some("HLG".to_string());
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                0xE1 => {
                    let mut apos = 0;
                    while let Some((aid, apay)) = next_ebml_elem(tpay, &mut apos) {
                        if aid == 0x9F {
                            aud_ch = Some(ebml_uint(apay) as u32);
                        }
                    }
                }
                _ => {}
            }
        }

        if track_type == 1 && info.video_codec.is_none() {
            let mut raw_codec = codec_id.unwrap_or_default();
            if raw_codec == "V_MS/VFW/FOURCC" {
                if let Some(cp) = codec_private {
                    if cp.len() >= 20 {
                        raw_codec = String::from_utf8_lossy(&cp[16..20]).to_string();
                    }
                }
            }
            info.video_codec = normalize_video_codec(&raw_codec);
            if let (Some(w), Some(h)) = (vid_w, vid_h) {
                if w > 0 && h > 0 {
                    info.width = Some(w);
                    info.height = Some(h);
                }
            }
            if let Some(ns) = default_dur_ns {
                let fps = 1_000_000_000.0 / ns as f64;
                if (1.0..=240.0).contains(&fps) {
                    info.fps = Some(fps);
                }
            }
            if info.hdr.is_none() && vid_hdr.is_some() {
                info.hdr = vid_hdr;
            }
        } else if track_type == 2 && info.audio_codec.is_none() {
            if let Some(cid) = codec_id {
                info.audio_codec = normalize_audio_codec(&cid);
            }
            if let Some(ch) = aud_ch {
                if ch > 0 && ch <= 32 {
                    info.audio_channels = Some(ch);
                }
            }
        }
    }
}

// ============================================================================
// AVI (RIFF) Parser
// ============================================================================

fn read_u32_le(b: &[u8], off: usize) -> Option<u32> {
    let slice = b.get(off..off + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u16_le(b: &[u8], off: usize) -> Option<u16> {
    let slice = b.get(off..off + 2)?;
    Some(u16::from_le_bytes([slice[0], slice[1]]))
}

pub fn parse_avi_reader<R: Read + Seek>(reader: &mut R, file_size_hint: u64) -> Option<VideoMediaInfo> {
    let stream_len = reader.seek(SeekFrom::End(0)).ok()?;
    let file_len = if stream_len > 0 { stream_len } else { file_size_hint };
    if file_len < 24 {
        return None;
    }
    reader.seek(SeekFrom::Start(0)).ok()?;

    let mut hdr = [0u8; 12];
    reader.read_exact(&mut hdr).ok()?;
    if &hdr[0..4] != b"RIFF" || &hdr[8..12] != b"AVI " {
        return None;
    }

    let mut info = VideoMediaInfo {
        container: Some("AVI".to_string()),
        source: MediaInfoSource::BuiltIn,
        ..Default::default()
    };

    let mut pos: u64 = 12;
    let mut chunks_seen = 0usize;
    while pos + 8 <= file_len && chunks_seen < 64 {
        chunks_seen += 1;
        reader.seek(SeekFrom::Start(pos)).ok()?;
        let mut chdr = [0u8; 8];
        if reader.read_exact(&mut chdr).is_err() {
            break;
        }
        let cid = &chdr[0..4];
        let csize = u32::from_le_bytes([chdr[4], chdr[5], chdr[6], chdr[7]]) as u64;
        if cid == b"LIST" && csize >= 4 {
            let mut ltype = [0u8; 4];
            if reader.read_exact(&mut ltype).is_err() {
                break;
            }
            if &ltype == b"hdrl" {
                let pay_len = (csize - 4).min(512 * 1024) as usize;
                let mut buf = vec![0u8; pay_len];
                if reader.read_exact(&mut buf).is_ok() {
                    parse_avi_hdrl(&buf, &mut info);
                }
                break;
            } else if &ltype == b"movi" {
                break;
            }
        }
        let padded = csize + (csize & 1);
        pos = pos.saturating_add(8).saturating_add(padded);
    }

    if let Some(dur) = info.duration_secs {
        if info.bitrate_bps.is_none() && file_len > 0 && dur > 0.1 {
            info.bitrate_bps = Some(((file_len as f64 * 8.0) / dur) as u64);
        }
    }

    if info.has_any_metadata() {
        Some(info)
    } else {
        None
    }
}

fn parse_avi_hdrl(data: &[u8], info: &mut VideoMediaInfo) {
    let mut avih_us_per_frame: u32 = 0;
    let mut avih_total_frames: u32 = 0;
    let mut pos = 0usize;

    while pos + 8 <= data.len() {
        let cid = &data[pos..pos + 4];
        let csize = read_u32_le(data, pos + 4).unwrap_or(0) as usize;
        let data_start = pos + 8;
        let data_end = data_start.saturating_add(csize).min(data.len());
        let payload = &data[data_start..data_end];

        if cid == b"avih" && payload.len() >= 40 {
            avih_us_per_frame = read_u32_le(payload, 0).unwrap_or(0);
            avih_total_frames = read_u32_le(payload, 16).unwrap_or(0);
            let w = read_u32_le(payload, 32).unwrap_or(0);
            let h = read_u32_le(payload, 36).unwrap_or(0);
            if w > 0 && h > 0 && info.width.is_none() {
                info.width = Some(w);
                info.height = Some(h);
            }
            if avih_us_per_frame > 0 && info.fps.is_none() {
                let fps = 1_000_000.0 / avih_us_per_frame as f64;
                if (1.0..=240.0).contains(&fps) {
                    info.fps = Some(fps);
                }
            }
            if avih_us_per_frame > 0 && avih_total_frames > 0 && info.duration_secs.is_none() {
                info.duration_secs =
                    Some((avih_total_frames as f64 * avih_us_per_frame as f64) / 1_000_000.0);
            }
        } else if cid == b"LIST" && payload.len() >= 4 && &payload[0..4] == b"strl" {
            parse_avi_strl(&payload[4..], info);
        }

        let padded = csize + (csize & 1);
        pos = data_start.saturating_add(padded);
    }
}

fn parse_avi_strl(data: &[u8], info: &mut VideoMediaInfo) {
    let mut fcc_type: Option<[u8; 4]> = None;
    let mut fcc_handler: Option<String> = None;
    let mut pos = 0usize;

    while pos + 8 <= data.len() {
        let cid = &data[pos..pos + 4];
        let csize = read_u32_le(data, pos + 4).unwrap_or(0) as usize;
        let data_start = pos + 8;
        let data_end = data_start.saturating_add(csize).min(data.len());
        let payload = &data[data_start..data_end];

        if cid == b"strh" && payload.len() >= 32 {
            fcc_type = payload[0..4].try_into().ok();
            let handler = String::from_utf8_lossy(&payload[4..8]).trim().to_string();
            if !handler.is_empty() {
                fcc_handler = Some(handler);
            }
            if fcc_type == Some(*b"vids") {
                let scale = read_u32_le(payload, 20).unwrap_or(0);
                let rate = read_u32_le(payload, 24).unwrap_or(0);
                let length = read_u32_le(payload, 32).unwrap_or(0);
                if scale > 0 && rate > 0 {
                    let fps = rate as f64 / scale as f64;
                    if (1.0..=240.0).contains(&fps) {
                        info.fps = Some(fps);
                        if length > 0 {
                            info.duration_secs = Some(length as f64 / fps);
                        }
                    }
                }
            }
        } else if cid == b"strf" {
            if fcc_type == Some(*b"vids") && payload.len() >= 20 && info.video_codec.is_none() {
                let w = read_u32_le(payload, 4).unwrap_or(0) as i32;
                let h = read_u32_le(payload, 8).unwrap_or(0) as i32;
                if w > 0 && h != 0 {
                    info.width = Some(w as u32);
                    info.height = Some(h.unsigned_abs());
                }
                let comp = &payload[16..20];
                let comp_str = if comp.iter().all(|&b| b.is_ascii_graphic()) && comp != b"DIB " {
                    Some(String::from_utf8_lossy(comp).to_string())
                } else {
                    fcc_handler.clone()
                };
                if let Some(raw) = comp_str {
                    info.video_codec = normalize_video_codec(&raw);
                }
            } else if fcc_type == Some(*b"auds") && payload.len() >= 4 && info.audio_codec.is_none()
            {
                let tag = read_u16_le(payload, 0).unwrap_or(0);
                let ch = read_u16_le(payload, 2).unwrap_or(0) as u32;
                let codec = match tag {
                    0x0001 | 0x0003 => Some("PCM"),
                    0x0050 | 0x0055 => Some("MP3"),
                    0x00FF | 0x1600 | 0x1602 | 0x706D => Some("AAC"),
                    0x0160..=0x0163 => Some("WMA"),
                    0x2000 => Some("AC3"),
                    0x2001 => Some("DTS"),
                    0xF1AC => Some("FLAC"),
                    _ => None,
                };
                info.audio_codec = codec.map(|s| s.to_string());
                if ch > 0 && ch <= 32 {
                    info.audio_channels = Some(ch);
                }
            }
        }

        let padded = csize + (csize & 1);
        pos = data_start.saturating_add(padded);
    }
}

// ============================================================================
// FLV (AMF0 onMetaData) Parser
// ============================================================================

pub fn parse_flv_reader<R: Read + Seek>(reader: &mut R, file_size_hint: u64) -> Option<VideoMediaInfo> {
    let stream_len = reader.seek(SeekFrom::End(0)).ok()?;
    let file_len = if stream_len > 0 { stream_len } else { file_size_hint };
    if file_len < 24 {
        return None;
    }
    reader.seek(SeekFrom::Start(0)).ok()?;

    let mut hdr = [0u8; 9];
    reader.read_exact(&mut hdr).ok()?;
    if &hdr[0..3] != b"FLV" {
        return None;
    }
    let data_offset = u32::from_be_bytes([hdr[5], hdr[6], hdr[7], hdr[8]]) as u64;
    reader.seek(SeekFrom::Start(data_offset + 4)).ok()?;

    let mut info = VideoMediaInfo {
        container: Some("FLV".to_string()),
        source: MediaInfoSource::BuiltIn,
        ..Default::default()
    };

    for _ in 0..4 {
        let mut tag_hdr = [0u8; 11];
        if reader.read_exact(&mut tag_hdr).is_err() {
            break;
        }
        let tag_type = tag_hdr[0];
        let data_size =
            ((tag_hdr[1] as usize) << 16) | ((tag_hdr[2] as usize) << 8) | (tag_hdr[3] as usize);

        if tag_type == 18 && (16..=65536).contains(&data_size) {
            let mut payload = vec![0u8; data_size];
            if reader.read_exact(&mut payload).is_ok() {
                parse_flv_amf0_metadata(&payload, &mut info);
            }
            break;
        } else {
            if reader.seek(SeekFrom::Current(data_size as i64 + 4)).is_err() {
                break;
            }
        }
    }

    if let Some(dur) = info.duration_secs {
        if info.bitrate_bps.is_none() && file_len > 0 && dur > 0.1 {
            info.bitrate_bps = Some(((file_len as f64 * 8.0) / dur) as u64);
        }
    }

    if info.has_any_metadata() {
        Some(info)
    } else {
        None
    }
}

fn parse_flv_amf0_metadata(data: &[u8], info: &mut VideoMediaInfo) {
    let mut pos = 0usize;
    if data.get(pos) != Some(&0x02) {
        return;
    }
    pos += 1;
    let Some(name_len) = read_u16_be(data, pos).map(|v| v as usize) else {
        return;
    };
    pos += 2;
    if pos + name_len > data.len() {
        return;
    }
    let name = &data[pos..pos + name_len];
    pos += name_len;
    if name != b"onMetaData" {
        return;
    }

    let marker = *data.get(pos).unwrap_or(&0);
    pos += 1;
    if marker == 0x08 {
        if pos + 4 > data.len() {
            return;
        }
        pos += 4;
    } else if marker != 0x03 {
        return;
    }

    while pos + 2 <= data.len() {
        let klen = read_u16_be(data, pos).unwrap_or(0) as usize;
        pos += 2;
        if klen == 0 && data.get(pos) == Some(&0x09) {
            break;
        }
        if pos + klen >= data.len() {
            break;
        }
        let key = String::from_utf8_lossy(&data[pos..pos + klen]).to_lowercase();
        pos += klen;

        let val_type = *data.get(pos).unwrap_or(&0xFF);
        pos += 1;
        match val_type {
            0x00 => {
                if pos + 8 > data.len() {
                    break;
                }
                let val = f64::from_be_bytes(data[pos..pos + 8].try_into().unwrap());
                pos += 8;
                match key.as_str() {
                    "width" if val > 0.0 => info.width = Some(val.round() as u32),
                    "height" if val > 0.0 => info.height = Some(val.round() as u32),
                    "duration" if val > 0.0 => info.duration_secs = Some(val),
                    "framerate" | "fps" if (1.0..=240.0).contains(&val) => info.fps = Some(val),
                    "videocodecid" => {
                        info.video_codec = match val.round() as i32 {
                            2 => Some("H263".to_string()),
                            4 | 5 => Some("VP6".to_string()),
                            7 => Some("H264".to_string()),
                            12 => Some("HEVC".to_string()),
                            _ => None,
                        };
                    }
                    "audiocodecid" => {
                        info.audio_codec = match val.round() as i32 {
                            2 | 14 => Some("MP3".to_string()),
                            10 => Some("AAC".to_string()),
                            0 | 3 => Some("PCM".to_string()),
                            _ => None,
                        };
                    }
                    _ => {}
                }
            }
            0x01 => pos = pos.saturating_add(1),
            0x02 => {
                let Some(slen) = read_u16_be(data, pos).map(|v| v as usize) else {
                    break;
                };
                pos += 2;
                if pos + slen > data.len() {
                    break;
                }
                let sval = String::from_utf8_lossy(&data[pos..pos + slen]).to_string();
                pos += slen;
                if key == "videocodecid" && info.video_codec.is_none() {
                    info.video_codec = normalize_video_codec(&sval);
                } else if key == "audiocodecid" && info.audio_codec.is_none() {
                    info.audio_codec = normalize_audio_codec(&sval);
                }
            }
            0x05 | 0x06 => {}
            0x0B => pos = pos.saturating_add(10),
            0x0C => {
                let Some(slen) = read_u32_be(data, pos).map(|v| v as usize) else {
                    break;
                };
                pos = pos.saturating_add(4 + slen);
            }
            _ => break,
        }
    }
}

// ============================================================================
// Optional External Fallback (MediaInfo CLI / ffprobe found on PATH)
// ============================================================================

#[derive(Clone)]
enum ExternalTool {
    MediaInfo(PathBuf),
    Ffprobe(PathBuf),
}

static DETECTED_EXTERNAL_TOOL: OnceLock<Option<ExternalTool>> = OnceLock::new();

fn find_in_path(names: &[&str]) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn get_external_tool() -> Option<&'static ExternalTool> {
    DETECTED_EXTERNAL_TOOL
        .get_or_init(|| {
            if let Some(p) = find_in_path(&["mediainfo.exe", "mediainfo"]) {
                return Some(ExternalTool::MediaInfo(p));
            }
            if let Some(p) = find_in_path(&["ffprobe.exe", "ffprobe"]) {
                return Some(ExternalTool::Ffprobe(p));
            }
            None
        })
        .as_ref()
}

fn run_hidden_command(mut cmd: Command) -> Option<String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn query_external_mediainfo(path: &Path) -> Option<VideoMediaInfo> {
    let tool = get_external_tool()?;
    match tool {
        ExternalTool::MediaInfo(bin) => {
            let mut cmd = Command::new(bin);
            cmd.arg(path);
            let out = run_hidden_command(cmd)?;
            let parsed = parse_mediainfo_cli_output(&out);
            if parsed.has_any_metadata() {
                Some(parsed)
            } else {
                None
            }
        }
        ExternalTool::Ffprobe(bin) => {
            let mut cmd = Command::new(bin);
            cmd.args([
                "-v",
                "error",
                "-show_entries",
                "format=duration,bit_rate,format_name:format_tags=creation_time:stream=index,codec_type,codec_name,width,height,r_frame_rate,avg_frame_rate,channels,color_transfer",
                "-of",
                "default=noprint_wrappers=1",
            ]);
            cmd.arg(path);
            let out = run_hidden_command(cmd)?;
            let parsed = parse_ffprobe_output(&out);
            if parsed.has_any_metadata() {
                Some(parsed)
            } else {
                None
            }
        }
    }
}

fn parse_spaced_digits_u32(val: &str) -> Option<u32> {
    let digits: String = val.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<u32>().ok().filter(|&v| v > 0)
}

fn parse_mediainfo_duration(val: &str) -> Option<f64> {
    let mut total: f64 = 0.0;
    let mut matched = false;
    for tok in val.split_whitespace() {
        if let Some(n) = tok.strip_suffix("h").and_then(|s| s.parse::<f64>().ok()) {
            total += n * 3600.0;
            matched = true;
        } else if let Some(n) = tok.strip_suffix("mn").and_then(|s| s.parse::<f64>().ok()) {
            total += n * 60.0;
            matched = true;
        } else if let Some(n) = tok.strip_suffix("min").and_then(|s| s.parse::<f64>().ok()) {
            total += n * 60.0;
            matched = true;
        } else if let Some(n) = tok.strip_suffix("ms").and_then(|s| s.parse::<f64>().ok()) {
            total += n / 1000.0;
            matched = true;
        } else if let Some(n) = tok.strip_suffix('s').and_then(|s| s.parse::<f64>().ok()) {
            total += n;
            matched = true;
        }
    }
    if matched && total > 0.0 {
        Some(total)
    } else {
        None
    }
}

fn parse_mediainfo_bitrate(val: &str) -> Option<u64> {
    let cleaned = val.replace(' ', "");
    let low = cleaned.to_lowercase();
    let num_end = low
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(low.len());
    let num = low[..num_end].parse::<f64>().ok()?;
    let unit = &low[num_end..];
    let mult = if unit.starts_with("gb/s") || unit.starts_with("gbps") {
        1_000_000_000.0
    } else if unit.starts_with("mb/s") || unit.starts_with("mbps") {
        1_000_000.0
    } else if unit.starts_with("kb/s") || unit.starts_with("kbps") {
        1_000.0
    } else {
        1.0
    };
    let bps = (num * mult).round() as u64;
    if bps > 0 { Some(bps) } else { None }
}

fn parse_date_prefix(raw: &str) -> Option<String> {
    let s = raw.trim();
    let bytes = s.as_bytes();
    for i in 0..bytes.len().saturating_sub(9) {
        if bytes[i].is_ascii_digit()
            && bytes[i + 1].is_ascii_digit()
            && bytes[i + 2].is_ascii_digit()
            && bytes[i + 3].is_ascii_digit()
            && (bytes[i + 4] == b'-' || bytes[i + 4] == b':')
            && bytes[i + 5].is_ascii_digit()
            && bytes[i + 6].is_ascii_digit()
            && (bytes[i + 7] == b'-' || bytes[i + 7] == b':')
            && bytes[i + 8].is_ascii_digit()
            && bytes[i + 9].is_ascii_digit()
        {
            let slice = &s[i..i + 10];
            return Some(slice.replace(':', "-"));
        }
    }
    None
}

pub fn parse_mediainfo_cli_output(text: &str) -> VideoMediaInfo {
    #[derive(Clone, Copy, PartialEq)]
    enum Section {
        None,
        General,
        Video,
        Audio,
        Other,
    }

    let mut info = VideoMediaInfo {
        source: MediaInfoSource::MediaInfoCli,
        ..Default::default()
    };
    let mut section = Section::None;
    let mut got_video_track = false;
    let mut got_audio_track = false;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if !line.contains(':') {
            let low = line.to_lowercase();
            if low == "general" {
                section = Section::General;
            } else if low == "video" || low.starts_with("video #") {
                if !got_video_track {
                    section = Section::Video;
                    got_video_track = true;
                } else {
                    section = Section::Other;
                }
            } else if low == "audio" || low.starts_with("audio #") {
                if !got_audio_track {
                    section = Section::Audio;
                    got_audio_track = true;
                } else {
                    section = Section::Other;
                }
            } else {
                section = Section::Other;
            }
            continue;
        }

        let Some((k_raw, v_raw)) = line.split_once(':') else {
            continue;
        };
        let key = k_raw.trim().to_lowercase();
        let val = v_raw.trim();

        match section {
            Section::General => match key.as_str() {
                "format" if info.container.is_none() => {
                    let up = val.to_uppercase();
                    if up.contains("MATROSKA") {
                        info.container = Some("MKV".to_string());
                    } else if up.contains("MPEG-4") {
                        info.container = Some("MP4".to_string());
                    } else if up.contains("QUICKTIME") {
                        info.container = Some("MOV".to_string());
                    } else if let Some(first) = val.split_whitespace().next() {
                        info.container = Some(first.to_string());
                    }
                }
                "duration" if info.duration_secs.is_none() => {
                    info.duration_secs = parse_mediainfo_duration(val);
                }
                "overall bit rate" if info.bitrate_bps.is_none() => {
                    info.bitrate_bps = parse_mediainfo_bitrate(val);
                }
                "recorded date" | "encoded date" if info.recorded_date.is_none() => {
                    info.recorded_date = parse_date_prefix(val);
                }
                _ => {}
            },
            Section::Video => match key.as_str() {
                "format" if info.video_codec.is_none() => {
                    info.video_codec = normalize_video_codec(val);
                }
                "width" if info.width.is_none() => {
                    info.width = parse_spaced_digits_u32(val);
                }
                "height" if info.height.is_none() => {
                    info.height = parse_spaced_digits_u32(val);
                }
                "frame rate" if info.fps.is_none() => {
                    if let Some(tok) = val.split_whitespace().next() {
                        if let Ok(fps) = tok.parse::<f64>() {
                            if (1.0..=240.0).contains(&fps) {
                                info.fps = Some(fps);
                            }
                        }
                    }
                }
                "hdr format" if info.hdr.is_none() => {
                    let low = val.to_lowercase();
                    if low.contains("dolby vision") {
                        info.hdr = Some("DV".to_string());
                    } else if low.contains("hdr10+") {
                        info.hdr = Some("HDR10+".to_string());
                    } else if low.contains("hdr10") || low.contains("smpte st 2086") {
                        info.hdr = Some("HDR10".to_string());
                    } else if low.contains("hlg") {
                        info.hdr = Some("HLG".to_string());
                    }
                }
                "encoded date" if info.recorded_date.is_none() => {
                    info.recorded_date = parse_date_prefix(val);
                }
                _ => {}
            },
            Section::Audio => match key.as_str() {
                "format" if info.audio_codec.is_none() => {
                    info.audio_codec = normalize_audio_codec(val);
                }
                "channel(s)" if info.audio_channels.is_none() => {
                    if let Some(tok) = val.split_whitespace().next() {
                        if let Ok(ch) = tok.parse::<u32>() {
                            if ch > 0 && ch <= 32 {
                                info.audio_channels = Some(ch);
                            }
                        }
                    }
                }
                _ => {}
            },
            Section::None | Section::Other => {}
        }
    }

    info
}

fn parse_ffprobe_rate(raw: &str) -> Option<f64> {
    let s = raw.trim();
    if let Some((num_s, den_s)) = s.split_once('/') {
        let num = num_s.parse::<f64>().ok()?;
        let den = den_s.parse::<f64>().ok()?;
        if den > 0.0 {
            let fps = num / den;
            if (1.0..=240.0).contains(&fps) {
                return Some(fps);
            }
        }
        None
    } else if let Ok(fps) = s.parse::<f64>() {
        if (1.0..=240.0).contains(&fps) {
            Some(fps)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn parse_ffprobe_output(text: &str) -> VideoMediaInfo {
    #[derive(Default)]
    struct StreamRec {
        codec_type: Option<String>,
        codec_name: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
        avg_fps: Option<f64>,
        r_fps: Option<f64>,
        channels: Option<u32>,
        color_transfer: Option<String>,
    }

    let mut info = VideoMediaInfo {
        source: MediaInfoSource::Ffprobe,
        ..Default::default()
    };
    let mut cur = StreamRec::default();

    let flush_stream = |rec: &mut StreamRec, info: &mut VideoMediaInfo| {
        let ctype = rec.codec_type.take();
        if ctype.as_deref() == Some("video") && info.video_codec.is_none() {
            if let Some(cname) = rec.codec_name.take() {
                info.video_codec = normalize_video_codec(&cname);
            }
            if let (Some(w), Some(h)) = (rec.width.take(), rec.height.take()) {
                if w > 0 && h > 0 {
                    info.width = Some(w);
                    info.height = Some(h);
                }
            }
            if info.fps.is_none() {
                info.fps = rec.avg_fps.or(rec.r_fps);
            }
            if info.hdr.is_none() {
                if let Some(tr) = rec.color_transfer.take() {
                    match tr.as_str() {
                        "smpte2084" => info.hdr = Some("HDR10".to_string()),
                        "arib-std-b67" => info.hdr = Some("HLG".to_string()),
                        _ => {}
                    }
                }
            }
        } else if ctype.as_deref() == Some("audio") && info.audio_codec.is_none() {
            if let Some(cname) = rec.codec_name.take() {
                info.audio_codec = normalize_audio_codec(&cname);
            }
            if let Some(ch) = rec.channels.take() {
                if ch > 0 && ch <= 32 {
                    info.audio_channels = Some(ch);
                }
            }
        }
        *rec = StreamRec::default();
    };

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let val = val.trim();
        match key.trim() {
            "index" => {
                if cur.codec_type.is_some() || cur.codec_name.is_some() {
                    flush_stream(&mut cur, &mut info);
                }
            }
            "codec_type" => cur.codec_type = Some(val.to_lowercase()),
            "codec_name" => cur.codec_name = Some(val.to_string()),
            "width" => cur.width = val.parse::<u32>().ok().filter(|&v| v > 0),
            "height" => cur.height = val.parse::<u32>().ok().filter(|&v| v > 0),
            "avg_frame_rate" => cur.avg_fps = parse_ffprobe_rate(val),
            "r_frame_rate" => cur.r_fps = parse_ffprobe_rate(val),
            "channels" => cur.channels = val.parse::<u32>().ok().filter(|&v| v > 0),
            "color_transfer" => cur.color_transfer = Some(val.to_lowercase()),
            "duration" => {
                flush_stream(&mut cur, &mut info);
                if info.duration_secs.is_none() {
                    if let Ok(d) = val.parse::<f64>() {
                        if d > 0.0 && d.is_finite() {
                            info.duration_secs = Some(d);
                        }
                    }
                }
            }
            "bit_rate" => {
                if info.bitrate_bps.is_none() {
                    if let Ok(bps) = val.parse::<u64>() {
                        if bps > 0 {
                            info.bitrate_bps = Some(bps);
                        }
                    }
                }
            }
            "format_name" => {
                if info.container.is_none() {
                    let low = val.to_lowercase();
                    if low.contains("matroska") {
                        info.container = Some("MKV".to_string());
                    } else if low.contains("webm") {
                        info.container = Some("WebM".to_string());
                    } else if low.contains("mp4") {
                        info.container = Some("MP4".to_string());
                    } else if low.contains("avi") {
                        info.container = Some("AVI".to_string());
                    } else if low.contains("mpegts") {
                        info.container = Some("TS".to_string());
                    }
                }
            }
            "TAG:creation_time" => {
                if info.recorded_date.is_none() {
                    info.recorded_date = parse_date_prefix(val);
                }
            }
            _ => {}
        }
    }
    flush_stream(&mut cur, &mut info);
    info
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn mp4_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let len = (8 + payload.len()) as u32;
        let mut out = Vec::with_capacity(len as usize);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(payload);
        out
    }

    fn ebml_elem(id: &[u8], payload: &[u8]) -> Vec<u8> {
        let len = payload.len() as u64;
        let mut out = Vec::from(id);
        if len < 127 {
            out.push(0x80 | (len as u8));
        } else {
            let sz = (0x4000u16) | (len as u16);
            out.extend_from_slice(&sz.to_be_bytes());
        }
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn test_resolution_and_tags() {
        let mut info = VideoMediaInfo {
            width: Some(3840),
            height: Some(1600),
            duration_secs: Some(3725.0),
            fps: Some(23.976),
            video_codec: Some("HEVC".to_string()),
            audio_codec: Some("AC3".to_string()),
            audio_channels: Some(6),
            bitrate_bps: Some(15_400_000),
            recorded_date: Some("2026-11-23".to_string()),
            hdr: Some("HDR10".to_string()),
            container: Some("MKV".to_string()),
            source: MediaInfoSource::BuiltIn,
        };

        assert_eq!(info.resolution_label().as_deref(), Some("2160p"));
        assert_eq!(info.fps_tag().as_deref(), Some("23.98fps"));
        assert_eq!(info.duration_tag().as_deref(), Some("01h02m05s"));
        assert_eq!(info.duration_display().as_deref(), Some("01:02:05"));
        assert_eq!(info.audio_channels_tag().as_deref(), Some("5.1"));
        assert_eq!(info.bitrate_tag().as_deref(), Some("15Mbps"));

        info.width = Some(1080);
        info.height = Some(1920);
        info.fps = Some(59.99);
        info.duration_secs = Some(275.0);
        info.bitrate_bps = Some(4_500_000);
        assert_eq!(info.resolution_label().as_deref(), Some("1080p"));
        assert_eq!(info.fps_tag().as_deref(), Some("60fps"));
        assert_eq!(info.duration_tag().as_deref(), Some("04m35s"));
        assert_eq!(info.bitrate_tag().as_deref(), Some("4.5Mbps"));
    }

    #[test]
    fn test_parse_mp4_moov_at_end() {
        let ftyp = mp4_box(b"ftyp", b"isom\x00\x00\x02\x00isommp41");
        let mdat = mp4_box(b"mdat", &[0u8; 64]);

        // mvhd v0: creation=2026-11-23 -> unix=1795411200 + 2082844800 = 3878256000
        let mut mvhd_pay = vec![0u8; 100];
        let mp4_ts: u32 = 3_878_256_000;
        mvhd_pay[4..8].copy_from_slice(&mp4_ts.to_be_bytes());
        mvhd_pay[12..16].copy_from_slice(&1000u32.to_be_bytes());
        mvhd_pay[16..20].copy_from_slice(&65_000u32.to_be_bytes()); // 65s
        let mvhd = mp4_box(b"mvhd", &mvhd_pay);

        // Video trak
        let mut hdlr_v = vec![0u8; 24];
        hdlr_v[8..12].copy_from_slice(b"vide");
        let mut mdhd_v = vec![0u8; 24];
        mdhd_v[12..16].copy_from_slice(&60000u32.to_be_bytes());
        mdhd_v[16..20].copy_from_slice(&3_900_000u32.to_be_bytes());

        let mut colr_pay = vec![0u8; 11];
        colr_pay[0..4].copy_from_slice(b"nclx");
        colr_pay[6..8].copy_from_slice(&16u16.to_be_bytes()); // HDR10
        let colr = mp4_box(b"colr", &colr_pay);

        let mut hvc1_pay = vec![0u8; 78];
        hvc1_pay[24..26].copy_from_slice(&1920u16.to_be_bytes());
        hvc1_pay[26..28].copy_from_slice(&1080u16.to_be_bytes());
        hvc1_pay.extend_from_slice(&colr);
        let hvc1 = mp4_box(b"hvc1", &hvc1_pay);

        let mut stsd_v_pay = vec![0u8; 8];
        stsd_v_pay[4..8].copy_from_slice(&1u32.to_be_bytes());
        stsd_v_pay.extend_from_slice(&hvc1);

        let mut stts_pay = vec![0u8; 16];
        stts_pay[4..8].copy_from_slice(&1u32.to_be_bytes());
        stts_pay[8..12].copy_from_slice(&3900u32.to_be_bytes());
        stts_pay[12..16].copy_from_slice(&1000u32.to_be_bytes()); // 60000/1000 = 60fps

        let stbl_v = mp4_box(
            b"stbl",
            &[mp4_box(b"stsd", &stsd_v_pay), mp4_box(b"stts", &stts_pay)].concat(),
        );
        let minf_v = mp4_box(b"minf", &stbl_v);
        let mdia_v = mp4_box(
            b"mdia",
            &[
                mp4_box(b"mdhd", &mdhd_v),
                mp4_box(b"hdlr", &hdlr_v),
                minf_v,
            ]
            .concat(),
        );
        let trak_v = mp4_box(b"trak", &mdia_v);

        // Audio trak with AC-3 + dac3 (5.1: acmod=7, lfeon=1)
        let mut hdlr_a = vec![0u8; 24];
        hdlr_a[8..12].copy_from_slice(b"soun");
        let dac3_bits: u32 = (7 << 11) | (1 << 10);
        let dac3 = mp4_box(
            b"dac3",
            &[
                ((dac3_bits >> 16) & 0xFF) as u8,
                ((dac3_bits >> 8) & 0xFF) as u8,
                (dac3_bits & 0xFF) as u8,
            ],
        );
        let mut ac3_pay = vec![0u8; 28];
        ac3_pay[16..18].copy_from_slice(&2u16.to_be_bytes());
        ac3_pay.extend_from_slice(&dac3);
        let mut stsd_a_pay = vec![0u8; 8];
        stsd_a_pay[4..8].copy_from_slice(&1u32.to_be_bytes());
        stsd_a_pay.extend_from_slice(&mp4_box(b"ac-3", &ac3_pay));

        let stbl_a = mp4_box(b"stbl", &mp4_box(b"stsd", &stsd_a_pay));
        let minf_a = mp4_box(b"minf", &stbl_a);
        let mdia_a = mp4_box(b"mdia", &[mp4_box(b"hdlr", &hdlr_a), minf_a].concat());
        let trak_a = mp4_box(b"trak", &mdia_a);

        let moov = mp4_box(b"moov", &[mvhd, trak_v, trak_a].concat());
        let data = [ftyp, mdat, moov].concat();
        let len = data.len() as u64;
        let mut cursor = Cursor::new(data);

        let info = parse_mp4_reader(&mut cursor, len).expect("should parse mp4");
        assert_eq!(info.container.as_deref(), Some("MP4"));
        assert_eq!(info.width, Some(1920));
        assert_eq!(info.height, Some(1080));
        assert_eq!(info.video_codec.as_deref(), Some("HEVC"));
        assert_eq!(info.audio_codec.as_deref(), Some("AC3"));
        assert_eq!(info.audio_channels, Some(6));
        assert_eq!(info.hdr.as_deref(), Some("HDR10"));
        assert_eq!(info.recorded_date.as_deref(), Some("2026-11-23"));
        assert!((info.duration_secs.unwrap() - 65.0).abs() < 0.01);
        assert!((info.fps.unwrap() - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_mkv_in_memory() {
        let ebml_hdr = ebml_elem(&[0x1A, 0x45, 0xDF, 0xA3], &ebml_elem(&[0x42, 0x82], b"matroska"));

        let info_pay = [
            ebml_elem(&[0x2A, 0xD7, 0xB1], &1_000_000u32.to_be_bytes()),
            ebml_elem(&[0x44, 0x89], &120_000.0f64.to_be_bytes()),
        ]
        .concat();
        let seg_info = ebml_elem(&[0x15, 0x49, 0xA9, 0x66], &info_pay);

        let video_sub = [
            ebml_elem(&[0xB0], &3840u16.to_be_bytes()),
            ebml_elem(&[0xBA], &2160u16.to_be_bytes()),
        ]
        .concat();
        let track_v = ebml_elem(
            &[0xAE],
            &[
                ebml_elem(&[0x83], &[1]),
                ebml_elem(&[0x86], b"V_MPEGH/ISO/HEVC"),
                ebml_elem(&[0x23, 0xE3, 0x83], &41_666_667u32.to_be_bytes()),
                ebml_elem(&[0xE0], &video_sub),
            ]
            .concat(),
        );

        let audio_sub = ebml_elem(&[0x9F], &[6]);
        let track_a = ebml_elem(
            &[0xAE],
            &[
                ebml_elem(&[0x83], &[2]),
                ebml_elem(&[0x86], b"A_OPUS"),
                ebml_elem(&[0xE1], &audio_sub),
            ]
            .concat(),
        );

        let tracks = ebml_elem(&[0x16, 0x54, 0xAE, 0x6B], &[track_v, track_a].concat());
        let seg_body = [seg_info, tracks].concat();
        let segment = ebml_elem(&[0x18, 0x53, 0x80, 0x67], &seg_body);

        let data = [ebml_hdr, segment].concat();
        let len = data.len() as u64;
        let mut cursor = Cursor::new(data);

        let info = parse_mkv_reader(&mut cursor, len).expect("should parse mkv");
        assert_eq!(info.container.as_deref(), Some("MKV"));
        assert_eq!(info.width, Some(3840));
        assert_eq!(info.height, Some(2160));
        assert_eq!(info.video_codec.as_deref(), Some("HEVC"));
        assert_eq!(info.audio_codec.as_deref(), Some("Opus"));
        assert_eq!(info.audio_channels, Some(6));
        assert_eq!(info.resolution_label().as_deref(), Some("2160p"));
        assert_eq!(info.fps_tag().as_deref(), Some("24fps"));
        assert!((info.duration_secs.unwrap() - 120.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_avi_in_memory() {
        let mut avih = vec![0u8; 56];
        avih[0..4].copy_from_slice(&33_333u32.to_le_bytes()); // ~30 fps
        avih[16..20].copy_from_slice(&900u32.to_le_bytes()); // 30s
        avih[32..36].copy_from_slice(&1280u32.to_le_bytes());
        avih[36..40].copy_from_slice(&720u32.to_le_bytes());

        let mut avih_chunk = Vec::new();
        avih_chunk.extend_from_slice(b"avih");
        avih_chunk.extend_from_slice(&(avih.len() as u32).to_le_bytes());
        avih_chunk.extend_from_slice(&avih);

        let mut strh_v = vec![0u8; 56];
        strh_v[0..4].copy_from_slice(b"vids");
        strh_v[4..8].copy_from_slice(b"H264");
        strh_v[20..24].copy_from_slice(&1000u32.to_le_bytes());
        strh_v[24..28].copy_from_slice(&30000u32.to_le_bytes());
        strh_v[32..36].copy_from_slice(&900u32.to_le_bytes());

        let mut strh_chunk = Vec::new();
        strh_chunk.extend_from_slice(b"strh");
        strh_chunk.extend_from_slice(&(strh_v.len() as u32).to_le_bytes());
        strh_chunk.extend_from_slice(&strh_v);

        let mut strf_v = vec![0u8; 40];
        strf_v[4..8].copy_from_slice(&1280i32.to_le_bytes());
        strf_v[8..12].copy_from_slice(&720i32.to_le_bytes());
        strf_v[16..20].copy_from_slice(b"H264");
        let mut strf_chunk = Vec::new();
        strf_chunk.extend_from_slice(b"strf");
        strf_chunk.extend_from_slice(&(strf_v.len() as u32).to_le_bytes());
        strf_chunk.extend_from_slice(&strf_v);

        let strl_body = [strh_chunk, strf_chunk].concat();
        let mut strl_list = Vec::new();
        strl_list.extend_from_slice(b"LIST");
        strl_list.extend_from_slice(&((4 + strl_body.len()) as u32).to_le_bytes());
        strl_list.extend_from_slice(b"strl");
        strl_list.extend_from_slice(&strl_body);

        let hdrl_body = [avih_chunk, strl_list].concat();
        let mut hdrl_list = Vec::new();
        hdrl_list.extend_from_slice(b"LIST");
        hdrl_list.extend_from_slice(&((4 + hdrl_body.len()) as u32).to_le_bytes());
        hdrl_list.extend_from_slice(b"hdrl");
        hdrl_list.extend_from_slice(&hdrl_body);

        let mut riff = Vec::new();
        riff.extend_from_slice(b"RIFF");
        riff.extend_from_slice(&((4 + hdrl_list.len()) as u32).to_le_bytes());
        riff.extend_from_slice(b"AVI ");
        riff.extend_from_slice(&hdrl_list);

        let len = riff.len() as u64;
        let mut cursor = Cursor::new(riff);
        let info = parse_avi_reader(&mut cursor, len).expect("should parse avi");
        assert_eq!(info.width, Some(1280));
        assert_eq!(info.height, Some(720));
        assert_eq!(info.video_codec.as_deref(), Some("H264"));
        assert!((info.fps.unwrap() - 30.0).abs() < 0.01);
        assert!((info.duration_secs.unwrap() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_external_parsers() {
        let mi_text = "\
General
Format                                   : Matroska
Duration                                 : 1 h 42 min
Overall bit rate                         : 12.5 Mb/s
Recorded date                            : 2026-11-20 18:30:00

Video
Format                                   : HEVC
HDR format                               : SMPTE ST 2086, HDR10 compatible
Width                                    : 3 840 pixels
Height                                   : 2 160 pixels
Frame rate                               : 23.976 (24000/1001) FPS

Audio
Format                                   : E-AC-3
Channel(s)                               : 6 channels
";
        let mi = parse_mediainfo_cli_output(mi_text);
        assert_eq!(mi.container.as_deref(), Some("MKV"));
        assert_eq!(mi.width, Some(3840));
        assert_eq!(mi.height, Some(2160));
        assert_eq!(mi.video_codec.as_deref(), Some("HEVC"));
        assert_eq!(mi.audio_codec.as_deref(), Some("EAC3"));
        assert_eq!(mi.audio_channels, Some(6));
        assert_eq!(mi.hdr.as_deref(), Some("HDR10"));
        assert_eq!(mi.recorded_date.as_deref(), Some("2026-11-20"));
        assert!((mi.duration_secs.unwrap() - 6120.0).abs() < 0.1);
        assert_eq!(mi.bitrate_bps, Some(12_500_000));

        let fp_text = "\
index=0
codec_name=h264
codec_type=video
width=1920
height=1080
r_frame_rate=60/1
avg_frame_rate=60/1
color_transfer=bt709
index=1
codec_name=aac
codec_type=audio
channels=2
format_name=mov,mp4,m4a,3gp,3g2,mj2
duration=185.500000
bit_rate=8450000
TAG:creation_time=2026-11-21T14:22:00.000000Z
";
        let fp = parse_ffprobe_output(fp_text);
        assert_eq!(fp.container.as_deref(), Some("MP4"));
        assert_eq!(fp.width, Some(1920));
        assert_eq!(fp.height, Some(1080));
        assert_eq!(fp.video_codec.as_deref(), Some("H264"));
        assert_eq!(fp.audio_codec.as_deref(), Some("AAC"));
        assert_eq!(fp.audio_channels, Some(2));
        assert_eq!(fp.fps, Some(60.0));
        assert_eq!(fp.recorded_date.as_deref(), Some("2026-11-21"));
        assert!((fp.duration_secs.unwrap() - 185.5).abs() < 0.01);
        assert_eq!(fp.bitrate_bps, Some(8_450_000));
    }
}
