use std::env;

fn main() {
    let arch = env::var("ARCH").unwrap_or_default();
    println!("cargo:rustc-link-arg=-T../../kernel/src/arch/{}/kernel.ld", arch);
}
