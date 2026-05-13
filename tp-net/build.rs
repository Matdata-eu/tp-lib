fn main() {
    // csbindgen scans lib.rs at build time and emits C# P/Invoke declarations.
    // Stub for Phase 1 — generation enabled in Phase 2 once FFI fns exist.
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/ffi.rs");

    csbindgen::Builder::default()
        .input_extern_file("src/lib.rs")
        .input_extern_file("src/ffi.rs")
        .csharp_dll_name("tp_lib_net")
        .csharp_namespace("TpLib")
        .csharp_class_name("NativeMethods")
        .csharp_class_accessibility("internal")
        .generate_csharp_file("csharp/NativeMethods.g.cs")
        .expect("failed to generate csharp/NativeMethods.g.cs with csbindgen");
}
