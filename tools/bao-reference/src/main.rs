//! Single-threaded reference CLI for the outboard study. Inputs must be regular
//! files whose bytes remain unchanged while mapped, as for b3sum's mapped path.
use bao_tree::{io::outboard::PreOrderMemOutboard, BaoTree, BlockSize};
use std::{fs::File, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--version" {
        let declarations: Vec<_> = include_str!("../Cargo.toml")
            .lines()
            .filter(|line| line.starts_with("bao-tree = ") || line.starts_with("blake3 = "))
            .collect();
        assert_eq!(
            declarations.len(),
            2,
            "the exact dependency declarations stay on single lines"
        );
        println!(
            "bao-reference {}+{}; {}",
            env!("CARGO_PKG_VERSION"),
            env!("BAO_REFERENCE_SOURCE"),
            declarations.join("; ")
        );
        return Ok(());
    }
    assert!(
        args.len() >= 3 && args[0] == "--outboard",
        "usage: bao-reference --outboard DIR FILE..."
    );
    let directory = PathBuf::from(&args[1]);
    debug_assert!(directory.is_dir(), "the output directory must exist");
    let mut outboard: PreOrderMemOutboard = Default::default();
    for input in &args[2..] {
        let path = PathBuf::from(input);
        let file = File::open(&path)?;
        let metadata = file.metadata()?;
        debug_assert!(metadata.is_file(), "study inputs must be regular files");
        let len = metadata.len();
        // SAFETY: study inputs stay unchanged until this process ends.
        let map = if len == 0 {
            None
        } else {
            Some(unsafe { memmap2::Mmap::map(&file)? })
        };
        let bytes = map.as_deref().unwrap_or(&[]);
        let tree = BaoTree::new(len, BlockSize::from_chunk_log(4));
        outboard.tree = tree;
        outboard
            .data
            .resize(usize::try_from(tree.outboard_size())?, 0);
        // Use bao-tree's public writer with kept output storage. Its own group
        // buffer and hashing strategy remain the library's implementation.
        let root = bao_tree::io::sync::outboard(bytes, tree, &mut outboard)?;
        outboard.root = root;
        std::fs::write(
            directory.join(format!("{}.obao4", root.to_hex())),
            &outboard.data,
        )?;
        println!("{}  {}", root.to_hex(), path.display());
    }
    Ok(())
}
