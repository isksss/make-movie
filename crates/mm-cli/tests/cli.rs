use assert_cmd::Command;
use flate2::read::GzDecoder;
use std::fs;
use std::path::Path;
use tar::Archive;

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
fn validate_command_outputs_english_with_lang() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_project(dir.path());

    let mut command = Command::cargo_bin("mm").unwrap();
    let assert = command
        .arg("--lang")
        .arg("en")
        .arg("validate")
        .arg("--project")
        .arg(dir.path().join("mm.toml"))
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("validate completed"));
}

#[test]
fn validate_command_outputs_japanese_with_lang() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_project(dir.path());

    let mut command = Command::cargo_bin("mm").unwrap();
    let assert = command
        .arg("--lang")
        .arg("ja")
        .arg("validate")
        .arg("--project")
        .arg(dir.path().join("mm.toml"))
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("validate が完了しました"));
}

#[test]
fn validate_error_outputs_english_with_lang() {
    let dir = tempfile::tempdir().unwrap();

    let mut command = Command::cargo_bin("mm").unwrap();
    let assert = command
        .arg("--lang")
        .arg("en")
        .arg("validate")
        .arg("--project")
        .arg(dir.path().join("missing.toml"))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("error: failed to read project"));
}

#[test]
fn validate_error_outputs_japanese_with_lang() {
    let dir = tempfile::tempdir().unwrap();

    let mut command = Command::cargo_bin("mm").unwrap();
    let assert = command
        .arg("--lang")
        .arg("ja")
        .arg("validate")
        .arg("--project")
        .arg(dir.path().join("missing.toml"))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("エラー: プロジェクトを読み込めません"));
}

#[test]
fn validate_rule_error_outputs_english_with_env_lang() {
    let dir = tempfile::tempdir().unwrap();
    write_invalid_project(dir.path());

    let mut command = Command::cargo_bin("mm").unwrap();
    let assert = command
        .env("MM_LANG", "en")
        .arg("validate")
        .arg("--project")
        .arg(dir.path().join("mm.toml"))
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("error: settings.width and settings.height must be at least 1"));
}

#[test]
fn help_outputs_japanese_with_lang() {
    let mut command = Command::cargo_bin("mm").unwrap();
    let assert = command
        .arg("--lang")
        .arg("ja")
        .arg("--help")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("使用方法: mm"));
    assert!(stdout.contains("動画を書き出す"));
    assert!(stdout.contains("オプション:"));
}

#[test]
fn help_outputs_english_with_lang() {
    let mut command = Command::cargo_bin("mm").unwrap();
    let assert = command
        .arg("--lang")
        .arg("en")
        .arg("--help")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("Usage: mm"));
    assert!(stdout.contains("Render a movie"));
    assert!(stdout.contains("Options:"));
}

#[test]
fn plugin_help_outputs_japanese_and_english() {
    let mut command = Command::cargo_bin("mm").unwrap();
    let assert = command
        .arg("--lang")
        .arg("ja")
        .arg("plugin")
        .arg("--help")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("使用方法: mm plugin"));
    assert!(stdout.contains("plugin をインストールする"));

    let mut command = Command::cargo_bin("mm").unwrap();
    let assert = command
        .arg("--lang")
        .arg("en")
        .arg("plugin")
        .arg("--help")
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("Usage: mm plugin"));
    assert!(stdout.contains("Install a plugin"));
}

#[test]
fn plugin_install_uses_project_and_global_config() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = dir.path().join("data");
    let component = dir.path().join("component.wasm");
    fs::write(&component, b"\0asm").unwrap();
    fs::write(
        dir.path().join("global.toml"),
        r#"
[[plugin]]
repository = "local"
name = "global-theme"
path = "/missing/global.wasm"
version = "1.0.0"
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("mm.toml"),
        format!(
            r#"
[[plugin]]
repository = "local"
name = "global-theme"
path = "{}"
version = "2.0.0"
"#,
            component.display()
        ),
    )
    .unwrap();

    let mut command = Command::cargo_bin("mm").unwrap();
    command
        .current_dir(dir.path())
        .env("XDG_DATA_HOME", &data_home)
        .arg("plugin")
        .arg("install")
        .arg("--project")
        .arg("mm.toml")
        .arg("--global-config")
        .arg("global.toml")
        .assert()
        .success();

    let lock = fs::read_to_string(dir.path().join("mm.lock")).unwrap();
    assert!(lock.contains("global-theme"));
    assert!(lock.contains("2.0.0"));
    assert!(data_home
        .join("mm/plugins/global-theme/global-theme.wasm")
        .exists());
}

#[test]
fn plugin_update_and_remove_use_project_lock() {
    let dir = tempfile::tempdir().unwrap();
    let data_home = dir.path().join("data");
    let component = dir.path().join("theme.wasm");
    fs::write(&component, b"\0asm").unwrap();
    fs::write(
        dir.path().join("mm.toml"),
        format!(
            r#"
[[plugin]]
repository = "local"
name = "project-theme"
path = "{}"
version = "1.0.0"
"#,
            component.display()
        ),
    )
    .unwrap();

    Command::cargo_bin("mm")
        .unwrap()
        .current_dir(dir.path())
        .env("XDG_DATA_HOME", &data_home)
        .arg("plugin")
        .arg("install")
        .arg("--project")
        .arg("mm.toml")
        .assert()
        .success();

    Command::cargo_bin("mm")
        .unwrap()
        .current_dir(dir.path())
        .env("XDG_DATA_HOME", &data_home)
        .arg("plugin")
        .arg("update")
        .arg("project-theme")
        .arg("--project")
        .arg("mm.toml")
        .assert()
        .success();

    let lock = fs::read_to_string(dir.path().join("mm.lock")).unwrap();
    assert!(lock.contains("project-theme"));

    Command::cargo_bin("mm")
        .unwrap()
        .current_dir(dir.path())
        .env("XDG_DATA_HOME", &data_home)
        .arg("plugin")
        .arg("remove")
        .arg("project-theme")
        .arg("--project")
        .arg("mm.toml")
        .assert()
        .success();

    let lock = fs::read_to_string(dir.path().join("mm.lock")).unwrap();
    assert!(!lock.contains("project-theme"));
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

#[test]
fn package_command_writes_project_archive() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_project(dir.path());
    fs::write(dir.path().join("mm.lock"), "[[plugin]]\nname = \"theme\"\n").unwrap();
    fs::create_dir_all(dir.path().join("cache")).unwrap();
    fs::write(dir.path().join("cache/skip.tmp"), "skip").unwrap();
    fs::create_dir_all(dir.path().join("output")).unwrap();
    fs::write(dir.path().join("output/skip.mp4"), "skip").unwrap();
    let output = dir.path().join("package/project.tar.gz");

    Command::cargo_bin("mm")
        .unwrap()
        .arg("package")
        .arg("--project-root")
        .arg(dir.path())
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    let entries = tar_gz_entries(&output);
    assert!(entries.iter().any(|entry| entry == "mm.toml"));
    assert!(entries.iter().any(|entry| entry == "mm.lock"));
    assert!(entries
        .iter()
        .any(|entry| entry == "media/image/sample.png"));
    assert!(!entries.iter().any(|entry| entry.starts_with("cache/")));
    assert!(!entries.iter().any(|entry| entry == "output/skip.mp4"));
}

#[test]
fn package_command_skips_archive_when_output_is_inside_media() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_project(dir.path());
    let output = dir.path().join("media/package.tar.gz");

    Command::cargo_bin("mm")
        .unwrap()
        .arg("package")
        .arg("--project-root")
        .arg(dir.path())
        .arg("--output")
        .arg(&output)
        .assert()
        .success();

    let entries = tar_gz_entries(&output);
    assert!(!entries.iter().any(|entry| entry == "media/package.tar.gz"));
}

#[test]
fn package_command_uses_default_output() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_project(dir.path());

    Command::cargo_bin("mm")
        .unwrap()
        .arg("package")
        .arg("--project-root")
        .arg(dir.path())
        .assert()
        .success();

    assert!(dir.path().join("output/cli.tar.gz").exists());
}

fn tar_gz_entries(path: &Path) -> Vec<String> {
    let file = fs::File::open(path).unwrap();
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    let mut entries = archive
        .entries()
        .unwrap()
        .map(|entry| {
            entry
                .unwrap()
                .path()
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries
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

fn write_invalid_project(root: &Path) {
    fs::write(
        root.join("mm.toml"),
        r#"
[settings]
title = "Invalid"
width = 0
height = 180
fps = 30
sample_rate = 48000
duration = 1.0
output = "output/movie.mp4"
"#,
    )
    .unwrap();
}
