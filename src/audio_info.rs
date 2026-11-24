use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AudioMediaInfo {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub title: Option<String>,
    pub track: Option<u32>,
    pub year: Option<String>,
    pub genre: Option<String>,
    pub duration_secs: Option<f64>,
    pub bitrate_kbps: Option<u32>,
    pub sample_rate_hz: Option<u32>,
    pub channels: Option<u32>,
    pub format: Option<String>,
}

impl AudioMediaInfo {
    pub fn track_tag(&self) -> Option<String> {
        self.track.filter(|&t| t > 0).map(|t| format!("{:02}", t))
    }

    pub fn bitrate_tag(&self) -> Option<String> {
        self.bitrate_kbps
            .filter(|&b| b > 0)
            .map(|b| format!("{}kbps", b))
    }

    pub fn sample_rate_tag(&self) -> Option<String> {
        let hz = self.sample_rate_hz.filter(|&h| h > 0)?;
        let khz = hz as f64 / 1000.0;
        let s = format!("{:.1}", khz);
        Some(format!("{}kHz", s.strip_suffix(".0").unwrap_or(&s)))
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

    pub fn has_any_metadata(&self) -> bool {
        self.artist.is_some()
            || self.album.is_some()
            || self.title.is_some()
            || self.track.is_some()
            || self.year.is_some()
            || self.genre.is_some()
            || self.duration_secs.is_some()
            || self.bitrate_kbps.is_some()
            || self.sample_rate_hz.is_some()
    }
}

pub fn is_audio_ext(ext: &str) -> bool {
    matches!(
        ext,
        "mp3" | "flac" | "ogg" | "oga" | "opus" | "m4a" | "aac" | "wav" | "wma" | "alac"
    )
}

fn clean_tag_text(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|&c| c != '\0' && !c.is_control())
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn extract_year_4(raw: &str) -> Option<String> {
    let digits: String = raw.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() >= 4 {
        Some(digits[0..4].to_string())
    } else {
        let found: Vec<char> = raw.chars().filter(|c| c.is_ascii_digit()).collect();
        if found.len() >= 4 {
            Some(found[0..4].iter().collect())
        } else {
            None
        }
    }
}

fn parse_track_number(raw: &str) -> Option<u32> {
    let head = raw.split('/').next().unwrap_or(raw).trim();
    head.parse::<u32>().ok().filter(|&n| n > 0)
}

fn format_from_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "mp3" => Some("MP3"),
        "flac" => Some("FLAC"),
        "ogg" | "oga" => Some("OGG"),
        "opus" => Some("Opus"),
        "m4a" => Some("M4A"),
        "aac" => Some("AAC"),
        "wav" => Some("WAV"),
        "wma" => Some("WMA"),
        "alac" => Some("ALAC"),
        _ => None,
    }
}

pub fn extract_audio_info(path: &Path, file_size: u64) -> Option<AudioMediaInfo> {
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    let mut file = File::open(path).ok()?;
    let mut magic = [0u8; 12];
    let n = file.read(&mut magic).unwrap_or(0);
    let _ = file.seek(SeekFrom::Start(0));

    let mut info = if ext == "mp3" || (n >= 3 && &magic[0..3] == b"ID3") {
        parse_mp3_reader(&mut file, file_size, false).0
    } else if ext == "flac" || (n >= 4 && &magic[0..4] == b"fLaC") {
        parse_flac_reader(&mut file, file_size, false).0
    } else if matches!(ext.as_str(), "ogg" | "oga" | "opus")
        || (n >= 4 && &magic[0..4] == b"OggS")
    {
        parse_ogg_reader(&mut file, file_size)
    } else if matches!(ext.as_str(), "m4a" | "alac")
        || (n >= 8 && &magic[4..8] == b"ftyp")
    {
        parse_m4a_reader(&mut file, file_size, false).0
    } else if ext == "wav" || (n >= 12 && &magic[0..4] == b"RIFF" && &magic[8..12] == b"WAVE") {
        parse_wav_reader(&mut file, file_size)
    } else {
        AudioMediaInfo::default()
    };

    if info.format.is_none() {
        info.format = format_from_ext(&ext).map(|s| s.to_string());
    }

    if info.has_any_metadata() || info.format.is_some() {
        Some(info)
    } else {
        None
    }
}

pub fn extract_audio_cover(path: &Path) -> Option<Vec<u8>> {
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mut file = File::open(path).ok()?;
    let mut magic = [0u8; 12];
    let n = file.read(&mut magic).unwrap_or(0);
    let _ = file.seek(SeekFrom::Start(0));

    if ext == "mp3" || (n >= 3 && &magic[0..3] == b"ID3") {
        parse_mp3_reader(&mut file, file_size, true).1
    } else if ext == "flac" || (n >= 4 && &magic[0..4] == b"fLaC") {
        parse_flac_reader(&mut file, file_size, true).1
    } else if matches!(ext.as_str(), "m4a" | "alac") || (n >= 8 && &magic[4..8] == b"ftyp") {
        parse_m4a_reader(&mut file, file_size, true).1
    } else {
        None
    }
}

// ============================================================================
// MP3 (ID3v2 + ID3v1 + MPEG Frame Header)
// ============================================================================

fn decode_synchsafe(b: &[u8]) -> u32 {
    ((b[0] as u32 & 0x7F) << 21)
        | ((b[1] as u32 & 0x7F) << 14)
        | ((b[2] as u32 & 0x7F) << 7)
        | (b[3] as u32 & 0x7F)
}

fn decode_id3_text(payload: &[u8]) -> Option<String> {
    if payload.is_empty() {
        return None;
    }
    let enc = payload[0];
    let data = &payload[1..];
    let raw = match enc {
        0 => {
            let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
            data[..end].iter().map(|&b| b as char).collect::<String>()
        }
        1 => decode_utf16_bom(data),
        2 => decode_utf16_be(data),
        3 => {
            let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
            String::from_utf8_lossy(&data[..end]).to_string()
        }
        _ => String::from_utf8_lossy(data).to_string(),
    };
    clean_tag_text(&raw)
}

fn decode_utf16_bom(data: &[u8]) -> String {
    if data.len() < 2 {
        return String::new();
    }
    if data[0] == 0xFF && data[1] == 0xFE {
        let words: Vec<u16> = data[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&w| w != 0)
            .collect();
        String::from_utf16_lossy(&words)
    } else if data[0] == 0xFE && data[1] == 0xFF {
        decode_utf16_be(&data[2..])
    } else {
        decode_utf16_be(data)
    }
}

fn decode_utf16_be(data: &[u8]) -> String {
    let words: Vec<u16> = data
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .take_while(|&w| w != 0)
        .collect();
    String::from_utf16_lossy(&words)
}

fn resolve_id3_genre(raw: &str) -> String {
    let t = raw.trim();
    if let Some(rest) = t.strip_prefix('(') {
        if let Some(end) = rest.find(')') {
            let num_str = &rest[..end];
            let after = rest[end + 1..].trim();
            if !after.is_empty() {
                return after.to_string();
            }
            if let Ok(code) = num_str.parse::<usize>() {
                return id3v1_genre_name(code).unwrap_or(t).to_string();
            }
        }
    }
    if let Ok(code) = t.parse::<usize>() {
        if let Some(name) = id3v1_genre_name(code) {
            return name.to_string();
        }
    }
    t.to_string()
}

fn id3v1_genre_name(idx: usize) -> Option<&'static str> {
    const GENRES: &[&str] = &[
        "Blues", "Classic Rock", "Country", "Dance", "Disco", "Funk", "Grunge", "Hip-Hop",
        "Jazz", "Metal", "New Age", "Oldies", "Other", "Pop", "R&B", "Rap", "Reggae", "Rock",
        "Techno", "Industrial", "Alternative", "Ska", "Death Metal", "Pranks", "Soundtrack",
        "Euro-Techno", "Ambient", "Trip-Hop", "Vocal", "Jazz+Funk", "Fusion", "Trance",
        "Classical", "Instrumental", "Acid", "House", "Game", "Sound Clip", "Gospel", "Noise",
        "AlternRock", "Bass", "Soul", "Punk", "Space", "Meditative", "Instrumental Pop",
        "Instrumental Rock", "Ethnic", "Gothic", "Darkwave", "Techno-Industrial", "Electronic",
        "Pop-Folk", "Eurodance", "Dream", "Southern Rock", "Comedy", "Cult", "Gangsta", "Top 40",
        "Christian Rap", "Pop/Funk", "Jungle", "Native American", "Cabaret", "New Wave",
        "Psychedelic", "Rave", "Showtunes", "Trailer", "Lo-Fi", "Tribal", "Acid Punk",
        "Acid Jazz", "Polka", "Retro", "Musical", "Rock & Roll", "Hard Rock",
    ];
    GENRES.get(idx).copied()
}

fn extract_apic_image(payload: &[u8], is_v22: bool) -> Option<Vec<u8>> {
    if payload.len() < 8 {
        return None;
    }
    let enc = payload[0];
    let mut pos = 1usize;
    if is_v22 {
        pos += 3; // 3-byte image format
    } else {
        let nul = payload[pos..].iter().position(|&b| b == 0)?;
        pos += nul + 1;
    }
    if pos >= payload.len() {
        return None;
    }
    pos += 1; // picture type
    if pos >= payload.len() {
        return None;
    }
    if enc == 1 || enc == 2 {
        while pos + 1 < payload.len() {
            if payload[pos] == 0 && payload[pos + 1] == 0 {
                pos += 2;
                break;
            }
            pos += 2;
        }
    } else if let Some(nul) = payload[pos..].iter().position(|&b| b == 0) {
        pos += nul + 1;
    }
    if pos < payload.len() && payload[pos..].len() >= 16 {
        Some(payload[pos..].to_vec())
    } else {
        None
    }
}

pub fn parse_mp3_reader<R: Read + Seek>(
    reader: &mut R,
    file_size_hint: u64,
    want_cover: bool,
) -> (AudioMediaInfo, Option<Vec<u8>>) {
    let stream_len = reader.seek(SeekFrom::End(0)).ok().unwrap_or(0);
    let file_len = if stream_len > 0 { stream_len } else { file_size_hint };
    let _ = reader.seek(SeekFrom::Start(0));

    let mut info = AudioMediaInfo {
        format: Some("MP3".to_string()),
        ..Default::default()
    };
    let mut cover: Option<Vec<u8>> = None;
    let mut audio_start: u64 = 0;

    let mut id3_hdr = [0u8; 10];
    if reader.read_exact(&mut id3_hdr).is_ok() && &id3_hdr[0..3] == b"ID3" {
        let ver = id3_hdr[3];
        let flags = id3_hdr[5];
        let tag_size = decode_synchsafe(&id3_hdr[6..10]) as u64;
        audio_start = 10 + tag_size;

        if (2..=4).contains(&ver) && tag_size <= 8 * 1024 * 1024 {
            let mut buf = vec![0u8; tag_size as usize];
            if reader.read_exact(&mut buf).is_ok() {
                let mut pos = 0usize;
                if (flags & 0x40) != 0 && ver >= 3 && buf.len() >= 4 {
                    let ext_sz = if ver == 4 {
                        decode_synchsafe(&buf[0..4]) as usize
                    } else {
                        u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize + 4
                    };
                    pos = ext_sz.min(buf.len());
                }

                while pos < buf.len() {
                    if buf[pos] == 0 {
                        break;
                    }
                    let (frame_id, frame_size, header_len) = if ver == 2 {
                        if pos + 6 > buf.len() {
                            break;
                        }
                        let id = String::from_utf8_lossy(&buf[pos..pos + 3]).to_string();
                        let sz = ((buf[pos + 3] as usize) << 16)
                            | ((buf[pos + 4] as usize) << 8)
                            | (buf[pos + 5] as usize);
                        (id, sz, 6usize)
                    } else {
                        if pos + 10 > buf.len() {
                            break;
                        }
                        let id = String::from_utf8_lossy(&buf[pos..pos + 4]).to_string();
                        let sz = if ver == 4 {
                            decode_synchsafe(&buf[pos + 4..pos + 8]) as usize
                        } else {
                            u32::from_be_bytes([
                                buf[pos + 4],
                                buf[pos + 5],
                                buf[pos + 6],
                                buf[pos + 7],
                            ]) as usize
                        };
                        (id, sz, 10usize)
                    };

                    if frame_size == 0 || pos + header_len + frame_size > buf.len() {
                        break;
                    }
                    let payload = &buf[pos + header_len..pos + header_len + frame_size];
                    pos += header_len + frame_size;

                    match frame_id.as_str() {
                        "TIT2" | "TT2" if info.title.is_none() => {
                            info.title = decode_id3_text(payload);
                        }
                        "TPE1" | "TP1" if info.artist.is_none() => {
                            info.artist = decode_id3_text(payload);
                        }
                        "TALB" | "TAL" if info.album.is_none() => {
                            info.album = decode_id3_text(payload);
                        }
                        "TRCK" | "TRK" if info.track.is_none() => {
                            if let Some(t) = decode_id3_text(payload) {
                                info.track = parse_track_number(&t);
                            }
                        }
                        "TDRC" | "TYER" | "TYE" if info.year.is_none() => {
                            if let Some(t) = decode_id3_text(payload) {
                                info.year = extract_year_4(&t);
                            }
                        }
                        "TCON" | "TCO" if info.genre.is_none() => {
                            if let Some(t) = decode_id3_text(payload) {
                                info.genre = clean_tag_text(&resolve_id3_genre(&t));
                            }
                        }
                        "APIC" | "PIC" if want_cover && cover.is_none() => {
                            cover = extract_apic_image(payload, frame_id == "PIC");
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Fallback to ID3v1
    if file_len >= 128
        && (info.title.is_none()
            || info.artist.is_none()
            || info.album.is_none()
            || info.year.is_none())
    {
        if reader.seek(SeekFrom::Start(file_len - 128)).is_ok() {
            let mut v1 = [0u8; 128];
            if reader.read_exact(&mut v1).is_ok() && &v1[0..3] == b"TAG" {
                let read_iso = |slice: &[u8]| -> Option<String> {
                    let s: String = slice
                        .iter()
                        .take_while(|&&b| b != 0)
                        .map(|&b| b as char)
                        .collect();
                    clean_tag_text(&s)
                };
                if info.title.is_none() {
                    info.title = read_iso(&v1[3..33]);
                }
                if info.artist.is_none() {
                    info.artist = read_iso(&v1[33..63]);
                }
                if info.album.is_none() {
                    info.album = read_iso(&v1[63..93]);
                }
                if info.year.is_none() {
                    if let Some(y) = read_iso(&v1[93..97]) {
                        info.year = extract_year_4(&y);
                    }
                }
                if info.track.is_none() && v1[125] == 0 && v1[126] > 0 {
                    info.track = Some(v1[126] as u32);
                }
                if info.genre.is_none() && v1[127] < 80 {
                    info.genre = id3v1_genre_name(v1[127] as usize).map(|s| s.to_string());
                }
            }
        }
    }

    // Scan first MPEG audio frame header
    if audio_start < file_len && reader.seek(SeekFrom::Start(audio_start)).is_ok() {
        let mut scan = vec![0u8; 16384];
        let n = reader.read(&mut scan).unwrap_or(0);
        for i in 0..n.saturating_sub(4) {
            if scan[i] == 0xFF && (scan[i + 1] & 0xE0) == 0xE0 {
                let ver_bits = (scan[i + 1] >> 3) & 0x03;
                let layer_bits = (scan[i + 1] >> 1) & 0x03;
                let br_idx = ((scan[i + 2] >> 4) & 0x0F) as usize;
                let sr_idx = ((scan[i + 2] >> 2) & 0x03) as usize;
                let ch_mode = (scan[i + 3] >> 6) & 0x03;

                if ver_bits != 1 && layer_bits == 1 && br_idx > 0 && br_idx < 15 && sr_idx < 3 {
                    const MPEG1_L3_BR: [u32; 16] = [
                        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
                    ];
                    const MPEG2_L3_BR: [u32; 16] = [
                        0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
                    ];
                    const SR_MPEG1: [u32; 3] = [44100, 48000, 32000];
                    const SR_MPEG2: [u32; 3] = [22050, 24000, 16000];
                    const SR_MPEG25: [u32; 3] = [11025, 12000, 8000];

                    let br = if ver_bits == 3 {
                        MPEG1_L3_BR[br_idx]
                    } else {
                        MPEG2_L3_BR[br_idx]
                    };
                    let sr = match ver_bits {
                        3 => SR_MPEG1[sr_idx],
                        2 => SR_MPEG2[sr_idx],
                        0 => SR_MPEG25[sr_idx],
                        _ => 0,
                    };
                    if br > 0 && sr > 0 {
                        info.bitrate_kbps = Some(br);
                        info.sample_rate_hz = Some(sr);
                        info.channels = Some(if ch_mode == 3 { 1 } else { 2 });
                        let audio_bytes = file_len.saturating_sub(audio_start);
                        if audio_bytes > 0 {
                            info.duration_secs = Some((audio_bytes as f64 * 8.0) / (br as f64 * 1000.0));
                        }
                        break;
                    }
                }
            }
        }
    }

    (info, cover)
}

// ============================================================================
// FLAC Parser
// ============================================================================

fn apply_vorbis_comments(data: &[u8], info: &mut AudioMediaInfo) {
    if data.len() < 8 {
        return;
    }
    let vendor_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let mut pos = 4usize.saturating_add(vendor_len);
    if pos + 4 > data.len() {
        return;
    }
    let count = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
    pos += 4;

    for _ in 0..count.min(256) {
        if pos + 4 > data.len() {
            break;
        }
        let clen =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;
        if pos + clen > data.len() {
            break;
        }
        let comment = String::from_utf8_lossy(&data[pos..pos + clen]);
        pos += clen;

        if let Some((k, v)) = comment.split_once('=') {
            let key = k.trim().to_uppercase();
            match key.as_str() {
                "TITLE" if info.title.is_none() => info.title = clean_tag_text(v),
                "ARTIST" if info.artist.is_none() => info.artist = clean_tag_text(v),
                "ALBUM" if info.album.is_none() => info.album = clean_tag_text(v),
                "TRACKNUMBER" if info.track.is_none() => info.track = parse_track_number(v),
                "DATE" | "YEAR" if info.year.is_none() => info.year = extract_year_4(v),
                "GENRE" if info.genre.is_none() => info.genre = clean_tag_text(v),
                _ => {}
            }
        }
    }
}

fn parse_flac_picture(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 32 {
        return None;
    }
    let mut pos = 4usize;
    let mime_len = u32::from_be_bytes(data.get(pos..pos + 4)?.try_into().ok()?) as usize;
    pos = pos + 4;
    let mime = std::str::from_utf8(data.get(pos..pos + mime_len)?).unwrap_or("");
    pos += mime_len;
    if mime == "-->" {
        return None;
    }
    let desc_len = u32::from_be_bytes(data.get(pos..pos + 4)?.try_into().ok()?) as usize;
    pos = pos + 4 + desc_len + 16;
    let pic_len = u32::from_be_bytes(data.get(pos..pos + 4)?.try_into().ok()?) as usize;
    pos += 4;
    let img = data.get(pos..pos + pic_len)?;
    if img.len() >= 16 {
        Some(img.to_vec())
    } else {
        None
    }
}

pub fn parse_flac_reader<R: Read + Seek>(
    reader: &mut R,
    file_size_hint: u64,
    want_cover: bool,
) -> (AudioMediaInfo, Option<Vec<u8>>) {
    let stream_len = reader.seek(SeekFrom::End(0)).ok().unwrap_or(0);
    let file_len = if stream_len > 0 { stream_len } else { file_size_hint };
    let _ = reader.seek(SeekFrom::Start(0));

    let mut info = AudioMediaInfo {
        format: Some("FLAC".to_string()),
        ..Default::default()
    };
    let mut cover: Option<Vec<u8>> = None;

    let mut magic = [0u8; 4];
    if reader.read_exact(&mut magic).is_err() || &magic != b"fLaC" {
        return (info, None);
    }

    for _ in 0..64 {
        let mut bhdr = [0u8; 4];
        if reader.read_exact(&mut bhdr).is_err() {
            break;
        }
        let is_last = (bhdr[0] & 0x80) != 0;
        let btype = bhdr[0] & 0x7F;
        let blen = ((bhdr[1] as usize) << 16) | ((bhdr[2] as usize) << 8) | (bhdr[3] as usize);

        if btype == 0 && blen >= 34 {
            let mut sbuf = vec![0u8; blen.min(64)];
            if reader.read_exact(&mut sbuf).is_ok() {
                let sr = ((sbuf[10] as u32) << 12)
                    | ((sbuf[11] as u32) << 4)
                    | ((sbuf[12] as u32) >> 4);
                let ch = (((sbuf[12] >> 1) & 0x07) + 1) as u32;
                let total_samples = (((sbuf[13] & 0x0F) as u64) << 32)
                    | (u32::from_be_bytes([sbuf[14], sbuf[15], sbuf[16], sbuf[17]]) as u64);
                if sr > 0 {
                    info.sample_rate_hz = Some(sr);
                    info.channels = Some(ch);
                    if total_samples > 0 {
                        let dur = total_samples as f64 / sr as f64;
                        info.duration_secs = Some(dur);
                        if file_len > 0 && dur > 0.1 {
                            info.bitrate_kbps =
                                Some(((file_len as f64 * 8.0) / dur / 1000.0).round() as u32);
                        }
                    }
                }
            }
            if blen > 64 {
                let _ = reader.seek(SeekFrom::Current((blen - 64) as i64));
            }
        } else if btype == 4 && blen <= 1024 * 1024 {
            let mut vbuf = vec![0u8; blen];
            if reader.read_exact(&mut vbuf).is_ok() {
                apply_vorbis_comments(&vbuf, &mut info);
            }
        } else if btype == 6 && want_cover && cover.is_none() && blen <= 16 * 1024 * 1024 {
            let mut pbuf = vec![0u8; blen];
            if reader.read_exact(&mut pbuf).is_ok() {
                cover = parse_flac_picture(&pbuf);
            }
        } else if reader.seek(SeekFrom::Current(blen as i64)).is_err() {
            break;
        }

        if is_last {
            break;
        }
    }

    (info, cover)
}

// ============================================================================
// OGG Vorbis / Opus Parser
// ============================================================================

pub fn parse_ogg_reader<R: Read + Seek>(reader: &mut R, file_size_hint: u64) -> AudioMediaInfo {
    let stream_len = reader.seek(SeekFrom::End(0)).ok().unwrap_or(0);
    let file_len = if stream_len > 0 { stream_len } else { file_size_hint };
    let _ = reader.seek(SeekFrom::Start(0));

    let mut info = AudioMediaInfo {
        format: Some("OGG".to_string()),
        ..Default::default()
    };

    let head_len = (file_len.min(256 * 1024)) as usize;
    let mut head = vec![0u8; head_len];
    let n = reader.read(&mut head).unwrap_or(0);
    head.truncate(n);

    let mut pos = 0usize;
    let mut packet_buf: Vec<u8> = Vec::new();
    let mut pages_read = 0usize;

    while pos + 27 <= head.len() && pages_read < 32 {
        if &head[pos..pos + 4] != b"OggS" {
            break;
        }
        pages_read += 1;
        let n_seg = head[pos + 26] as usize;
        if pos + 27 + n_seg > head.len() {
            break;
        }
        let seg_table = &head[pos + 27..pos + 27 + n_seg];
        let body_len: usize = seg_table.iter().map(|&s| s as usize).sum();
        let body_start = pos + 27 + n_seg;
        if body_start + body_len > head.len() {
            break;
        }

        let mut bpos = body_start;
        for &seg in seg_table {
            let slen = seg as usize;
            packet_buf.extend_from_slice(&head[bpos..bpos + slen]);
            bpos += slen;
            if seg < 255 {
                if packet_buf.starts_with(b"\x01vorbis") && packet_buf.len() >= 28 {
                    info.format = Some("OGG".to_string());
                    info.channels = Some(packet_buf[11] as u32);
                    let sr = u32::from_le_bytes(packet_buf[12..16].try_into().unwrap());
                    if sr > 0 {
                        info.sample_rate_hz = Some(sr);
                    }
                    let br_nom = i32::from_le_bytes(packet_buf[20..24].try_into().unwrap());
                    if br_nom > 0 {
                        info.bitrate_kbps = Some((br_nom / 1000) as u32);
                    }
                } else if packet_buf.starts_with(b"\x03vorbis") && packet_buf.len() > 7 {
                    apply_vorbis_comments(&packet_buf[7..], &mut info);
                } else if packet_buf.starts_with(b"OpusHead") && packet_buf.len() >= 12 {
                    info.format = Some("Opus".to_string());
                    info.channels = Some(packet_buf[9] as u32);
                    info.sample_rate_hz = Some(48000);
                } else if packet_buf.starts_with(b"OpusTags") && packet_buf.len() > 8 {
                    apply_vorbis_comments(&packet_buf[8..], &mut info);
                }
                packet_buf.clear();
            }
        }
        pos = body_start + body_len;
    }

    if let Some(sr) = info.sample_rate_hz {
        if sr > 0 && file_len >= 32 {
            let tail_start = file_len.saturating_sub(65536);
            if reader.seek(SeekFrom::Start(tail_start)).is_ok() {
                let mut tail = vec![0u8; (file_len - tail_start) as usize];
                let tn = reader.read(&mut tail).unwrap_or(0);
                tail.truncate(tn);
                for i in (0..tail.len().saturating_sub(14)).rev() {
                    if &tail[i..i + 4] == b"OggS" && tail[i + 4] == 0 {
                        let gran = i64::from_le_bytes(tail[i + 6..i + 14].try_into().unwrap());
                        if gran > 0 {
                            let dur = gran as f64 / sr as f64;
                            if dur > 0.1 && dur.is_finite() {
                                info.duration_secs = Some(dur);
                                if info.bitrate_kbps.is_none() {
                                    info.bitrate_kbps = Some(
                                        ((file_len as f64 * 8.0) / dur / 1000.0).round() as u32,
                                    );
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    info
}

// ============================================================================
// M4A (ISO BMFF ilst) Parser
// ============================================================================

fn next_atom<'a>(data: &'a [u8], pos: &mut usize) -> Option<([u8; 4], &'a [u8])> {
    if *pos + 8 > data.len() {
        return None;
    }
    let sz32 = u32::from_be_bytes(data[*pos..*pos + 4].try_into().ok()?);
    let kind: [u8; 4] = data[*pos + 4..*pos + 8].try_into().ok()?;
    let (hdr, total) = if sz32 == 1 {
        if *pos + 16 > data.len() {
            return None;
        }
        let sz64 = u64::from_be_bytes(data[*pos + 8..*pos + 16].try_into().ok()?);
        (16usize, usize::try_from(sz64).ok()?)
    } else if sz32 == 0 {
        (8usize, data.len() - *pos)
    } else {
        (8usize, sz32 as usize)
    };
    if total < hdr || *pos + total > data.len() {
        return None;
    }
    let payload = &data[*pos + hdr..*pos + total];
    *pos += total;
    Some((kind, payload))
}

pub fn parse_m4a_reader<R: Read + Seek>(
    reader: &mut R,
    file_size_hint: u64,
    want_cover: bool,
) -> (AudioMediaInfo, Option<Vec<u8>>) {
    let stream_len = reader.seek(SeekFrom::End(0)).ok().unwrap_or(0);
    let file_len = if stream_len > 0 { stream_len } else { file_size_hint };
    let _ = reader.seek(SeekFrom::Start(0));

    let mut info = AudioMediaInfo {
        format: Some("M4A".to_string()),
        ..Default::default()
    };
    let mut cover: Option<Vec<u8>> = None;

    let mut pos: u64 = 0;
    for _ in 0..128 {
        if pos + 8 > file_len || reader.seek(SeekFrom::Start(pos)).is_err() {
            break;
        }
        let mut hdr = [0u8; 8];
        if reader.read_exact(&mut hdr).is_err() {
            break;
        }
        let sz32 = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        let kind = [hdr[4], hdr[5], hdr[6], hdr[7]];
        let (hlen, bsz) = if sz32 == 1 {
            let mut ext = [0u8; 8];
            if reader.read_exact(&mut ext).is_err() {
                break;
            }
            (16u64, u64::from_be_bytes(ext))
        } else if sz32 == 0 {
            (8u64, file_len - pos)
        } else {
            (8u64, sz32 as u64)
        };
        if bsz < hlen || pos + bsz > file_len {
            break;
        }
        if &kind == b"moov" {
            let plen = bsz - hlen;
            if plen <= 16 * 1024 * 1024 {
                let mut moov = vec![0u8; plen as usize];
                if reader.read_exact(&mut moov).is_ok() {
                    parse_m4a_moov(&moov, &mut info, want_cover, &mut cover);
                }
            }
            break;
        }
        pos += bsz;
    }

    if let Some(dur) = info.duration_secs {
        if info.bitrate_kbps.is_none() && file_len > 0 && dur > 0.1 {
            info.bitrate_kbps = Some(((file_len as f64 * 8.0) / dur / 1000.0).round() as u32);
        }
    }

    (info, cover)
}

fn parse_m4a_moov(
    moov: &[u8],
    info: &mut AudioMediaInfo,
    want_cover: bool,
    cover: &mut Option<Vec<u8>>,
) {
    let mut pos = 0usize;
    while let Some((kind, pay)) = next_atom(moov, &mut pos) {
        if &kind == b"mvhd" && pay.len() >= 20 {
            let ver = pay[0];
            let (ts, dur) = if ver == 0 && pay.len() >= 20 {
                (
                    u32::from_be_bytes(pay[12..16].try_into().unwrap()),
                    u32::from_be_bytes(pay[16..20].try_into().unwrap()) as u64,
                )
            } else if ver == 1 && pay.len() >= 32 {
                (
                    u32::from_be_bytes(pay[20..24].try_into().unwrap()),
                    u64::from_be_bytes(pay[24..32].try_into().unwrap()),
                )
            } else {
                (0, 0)
            };
            if ts > 0 && dur > 0 {
                info.duration_secs = Some(dur as f64 / ts as f64);
            }
        } else if &kind == b"trak" {
            let mut tpos = 0usize;
            while let Some((tk, tp)) = next_atom(pay, &mut tpos) {
                if &tk == b"mdia" {
                    let mut mpos = 0usize;
                    while let Some((mk, mp)) = next_atom(tp, &mut mpos) {
                        if &mk == b"minf" {
                            let mut minf_p = 0usize;
                            while let Some((k2, p2)) = next_atom(mp, &mut minf_p) {
                                if &k2 == b"stbl" {
                                    let mut s_p = 0usize;
                                    while let Some((k3, p3)) = next_atom(p2, &mut s_p) {
                                        if &k3 == b"stsd" && p3.len() > 8 {
                                            let mut e_p = 8usize;
                                            if let Some((_, entry)) = next_atom(p3, &mut e_p) {
                                                if entry.len() >= 26 {
                                                    let ch = u16::from_be_bytes(
                                                        entry[16..18].try_into().unwrap(),
                                                    ) as u32;
                                                    let sr = u16::from_be_bytes(
                                                        entry[24..26].try_into().unwrap(),
                                                    ) as u32;
                                                    if ch > 0 && info.channels.is_none() {
                                                        info.channels = Some(ch);
                                                    }
                                                    if sr > 0 && info.sample_rate_hz.is_none() {
                                                        info.sample_rate_hz = Some(sr);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if &kind == b"udta" {
            let mut upos = 0usize;
            while let Some((uk, up)) = next_atom(pay, &mut upos) {
                if &uk == b"meta" && up.len() > 4 {
                    let mut mpos = 4usize; // meta FullBox header
                    while let Some((mk, mp)) = next_atom(up, &mut mpos) {
                        if &mk == b"ilst" {
                            parse_m4a_ilst(mp, info, want_cover, cover);
                        }
                    }
                }
            }
        }
    }
}

fn parse_m4a_ilst(
    ilst: &[u8],
    info: &mut AudioMediaInfo,
    want_cover: bool,
    cover: &mut Option<Vec<u8>>,
) {
    let mut pos = 0usize;
    while let Some((item_kind, item_pay)) = next_atom(ilst, &mut pos) {
        let mut dpos = 0usize;
        while let Some((dk, dp)) = next_atom(item_pay, &mut dpos) {
            if &dk == b"data" && dp.len() > 8 {
                let val_bytes = &dp[8..];
                match &item_kind {
                    b"\xa9nam" if info.title.is_none() => {
                        info.title = clean_tag_text(&String::from_utf8_lossy(val_bytes));
                    }
                    b"\xa9ART" if info.artist.is_none() => {
                        info.artist = clean_tag_text(&String::from_utf8_lossy(val_bytes));
                    }
                    b"\xa9alb" if info.album.is_none() => {
                        info.album = clean_tag_text(&String::from_utf8_lossy(val_bytes));
                    }
                    b"\xa9day" if info.year.is_none() => {
                        info.year = extract_year_4(&String::from_utf8_lossy(val_bytes));
                    }
                    b"\xa9gen" if info.genre.is_none() => {
                        info.genre = clean_tag_text(&String::from_utf8_lossy(val_bytes));
                    }
                    b"trkn" if info.track.is_none() && val_bytes.len() >= 4 => {
                        let t = u16::from_be_bytes([val_bytes[2], val_bytes[3]]) as u32;
                        if t > 0 {
                            info.track = Some(t);
                        }
                    }
                    b"covr" if want_cover && cover.is_none() && val_bytes.len() >= 16 => {
                        *cover = Some(val_bytes.to_vec());
                    }
                    _ => {}
                }
            }
        }
    }
}

// ============================================================================
// WAV (RIFF WAVE) Parser
// ============================================================================

pub fn parse_wav_reader<R: Read + Seek>(reader: &mut R, file_size_hint: u64) -> AudioMediaInfo {
    let stream_len = reader.seek(SeekFrom::End(0)).ok().unwrap_or(0);
    let file_len = if stream_len > 0 { stream_len } else { file_size_hint };
    let _ = reader.seek(SeekFrom::Start(0));

    let mut info = AudioMediaInfo {
        format: Some("WAV".to_string()),
        ..Default::default()
    };

    let mut hdr = [0u8; 12];
    if reader.read_exact(&mut hdr).is_err() || &hdr[0..4] != b"RIFF" || &hdr[8..12] != b"WAVE" {
        return info;
    }

    let mut byte_rate: u32 = 0;
    let mut data_size: u64 = 0;
    let mut pos: u64 = 12;

    for _ in 0..64 {
        if pos + 8 > file_len || reader.seek(SeekFrom::Start(pos)).is_err() {
            break;
        }
        let mut chdr = [0u8; 8];
        if reader.read_exact(&mut chdr).is_err() {
            break;
        }
        let cid = [chdr[0], chdr[1], chdr[2], chdr[3]];
        let csize = u32::from_le_bytes([chdr[4], chdr[5], chdr[6], chdr[7]]) as u64;
        if &cid == b"fmt " && csize >= 16 && csize <= 1024 {
            let mut buf = vec![0u8; csize as usize];
            if reader.read_exact(&mut buf).is_ok() {
                let ch = u16::from_le_bytes([buf[2], buf[3]]) as u32;
                let sr = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
                byte_rate = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
                if ch > 0 {
                    info.channels = Some(ch);
                }
                if sr > 0 {
                    info.sample_rate_hz = Some(sr);
                }
                if byte_rate > 0 {
                    info.bitrate_kbps = Some((byte_rate * 8) / 1000);
                }
            }
        } else if &cid == b"data" {
            data_size = csize;
        } else if &cid == b"LIST" && csize >= 4 && csize <= 256 * 1024 {
            let mut buf = vec![0u8; csize as usize];
            if reader.read_exact(&mut buf).is_ok() && &buf[0..4] == b"INFO" {
                let mut ipos = 4usize;
                while ipos + 8 <= buf.len() {
                    let ik = &buf[ipos..ipos + 4];
                    let isz = u32::from_le_bytes(buf[ipos + 4..ipos + 8].try_into().unwrap()) as usize;
                    let start = ipos + 8;
                    let end = start.saturating_add(isz).min(buf.len());
                    let val = String::from_utf8_lossy(&buf[start..end]);
                    match ik {
                        b"INAM" if info.title.is_none() => info.title = clean_tag_text(&val),
                        b"IART" if info.artist.is_none() => info.artist = clean_tag_text(&val),
                        b"IPRD" if info.album.is_none() => info.album = clean_tag_text(&val),
                        b"ICRD" if info.year.is_none() => info.year = extract_year_4(&val),
                        b"IGNR" if info.genre.is_none() => info.genre = clean_tag_text(&val),
                        _ => {}
                    }
                    ipos = start.saturating_add(isz + (isz & 1));
                }
            }
        }
        pos = pos.saturating_add(8).saturating_add(csize + (csize & 1));
    }

    if byte_rate > 0 && data_size > 0 {
        info.duration_secs = Some(data_size as f64 / byte_rate as f64);
    }

    info
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn id3_frame(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(10 + payload.len());
        out.extend_from_slice(id);
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(payload);
        out
    }

    fn atom(kind: &[u8; 4], pay: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + pay.len());
        out.extend_from_slice(&((8 + pay.len()) as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(pay);
        out
    }

    #[test]
    fn test_parse_mp3_id3v2_and_cover() {
        let mut frames = Vec::new();
        frames.extend_from_slice(&id3_frame(b"TIT2", &[3, b'N', b'i', b'g', b'h', b't']));
        frames.extend_from_slice(&id3_frame(b"TPE1", &[3, b'B', b'a', b'n', b'd']));
        frames.extend_from_slice(&id3_frame(b"TALB", &[3, b'L', b'i', b'v', b'e']));
        frames.extend_from_slice(&id3_frame(b"TRCK", &[3, b'3', b'/', b'1', b'0']));
        frames.extend_from_slice(&id3_frame(b"TYER", &[3, b'2', b'0', b'2', b'5']));
        frames.extend_from_slice(&id3_frame(b"TCON", &[3, b'(', b'1', b'7', b')']));

        let mut apic = vec![0];
        apic.extend_from_slice(b"image/png\0");
        apic.push(3); // front cover
        apic.push(0); // empty desc
        apic.extend_from_slice(b"\x89PNG\r\n\x1a\nFAKE_PNG_DATA_1234");
        frames.extend_from_slice(&id3_frame(b"APIC", &apic));

        let sz = frames.len() as u32;
        let ss = [
            ((sz >> 21) & 0x7F) as u8,
            ((sz >> 14) & 0x7F) as u8,
            ((sz >> 7) & 0x7F) as u8,
            (sz & 0x7F) as u8,
        ];
        let mut mp3 = Vec::new();
        mp3.extend_from_slice(b"ID3\x03\x00\x00");
        mp3.extend_from_slice(&ss);
        mp3.extend_from_slice(&frames);
        // MPEG-1 Layer III, 320 kbps (idx=14), 44100 Hz (idx=0), stereo (mode=0)
        mp3.extend_from_slice(&[0xFF, 0xFB, 0xE0, 0x00]);
        mp3.extend_from_slice(&[0u8; 4000]);

        let len = mp3.len() as u64;
        let mut cursor = Cursor::new(mp3);
        let (info, cover) = parse_mp3_reader(&mut cursor, len, true);
        assert_eq!(info.title.as_deref(), Some("Night"));
        assert_eq!(info.artist.as_deref(), Some("Band"));
        assert_eq!(info.album.as_deref(), Some("Live"));
        assert_eq!(info.track, Some(3));
        assert_eq!(info.track_tag().as_deref(), Some("03"));
        assert_eq!(info.year.as_deref(), Some("2025"));
        assert_eq!(info.genre.as_deref(), Some("Rock"));
        assert_eq!(info.bitrate_kbps, Some(320));
        assert_eq!(info.sample_rate_hz, Some(44100));
        assert_eq!(info.channels, Some(2));
        assert!(cover.unwrap().starts_with(b"\x89PNG"));
    }

    #[test]
    fn test_parse_flac_in_memory() {
        let mut si = vec![0u8; 34];
        // sample rate = 44100 (0x0AC44), ch=2 -> (1<<1)=2, bps=16 -> 15
        let sr: u32 = 44100;
        si[10] = (sr >> 12) as u8;
        si[11] = ((sr >> 4) & 0xFF) as u8;
        si[12] = (((sr & 0x0F) as u8) << 4) | (1 << 1);
        let total_samples: u32 = 44100 * 120; // 2 min
        si[14..18].copy_from_slice(&total_samples.to_be_bytes());

        let mut vc = Vec::new();
        vc.extend_from_slice(&4u32.to_le_bytes());
        vc.extend_from_slice(b"Test");
        let comments = ["ARTIST=Daft", "TITLE=One", "ALBUM=Discovery", "TRACKNUMBER=01", "DATE=2001"];
        vc.extend_from_slice(&(comments.len() as u32).to_le_bytes());
        for c in comments {
            vc.extend_from_slice(&(c.len() as u32).to_le_bytes());
            vc.extend_from_slice(c.as_bytes());
        }

        let mut flac = Vec::from(b"fLaC".as_slice());
        // block 0 STREAMINFO (not last)
        flac.push(0x00);
        flac.extend_from_slice(&(si.len() as u32).to_be_bytes()[1..4]);
        flac.extend_from_slice(&si);
        // block 4 VORBIS_COMMENT (last)
        flac.push(0x84);
        flac.extend_from_slice(&(vc.len() as u32).to_be_bytes()[1..4]);
        flac.extend_from_slice(&vc);

        let len = flac.len() as u64;
        let mut cursor = Cursor::new(flac);
        let (info, _) = parse_flac_reader(&mut cursor, len, false);
        assert_eq!(info.artist.as_deref(), Some("Daft"));
        assert_eq!(info.title.as_deref(), Some("One"));
        assert_eq!(info.album.as_deref(), Some("Discovery"));
        assert_eq!(info.track, Some(1));
        assert_eq!(info.year.as_deref(), Some("2001"));
        assert_eq!(info.sample_rate_hz, Some(44100));
        assert_eq!(info.sample_rate_tag().as_deref(), Some("44.1kHz"));
        assert!((info.duration_secs.unwrap() - 120.0).abs() < 0.01);
        assert_eq!(info.duration_tag().as_deref(), Some("02m00s"));
    }

    #[test]
    fn test_parse_m4a_in_memory() {
        let mut mvhd_pay = vec![0u8; 100];
        mvhd_pay[12..16].copy_from_slice(&1000u32.to_be_bytes());
        mvhd_pay[16..20].copy_from_slice(&95_000u32.to_be_bytes());

        let data_txt = |s: &[u8]| {
            let mut d = vec![0, 0, 0, 1, 0, 0, 0, 0];
            d.extend_from_slice(s);
            atom(b"data", &d)
        };
        let mut ilst_pay = Vec::new();
        ilst_pay.extend_from_slice(&atom(b"\xa9nam", &data_txt(b"Solar")));
        ilst_pay.extend_from_slice(&atom(b"\xa9ART", &data_txt(b"Echo")));
        let trkn_data = [0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 10, 0, 0];
        ilst_pay.extend_from_slice(&atom(b"trkn", &atom(b"data", &trkn_data)));

        let mut meta_pay = vec![0u8; 4];
        meta_pay.extend_from_slice(&atom(b"ilst", &ilst_pay));
        let udta = atom(b"udta", &atom(b"meta", &meta_pay));
        let moov = atom(b"moov", &[atom(b"mvhd", &mvhd_pay), udta].concat());
        let m4a = [atom(b"ftyp", b"M4A \x00\x00\x00\x00M4A "), moov].concat();

        let len = m4a.len() as u64;
        let mut cursor = Cursor::new(m4a);
        let (info, _) = parse_m4a_reader(&mut cursor, len, false);
        assert_eq!(info.title.as_deref(), Some("Solar"));
        assert_eq!(info.artist.as_deref(), Some("Echo"));
        assert_eq!(info.track, Some(5));
        assert!((info.duration_secs.unwrap() - 95.0).abs() < 0.01);
    }
}
