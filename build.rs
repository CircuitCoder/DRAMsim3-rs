use cmake;

fn main() {
    println!("cargo::rerun-if-changed=DRAMsim3");

    let dst = cmake::build("DRAMsim3");
    println!("cargo::rustc-link-search=native={}/lib", dst.display());
    println!("cargo::rustc-link-lib=static=dramsim3");
    println!("cargo::rustc-link-lib=dylib=stdc++");
}
