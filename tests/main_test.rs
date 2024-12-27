use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};

use assert_cmd::Command;
use predicates::prelude::predicate;
use tempfile::TempDir;

fn copy_test_class(target_dir: &PathBuf) -> io::Result<()> {
    let test_class_bytes = include_bytes!("resources/com/example/TestClass.class");
    let target_file = target_dir.join("TestClass.class");
    let mut file = File::create(target_file)?;
    file.write_all(test_class_bytes)
}

#[test]
fn should_find_method_calls() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let classes_dir = temp_dir.path().join("classes");
    fs::create_dir_all(&classes_dir)?;
    copy_test_class(&classes_dir)?;

    let mut cmd = Command::cargo_bin("jmf")?;
    cmd.args([
        "-c",
        "java.lang.String",
        "-m",
        "toString",
        "-s",
        classes_dir.to_str().unwrap(),
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("java.lang.String#toString"))
        .stdout(predicate::str::contains("- com.example.TestClass#testMethod (L8)"))
        .stdout(predicate::str::contains("- com.example.TestClass#testMethod (L10)"));

    Ok(())
}

#[test]
fn should_handle_invalid_class_path() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("jmf")?;
    cmd.args(["-c", "java.lang.String", "-m", "toString", "-s", "/invalid/path"]);

    cmd.assert().failure().stderr(predicate::str::contains("Error:"));

    Ok(())
}

#[test]
fn should_handle_verbose_mode() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let classes_dir = temp_dir.path().join("classes");
    fs::create_dir_all(&classes_dir)?;
    copy_test_class(&classes_dir)?;

    let mut cmd = Command::cargo_bin("jmf")?;
    cmd.args([
        "-c",
        "java.lang.String",
        "-m",
        "toString",
        "-s",
        classes_dir.to_str().unwrap(),
        "-v",
    ]);

    cmd.assert().success().stdout(predicate::str::contains("DEBUG"));

    Ok(())
}
