use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PhotoMeta {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub exif_date: Option<String>,
    pub exif_time: Option<String>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<u32>,
    pub aperture: Option<f64>,
    pub shutter: Option<(u32, u32)>,
}

impl PhotoMeta {
    pub fn dimensions(&self) -> Option<(u32, u32)> {
        match (self.width, self.height) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some((w, h)),
            _ => None,
        }
    }

    pub fn iso_tag(&self) -> Option<String> {
        self.iso.filter(|&i| i > 0).map(|i| format!("ISO{}", i))
    }

    pub fn aperture_tag(&self) -> Option<String> {
        let f = self.aperture?;
        if !f.is_finite() || f <= 0.0 {
            return None;
        }
        let s = format!("{:.1}", f);
        let trimmed = s.strip_suffix(".0").unwrap_or(&s);
        Some(format!("f{}", trimmed))
    }

    pub fn shutter_tag(&self) -> Option<String> {
        let (num, den) = self.shutter?;
        if num == 0 || den == 0 {
            return None;
        }
        if num < den {
            let inv = (den as f64 / num as f64).round() as u32;
            Some(format!("1-{}s", inv.max(1)))
        } else if num % den == 0 {
            Some(format!("{}s", num / den))
        } else {
            let secs = num as f64 / den as f64;
            let s = format!("{:.1}", secs);
            Some(format!("{}s", s.strip_suffix(".0").unwrap_or(&s)))
        }
    }

    pub fn shutter_display(&self) -> Option<String> {
        let (num, den) = self.shutter?;
        if num == 0 || den == 0 {
            return None;
        }
        if num < den {
            let inv = (den as f64 / num as f64).round() as u32;
            Some(format!("1/{}s", inv.max(1)))
        } else if num % den == 0 {
            Some(format!("{}s", num / den))
        } else {
            let secs = num as f64 / den as f64;
            let s = format!("{:.1}", secs);
            Some(format!("{}s", s.strip_suffix(".0").unwrap_or(&s)))
        }
    }

    pub fn has_exif(&self) -> bool {
        self.exif_date.is_some()
            || self.exif_time.is_some()
            || self.camera.is_some()
            || self.lens.is_some()
            || self.iso.is_some()
            || self.aperture.is_some()
            || self.shutter.is_some()
    }
}

pub fn is_previewable_image_ext(ext: &str) -> bool {
    matches!(
        ext,
        "jpg" | "jpeg" | "png" | "webp" | "bmp" | "ico" | "gif"
    )
}

pub fn is_photo_meta_ext(ext: &str) -> bool {
    is_previewable_image_ext(ext)
        || matches!(
            ext,
            "heic" | "heif" | "tif" | "tiff" | "dng" | "cr2" | "nef" | "arw" | "orf" | "rw2"
        )
}

fn sanitize_model_tag(val: &str) -> Option<String> {
    let s: String = val
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '-' || ch == '.' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let mut collapsed = String::with_capacity(s.len());
    let mut prev_under = false;
    for ch in s.chars() {
        if ch == '_' {
            if !prev_under {
                collapsed.push('_');
                prev_under = true;
            }
        } else {
            collapsed.push(ch);
            prev_under = false;
        }
    }
    let trimmed = collapsed.trim_matches('_').to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn parse_exif_datetime(raw: &str) -> (Option<String>, Option<String>) {
    let trimmed = raw.trim();
    let date_part = if trimmed.len() >= 10 {
        let d = &trimmed[0..10];
        let clean: String = d.chars().map(|c| if c == ':' { '-' } else { c }).collect();
        if clean.chars().take(4).all(|c| c.is_ascii_digit()) {
            Some(clean)
        } else {
            None
        }
    } else {
        None
    };

    let time_part = if trimmed.len() >= 19 {
        let t = &trimmed[11..19];
        let clean: String = t.chars().map(|c| if c == ':' { '-' } else { c }).collect();
        if clean.chars().take(2).all(|c| c.is_ascii_digit()) {
            Some(clean)
        } else {
            None
        }
    } else {
        None
    };

    (date_part, time_part)
}

pub fn parse_tiff_exif_full(tiff: &[u8]) -> PhotoMeta {
    let mut meta = PhotoMeta::default();
    if tiff.len() < 8 {
        return meta;
    }
    let le = match &tiff[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return meta,
    };

    let r16 = |offset: usize| -> Option<u16> {
        let b = tiff.get(offset..offset + 2)?;
        Some(if le {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            u16::from_be_bytes([b[0], b[1]])
        })
    };
    let r32 = |offset: usize| -> Option<u32> {
        let b = tiff.get(offset..offset + 4)?;
        Some(if le {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        })
    };
    let read_ascii = |val_offset: usize, count: usize| -> Option<String> {
        let slice = tiff.get(val_offset..val_offset + count)?;
        let s = String::from_utf8_lossy(slice)
            .trim_matches(char::from(0))
            .trim()
            .to_string();
        if s.is_empty() { None } else { Some(s) }
    };
    let read_rational = |val_offset: usize| -> Option<(u32, u32)> {
        let num = r32(val_offset)?;
        let den = r32(val_offset + 4)?;
        if den == 0 {
            None
        } else {
            Some((num, den))
        }
    };

    let Some(ifd0_off) = r32(4).map(|v| v as usize) else {
        return meta;
    };

    let mut exif_sub_ifd: Option<usize> = None;

    if let Some(entries) = r16(ifd0_off) {
        for i in 0..(entries as usize).min(256) {
            let entry_off = ifd0_off + 2 + i * 12;
            let (Some(tag), Some(dtype), Some(count), Some(val_raw)) = (
                r16(entry_off),
                r16(entry_off + 2),
                r32(entry_off + 4),
                r32(entry_off + 8),
            ) else {
                break;
            };

            if tag == 0x8769 {
                exif_sub_ifd = Some(val_raw as usize);
            } else if tag == 0x0100 && meta.width.is_none() {
                let w = if dtype == 3 {
                    r16(entry_off + 8).map(|v| v as u32)
                } else {
                    Some(val_raw)
                };
                if let Some(w) = w.filter(|&v| v > 0) {
                    meta.width = Some(w);
                }
            } else if tag == 0x0101 && meta.height.is_none() {
                let h = if dtype == 3 {
                    r16(entry_off + 8).map(|v| v as u32)
                } else {
                    Some(val_raw)
                };
                if let Some(h) = h.filter(|&v| v > 0) {
                    meta.height = Some(h);
                }
            } else if dtype == 2 && (tag == 0x0110 || tag == 0x0132) {
                let c = count as usize;
                let str_off = if c <= 4 { entry_off + 8 } else { val_raw as usize };
                if let Some(val) = read_ascii(str_off, c) {
                    if tag == 0x0110 {
                        meta.camera = sanitize_model_tag(&val);
                    } else if tag == 0x0132 && meta.exif_date.is_none() {
                        let (d, t) = parse_exif_datetime(&val);
                        meta.exif_date = d;
                        if meta.exif_time.is_none() {
                            meta.exif_time = t;
                        }
                    }
                }
            }
        }
    }

    if let Some(sub_off) = exif_sub_ifd {
        if let Some(entries) = r16(sub_off) {
            for i in 0..(entries as usize).min(256) {
                let entry_off = sub_off + 2 + i * 12;
                let (Some(tag), Some(dtype), Some(count), Some(val_raw)) = (
                    r16(entry_off),
                    r16(entry_off + 2),
                    r32(entry_off + 4),
                    r32(entry_off + 8),
                ) else {
                    break;
                };

                match tag {
                    0x9003 | 0x9004 if dtype == 2 => {
                        let c = count as usize;
                        let str_off = if c <= 4 { entry_off + 8 } else { val_raw as usize };
                        if let Some(val) = read_ascii(str_off, c) {
                            let (d, t) = parse_exif_datetime(&val);
                            if d.is_some() && (tag == 0x9003 || meta.exif_date.is_none()) {
                                meta.exif_date = d;
                                if t.is_some() {
                                    meta.exif_time = t;
                                }
                            }
                        }
                    }
                    0xA434 if dtype == 2 && meta.lens.is_none() => {
                        let c = count as usize;
                        let str_off = if c <= 4 { entry_off + 8 } else { val_raw as usize };
                        if let Some(val) = read_ascii(str_off, c) {
                            meta.lens = sanitize_model_tag(&val);
                        }
                    }
                    0x8827 | 0x8833 if meta.iso.is_none() => {
                        let iso = if dtype == 3 {
                            r16(entry_off + 8).map(|v| v as u32)
                        } else if dtype == 4 {
                            Some(val_raw)
                        } else {
                            None
                        };
                        if let Some(i) = iso.filter(|&v| v > 0) {
                            meta.iso = Some(i);
                        }
                    }
                    0x829D if dtype == 5 && meta.aperture.is_none() => {
                        if let Some((num, den)) = read_rational(val_raw as usize) {
                            let f = num as f64 / den as f64;
                            if f > 0.0 && f.is_finite() {
                                meta.aperture = Some(f);
                            }
                        }
                    }
                    0x829A if dtype == 5 && meta.shutter.is_none() => {
                        if let Some((num, den)) = read_rational(val_raw as usize) {
                            if num > 0 {
                                meta.shutter = Some((num, den));
                            }
                        }
                    }
                    0xA002 => {
                        let w = if dtype == 3 {
                            r16(entry_off + 8).map(|v| v as u32)
                        } else {
                            Some(val_raw)
                        };
                        if let Some(w) = w.filter(|&v| v > 0) {
                            meta.width = Some(w);
                        }
                    }
                    0xA003 => {
                        let h = if dtype == 3 {
                            r16(entry_off + 8).map(|v| v as u32)
                        } else {
                            Some(val_raw)
                        };
                        if let Some(h) = h.filter(|&v| v > 0) {
                            meta.height = Some(h);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    meta
}

pub fn extract_photo_meta(path: &Path, ext_lower: &str) -> PhotoMeta {
    let mut meta = match ext_lower {
        "jpg" | "jpeg" => parse_jpeg_exif(path),
        "png" => parse_png_exif(path),
        "webp" => parse_webp_exif(path),
        "heic" | "heif" => parse_heic_exif(path),
        "tif" | "tiff" | "dng" | "cr2" | "nef" | "arw" | "orf" | "rw2" => parse_tiff_file(path),
        _ => PhotoMeta::default(),
    };

    if (meta.width.is_none() || meta.height.is_none()) && is_previewable_image_ext(ext_lower) {
        if let Ok((w, h)) = image::image_dimensions(path) {
            if w > 0 && h > 0 {
                meta.width = Some(w);
                meta.height = Some(h);
            }
        }
    }

    meta
}

fn parse_tiff_file(path: &Path) -> PhotoMeta {
    let Ok(mut file) = File::open(path) else {
        return PhotoMeta::default();
    };
    let mut buf = vec![0u8; 256 * 1024];
    let Ok(n) = file.read(&mut buf) else {
        return PhotoMeta::default();
    };
    buf.truncate(n);
    parse_tiff_exif_full(&buf)
}

pub fn parse_jpeg_exif_bytes(buf: &[u8]) -> PhotoMeta {
    if buf.len() < 4 || buf[0] != 0xFF || buf[1] != 0xD8 {
        return PhotoMeta::default();
    }
    let mut pos = 2;
    while pos + 4 <= buf.len() {
        if buf[pos] != 0xFF {
            break;
        }
        let marker = buf[pos + 1];
        if marker == 0xDA || marker == 0xD9 {
            break;
        }
        let seg_len = u16::from_be_bytes([buf[pos + 2], buf[pos + 3]]) as usize;
        if seg_len < 2 || pos + 2 + seg_len > buf.len() {
            break;
        }
        if marker == 0xE1 && seg_len >= 14 {
            let seg = &buf[pos + 4..pos + 2 + seg_len];
            if seg.starts_with(b"Exif\0\0") {
                return parse_tiff_exif_full(&seg[6..]);
            }
        }
        pos += 2 + seg_len;
    }
    PhotoMeta::default()
}

fn parse_jpeg_exif(path: &Path) -> PhotoMeta {
    let Ok(mut file) = File::open(path) else {
        return PhotoMeta::default();
    };
    let mut buf = vec![0u8; 128 * 1024];
    let Ok(n) = file.read(&mut buf) else {
        return PhotoMeta::default();
    };
    buf.truncate(n);
    parse_jpeg_exif_bytes(&buf)
}

pub fn parse_png_exif_reader<R: Read + Seek>(reader: &mut R) -> PhotoMeta {
    let mut sig = [0u8; 8];
    if reader.read_exact(&mut sig).is_err() || &sig != b"\x89PNG\r\n\x1a\n" {
        return PhotoMeta::default();
    }

    let mut meta = PhotoMeta::default();
    for _ in 0..128 {
        let mut chdr = [0u8; 8];
        if reader.read_exact(&mut chdr).is_err() {
            break;
        }
        let clen = u32::from_be_bytes([chdr[0], chdr[1], chdr[2], chdr[3]]) as u64;
        let ctype = [chdr[4], chdr[5], chdr[6], chdr[7]];

        if &ctype == b"IHDR" && clen >= 8 && clen <= 1024 {
            let mut buf = vec![0u8; clen as usize];
            if reader.read_exact(&mut buf).is_ok() {
                let w = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                let h = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
                if w > 0 && h > 0 {
                    meta.width = Some(w);
                    meta.height = Some(h);
                }
            }
            let _ = reader.seek(SeekFrom::Current(4));
        } else if &ctype == b"eXIf" && clen >= 8 && clen <= 512 * 1024 {
            let mut buf = vec![0u8; clen as usize];
            if reader.read_exact(&mut buf).is_ok() {
                let tiff = if buf.starts_with(b"Exif\0\0") {
                    &buf[6..]
                } else {
                    &buf[..]
                };
                let parsed = parse_tiff_exif_full(tiff);
                let w = meta.width.or(parsed.width);
                let h = meta.height.or(parsed.height);
                meta = parsed;
                meta.width = w;
                meta.height = h;
            }
            break;
        } else if &ctype == b"IEND" {
            break;
        } else if reader.seek(SeekFrom::Current(clen as i64 + 4)).is_err() {
            break;
        }
    }
    meta
}

fn parse_png_exif(path: &Path) -> PhotoMeta {
    let Ok(mut file) = File::open(path) else {
        return PhotoMeta::default();
    };
    parse_png_exif_reader(&mut file)
}

pub fn parse_webp_exif_reader<R: Read + Seek>(reader: &mut R) -> PhotoMeta {
    let mut hdr = [0u8; 12];
    if reader.read_exact(&mut hdr).is_err() || &hdr[0..4] != b"RIFF" || &hdr[8..12] != b"WEBP" {
        return PhotoMeta::default();
    }

    let mut meta = PhotoMeta::default();
    for _ in 0..64 {
        let mut chdr = [0u8; 8];
        if reader.read_exact(&mut chdr).is_err() {
            break;
        }
        let cid = [chdr[0], chdr[1], chdr[2], chdr[3]];
        let csize = u32::from_le_bytes([chdr[4], chdr[5], chdr[6], chdr[7]]) as u64;
        let padded = csize + (csize & 1);
        if &cid == b"EXIF" && csize >= 8 && csize <= 512 * 1024 {
            let mut buf = vec![0u8; csize as usize];
            if reader.read_exact(&mut buf).is_ok() {
                let tiff = if buf.starts_with(b"Exif\0\0") {
                    &buf[6..]
                } else {
                    &buf[..]
                };
                meta = parse_tiff_exif_full(tiff);
            }
            break;
        } else if reader.seek(SeekFrom::Current(padded as i64)).is_err() {
            break;
        }
    }
    meta
}

fn parse_webp_exif(path: &Path) -> PhotoMeta {
    let Ok(mut file) = File::open(path) else {
        return PhotoMeta::default();
    };
    parse_webp_exif_reader(&mut file)
}

fn next_heic_box<'a>(data: &'a [u8], pos: &mut usize) -> Option<([u8; 4], &'a [u8])> {
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

fn read_var_be(data: &[u8], pos: &mut usize, n: usize) -> Option<u64> {
    if *pos + n > data.len() {
        return None;
    }
    let mut v = 0u64;
    for i in 0..n {
        v = (v << 8) | (data[*pos + i] as u64);
    }
    *pos += n;
    Some(v)
}

fn parse_heic_exif(path: &Path) -> PhotoMeta {
    let Ok(mut file) = File::open(path) else {
        return PhotoMeta::default();
    };
    let Ok(file_len) = file.seek(SeekFrom::End(0)) else {
        return PhotoMeta::default();
    };
    let _ = file.seek(SeekFrom::Start(0));

    let mut meta = PhotoMeta::default();
    let mut pos = 0u64;
    let mut meta_buf: Option<Vec<u8>> = None;

    for _ in 0..64 {
        if pos + 8 > file_len || file.seek(SeekFrom::Start(pos)).is_err() {
            break;
        }
        let mut hdr = [0u8; 8];
        if file.read_exact(&mut hdr).is_err() {
            break;
        }
        let sz32 = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
        let kind = [hdr[4], hdr[5], hdr[6], hdr[7]];
        let (hlen, bsz) = if sz32 == 1 {
            let mut ext = [0u8; 8];
            if file.read_exact(&mut ext).is_err() {
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
        if &kind == b"meta" && bsz - hlen <= 4 * 1024 * 1024 {
            let mut buf = vec![0u8; (bsz - hlen) as usize];
            if file.read_exact(&mut buf).is_ok() {
                meta_buf = Some(buf);
            }
            break;
        }
        pos += bsz;
    }

    let Some(mbuf) = meta_buf else {
        return meta;
    };
    if mbuf.len() <= 4 {
        return meta;
    }

    let mut ispe_dims: Option<(u32, u32)> = None;
    let mut exif_item_id: Option<u32> = None;
    let mut iloc_pay: Option<&[u8]> = None;

    let mut mpos = 4usize; // skip meta FullBox header
    while let Some((kind, pay)) = next_heic_box(&mbuf, &mut mpos) {
        if &kind == b"iprp" {
            let mut p1 = 0usize;
            while let Some((k1, v1)) = next_heic_box(pay, &mut p1) {
                if &k1 == b"ipco" {
                    let mut p2 = 0usize;
                    while let Some((k2, v2)) = next_heic_box(v1, &mut p2) {
                        if &k2 == b"ispe" && v2.len() >= 12 {
                            let w = u32::from_be_bytes(v2[4..8].try_into().unwrap());
                            let h = u32::from_be_bytes(v2[8..12].try_into().unwrap());
                            if w > 0 && h > 0 {
                                ispe_dims = Some((w, h));
                            }
                        }
                    }
                }
            }
        } else if &kind == b"iinf" && pay.len() > 6 {
            let ver = pay[0];
            let (count, mut ipos) = if ver == 0 {
                (u16::from_be_bytes([pay[4], pay[5]]) as usize, 6usize)
            } else if pay.len() >= 8 {
                (
                    u32::from_be_bytes(pay[4..8].try_into().unwrap()) as usize,
                    8usize,
                )
            } else {
                (0, pay.len())
            };
            for _ in 0..count.min(1024) {
                let Some((ik, ip)) = next_heic_box(pay, &mut ipos) else {
                    break;
                };
                if &ik == b"infe" && ip.len() >= 12 {
                    let iver = ip[0];
                    if iver == 2 && ip.len() >= 12 {
                        let id = u16::from_be_bytes([ip[4], ip[5]]) as u32;
                        if &ip[8..12] == b"Exif" {
                            exif_item_id = Some(id);
                        }
                    } else if iver == 3 && ip.len() >= 14 {
                        let id = u32::from_be_bytes(ip[4..8].try_into().unwrap());
                        if &ip[10..14] == b"Exif" {
                            exif_item_id = Some(id);
                        }
                    }
                }
            }
        } else if &kind == b"iloc" {
            iloc_pay = Some(pay);
        }
    }

    if let (Some(target_id), Some(iloc)) = (exif_item_id, iloc_pay) {
        if iloc.len() >= 8 {
            let ver = iloc[0];
            let off_sz = (iloc[4] >> 4) as usize;
            let len_sz = (iloc[4] & 0x0F) as usize;
            let base_sz = (iloc[5] >> 4) as usize;
            let idx_sz = if ver == 1 || ver == 2 {
                (iloc[5] & 0x0F) as usize
            } else {
                0
            };

            let mut p = 6usize;
            let item_count = if ver < 2 {
                read_var_be(iloc, &mut p, 2).unwrap_or(0) as usize
            } else {
                read_var_be(iloc, &mut p, 4).unwrap_or(0) as usize
            };

            for _ in 0..item_count.min(2048) {
                let id_sz = if ver < 2 { 2 } else { 4 };
                let Some(item_id) = read_var_be(iloc, &mut p, id_sz) else {
                    break;
                };
                if ver == 1 || ver == 2 {
                    let _ = read_var_be(iloc, &mut p, 2); // construction_method
                }
                let _ = read_var_be(iloc, &mut p, 2); // data_reference_index
                let Some(base_off) = read_var_be(iloc, &mut p, base_sz) else {
                    break;
                };
                let Some(ext_cnt) = read_var_be(iloc, &mut p, 2) else {
                    break;
                };
                let mut found_loc: Option<(u64, u64)> = None;
                for e_idx in 0..ext_cnt {
                    let _ = read_var_be(iloc, &mut p, idx_sz);
                    let Some(eo) = read_var_be(iloc, &mut p, off_sz) else {
                        break;
                    };
                    let Some(el) = read_var_be(iloc, &mut p, len_sz) else {
                        break;
                    };
                    if e_idx == 0 {
                        found_loc = Some((base_off.saturating_add(eo), el));
                    }
                }
                if item_id as u32 == target_id {
                    if let Some((abs_off, elen)) = found_loc {
                        if elen > 8 && elen <= 256 * 1024 && file.seek(SeekFrom::Start(abs_off)).is_ok() {
                            let mut ebuf = vec![0u8; elen as usize];
                            if file.read_exact(&mut ebuf).is_ok() && ebuf.len() > 8 {
                                let tiff_off = u32::from_be_bytes([
                                    ebuf[0], ebuf[1], ebuf[2], ebuf[3],
                                ]) as usize;
                                let start = 4usize.saturating_add(tiff_off);
                                if start < ebuf.len() {
                                    let slice = &ebuf[start..];
                                    let tiff = if slice.starts_with(b"Exif\0\0") {
                                        &slice[6..]
                                    } else {
                                        slice
                                    };
                                    meta = parse_tiff_exif_full(tiff);
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    if let Some((w, h)) = ispe_dims {
        if meta.width.is_none() || meta.height.is_none() {
            meta.width = Some(w);
            meta.height = Some(h);
        }
    }

    meta
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn build_sample_tiff_le() -> Vec<u8> {
        let mut buf = vec![0u8; 512];
        buf[0..4].copy_from_slice(b"II*\x00");
        buf[4..8].copy_from_slice(&8u32.to_le_bytes());

        // IFD0 at 8: 2 entries -> Model (0x0110), ExifSubIFD (0x8769)
        buf[8..10].copy_from_slice(&2u16.to_le_bytes());
        let model = b"Canon EOS R6\0";
        let model_off = 200usize;
        buf[model_off..model_off + model.len()].copy_from_slice(model);

        let e0 = 10usize;
        buf[e0..e0 + 2].copy_from_slice(&0x0110u16.to_le_bytes());
        buf[e0 + 2..e0 + 4].copy_from_slice(&2u16.to_le_bytes());
        buf[e0 + 4..e0 + 8].copy_from_slice(&(model.len() as u32).to_le_bytes());
        buf[e0 + 8..e0 + 12].copy_from_slice(&(model_off as u32).to_le_bytes());

        let sub_off = 60usize;
        let e1 = 22usize;
        buf[e1..e1 + 2].copy_from_slice(&0x8769u16.to_le_bytes());
        buf[e1 + 2..e1 + 4].copy_from_slice(&4u16.to_le_bytes());
        buf[e1 + 4..e1 + 8].copy_from_slice(&1u32.to_le_bytes());
        buf[e1 + 8..e1 + 12].copy_from_slice(&(sub_off as u32).to_le_bytes());

        // SubIFD at 60: 5 entries
        buf[sub_off..sub_off + 2].copy_from_slice(&5u16.to_le_bytes());
        let dt = b"2026:11:24 14:35:08\0";
        let dt_off = 240usize;
        buf[dt_off..dt_off + dt.len()].copy_from_slice(dt);

        let lens = b"RF24-70mm F2.8\0";
        let lens_off = 280usize;
        buf[lens_off..lens_off + lens.len()].copy_from_slice(lens);

        let fnum_off = 320usize;
        buf[fnum_off..fnum_off + 4].copy_from_slice(&28u32.to_le_bytes());
        buf[fnum_off + 4..fnum_off + 8].copy_from_slice(&10u32.to_le_bytes());

        let shut_off = 330usize;
        buf[shut_off..shut_off + 4].copy_from_slice(&1u32.to_le_bytes());
        buf[shut_off + 4..shut_off + 8].copy_from_slice(&500u32.to_le_bytes());

        let write_entry = |b: &mut [u8], idx: usize, tag: u16, dtype: u16, cnt: u32, val: u32| {
            let p = sub_off + 2 + idx * 12;
            b[p..p + 2].copy_from_slice(&tag.to_le_bytes());
            b[p + 2..p + 4].copy_from_slice(&dtype.to_le_bytes());
            b[p + 4..p + 8].copy_from_slice(&cnt.to_le_bytes());
            b[p + 8..p + 12].copy_from_slice(&val.to_le_bytes());
        };
        write_entry(&mut buf, 0, 0x9003, 2, dt.len() as u32, dt_off as u32);
        write_entry(&mut buf, 1, 0xA434, 2, lens.len() as u32, lens_off as u32);
        write_entry(&mut buf, 2, 0x8827, 3, 1, 400);
        write_entry(&mut buf, 3, 0x829D, 5, 1, fnum_off as u32);
        write_entry(&mut buf, 4, 0x829A, 5, 1, shut_off as u32);

        buf
    }

    #[test]
    fn test_extended_exif_across_formats() {
        let tiff = build_sample_tiff_le();

        // 1. Direct TIFF / RAW
        let m1 = parse_tiff_exif_full(&tiff);
        assert_eq!(m1.exif_date.as_deref(), Some("2026-11-24"));
        assert_eq!(m1.exif_time.as_deref(), Some("14-35-08"));
        assert_eq!(m1.camera.as_deref(), Some("Canon_EOS_R6"));
        assert_eq!(m1.lens.as_deref(), Some("RF24-70mm_F2.8"));
        assert_eq!(m1.iso_tag().as_deref(), Some("ISO400"));
        assert_eq!(m1.aperture_tag().as_deref(), Some("f2.8"));
        assert_eq!(m1.shutter_tag().as_deref(), Some("1-500s"));
        assert_eq!(m1.shutter_display().as_deref(), Some("1/500s"));

        // 2. JPEG APP1 wrapper
        let mut jpg = vec![0xFF, 0xD8, 0xFF, 0xE1];
        let seg_len = (2 + 6 + tiff.len()) as u16;
        jpg.extend_from_slice(&seg_len.to_be_bytes());
        jpg.extend_from_slice(b"Exif\0\0");
        jpg.extend_from_slice(&tiff);
        let m2 = parse_jpeg_exif_bytes(&jpg);
        assert_eq!(m2.iso_tag().as_deref(), Some("ISO400"));
        assert_eq!(m2.exif_time.as_deref(), Some("14-35-08"));

        // 3. PNG IHDR + eXIf
        let mut png = Vec::from(b"\x89PNG\r\n\x1a\n".as_slice());
        let mut ihdr = vec![0u8; 13];
        ihdr[0..4].copy_from_slice(&4000u32.to_be_bytes());
        ihdr[4..8].copy_from_slice(&3000u32.to_be_bytes());
        png.extend_from_slice(&(ihdr.len() as u32).to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&ihdr);
        png.extend_from_slice(&[0u8; 4]);
        png.extend_from_slice(&(tiff.len() as u32).to_be_bytes());
        png.extend_from_slice(b"eXIf");
        png.extend_from_slice(&tiff);
        png.extend_from_slice(&[0u8; 4]);
        let mut pcur = Cursor::new(png);
        let m3 = parse_png_exif_reader(&mut pcur);
        assert_eq!(m3.dimensions(), Some((4000, 3000)));
        assert_eq!(m3.camera.as_deref(), Some("Canon_EOS_R6"));

        // 4. WebP EXIF chunk
        let mut webp = Vec::new();
        let exif_chunk_len = (6 + tiff.len()) as u32;
        webp.extend_from_slice(b"RIFF");
        webp.extend_from_slice(&(4 + 8 + exif_chunk_len).to_le_bytes());
        webp.extend_from_slice(b"WEBP");
        webp.extend_from_slice(b"EXIF");
        webp.extend_from_slice(&exif_chunk_len.to_le_bytes());
        webp.extend_from_slice(b"Exif\0\0");
        webp.extend_from_slice(&tiff);
        let mut wcur = Cursor::new(webp);
        let m4 = parse_webp_exif_reader(&mut wcur);
        assert_eq!(m4.aperture_tag().as_deref(), Some("f2.8"));
        assert_eq!(m4.lens.as_deref(), Some("RF24-70mm_F2.8"));
    }
}
