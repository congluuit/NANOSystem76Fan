// Embeds the application manifest (administrator + DPI awareness), the icon and
// version information into the Windows executable.

include!("src/icon_art.rs");

use std::io::Write;
use std::path::Path;

/// Accent colour of the app icon; keep in sync with `ui::theme::ACCENT`.
const ACCENT: [u8; 3] = [0x2E, 0xA8, 0xF0];

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="S76FanControl" version="1.0.0.0"/>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <!-- Port I/O to the embedded controller needs administrator rights. -->
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
        processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
</assembly>
"#;

fn main() {
    println!("cargo:rerun-if-changed=src/icon_art.rs");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=vendor/inpout/inpoutx64.dll");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let ico = Path::new(&out_dir).join("app.ico");
    write_ico(&ico, &[16, 20, 24, 32, 40, 48, 64, 256]).expect("write icon");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().unwrap())
        .set_manifest(MANIFEST)
        .set("ProductName", "S76 Fan Control")
        .set("FileDescription", "S76 Fan Control")
        .set("LegalCopyright", "MIT License");
    // Only the .exe gets the resources: the test binaries must not require elevation.
    res.compile_for("S76FanControl").expect("compile Windows resources");

    copy_inpout(&out_dir);
}

/// Copies the bundled InpOut DLL next to the built executable so `cargo run` works.
fn copy_inpout(out_dir: &str) {
    let src = Path::new("vendor/inpout/inpoutx64.dll");
    // OUT_DIR = target/<profile>/build/<pkg>-<hash>/out
    let Some(profile_dir) = Path::new(out_dir).ancestors().nth(3) else { return };
    if src.is_file() {
        let _ = std::fs::copy(src, profile_dir.join("inpoutx64.dll"));
    }
}

/// Writes a multi-size .ico with 32-bit BMP images.
fn write_ico(path: &Path, sizes: &[u32]) -> std::io::Result<()> {
    let images: Vec<Vec<u8>> = sizes.iter().map(|&s| bmp_entry(s)).collect();
    let mut f = std::fs::File::create(path)?;
    f.write_all(&[0, 0, 1, 0])?;
    f.write_all(&(sizes.len() as u16).to_le_bytes())?;
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (s, img) in sizes.iter().zip(&images) {
        let dim = if *s >= 256 { 0 } else { *s as u8 };
        f.write_all(&[dim, dim, 0, 0])?;
        f.write_all(&1u16.to_le_bytes())?; // planes
        f.write_all(&32u16.to_le_bytes())?; // bpp
        f.write_all(&(img.len() as u32).to_le_bytes())?;
        f.write_all(&offset.to_le_bytes())?;
        offset += img.len() as u32;
    }
    for img in &images {
        f.write_all(img)?;
    }
    Ok(())
}

fn bmp_entry(size: u32) -> Vec<u8> {
    let rgba = render_icon(size, ACCENT);
    let mask_row = size.div_ceil(32) * 4;
    let mut v = Vec::new();
    // BITMAPINFOHEADER; height is doubled (XOR + AND masks).
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&(size as i32).to_le_bytes());
    v.extend_from_slice(&((size * 2) as i32).to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&32u16.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&(size * size * 4 + mask_row * size).to_le_bytes());
    v.extend_from_slice(&[0u8; 16]);
    // Pixels, bottom-up, BGRA.
    for y in (0..size).rev() {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            v.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    // AND mask: all zero, transparency comes from the alpha channel.
    v.extend(std::iter::repeat_n(0u8, (mask_row * size) as usize));
    v
}
