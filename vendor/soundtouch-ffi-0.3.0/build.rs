use std::path::PathBuf;
const SOUNDTOUCH_DIR: &str = "soundtouch-2_3_2";

fn build() {
    let soundtouch_dir = std::path::Path::new(SOUNDTOUCH_DIR);
    let source_dir = soundtouch_dir.join("source").join("SoundTouch");

    let mut cc = cc::Build::new();
    cc.warnings(true)
        .cpp(true)
        .extra_warnings(true)
        .file(source_dir.join("AAFilter.cpp"))
        .file(source_dir.join("BPMDetect.cpp"))
        .file(source_dir.join("FIFOSampleBuffer.cpp"))
        .file(source_dir.join("FIRFilter.cpp"))
        .file(source_dir.join("InterpolateCubic.cpp"))
        .file(source_dir.join("InterpolateLinear.cpp"))
        .file(source_dir.join("InterpolateShannon.cpp"))
        .file(source_dir.join("PeakFinder.cpp"))
        .file(source_dir.join("RateTransposer.cpp"))
        .file(source_dir.join("SoundTouch.cpp"))
        .file(source_dir.join("TDStretch.cpp"))
        .file(source_dir.join("cpu_detect_x86.cpp"))
        .file(source_dir.join("mmx_optimized.cpp"))
        .file(source_dir.join("sse_optimized.cpp"))
        .include(soundtouch_dir.join("include"))
        .include(soundtouch_dir.join("source/SoundTouch"))
        .pic(false)
        .warnings(false);

    if let Ok(compiler) = std::env::var("CC") {
        let compiler = std::path::Path::new(&compiler);
        let compiler = compiler
            .file_stem()
            .expect("To have file name in CC")
            .to_str()
            .unwrap();
        if compiler == "clang-cl" {
            cc.flag("/W0");
        }
    }

    cc.compile("SoundTouch")
}

fn main() {
    if std::env::var("DOCS_RS")
        .map(|docs| docs == "1")
        .unwrap_or(false)
    {
        //skip docs.rs build
        return;
    }
    const PREPEND_LIB: &'static str = "
#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub use root::{soundtouch::*, TDStretch, uint};
";

    let mut out = PathBuf::new();
    out.push("src");
    out.push("lib.rs");
    let header = PathBuf::from("wrapper.hpp");

    let bindings = bindgen::Builder::default()
        .header(header.display().to_string())
        .raw_line(PREPEND_LIB)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate_comments(true)
        .layout_tests(false)
        .constified_enum_module("*")
        .allowlist_type("soundtouch::SoundTouch")
        .allowlist_type("soundtouch::SAMPLETYPE")
        .allowlist_type("soundtouch::BPMDetect")
        .allowlist_type("soundtouch::TDStretch")
        .allowlist_type("soundtouch::RateTransposer")
        .opaque_type("std::.*")
        .manually_drop_union(".*")
        .default_non_copy_union_style(bindgen::NonCopyUnionStyle::ManuallyDrop)
        .use_core()
        .enable_cxx_namespaces()
        .trust_clang_mangling(true)
        .clang_arg("-x")
        .clang_arg("c++")
        .generate()
        .expect("Unable to generate SoundTouch bindings");

    bindings
        .write_to_file(out)
        .expect("Couldn't write bindings!");

    // Platform default logic when no feature is explicitly set:
    // - musl => static
    // - linux/bsd, non-musl => dynamic
    // - macos => static
    // - windows => static

    #[cfg(all(feature = "bundled", feature = "dynamic"))]
    compile_error!("Choose exactly one of 'bundled' or 'dynamic'.");

    // If user explicitly opted into dynamic (feature = "dynamic"), override below.
    #[cfg(all(not(feature = "bundled"), any(
        // dynamic by default: linux & bsd non-musl
        all(unix, not(target_env = "musl"), not(target_os = "macos"))
    , feature = "dynamic")))]
    link_system();

    #[cfg(any(
        feature = "bundled",
        // static by default: musl, macos, windows
        all(unix, target_env = "musl", not(feature = "dynamic")),
        target_os = "macos",
        windows
    ))]
    build();
}
