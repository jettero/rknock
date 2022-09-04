use assert_cmd::Command;
use predicates::prelude::predicate;

#[test]
fn knock_help_works() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("knock")?;

    cmd.arg("--help"); // .arg().arg().arg()
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Knocks on doors"));

    Ok(())
}

#[test]
fn door_help_works() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("door")?;

    cmd.arg("--help"); // .arg().arg().arg()
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Watches the doors"));

    Ok(())
}
