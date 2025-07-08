fn main() {
    #[cfg(target_os = "windows")]
    {
        // Link Windows CRT libraries
        println!("cargo:rustc-link-lib=msvcrt");
        println!("cargo:rustc-link-lib=kernel32");
        println!("cargo:rustc-link-lib=user32");
    }
} 