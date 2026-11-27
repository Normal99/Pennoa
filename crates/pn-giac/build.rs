fn main() {
    // Prefer pkg-config (Arch, Fedora); fall back to the plain system paths (Debian/Ubuntu
    // ship libgiac without a .pc file).
    let mut includes = Vec::new();
    match pkg_config::Config::new().cargo_metadata(true).probe("giac") {
        Ok(lib) => includes.extend(lib.include_paths),
        Err(_) => {
            println!("cargo:rustc-link-lib=giac");
            println!("cargo:rustc-link-lib=gmp");
        }
    }

    let mut build = cxx_build::bridge("src/lib.rs");
    build
        .file("cpp/shim.cpp")
        .include("cpp")
        .std("c++17")
        // Giac's headers trigger many warnings we can't fix.
        .flag_if_supported("-w")
        .define("HAVE_CONFIG_H", None)
        .define("IN_GIAC", None);
    for inc in &includes {
        build.include(inc);
    }
    build.compile("pn_giac_shim");

    println!("cargo:rerun-if-changed=cpp/shim.cpp");
    println!("cargo:rerun-if-changed=cpp/shim.h");
    println!("cargo:rerun-if-changed=src/lib.rs");
}
