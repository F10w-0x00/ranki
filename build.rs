fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // Only compile Kindle Linux 3.0.35 glibc shim & link ARMEL GTK2/Cairo/WebKit libraries
    // when cross-compiling for ARM Linux (so host `cargo test --lib` runs natively with zero GTK deps).
    if target_arch == "arm" && target_os == "linux" {
        // 1. Compile glibc compatibility shim
        cc::Build::new()
            .file("tools/glibc_compat.c")
            .compile("glibc_compat");

        // 2. Link against Kindle OS 5.14.3 whitelisted libraries (Route B: GTK2 + Cairo + Pango + Fontconfig + WebKit)
        println!("cargo:rustc-link-lib=gtk-x11-2.0");
        println!("cargo:rustc-link-lib=gdk-x11-2.0");
        println!("cargo:rustc-link-lib=cairo");
        println!("cargo:rustc-link-lib=pango-1.0");
        println!("cargo:rustc-link-lib=pangocairo-1.0");
        println!("cargo:rustc-link-lib=fontconfig");
        println!("cargo:rustc-link-lib=webkitgtk-1.0");
        println!("cargo:rustc-link-lib=gobject-2.0");
        println!("cargo:rustc-link-lib=glib-2.0");
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=dl");
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=z");

        // 3. Linker flags for armel & Kindle OS 3.0.35 kernel
        println!("cargo:rustc-link-arg=-no-pie");
        println!("cargo:rustc-link-arg=-Wl,--wrap=expf");
        println!("cargo:rustc-link-arg=-Wl,--wrap=exp2f");
        println!("cargo:rustc-link-arg=-Wl,--wrap=powf");
        println!("cargo:rustc-link-arg=-Wl,--wrap=logf");
        println!("cargo:rustc-link-arg=-Wl,--wrap=fcntl");
        println!("cargo:rustc-link-arg=-Wl,--wrap=fcntl64");
        println!("cargo:rustc-link-arg=-Wl,--wrap=copy_file_range");
        println!("cargo:rustc-link-arg=-Wl,--allow-shlib-undefined");
        println!("cargo:rustc-link-arg=-Wl,--as-needed");
    }
}
