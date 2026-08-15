use std::path::Path;
use std::process::Command;

fn run_guard(name: &str) {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository = manifest.parent().unwrap().parent().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_architecture-guards"))
        .arg(repository.join("src"))
        .arg(repository.join("checks/fixtures/architecture-guards"))
        .arg("--guard")
        .arg(name)
        .status()
        .expect("architecture guard checker should start");
    assert!(status.success(), "guard {name} failed with {status}");
}

#[test]
fn no_production_free_functions_fixture() {
    run_guard("free-functions");
}

#[test]
fn no_production_inherent_methods_fixture() {
    run_guard("inherent-methods");
}

#[test]
fn no_zst_behavior_fixture() {
    run_guard("zst-behavior");
}

#[test]
fn no_forbidden_vocabulary_fixture() {
    run_guard("forbidden-vocabulary");
}

#[test]
fn architecture_guards_aggregate_fixtures() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository = manifest.parent().unwrap().parent().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_architecture-guards"))
        .arg(repository.join("src"))
        .arg(repository.join("checks/fixtures/architecture-guards"))
        .status()
        .expect("architecture guard checker should start");
    assert!(
        status.success(),
        "aggregate architecture guard failed with {status}"
    );
}
