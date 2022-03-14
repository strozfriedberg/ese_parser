#[cfg(target_os = "macos")]
fn main() {
    println!(r"cargo:rustc-cdylib-link-arg=-undefined");
    println!(r"cargo:rustc-cdylib-link-arg=dynamic_lookup");
}
#[cfg(target_os = "windows")]
fn main() {
    println!(r"cargo:rustc-link-search=../docker/Python37/libs");
}
#[cfg(target_os = "linux")]
fn main() {}
