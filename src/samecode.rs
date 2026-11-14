//! Whether two builds hold the same allocated code and data (bench-hashes #125).
//! `check` builds both library commits at one path and compares every allocated
//! section except the build-ID note, masking the declared
//! provenance strings that intentionally name different commits.
//!
//! Every other data byte participates, including panic source locations. A
//! filename pointer and length followed by two words can also describe ordinary
//! program data, such as a file's size limit (bench-hashes #219). Retaining those
//! words sends line-only location changes to normal measurement too.
//!
//! Both builds use one path because source paths and crate metadata also reach
//! the executable. Separate paths introduce differences unrelated to the change.
//! ELF64 little-endian only: `compare` returns None on other formats, and
//! `check` measures as before.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

const SHF_ALLOC: u64 = 2;
const SHT_NOBITS: u32 = 8;
const SHT_NOTE: u32 = 7;

/// Allocated sections: name, address, size and contents (empty for NOBITS).
struct Image {
    sections: BTreeMap<String, (u64, u64, Vec<u8>)>,
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(d[o..o + 2].try_into().unwrap())
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn u64_at(d: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(d[o..o + 8].try_into().unwrap())
}

/// A 64-bit little-endian ELF file's image; None for any other file.
fn read(path: &Path) -> Option<Image> {
    let d = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    if d.len() < 64 || &d[..4] != b"\x7fELF" || d[4] != 2 || d[5] != 1 {
        return None;
    }
    let shoff = u64_at(&d, 0x28) as usize;
    let (entsize, count, names) = (
        u16_at(&d, 0x3A) as usize,
        u16_at(&d, 0x3C) as usize,
        u16_at(&d, 0x3E) as usize,
    );
    let header = |i: usize| {
        let o = shoff + i * entsize;
        // name, type, flags, addr, offset, size
        (
            u32_at(&d, o),
            u32_at(&d, o + 4),
            u64_at(&d, o + 8),
            u64_at(&d, o + 16),
            u64_at(&d, o + 24) as usize,
            u64_at(&d, o + 32),
        )
    };
    let names_at = header(names).4;
    let name_of = |off: u32| {
        let start = names_at + off as usize;
        let end = start
            + d[start..]
                .iter()
                .position(|&b| b == 0)
                .expect("a section name ends");
        String::from_utf8_lossy(&d[start..end]).into_owned()
    };
    let mut sections = BTreeMap::new();
    for i in 0..count {
        let (name, kind, flags, addr, offset, size) = header(i);
        if flags & SHF_ALLOC == 0 {
            continue;
        }
        let name = name_of(name);
        if kind == SHT_NOTE && name == ".note.gnu.build-id" {
            continue;
        }
        let body = if kind == SHT_NOBITS {
            Vec::new()
        } else {
            d[offset..offset + size as usize].to_vec()
        };
        sections.insert(name, (addr, size, body));
    }
    Some(Image { sections })
}

/// `body` with every occurrence of each of `masks` zeroed.
fn mask_strings(body: &mut [u8], masks: &[&str]) {
    for m in masks.iter().map(|m| m.as_bytes()).filter(|m| !m.is_empty()) {
        let mut i = 0;
        while i + m.len() <= body.len() {
            if &body[i..i + m.len()] == m {
                body[i..i + m.len()].fill(b'0');
                i += m.len();
            } else {
                i += 1;
            }
        }
    }
}

/// What two builds hold.
#[derive(Debug, PartialEq)]
pub enum Verdict {
    /// The same code and data, as masked.
    Same,
    /// The sections that differ, and the functions (by name, demangled, the
    /// legacy hash suffix dropped): those of another size or on one side
    /// alone, where a change lands; and those of the same size and other
    /// bytes, among them every function whose calls' targets moved.
    Differs {
        sections: Vec<String>,
        resized: Vec<String>,
        changed: Vec<String>,
    },
}

/// Each function's sizes and bytes, by name (nm -S -C over the symbol table).
fn functions(
    path: &Path,
    text: Option<&(u64, u64, Vec<u8>)>,
) -> BTreeMap<String, Vec<(u64, Vec<u8>)>> {
    let out = Command::new("nm")
        .args(["-S", "-C", "--defined-only"])
        .arg(path)
        .output()
        .expect("nm runs");
    assert!(
        out.status.success(),
        "nm {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut found: BTreeMap<String, Vec<(u64, Vec<u8>)>> = BTreeMap::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let f: Vec<&str> = line.splitn(4, ' ').collect();
        if f.len() != 4 || !matches!(f[2], "t" | "T") {
            continue;
        }
        let (Ok(addr), Ok(size)) = (u64::from_str_radix(f[0], 16), u64::from_str_radix(f[1], 16))
        else {
            continue;
        };
        let name = match f[3].rfind("::h") {
            Some(i)
                if f[3].len() - i == 19 && f[3][i + 3..].bytes().all(|b| b.is_ascii_hexdigit()) =>
            {
                &f[3][..i]
            }
            _ => f[3],
        };
        let bytes = match text {
            Some((t, tsize, body)) if (*t..t + tsize).contains(&addr) => {
                let o = (addr - t) as usize;
                body[o..(o + size as usize).min(body.len())].to_vec()
            }
            _ => Vec::new(),
        };
        found
            .entry(name.to_string())
            .or_default()
            .push((size, bytes));
    }
    for v in found.values_mut() {
        v.sort();
    }
    found
}

/// Whether `a` and `b` hold the same allocated code/data, with `masks` (the
/// declared provenance strings) zeroed in both. The build-ID note is excluded;
/// other allocated notes participate. None where either file has another format.
pub fn compare(a: &Path, b: &Path, masks: &[&str]) -> Option<Verdict> {
    let (mut ia, mut ib) = (read(a)?, read(b)?);
    for image in [&mut ia, &mut ib] {
        for (_, _, body) in image.sections.values_mut() {
            mask_strings(body, masks);
        }
    }
    let names: BTreeSet<&String> = ia.sections.keys().chain(ib.sections.keys()).collect();
    let sections: Vec<String> = names
        .into_iter()
        .filter(|n| match (ia.sections.get(*n), ib.sections.get(*n)) {
            (Some((_, sa, ba)), Some((_, sb, bb))) => sa != sb || ba != bb,
            _ => true,
        })
        .cloned()
        .collect();
    if sections.is_empty() {
        return Some(Verdict::Same);
    }
    let (fa, fb) = (
        functions(a, ia.sections.get(".text")),
        functions(b, ib.sections.get(".text")),
    );
    let sizes = |f: &BTreeMap<String, Vec<(u64, Vec<u8>)>>, n: &String| {
        f.get(n)
            .map(|v| v.iter().map(|(s, _)| *s).collect::<Vec<_>>())
    };
    let (mut resized, mut changed) = (Vec::new(), Vec::new());
    for n in fa.keys().chain(fb.keys()).collect::<BTreeSet<_>>() {
        if sizes(&fa, n) != sizes(&fb, n) {
            resized.push(n.clone());
        } else if fa.get(n) != fb.get(n) {
            changed.push(n.clone());
        }
    }
    Some(Verdict::Differs {
        sections,
        resized,
        changed,
    })
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    /// A string the test binary holds, as the bench holds its commits.
    const MARK: &str = "0123456789abcdef0123456789abcdef01234567";

    #[inline(never)]
    fn marker_function(x: u64) -> u64 {
        std::hint::black_box(x).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x1234
    }

    #[repr(C)]
    struct FileLimit {
        path: &'static str,
        max_bytes: u32,
        flags: u32,
    }

    #[used]
    static FILE_LIMIT: FileLimit = FileLimit {
        path: "input.rs",
        max_bytes: 111,
        flags: 0,
    };

    fn symbol_address(exe: &Path, name: &str) -> u64 {
        let out = Command::new("nm")
            .args(["-S", "-C", "--defined-only"])
            .arg(exe)
            .output()
            .unwrap();
        assert!(out.status.success());
        let text = String::from_utf8_lossy(&out.stdout);
        let line = text
            .lines()
            .find(|line| line.contains(name))
            .expect("the named symbol");
        u64::from_str_radix(line.split_whitespace().next().unwrap(), 16).unwrap()
    }

    /// A copy of this test binary in a directory of the test's own, changed by `edit`.
    fn copy_with(tag: &str, edit: impl FnOnce(&mut Vec<u8>)) -> std::path::PathBuf {
        let exe = std::env::current_exe().unwrap();
        let dir = std::env::temp_dir().join(format!("samecode-{}-{tag}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let copy = dir.join("exe");
        let mut data = std::fs::read(&exe).unwrap();
        edit(&mut data);
        std::fs::write(&copy, data).unwrap();
        copy
    }

    /// The file offset of virtual address `addr` in `section` of `exe`.
    fn offset_of(exe: &[u8], section: &str, addr: u64) -> usize {
        let shoff = u64_at(exe, 0x28) as usize;
        let (entsize, count, names) = (
            u16_at(exe, 0x3A) as usize,
            u16_at(exe, 0x3C) as usize,
            u16_at(exe, 0x3E) as usize,
        );
        let names_at = u64_at(exe, shoff + names * entsize + 24) as usize;
        (0..count)
            .map(|i| shoff + i * entsize)
            .find(|&o| {
                let start = names_at + u32_at(exe, o) as usize;
                exe[start..].starts_with(section.as_bytes()) && exe[start + section.len()] == 0
            })
            .map(|o| u64_at(exe, o + 24) as usize + (addr - u64_at(exe, o + 16)) as usize)
            .expect("the section")
    }

    /// A fixed ELF64 relocatable object with two valid allocated notes.
    /// readelf/nm independently check this layout in the issue's evidence.
    fn note_object(build_id: u8, value: u32) -> Vec<u8> {
        let mut data = vec![0; 416];
        data[..8].copy_from_slice(b"\x7fELF\x02\x01\x01\0");
        let machine = if cfg!(target_arch = "aarch64") {
            183u16
        } else {
            62u16
        };
        for (at, word) in [
            (16, 1u16),
            (18, machine),
            (52, 64),
            (58, 64),
            (60, 4),
            (62, 1),
        ] {
            data[at..at + 2].copy_from_slice(&word.to_le_bytes());
        }
        data[20..24].copy_from_slice(&1u32.to_le_bytes());
        data[40..48].copy_from_slice(&160u64.to_le_bytes());
        data[64..104].copy_from_slice(b"\0.shstrtab\0.note.gnu.build-id\0.note.app\0");
        for (at, words) in [(104, [4u32, 16, 3]), (136, [4u32, 4, 1])] {
            for (i, word) in words.into_iter().enumerate() {
                data[at + i * 4..at + (i + 1) * 4].copy_from_slice(&word.to_le_bytes());
            }
        }
        data[116..120].copy_from_slice(b"GNU\0");
        data[120..136].fill(build_id);
        data[148..152].copy_from_slice(b"APP\0");
        data[152..156].copy_from_slice(&value.to_le_bytes());
        for (i, (name, kind, flags, offset, size, align)) in [
            (1u32, 3u32, 0u64, 64u64, 40u64, 1u64),
            (11, 7, 2, 104, 32, 4),
            (30, 7, 2, 136, 20, 4),
        ]
        .into_iter()
        .enumerate()
        {
            let at = 160 + (i + 1) * 64;
            data[at..at + 4].copy_from_slice(&name.to_le_bytes());
            data[at + 4..at + 8].copy_from_slice(&kind.to_le_bytes());
            for (field, word) in [(8, flags), (24, offset), (32, size), (48, align)] {
                data[at + field..at + field + 8].copy_from_slice(&word.to_le_bytes());
            }
        }
        data
    }

    #[test]
    fn only_the_build_id_note_is_ignored() {
        let dir = std::env::temp_dir().join(format!("samecode-{}-notes", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (base, id, app) = (dir.join("base.o"), dir.join("id.o"), dir.join("app.o"));
        std::fs::write(&base, note_object(1, 111)).unwrap();
        std::fs::write(&id, note_object(2, 111)).unwrap();
        std::fs::write(&app, note_object(1, 222)).unwrap();
        assert_eq!(compare(&base, &id, &[]), Some(Verdict::Same));
        assert!(
            matches!(compare(&base, &app, &[]),
                Some(Verdict::Differs { sections, resized, changed })
                    if sections == [".note.app"] && resized.is_empty() && changed.is_empty()
            ),
            "ordinary allocated notes can affect program behavior"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_build_is_the_same_as_itself() {
        let exe = std::env::current_exe().unwrap();
        assert_eq!(compare(&exe, &exe, &[]), Some(Verdict::Same));
        std::hint::black_box(MARK);
    }

    #[test]
    fn a_changed_function_differs_by_name() {
        let exe = std::env::current_exe().unwrap();
        assert_eq!(
            marker_function(1),
            std::hint::black_box(1u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x1234
        );
        let image = read(&exe).unwrap();
        let fs = functions(&exe, image.sections.get(".text"));
        let (name, _) = fs
            .iter()
            .find(|(n, _)| n.ends_with("samecode::tests::marker_function"))
            .expect("the marker in the symbol table");
        let addr = symbol_address(&exe, "samecode::tests::marker_function");
        let copy = copy_with("function", |data| {
            let o = offset_of(data, ".text", addr);
            data[o] ^= 0xFF;
        });
        match compare(&exe, &copy, &[]) {
            Some(Verdict::Differs {
                sections,
                resized,
                changed,
            }) => {
                assert_eq!(sections, [".text"]);
                assert_eq!(resized, Vec::<String>::new());
                assert_eq!(changed, [name.clone()]);
            }
            other => panic!("one changed byte reads {other:?}"),
        }
        std::fs::remove_dir_all(copy.parent().unwrap()).unwrap();
    }

    #[test]
    fn the_commits_are_masked() {
        let exe = std::env::current_exe().unwrap();
        std::hint::black_box(MARK);
        let other = "fedcba9876543210fedcba9876543210fedcba98";
        let copy = copy_with("commit", |data| {
            let at = data
                .windows(MARK.len())
                .position(|w| w == MARK.as_bytes())
                .expect("the mark in the binary");
            data[at..at + MARK.len()].copy_from_slice(other.as_bytes());
        });
        assert!(
            matches!(compare(&exe, &copy, &[]), Some(Verdict::Differs { .. })),
            "unmasked, a commit differs"
        );
        assert_eq!(compare(&exe, &copy, &[MARK, other]), Some(Verdict::Same));
        std::fs::remove_dir_all(copy.parent().unwrap()).unwrap();
    }

    #[test]
    fn ordinary_data_with_a_rust_filename_still_differs() {
        let exe = std::env::current_exe().unwrap();
        assert_eq!(std::hint::black_box(&FILE_LIMIT).max_bytes, 111);
        assert_eq!(std::mem::size_of::<FileLimit>(), 24);
        let addr = symbol_address(&exe, "samecode::tests::FILE_LIMIT");
        let image = read(&exe).unwrap();
        let section = image
            .sections
            .iter()
            .find(|(_, (start, size, _))| (*start..start + size).contains(&addr))
            .map(|(name, _)| name.clone())
            .expect("the file-limit record is allocated data");
        let copy = copy_with("file-limit", |data| {
            let o = offset_of(data, &section, addr) + std::mem::offset_of!(FileLimit, max_bytes);
            assert_eq!(u32_at(data, o), 111);
            data[o..o + 4].copy_from_slice(&222u32.to_le_bytes());
        });
        assert!(
            matches!(
                compare(&exe, &copy, &[]),
                Some(Verdict::Differs { sections, resized, changed })
                    if sections == [section] && resized.is_empty() && changed.is_empty()
            ),
            "an ordinary limit beside a .rs filename affects the program"
        );
        std::fs::remove_dir_all(copy.parent().unwrap()).unwrap();
    }
}
