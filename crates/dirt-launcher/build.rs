// Version information, an icon and a manifest in DiRTLauncher.exe: a named,
// versioned program is less likely to be flagged by antivirus heuristics
// (sdk/docs/STANDARDS.md, "Antivirus"). Does nothing for non-Windows targets.
fn main() {
    rea_launcher_build::Resources::new("DiRT Launcher", "Colin McRae: DiRT launcher")
        .company("AkitaOnRails")
        .file_name("DiRTLauncher.exe")
        .icon("../../packaging/appimage/dirt-launcher.png")
        .embed()
        .unwrap();
}
