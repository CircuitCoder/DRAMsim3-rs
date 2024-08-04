use cmake;

fn main() {
    let dst = cmake::build("DRAMsim3");
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=dylib=stdc++");
    println!("cargo:rustc-link-lib=static=dramsim3");
}
