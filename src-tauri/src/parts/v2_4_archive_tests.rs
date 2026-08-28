#[cfg(test)]
mod v2_4_archive_tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    fn temp_archive(label: &str, file_name: &str) -> PathBuf {
        let directory = env::temp_dir().join(format!(
            "pulsedag-desktop-v24-{}-{}-{label}",
            std::process::id(),
            unix_time_ms()
        ));
        fs::create_dir_all(&directory).expect("create candidate fixture directory");
        directory.join(file_name)
    }

    fn cleanup_archive(path: &Path) {
        let parent = path.parent().map(Path::to_path_buf);
        let _ = fs::remove_file(path);
        if let Some(parent) = parent {
            let _ = fs::remove_dir(parent);
        }
    }

    fn append_tar_file<W: Write>(
        builder: &mut tar::Builder<W>,
        path: &str,
        bytes: &[u8],
        mode: u32,
    ) {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(mode);
        header.set_mtime(0);
        header.set_cksum();
        builder
            .append_data(&mut header, path, bytes)
            .expect("append candidate TAR entry");
    }

    fn write_linux_node_candidate(path: &Path, include_extra: bool) {
        let file = File::create(path).expect("create candidate tar.gz");
        let encoder = GzEncoder::new(file, Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let root = "pulsedagd-v2.4.0-x86_64-unknown-linux-gnu";
        append_tar_file(
            &mut builder,
            &format!("{root}/pulsedagd"),
            b"node-bytes-v24",
            0o755,
        );
        append_tar_file(
            &mut builder,
            &format!("{root}/README.md"),
            b"readme",
            0o644,
        );
        append_tar_file(
            &mut builder,
            &format!("{root}/INSTALL_BINARIES_V2_4_0.md"),
            b"install",
            0o644,
        );
        if include_extra {
            append_tar_file(
                &mut builder,
                &format!("{root}/unexpected.txt"),
                b"unexpected",
                0o644,
            );
        }
        let encoder = builder.into_inner().expect("finish candidate TAR");
        encoder.finish().expect("finish candidate gzip");
    }

    fn write_windows_miner_candidate(path: &Path, include_extra: bool) {
        let file = File::create(path).expect("create candidate zip");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let root = "pulsedag-miner-v2.4.0-x86_64-pc-windows-msvc";
        for (name, bytes) in [
            ("pulsedag-miner.exe", b"miner-bytes-v24".as_slice()),
            ("README.md", b"readme".as_slice()),
            ("INSTALL_BINARIES_V2_4_0.md", b"install".as_slice()),
        ] {
            writer
                .start_file(format!("{root}/{name}"), options)
                .expect("start candidate ZIP entry");
            writer.write_all(bytes).expect("write candidate ZIP entry");
        }
        if include_extra {
            writer
                .start_file(format!("{root}/unexpected.txt"), options)
                .expect("start unexpected ZIP entry");
            writer
                .write_all(b"unexpected")
                .expect("write unexpected ZIP entry");
        }
        writer.finish().expect("finish candidate ZIP");
    }

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    #[test]
    fn v2_4_candidate_tar_is_inspected_but_never_approved() {
        let path = temp_archive(
            "node-valid",
            "pulsedagd-v2.4.0-x86_64-unknown-linux-gnu.tar.gz",
        );
        write_linux_node_candidate(&path, false);
        let result = inspect_v2_4_candidate_archive_path(&path, V2_4CandidateBinaryKind::Node)
            .expect("valid local v2.4 node candidate");
        assert!(result.structurally_valid);
        assert!(!result.approved);
        assert_eq!(result.release_tag, V2_4_CANDIDATE_TAG);
        assert_eq!(result.source_commit, "unfrozen");
        assert_eq!(result.target, "x86_64-unknown-linux-gnu");
        assert_eq!(result.embedded_binary_size_bytes, b"node-bytes-v24".len() as u64);
        assert_eq!(result.binary_kind, "node");
        assert!(result.message.contains("cannot become trusted provenance"));
        cleanup_archive(&path);
    }

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    #[test]
    fn v2_4_candidate_tar_rejects_unexpected_file() {
        let path = temp_archive(
            "node-extra",
            "pulsedagd-v2.4.0-x86_64-unknown-linux-gnu.tar.gz",
        );
        write_linux_node_candidate(&path, true);
        let error = inspect_v2_4_candidate_archive_path(&path, V2_4CandidateBinaryKind::Node)
            .expect_err("candidate with extra file must fail");
        assert!(error.contains("unexpected or duplicate file"));
        cleanup_archive(&path);
    }

    #[test]
    fn v2_4_candidate_zip_reads_exact_windows_miner_layout() {
        let path = temp_archive(
            "miner-valid",
            "pulsedag-miner-v2.4.0-x86_64-pc-windows-msvc.zip",
        );
        write_windows_miner_candidate(&path, false);
        let layout = v2_4_candidate_archive_layout(
            "pulsedag-miner-v2.4.0-x86_64-pc-windows-msvc.zip",
            V2_4CandidateBinaryKind::Miner,
        )
        .expect("windows miner candidate layout");
        let archive_sha = sha256_path(&path).expect("hash candidate zip");
        let evidence = inspect_zip_binary(
            File::open(&path).expect("open candidate zip"),
            &layout,
            archive_sha,
        )
        .expect("inspect candidate zip");
        assert_eq!(evidence.target, "x86_64-pc-windows-msvc");
        assert_eq!(evidence.binary_size_bytes, b"miner-bytes-v24".len() as u64);
        assert_eq!(
            Path::new(&evidence.embedded_path)
                .file_name()
                .and_then(|value| value.to_str()),
            Some("pulsedag-miner.exe")
        );
        cleanup_archive(&path);
    }

    #[test]
    fn v2_4_candidate_zip_rejects_unexpected_file() {
        let path = temp_archive(
            "miner-extra",
            "pulsedag-miner-v2.4.0-x86_64-pc-windows-msvc.zip",
        );
        write_windows_miner_candidate(&path, true);
        let layout = v2_4_candidate_archive_layout(
            "pulsedag-miner-v2.4.0-x86_64-pc-windows-msvc.zip",
            V2_4CandidateBinaryKind::Miner,
        )
        .expect("windows miner candidate layout");
        let archive_sha = sha256_path(&path).expect("hash candidate zip");
        let error = inspect_zip_binary(
            File::open(&path).expect("open candidate zip"),
            &layout,
            archive_sha,
        )
        .expect_err("candidate ZIP with extra file must fail");
        assert!(error.contains("unexpected or duplicate file"));
        cleanup_archive(&path);
    }
}
