#[path = "../../require_nextest.rs"]
mod require_nextest;

use std::path::PathBuf;
use std::process::Command;

#[test]
fn complete_files_include_empty_partial_groups_and_reused_storage() {
    let root = std::env::temp_dir().join(format!("bao-reference-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let out = root.join("out");
    std::fs::create_dir(&out).unwrap();
    let mut paths = Vec::<PathBuf>::new();
    let mut expected = Vec::new();
    for (i, len) in [1_048_593usize, 3, 0, 16_384, 16_385, 32_773]
        .into_iter()
        .enumerate()
    {
        let data: Vec<u8> = if len == 3 {
            b"abc".to_vec()
        } else {
            (0..len).map(|x| (x % 251) as u8).collect()
        };
        let path = root.join(format!("input-{i}"));
        std::fs::write(&path, &data).unwrap();
        let reference = bao_tree::io::outboard::PreOrderMemOutboard::create(
            &data,
            bao_tree::BlockSize::from_chunk_log(4),
        );
        if len == 3 {
            assert_eq!(
                reference.root.to_hex().as_str(),
                "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
            );
        }
        assert_eq!(reference.root, blake3::hash(&data));
        assert_eq!(
            reference.data.len(),
            64 * len.div_ceil(16_384).saturating_sub(1)
        );
        expected.push((reference.root.to_hex().to_string(), reference.data));
        paths.push(path);
    }
    for _ in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_bao-reference"))
            .arg("--outboard")
            .arg(&out)
            .args(&paths)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let lines: Vec<_> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect();
        assert_eq!(lines.len(), paths.len());
        for ((line, path), (hash, bytes)) in lines.iter().zip(&paths).zip(&expected) {
            assert_eq!(line, &format!("{hash}  {}", path.display()));
            assert_eq!(
                &std::fs::read(out.join(format!("{hash}.obao4"))).unwrap(),
                bytes
            );
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn version_names_the_compiled_adapter_and_dependencies() {
    let output = Command::new(env!("CARGO_BIN_EXE_bao-reference"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains(env!("BAO_REFERENCE_SOURCE")));
    assert!(text.contains("bao-tree = { version = \"=0.17.0\", default-features = false }"));
    assert!(text.contains("blake3 = \"=1.8.7\""));
    assert_eq!(text.lines().count(), 1);
}
