use std::path::PathBuf;
use std::process::{Command, Output};

fn kernel_base() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../kernels/data")
}

fn kernels_available() -> bool {
    let base = kernel_base();
    base.join("de442s.bsp").exists()
        && base.join("naif0012.tls").exists()
        && base.join("finals2000A.all").exists()
}

fn cli_bin() -> &'static str {
    env!("CARGO_BIN_EXE_dhruv_cli")
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(cli_bin())
        .args(args)
        .output()
        .expect("cli process should start")
}

fn kernel_args() -> Vec<String> {
    let base = kernel_base();
    vec![
        "--bsp".to_string(),
        base.join("de442s.bsp").display().to_string(),
        "--lsk".to_string(),
        base.join("naif0012.tls").display().to_string(),
        "--eop".to_string(),
        base.join("finals2000A.all").display().to_string(),
    ]
}

fn common_date_location_args() -> Vec<String> {
    vec![
        "--date".to_string(),
        "2025-01-15T12:00:00Z".to_string(),
        "--lat".to_string(),
        "12.9716".to_string(),
        "--lon".to_string(),
        "77.5946".to_string(),
        "--alt".to_string(),
        "920".to_string(),
    ]
}

fn command_args(subcommand: &str) -> Vec<String> {
    let mut args = vec!["--no-config".to_string(), subcommand.to_string()];
    args.extend(common_date_location_args());
    args.extend(kernel_args());
    args
}

fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cli_bala_commands_accept_amsha_selection() {
    if !kernels_available() {
        eprintln!("Skipping: kernel files not found");
        return;
    }

    let shadbala_default_args = command_args("shadbala");
    let shadbala_default_refs: Vec<&str> = shadbala_default_args.iter().map(String::as_str).collect();
    let shadbala_default = run_cli(&shadbala_default_refs);
    assert_success(&shadbala_default, "shadbala default");

    let mut shadbala_override_args = command_args("shadbala");
    shadbala_override_args.extend(["--amsha".to_string(), "D2:cancer-leo-only".to_string()]);
    let shadbala_override_refs: Vec<&str> =
        shadbala_override_args.iter().map(String::as_str).collect();
    let shadbala_override = run_cli(&shadbala_override_refs);
    assert_success(&shadbala_override, "shadbala override");
    let shadbala_default_stdout = String::from_utf8_lossy(&shadbala_default.stdout);
    let shadbala_override_stdout = String::from_utf8_lossy(&shadbala_override.stdout);
    assert!(shadbala_override_stdout.contains("Shadbala for"));
    assert_ne!(shadbala_default_stdout, shadbala_override_stdout);

    let vimsopaka_default_args = command_args("vimsopaka");
    let vimsopaka_default_refs: Vec<&str> =
        vimsopaka_default_args.iter().map(String::as_str).collect();
    let vimsopaka_default = run_cli(&vimsopaka_default_refs);
    assert_success(&vimsopaka_default, "vimsopaka default");

    let mut vimsopaka_override_args = command_args("vimsopaka");
    vimsopaka_override_args.extend(["--amsha".to_string(), "D2:cancer-leo-only".to_string()]);
    let vimsopaka_override_refs: Vec<&str> =
        vimsopaka_override_args.iter().map(String::as_str).collect();
    let vimsopaka_override = run_cli(&vimsopaka_override_refs);
    assert_success(&vimsopaka_override, "vimsopaka override");
    let vimsopaka_default_stdout = String::from_utf8_lossy(&vimsopaka_default.stdout);
    let vimsopaka_override_stdout = String::from_utf8_lossy(&vimsopaka_override.stdout);
    assert!(vimsopaka_override_stdout.contains("Vimsopaka Bala"));
    assert_ne!(vimsopaka_default_stdout, vimsopaka_override_stdout);

    let balas_default_args = command_args("balas");
    let balas_default_refs: Vec<&str> = balas_default_args.iter().map(String::as_str).collect();
    let balas_default = run_cli(&balas_default_refs);
    assert_success(&balas_default, "balas default");

    let mut balas_override_args = command_args("balas");
    balas_override_args.extend(["--amsha".to_string(), "D2:cancer-leo-only".to_string()]);
    let balas_override_refs: Vec<&str> = balas_override_args.iter().map(String::as_str).collect();
    let balas_override = run_cli(&balas_override_refs);
    assert_success(&balas_override, "balas override");
    let balas_default_stdout = String::from_utf8_lossy(&balas_default.stdout);
    let balas_override_stdout = String::from_utf8_lossy(&balas_override.stdout);
    assert!(balas_override_stdout.contains("Shadbala:"));
    assert!(balas_override_stdout.contains("Vimsopaka Bala:"));
    assert_ne!(balas_default_stdout, balas_override_stdout);

    let mut avastha_args = command_args("avastha");
    avastha_args.extend(["--amsha".to_string(), "D9".to_string()]);
    let avastha_refs: Vec<&str> = avastha_args.iter().map(String::as_str).collect();
    let avastha = run_cli(&avastha_refs);
    assert_success(&avastha, "avastha override");
    assert!(String::from_utf8_lossy(&avastha.stdout).contains("Graha Avasthas"));
}

#[test]
fn cli_kundali_amsha_output_includes_resolved_union() {
    if !kernels_available() {
        eprintln!("Skipping: kernel files not found");
        return;
    }

    let mut args = command_args("kundali");
    args.extend([
        "--include-amshas".to_string(),
        "--include-shadbala".to_string(),
        "--include-vimsopaka".to_string(),
        "--amsha".to_string(),
        "D2:cancer-leo-only".to_string(),
    ]);

    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run_cli(&refs);
    assert_success(&output, "kundali resolved union");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Amsha Charts (16 chart(s)):"));
    assert!(stdout.contains("Hora (D2) [cancer-leo-only]:"));
    assert!(stdout.contains("Shashtiamsha (D60):"));
}

#[test]
fn cli_amsha_variations_lists_catalogs() {
    let output = run_cli(&["--no-config", "amsha-variations", "--amsha", "D2,D9"]);
    assert_success(&output, "amsha-variations");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hora (D2):"));
    assert!(stdout.contains("0   default"));
    assert!(stdout.contains("1   cancer-leo-only"));
    assert!(stdout.contains("Navamsha (D9):"));
}
