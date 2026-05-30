use assert_cmd::Command;
use std::fs;
use std::path::Path;

#[test]
fn doctor_command_succeeds() {
    let mut command = Command::cargo_bin("mm").unwrap();
    command.arg("doctor").assert().success();
}

#[test]
fn validate_command_accepts_valid_project() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_project(dir.path());

    let mut command = Command::cargo_bin("mm").unwrap();
    command
        .arg("validate")
        .arg("--project")
        .arg(dir.path().join("mm.toml"))
        .assert()
        .success();
}

#[test]
fn preview_command_writes_png_frame() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_project(dir.path());
    let output = dir.path().join("preview.png");

    let mut command = Command::cargo_bin("mm").unwrap();
    command
        .arg("preview")
        .arg("--project")
        .arg(dir.path().join("mm.toml"))
        .arg("--time")
        .arg("0.5")
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    let bytes = fs::read(output).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
}

fn write_valid_project(root: &Path) {
    let media = root.join("media/image");
    fs::create_dir_all(&media).unwrap();
    fs::write(media.join("sample.png"), []).unwrap();
    fs::write(
        root.join("mm.toml"),
        r##"
[settings]
title = "CLI テスト"
width = 320
height = 180
fps = 30
sample_rate = 48000
duration = 1.0
output = "output/movie.mp4"
asset_mode = "copy"

[[assets]]
id = "sample"
kind = "image"
path = "media/image/sample.png"

[[tracks]]
id = "v1"
name = "V1"
kind = "video"

[[tracks.layers]]
id = "text1"
start = 0.0
duration = 1.0
z_index = 1

[tracks.layers.content]
type = "text"
text = "テスト"
font_size = 48.0
color = "#ffffff"
align = "center"

[tracks.layers.transform]
x = 160.0
y = 80.0
width = 120.0
height = 48.0
scale = 1.0
rotation = 0.0
opacity = 1.0
"##,
    )
    .unwrap();
}
