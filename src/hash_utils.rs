use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct DuplicateGroup {
    pub sha256: String,
    pub file_size: u64,
    pub paths: Vec<PathBuf>,
}

pub enum DupScanMsg {
    Progress { done: usize, total: usize },
    Done(Vec<DuplicateGroup>),
}

pub fn format_size_tag(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{}KB", (bytes as f64 / 1024.0).round() as u64)
    } else if bytes < 1024 * 1024 * 1024 {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        if mb >= 10.0 {
            format!("{}MB", mb.round() as u64)
        } else {
            let s = format!("{:.1}", mb);
            format!("{}MB", s.strip_suffix(".0").unwrap_or(&s))
        }
    } else {
        let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let s = format!("{:.1}", gb);
        format!("{}GB", s.strip_suffix(".0").unwrap_or(&s))
    }
}

const fn make_crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if (crc & 1) != 0 {
                crc = 0xEDB8_8320 ^ (crc >> 1);
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

const CRC32_TABLE: [u32; 256] = make_crc32_table();

pub fn crc32_update(mut crc: u32, data: &[u8]) -> u32 {
    crc = !crc;
    for &b in data {
        let idx = ((crc ^ (b as u32)) & 0xFF) as usize;
        crc = CRC32_TABLE[idx] ^ (crc >> 8);
    }
    !crc
}

pub fn crc32_bytes(data: &[u8]) -> u32 {
    crc32_update(0, data)
}

pub fn file_crc32_hex(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut buf = vec![0u8; 65536];
    let mut crc = 0u32;
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        crc = crc32_update(crc, &buf[..n]);
    }
    Some(format!("{:08X}", crc))
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

pub struct Sha256 {
    state: [u32; 8],
    bit_len: u64,
    buf: [u8; 64],
    buf_len: usize,
}

impl Sha256 {
    pub fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            bit_len: 0,
            buf: [0u8; 64],
            buf_len: 0,
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.bit_len = self.bit_len.wrapping_add((data.len() as u64).wrapping_mul(8));
        let mut offset = 0;

        if self.buf_len > 0 {
            let take = (64 - self.buf_len).min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            offset += take;
            if self.buf_len == 64 {
                let block = self.buf;
                self.compress(&block);
                self.buf_len = 0;
            }
        }

        while offset + 64 <= data.len() {
            let block: &[u8; 64] = data[offset..offset + 64].try_into().unwrap();
            self.compress(block);
            offset += 64;
        }

        if offset < data.len() {
            let rem = data.len() - offset;
            self.buf[..rem].copy_from_slice(&data[offset..]);
            self.buf_len = rem;
        }
    }

    pub fn finalize(mut self) -> [u8; 32] {
        let total_bits = self.bit_len;
        self.buf[self.buf_len] = 0x80;
        self.buf_len += 1;

        if self.buf_len > 56 {
            for b in &mut self.buf[self.buf_len..64] {
                *b = 0;
            }
            let block = self.buf;
            self.compress(&block);
            self.buf_len = 0;
        }

        for b in &mut self.buf[self.buf_len..56] {
            *b = 0;
        }
        self.buf[56..64].copy_from_slice(&total_bits.to_be_bytes());
        let block = self.buf;
        self.compress(&block);

        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

fn to_hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    to_hex_lower(&hasher.finalize())
}

pub fn file_sha256_hex(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut buf = vec![0u8; 65536];
    let mut hasher = Sha256::new();
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(to_hex_lower(&hasher.finalize()))
}

pub fn file_sha256_8(path: &Path) -> Option<String> {
    file_sha256_hex(path).map(|s| s.chars().take(8).collect())
}

pub fn file_partial_sha256(path: &Path, file_size: u64) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&file_size.to_le_bytes());

    if file_size <= 131_072 {
        let mut buf = vec![0u8; 65536];
        loop {
            let n = file.read(&mut buf).ok()?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
    } else {
        let mut head = vec![0u8; 65536];
        file.read_exact(&mut head).ok()?;
        hasher.update(&head);

        file.seek(SeekFrom::Start(file_size - 65536)).ok()?;
        let mut tail = vec![0u8; 65536];
        file.read_exact(&mut tail).ok()?;
        hasher.update(&tail);
    }

    Some(to_hex_lower(&hasher.finalize()))
}

pub fn find_duplicates_bg(
    candidates: Vec<(PathBuf, u64)>,
    cancel: Arc<AtomicBool>,
    on_progress: impl Fn(usize, usize),
) -> Vec<DuplicateGroup> {
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    for (p, sz) in candidates {
        if sz > 0 {
            by_size.entry(sz).or_default().push(p);
        }
    }

    let size_groups: Vec<(u64, Vec<PathBuf>)> = by_size
        .into_iter()
        .filter(|(_, list)| list.len() >= 2)
        .collect();

    let stage2_total: usize = size_groups.iter().map(|(_, list)| list.len()).sum();
    if stage2_total == 0 {
        return Vec::new();
    }

    let mut done = 0usize;
    let mut partial_buckets: HashMap<(u64, String), Vec<PathBuf>> = HashMap::new();

    for (sz, paths) in size_groups {
        for p in paths {
            if cancel.load(Ordering::Relaxed) {
                return Vec::new();
            }
            if let Some(phash) = file_partial_sha256(&p, sz) {
                partial_buckets.entry((sz, phash)).or_default().push(p);
            }
            done += 1;
            on_progress(done, stage2_total);
        }
    }

    let full_candidates: Vec<(u64, Vec<PathBuf>)> = partial_buckets
        .into_iter()
        .filter(|(_, list)| list.len() >= 2)
        .map(|((sz, _), list)| (sz, list))
        .collect();

    let mut exact_buckets: HashMap<(u64, String), Vec<PathBuf>> = HashMap::new();
    for (sz, paths) in full_candidates {
        for p in paths {
            if cancel.load(Ordering::Relaxed) {
                return Vec::new();
            }
            if let Some(fhash) = file_sha256_hex(&p) {
                exact_buckets.entry((sz, fhash)).or_default().push(p);
            }
        }
    }

    let mut groups: Vec<DuplicateGroup> = exact_buckets
        .into_iter()
        .filter(|(_, list)| list.len() >= 2)
        .map(|((sz, hash), mut paths)| {
            paths.sort();
            DuplicateGroup {
                sha256: hash,
                file_size: sz,
                paths,
            }
        })
        .collect();

    groups.sort_by(|a, b| {
        let waste_a = a.file_size.saturating_mul((a.paths.len() - 1) as u64);
        let waste_b = b.file_size.saturating_mul((b.paths.len() - 1) as u64);
        waste_b.cmp(&waste_a).then_with(|| a.sha256.cmp(&b.sha256))
    });

    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_crc32_and_sha256() {
        assert_eq!(crc32_bytes(b"123456789"), 0xCBF4_3926);
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_size_tag_formatting() {
        assert_eq!(format_size_tag(512), "512B");
        assert_eq!(format_size_tag(1536), "2KB");
        assert_eq!(format_size_tag(4_500_000), "4.3MB");
        assert_eq!(format_size_tag(25_000_000), "24MB");
        assert_eq!(format_size_tag(1_610_612_736), "1.5GB");
    }

    #[test]
    fn test_duplicate_finder_on_temp_files() {
        let dir = std::env::temp_dir().join(format!("restem_dup_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let a = dir.join("copy1.bin");
        let b = dir.join("copy2.bin");
        let c = dir.join("diff_same_size.bin");
        let d = dir.join("unique.bin");

        fs::write(&a, b"HELLO_WORLD_12345").unwrap();
        fs::write(&b, b"HELLO_WORLD_12345").unwrap();
        fs::write(&c, b"HELLO_WORLD_99999").unwrap();
        fs::write(&d, b"SHORT").unwrap();

        let candidates = vec![
            (a.clone(), 17),
            (b.clone(), 17),
            (c.clone(), 17),
            (d.clone(), 5),
        ];

        let cancel = Arc::new(AtomicBool::new(false));
        let groups = find_duplicates_bg(candidates, cancel, |_, _| {});
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].file_size, 17);
        assert_eq!(groups[0].paths.len(), 2);
        assert!(groups[0].paths.contains(&a));
        assert!(groups[0].paths.contains(&b));

        let _ = fs::remove_dir_all(&dir);
    }
}
