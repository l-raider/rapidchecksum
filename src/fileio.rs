use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use crate::hasher::HashKind;
use crate::model::FileEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SfvRecord {
    pub filename: String,
    pub crc32: String,
}

/// Read a UTF-8 SFV file from disk.
pub fn read_sfv_file(path: &Path) -> io::Result<Vec<SfvRecord>> {
    let bytes = std::fs::read(path)?;
    let content = String::from_utf8(bytes).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("SFV file is not valid UTF-8: {err}"),
        )
    })?;

    parse_sfv(&content)
}

/// Parse UTF-8 SFV content.
pub fn parse_sfv(content: &str) -> io::Result<Vec<SfvRecord>> {
    let mut records = Vec::new();

    for (line_number, raw_line) in content.lines().enumerate() {
        let mut line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line_number == 0 {
            line = line.strip_prefix('\u{feff}').unwrap_or(line);
        }

        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        let line = line.trim_end_matches(' ');
        if line.is_empty() {
            continue;
        }

        if line.contains('\t') {
            return Err(invalid_sfv_line(
                line_number + 1,
                "tab characters are not valid in SFV entries",
            ));
        }

        let checksum_start = line.len().checked_sub(8).ok_or_else(|| {
            invalid_sfv_line(line_number + 1, "missing 8-digit CRC32 checksum")
        })?;
        let checksum = line.get(checksum_start..).ok_or_else(|| {
            invalid_sfv_line(
                line_number + 1,
                "checksum must be exactly 8 hexadecimal characters",
            )
        })?;
        if !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(invalid_sfv_line(
                line_number + 1,
                "checksum must be exactly 8 hexadecimal characters",
            ));
        }

        let prefix = &line[..checksum_start];
        let filename_end = prefix.trim_end_matches(' ').len();
        if filename_end == prefix.len() {
            return Err(invalid_sfv_line(
                line_number + 1,
                "missing space separator before checksum",
            ));
        }

        let filename = &prefix[..filename_end];
        if filename.is_empty() {
            return Err(invalid_sfv_line(line_number + 1, "missing filename"));
        }

        records.push(SfvRecord {
            filename: filename.to_string(),
            crc32: checksum.to_ascii_uppercase(),
        });
    }

    Ok(records)
}

fn invalid_sfv_line(line_number: usize, message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("SFV parse error on line {line_number}: {message}"),
    )
}

fn write_standard_hash_line(
    writer: &mut impl Write,
    hash: &str,
    filename: &str,
) -> io::Result<()> {
    let needs_escape = filename
        .chars()
        .any(|ch| matches!(ch, '\\' | '\n' | '\r'));

    if !needs_escape {
        return writeln!(writer, "{hash} *{filename}");
    }

    write!(writer, "\\{hash} *")?;
    for ch in filename.chars() {
        match ch {
            '\\' => writer.write_all(br"\\")?,
            '\n' => writer.write_all(br"\n")?,
            '\r' => writer.write_all(br"\r")?,
            _ => write!(writer, "{ch}")?,
        }
    }
    writeln!(writer)
}

fn invalid_sfv_filename(message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("filename cannot be represented in SFV: {message}"),
    )
}

fn validate_sfv_filename(filename: &str) -> io::Result<()> {
    if filename.is_empty() {
        return Err(invalid_sfv_filename("filenames must not be empty"));
    }
    if filename.starts_with(';') {
        return Err(invalid_sfv_filename("filenames must not start with ';'"));
    }
    if filename.ends_with(' ') {
        return Err(invalid_sfv_filename("filenames must not end with spaces"));
    }
    if filename.contains('\t') {
        return Err(invalid_sfv_filename("filenames must not contain tabs"));
    }
    if filename.chars().any(|ch| matches!(ch, '\n' | '\r')) {
        return Err(invalid_sfv_filename(
            "filenames must not contain line breaks",
        ));
    }
    Ok(())
}

/// Write a hash file for the given entries. Format depends on hash kind:
/// - CRC32 (SFV): `filename CRC32VALUE`
/// - Others: `hashvalue *filename`
pub fn write_hash_file(
    entries: &[FileEntry],
    output_path: &Path,
    kind: HashKind,
    uppercase: bool,
) -> io::Result<()> {
    let lines: Vec<_> = entries
        .iter()
        .filter_map(|entry| {
            let hash = entry.formatted_hash_value(kind, uppercase);
            if hash.is_empty() {
                None
            } else {
                Some((entry.filename.as_str(), hash))
            }
        })
        .collect();

    if lines.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("no {} hashes available to write", kind.name()),
        ));
    }

    if kind == HashKind::CRC32 {
        for (filename, _) in &lines {
            validate_sfv_filename(filename)?;
        }
    }

    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    let result = (|| -> io::Result<()> {
        if kind == HashKind::CRC32 {
            writeln!(writer, "; Generated by RapidChecksum")?;
            writeln!(writer, ";")?;
        }

        for (filename, hash) in lines {
            match kind {
                // SFV format: filename HASH
                HashKind::CRC32 => {
                    writeln!(writer, "{} {}", filename, hash)?;
                }
                // Standard hash format: hash *filename (binary mode indicator)
                _ => {
                    write_standard_hash_line(&mut writer, &hash, filename)?;
                }
            }
        }

        writer.flush()
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(output_path);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_dir(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "rapidchecksum-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parse_sfv_supports_utf8_bom_comments_and_spaces() {
        let content = concat!(
            "\u{feff}; generated by test\n",
            "movie part 1.mkv   deadbeef\n",
            "über file.bin CAFEBABE\n",
            "\n"
        );

        let records = parse_sfv(content).unwrap();

        assert_eq!(
            records,
            vec![
                SfvRecord {
                    filename: "movie part 1.mkv".to_string(),
                    crc32: "DEADBEEF".to_string(),
                },
                SfvRecord {
                    filename: "über file.bin".to_string(),
                    crc32: "CAFEBABE".to_string(),
                },
            ]
        );
    }

    #[test]
    fn parse_sfv_rejects_invalid_crc32_values() {
        let err = parse_sfv("movie.mkv deadbee\n").unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "SFV parse error on line 1: checksum must be exactly 8 hexadecimal characters"
        );
    }

    #[test]
    fn parse_sfv_rejects_tab_separators() {
        let err = parse_sfv("movie.mkv\tDEADBEEF\n").unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            "SFV parse error on line 1: tab characters are not valid in SFV entries"
        );
    }

    #[test]
    fn write_hash_file_round_trips_through_sfv_reader() {
        let output_dir = create_temp_dir("fileio-sfv-roundtrip");
        let output_path = output_dir.join("checksums.sfv");

        let mut first = FileEntry::default();
        first.filename = "movie part 1.mkv".to_string();
        first
            .hashes
            .insert(HashKind::CRC32, "deadbeef".to_string());

        let mut second = FileEntry::default();
        second.filename = "über file.bin".to_string();
        second
            .hashes
            .insert(HashKind::CRC32, "cafebabe".to_string());

        write_hash_file(&[first, second], &output_path, HashKind::CRC32, false).unwrap();

        let records = read_sfv_file(&output_path).unwrap();

        assert_eq!(
            records,
            vec![
                SfvRecord {
                    filename: "movie part 1.mkv".to_string(),
                    crc32: "DEADBEEF".to_string(),
                },
                SfvRecord {
                    filename: "über file.bin".to_string(),
                    crc32: "CAFEBABE".to_string(),
                },
            ]
        );

        fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn write_hash_file_omits_sfv_header_for_non_crc_formats() {
        let output_dir = create_temp_dir("fileio-md5-format");
        let output_path = output_dir.join("checksums.md5");

        let mut entry = FileEntry::default();
        entry.filename = "movie part 1.mkv".to_string();
        entry.hashes.insert(HashKind::MD5, "deadbeef".to_string());

        write_hash_file(&[entry], &output_path, HashKind::MD5, false).unwrap();

        let content = fs::read_to_string(&output_path).unwrap();

        assert_eq!(content, "deadbeef *movie part 1.mkv\n");

        fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn write_hash_file_escapes_special_filenames_for_non_crc_formats() {
        let output_dir = create_temp_dir("fileio-md5-escaped-format");
        let output_path = output_dir.join("checksums.md5");

        let mut entry = FileEntry::default();
        entry.filename = "line\nbreak\\name.bin".to_string();
        entry.hashes.insert(HashKind::MD5, "deadbeef".to_string());

        write_hash_file(&[entry], &output_path, HashKind::MD5, false).unwrap();

        let content = fs::read_to_string(&output_path).unwrap();

        assert_eq!(content, "\\deadbeef *line\\nbreak\\\\name.bin\n");

        fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn write_hash_file_rejects_empty_exports() {
        let output_dir = create_temp_dir("fileio-empty-export");
        let output_path = output_dir.join("checksums.md5");

        let mut entry = FileEntry::default();
        entry.filename = "movie part 1.mkv".to_string();

        let err = write_hash_file(&[entry], &output_path, HashKind::MD5, false).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(err.to_string(), "no MD5 hashes available to write");
        assert!(!output_path.exists());

        fs::remove_dir_all(&output_dir).unwrap();
    }

    #[test]
    fn write_hash_file_rejects_unrepresentable_sfv_filenames() {
        let output_dir = create_temp_dir("fileio-sfv-invalid-filename");

        for (index, filename) in [
            ";comment.bin",
            "tab\tname.bin",
            "trail.bin ",
            "line\nbreak.bin",
        ]
        .into_iter()
        .enumerate()
        {
            let output_path = output_dir.join(format!("invalid-{index}.sfv"));
            let mut entry = FileEntry::default();
            entry.filename = filename.to_string();
            entry.hashes.insert(HashKind::CRC32, "deadbeef".to_string());

            let err = write_hash_file(&[entry], &output_path, HashKind::CRC32, true)
                .unwrap_err();

            assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
            assert!(err.to_string().contains("cannot be represented in SFV"));
            assert!(!output_path.exists());
        }

        fs::remove_dir_all(&output_dir).unwrap();
    }
}
