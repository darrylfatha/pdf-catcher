# Lingkungan build PDF Catcher (Rust di D:\rust, toolchain x86_64-pc-windows-gnu).
# Pakai:  . .\env.ps1   (dengan titik di depan) sebelum menjalankan cargo.
$env:RUSTUP_HOME = 'D:\rust\rustup'
$env:CARGO_HOME = 'D:\rust\cargo'
$sc = 'D:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained'
$env:Path = "D:\rust\cargo\bin;$sc;$env:Path"
# dlltool GNU butuh assembler yang tidak ikut di toolchain minimal; pakai llvm-dlltool.
# Link langsung dengan rust-lld (bawaan Rust): GNU ld menjatuhkan sebagian import descriptor
# sehingga exe crash (0xC0000005) sebelum main.
$lld = 'D:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\rust-lld.exe'
$env:RUSTFLAGS = "-C dlltool=D:\rust\tools\llvm-dlltool.exe -C linker=$lld -C linker-flavor=ld.lld"
