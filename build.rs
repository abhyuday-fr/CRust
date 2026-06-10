fn main() {
    cc::Build::new()
        .file("c_src/telemetry.c")
        .flag("-std=c11")
        .flag("-Wall")
        .compile("telemetry");
    println!("cargo:rerun-if-changed=c_src/telemetry.c");
    println!("cargo:rerun-if-changed=c_src/telemetry.h");
}
