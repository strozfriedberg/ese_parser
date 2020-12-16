fn main() {
    println!(r"cargo:rustc-link-lib=esent");
    println!(r"cargo:rustc-link-lib=Ole32");
    println!(r"cargo:rustc-link-lib=OleAut32");
    println!(r"cargo:rustc-link-search=C:\Program Files (x86)\Windows Kits\10\Lib\10.0.17763.0\um\x64");
}
