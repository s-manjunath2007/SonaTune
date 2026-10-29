fn main() {
    let manifest_content = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>"#;

    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        let out_path = std::path::Path::new(&out_dir);
        let _ = std::fs::write(out_path.join("sonatune.exe.manifest"), manifest_content);
        // Also place directly into the target binary directory (target/debug or target/release)
        if let Some(target_dir) = out_path.ancestors().nth(3) {
            let _ = std::fs::write(target_dir.join("sonatune.exe.manifest"), manifest_content);
        }
    }

    tauri_build::build()
}
