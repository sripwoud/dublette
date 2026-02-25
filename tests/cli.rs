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
        .stdout(predicate::str::contains("Deduplicate images and videos"));
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
