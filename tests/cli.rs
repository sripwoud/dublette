use std::fs;

use assert_cmd::Command;
use image::{ImageBuffer, Rgb, RgbImage};
use predicates::prelude::*;

#[allow(deprecated)]
fn cmd() -> Command {
    Command::cargo_bin("dublette").unwrap()
}

fn create_gradient_image(path: &std::path::Path, horizontal: bool) {
    let img: RgbImage = ImageBuffer::from_fn(100, 100, |x, y| {
        let val = if horizontal { x as u8 } else { y as u8 };
        Rgb([val, val, val])
    });
    img.save(path).unwrap();
}

fn create_checkerboard_image(path: &std::path::Path, block_size: u32) {
    let img: RgbImage = ImageBuffer::from_fn(100, 100, |x, y| {
        if ((x / block_size) + (y / block_size)) % 2 == 0 {
            Rgb([255, 255, 255])
        } else {
            Rgb([0, 0, 0])
        }
    });
    img.save(path).unwrap();
}

#[test]
fn help_output() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Deduplicate images, videos, and audio",
        ))
        .stdout(predicate::str::contains("--version"));
}

#[test]
fn version_output() {
    let expected = format!("dublette {}", env!("CARGO_PKG_VERSION"));
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(expected));
}

#[test]
fn nonexistent_directory_exits_2() {
    cmd()
        .arg("/nonexistent/path/that/does/not/exist")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn no_args_exits_2() {
    cmd().assert().failure().code(2);
}

#[test]
fn empty_directory_exits_0() {
    let dir = tempfile::tempdir().unwrap();
    cmd().arg(dir.path()).assert().success();
}

#[test]
fn dry_run_with_duplicates_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    create_gradient_image(&dir.path().join("a.png"), true);
    create_gradient_image(&dir.path().join("b.png"), true);

    cmd()
        .arg(dir.path())
        .arg("-n")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("would delete"));
}

#[test]
fn dry_run_preserves_files() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.png");
    let b = dir.path().join("b.png");
    create_gradient_image(&a, true);
    create_gradient_image(&b, true);

    cmd().arg(dir.path()).arg("-n").assert().code(1);

    assert!(a.exists());
    assert!(b.exists());
}

#[test]
fn no_duplicates_exits_0() {
    let dir = tempfile::tempdir().unwrap();
    create_gradient_image(&dir.path().join("a.png"), true);
    create_checkerboard_image(&dir.path().join("b.png"), 10);

    cmd().arg(dir.path()).arg("-n").assert().success();
}

#[test]
fn no_duplicates_prints_reassurance_message() {
    let dir = tempfile::tempdir().unwrap();
    create_gradient_image(&dir.path().join("a.png"), true);
    create_checkerboard_image(&dir.path().join("b.png"), 10);

    cmd()
        .arg(dir.path())
        .arg("-n")
        .assert()
        .success()
        .stdout(predicate::str::contains("No duplicates found."));
}

#[test]
fn only_images_skips_videos() {
    let dir = tempfile::tempdir().unwrap();
    create_gradient_image(&dir.path().join("a.png"), true);
    fs::write(dir.path().join("video.mp4"), &[0xFF]).unwrap();

    cmd()
        .arg(dir.path())
        .args(["--only", "images", "-n"])
        .assert()
        .success()
        .stdout(predicate::str::contains("video").not());
}

fn create_wav(path: &std::path::Path, frequency: f64) {
    let samples: Vec<i16> = (0..4410)
        .map(|i| {
            let t = i as f64 / 44100.0;
            ((t * frequency * 2.0 * std::f64::consts::PI).sin() * 20000.0) as i16
        })
        .collect();
    let data_len = (samples.len() * 2) as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&44100u32.to_le_bytes());
    bytes.extend_from_slice(&(44100u32 * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in &samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
}

#[test]
fn missing_ffmpeg_warns_for_video_and_audio_and_keeps_image_results() {
    let dir = tempfile::tempdir().unwrap();
    create_gradient_image(&dir.path().join("a.png"), true);
    create_gradient_image(&dir.path().join("b.png"), true);
    create_wav(&dir.path().join("x.wav"), 440.0);

    cmd()
        .arg(dir.path())
        .arg("-n")
        .env("PATH", "")
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("skipping video pass")
                .and(predicate::str::contains("skipping audio pass")),
        )
        .stdout(predicate::str::contains("would delete"));
}

#[test]
fn missing_ffmpeg_pass_warnings_appear_in_json() {
    let dir = tempfile::tempdir().unwrap();
    create_gradient_image(&dir.path().join("a.png"), true);
    create_wav(&dir.path().join("x.wav"), 440.0);

    let output = cmd()
        .arg(dir.path())
        .args(["-n", "--json"])
        .env("PATH", "")
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let warnings: Vec<&str> = json["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w.as_str().unwrap())
        .collect();
    assert!(warnings.iter().any(|w| w.contains("skipping video pass")));
    assert!(warnings.iter().any(|w| w.contains("skipping audio pass")));
}

#[test]
fn encoding_match_needs_no_ffmpeg() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.wav");
    let b = dir.path().join("b.wav");
    create_wav(&a, 440.0);
    fs::copy(&a, &b).unwrap();

    cmd()
        .arg(dir.path())
        .args(["--only", "audio", "--audio-match", "encoding", "-n"])
        .env("PATH", "")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("skipping audio pass").not())
        .stdout(predicate::str::contains("would delete"));
}

#[test]
fn audio_threshold_with_encoding_match_errors() {
    let dir = tempfile::tempdir().unwrap();

    cmd()
        .arg(dir.path())
        .args(["--audio-match", "encoding", "--audio-threshold", "0.2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--audio-threshold applies only to recording match",
        ));
}

#[test]
fn quiet_suppresses_progress() {
    let dir = tempfile::tempdir().unwrap();
    create_gradient_image(&dir.path().join("a.png"), true);

    cmd()
        .arg(dir.path())
        .arg("-q")
        .assert()
        .success()
        .stderr(predicate::str::contains("Scanning").not());
}

#[test]
fn json_output_valid() {
    let dir = tempfile::tempdir().unwrap();
    create_gradient_image(&dir.path().join("a.png"), true);
    create_gradient_image(&dir.path().join("b.png"), true);

    let output = cmd()
        .arg(dir.path())
        .args(["-n", "--json"])
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["dry_run"].as_bool().unwrap());
    assert!(!json["groups"].as_array().unwrap().is_empty());
    assert!(json["skipped"].as_array().unwrap().is_empty());
    assert_eq!(json["total_skipped"], 0);
    assert!(json["warnings"].is_array());
}

#[test]
fn json_output_reports_skipped_files() {
    let dir = tempfile::tempdir().unwrap();
    let corrupt = dir.path().join("corrupt.jpg");
    fs::write(&corrupt, b"not-an-image-payload").unwrap();

    let output = cmd()
        .arg(dir.path())
        .args(["-n", "--json", "--only", "images"])
        .output()
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["total_skipped"], 1);
    assert_eq!(json["skipped"][0]["reason"], "decode_failed");
    assert_eq!(
        json["skipped"][0]["path"],
        corrupt.display().to_string().as_str()
    );
    assert!(
        json["skipped"][0]["detail"]
            .as_str()
            .unwrap()
            .contains("failed to open")
    );

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains(&format!("Warning: skipping {}: ", corrupt.display())),
        "stderr warning shape must be unchanged: {stderr}"
    );
}

#[test]
fn delete_empty_removes_zero_byte_files() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("empty.jpg");
    fs::write(&empty, &[]).unwrap();

    cmd()
        .arg(dir.path())
        .args(["--delete-empty", "-y"])
        .assert()
        .success();

    assert!(!empty.exists());
}

#[test]
fn yes_flag_deletes_without_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.png");
    let b = dir.path().join("b.png");
    create_gradient_image(&a, true);
    create_gradient_image(&b, true);

    cmd().arg(dir.path()).arg("-y").assert().success();

    let remaining: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(remaining.len(), 1);
}

#[test]
fn multiple_directories_cross_dir_duplicates() {
    let dir1 = tempfile::tempdir().unwrap();
    let dir2 = tempfile::tempdir().unwrap();
    create_gradient_image(&dir1.path().join("a.png"), true);
    create_gradient_image(&dir2.path().join("b.png"), true);

    cmd()
        .arg(dir1.path())
        .arg(dir2.path())
        .arg("-n")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("would delete"));
}

#[test]
fn multiple_directories_no_cross_dir_duplicates() {
    let dir1 = tempfile::tempdir().unwrap();
    let dir2 = tempfile::tempdir().unwrap();
    create_gradient_image(&dir1.path().join("a.png"), true);
    create_checkerboard_image(&dir2.path().join("b.png"), 10);

    cmd()
        .arg(dir1.path())
        .arg(dir2.path())
        .arg("-n")
        .assert()
        .success();
}

#[test]
fn keep_in_nonexistent_directory_exits_2() {
    let dir = tempfile::tempdir().unwrap();

    cmd()
        .arg(dir.path())
        .args(["--keep-in", "/nonexistent/keep/dir"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--keep-in"))
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn keep_in_file_exits_2() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.png");
    create_gradient_image(&file, true);

    cmd()
        .arg(dir.path())
        .arg("--keep-in")
        .arg(&file)
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("not a directory"));
}

#[test]
fn keep_in_keeps_library_copy_and_deletes_inbox_copy() {
    let root = tempfile::tempdir().unwrap();
    let inbox = root.path().join("inbox");
    let library = root.path().join("library");
    fs::create_dir(&inbox).unwrap();
    fs::create_dir(&library).unwrap();
    create_gradient_image(&inbox.join("a.png"), true);
    create_gradient_image(&library.join("z.png"), true);

    cmd()
        .arg(&inbox)
        .arg(&library)
        .arg("--keep-in")
        .arg(&library)
        .arg("-y")
        .assert()
        .success();

    assert!(!inbox.join("a.png").exists());
    assert!(library.join("z.png").exists());
}

#[test]
fn keep_in_relative_path_with_trailing_slash_matches() {
    let root = tempfile::tempdir().unwrap();
    let inbox = root.path().join("inbox");
    let library = root.path().join("library");
    fs::create_dir(&inbox).unwrap();
    fs::create_dir(&library).unwrap();
    create_gradient_image(&inbox.join("a.png"), true);
    create_gradient_image(&library.join("z.png"), true);

    cmd()
        .current_dir(root.path())
        .args(["inbox", "library", "--keep-in", "library/", "-y"])
        .assert()
        .success();

    assert!(!inbox.join("a.png").exists());
    assert!(library.join("z.png").exists());
}

#[test]
fn verbose_shows_distances() {
    let dir = tempfile::tempdir().unwrap();
    create_gradient_image(&dir.path().join("a.png"), true);
    create_gradient_image(&dir.path().join("b.png"), true);

    cmd()
        .arg(dir.path())
        .args(["-n", "-v"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("distance="));
}

#[test]
fn verbose_with_json_still_shows_distances() {
    let dir = tempfile::tempdir().unwrap();
    create_gradient_image(&dir.path().join("a.png"), true);
    create_gradient_image(&dir.path().join("b.png"), true);

    cmd()
        .arg(dir.path())
        .args(["-n", "-v", "--json"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("distance="));
}
