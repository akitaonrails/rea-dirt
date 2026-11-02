use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use rea_launcher::fsx;

use quick_xml::Reader;
use quick_xml::Writer;
use quick_xml::events::{BytesStart, Event};

use crate::config::{Quality, Workers};
use crate::form::Picture;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerSelection {
    pub threshold: u32,
    pub configured_cores: u32,
}

pub fn select_workers(choice: Workers, logical_cpus: usize) -> WorkerSelection {
    match choice {
        Workers::Four => WorkerSelection {
            threshold: 4,
            configured_cores: 4,
        },
        Workers::Eight => WorkerSelection {
            threshold: 8,
            configured_cores: 8,
        },
        Workers::Twelve => WorkerSelection {
            threshold: 12,
            configured_cores: 12,
        },
        Workers::Auto => WorkerSelection {
            threshold: if logical_cpus >= 12 { 12 } else { 8 },
            configured_cores: if logical_cpus >= 12 {
                12
            } else if logical_cpus >= 8 {
                8
            } else if logical_cpus >= 4 {
                4
            } else {
                2
            },
        },
    }
}

#[derive(Debug, Clone, Copy)]
struct Strategy {
    parallel: bool,
    map: &'static str,
    force_feedback: u32,
    dvd: u32,
    dataset: u32,
    update: u32,
    file_stream: u32,
}

fn strategy(cores: u32) -> Strategy {
    match cores {
        12 | 8 => Strategy {
            parallel: true,
            map: if cores == 12 {
                "system/workerMap12Core.xml"
            } else {
                "system/workerMap8Core.xml"
            },
            force_feedback: 6,
            dvd: 7,
            dataset: 4,
            update: 2,
            file_stream: 5,
        },
        4 => Strategy {
            parallel: true,
            map: "system/workerMap4Core.xml",
            force_feedback: 3,
            dvd: 3,
            dataset: 1,
            update: 2,
            file_stream: 3,
        },
        _ => Strategy {
            parallel: true,
            map: "system/workerMap2Core.xml",
            force_feedback: 1,
            dvd: 1,
            dataset: 0,
            update: 1,
            file_stream: 1,
        },
    }
}

fn thread_strategy(cores: u32) -> String {
    let s = strategy(cores);
    format!(
        "parallelUpdateRender=\"{}\" workerMapFile=\"{}\" forceFeedbackProcessor=\"{}\" dvdStorageProcessor=\"{}\" dataSetMonitorProcessor=\"{}\" renderProcessor=\"0\" updateProcessor=\"{}\" fileStreamProcessor=\"{}\"",
        s.parallel, s.map, s.force_feedback, s.dvd, s.dataset, s.update, s.file_stream
    )
}

fn restriction(out: &mut String, condition: &str, cores: u32) {
    writeln!(out, "      <restrict_if {condition}>").unwrap();
    writeln!(out, "        <restrict_add>").unwrap();
    writeln!(
        out,
        "          <threadStrategy {} />",
        thread_strategy(cores)
    )
    .unwrap();
    writeln!(out, "        </restrict_add>").unwrap();
    writeln!(out, "      </restrict_if>").unwrap();
}

pub fn restrictions_xml(choice: Workers, logical_cpus: usize) -> String {
    let selected = select_workers(choice, logical_cpus);
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<hardware_settings_restrictions>\n  <cpu>\n    <restrict_info>\n",
    );
    restriction(&mut out, "name=\"cores\" op=\"int_eq\" value=\"2\"", 2);
    restriction(&mut out, "name=\"cores\" op=\"int_eq\" value=\"3\"", 2);
    if choice != Workers::Four {
        restriction(&mut out, "name=\"cores\" op=\"int_eq\" value=\"4\"", 4);
        // The original and community 6-core maps name workers they do not
        // declare. Keep these machines on the complete 4-core map.
        restriction(&mut out, "name=\"cores\" op=\"int_eq\" value=\"6\"", 4);
    }
    restriction(
        &mut out,
        &format!(
            "name=\"cores\" op=\"int_ge\" value=\"{}\"",
            selected.threshold
        ),
        selected.threshold,
    );
    restriction(
        &mut out,
        "name=\"hyperThreading\" op=\"bo_eq\" value=\"true\"",
        2,
    );
    out.push_str(
        "      <restrict_else>\n        <restrict_add>\n          <threadStrategy parallelUpdateRender=\"false\" workerMapFile=\"system/workerMap.xml\" forceFeedbackProcessor=\"0\" dvdStorageProcessor=\"0\" dataSetMonitorProcessor=\"0\" renderProcessor=\"0\" updateProcessor=\"0\" fileStreamProcessor=\"0\" />\n        </restrict_add>\n      </restrict_else>\n    </restrict_info>\n  </cpu>\n</hardware_settings_restrictions>\n",
    );
    out
}

pub fn worker_map_xml(cores: u32) -> Result<String, String> {
    if ![8, 12].contains(&cores) {
        return Err(format!("no corrected worker map for {cores} cores"));
    }
    let last = cores - 1;
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<workerMap>\n");
    for core in 0..cores {
        writeln!(out, "  <workerThread name=\"core_{core}\" priority=\"abovenormal\" core=\"{core}\" stacksize=\"131072\" jobpoolsize=\"131072\" maxMessages=\"512\" />").unwrap();
    }
    let workers = |out: &mut String, name: &str, cores: &[u32]| {
        writeln!(out, "  <group name=\"{name}\">").unwrap();
        for core in cores {
            writeln!(out, "    <worker name=\"core_{core}\" />").unwrap();
        }
        writeln!(out, "  </group>").unwrap();
    };
    let all_except_main: Vec<u32> = (1..cores).collect();
    let all: Vec<u32> = (0..cores).collect();
    workers(&mut out, "generic_workers", &all_except_main);
    workers(&mut out, "collision", &[last, 5]);
    workers(&mut out, "solver", &[last, 6]);
    workers(&mut out, "plugin_finalise", &[5]);
    workers(&mut out, "cloth", &[]);
    workers(&mut out, "handling", &[last, 5]);
    workers(&mut out, "ai", &[last, 6]);
    workers(&mut out, "vehicleSingleThreaded", &all);
    workers(&mut out, "pfx", &[last]);
    workers(&mut out, "anim", &[3, 4]);
    let in_car: Vec<u32> = if cores == 8 {
        (1..cores).collect()
    } else {
        vec![5, 6, 7]
    };
    workers(&mut out, "in_car_characters", &in_car);
    workers(&mut out, "view_manager", &[2, 3]);
    workers(&mut out, "storage", &all_except_main);
    workers(&mut out, "pssg_storage", &[5]);
    workers(&mut out, "osdUpdateSet", &[6]);
    workers(&mut out, "ground_cover", &[6]);
    out.push_str("</workerMap>\n");
    Ok(out)
}

/// The `hardware_settings_config.xml` the launcher starts from when the game
/// folder has none (launcher-made, every value set by `configure_xml`).
pub const DEFAULT_CONFIG: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<hardware_settings_config version="30">
  <cpu><threadStrategy /></cpu>
  <graphics_card>
    <resolution><refreshRate rate="60" /></resolution>
    <gamma level="1.0" />
    <multisampling><back_buffer /><colour_target /><minimap /><codriver_icon /><dynamic_envmap /><static_envmap /><rear_view_mirror /></multisampling>
    <textures lod="0" />
  </graphics_card>
  <graphics_detail />
  <wind />
  <shadows />
  <particles />
  <crowd />
  <postprocess blur="true" hdr="true" />
  <groundcover />
  <trees />
  <visibility />
  <vehicles />
  <reflections />
  <mirrors enabled="true" forceBilinear="true" left_w="128" left_h="128" middle_w="384" middle_h="256" right_w="64" right_h="64" fmt16bit="true" />
  <skidmarks enabled="true" />
  <dynamic_ambient_occ enabled="true" />
  <motion enabled="false" ip="127.0.0.1" port="20777" delay="1" extradata="0" />
</hardware_settings_config>
"#;

fn set_attr(element: &mut BytesStart, key: &str, value: &str) {
    let attributes: Vec<_> = element
        .attributes()
        .flatten()
        .filter(|attribute| attribute.key.as_ref() != key.as_bytes())
        .map(|attribute| {
            (
                attribute.key.as_ref().to_vec(),
                attribute.value.into_owned(),
            )
        })
        .collect();
    element.clear_attributes();
    for (name, value) in attributes {
        element.push_attribute(quick_xml::events::attributes::Attribute {
            key: quick_xml::name::QName(&name),
            value: std::borrow::Cow::Owned(value),
        });
    }
    element.push_attribute((key, value));
}

fn yes(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn configure_tag(element: &mut BytesStart, picture: &Picture, selection: WorkerSelection) {
    let name = element.name().as_ref().to_vec();
    let (width, height) = picture.resolution.unwrap_or((1920, 1080));
    let quality = picture.quality;
    let (particles, crowd, groundcover, tree_lod, character, vehicle_lod) = match quality {
        Quality::Medium => (2000, 1, 2, 1, 2, 1),
        Quality::High => (4000, 2, 4, 2, 2, 2),
        Quality::Ultra => (6000, 2, 4, 2, 4, 3),
    };
    match name.as_slice() {
        b"threadStrategy" => {
            let s = strategy(selection.configured_cores);
            set_attr(element, "parallelUpdateRender", yes(s.parallel));
            set_attr(element, "workerMapFile", s.map);
            set_attr(
                element,
                "forceFeedbackProcessor",
                &s.force_feedback.to_string(),
            );
            set_attr(element, "dvdStorageProcessor", &s.dvd.to_string());
            set_attr(element, "dataSetMonitorProcessor", &s.dataset.to_string());
            set_attr(element, "renderProcessor", "0");
            set_attr(element, "updateProcessor", &s.update.to_string());
            set_attr(element, "fileStreamProcessor", &s.file_stream.to_string());
        }
        b"resolution" => {
            set_attr(element, "width", &width.to_string());
            set_attr(element, "height", &height.to_string());
            set_attr(
                element,
                "aspect",
                picture
                    .aspect
                    .unwrap_or(crate::config::Aspect::Ratio169)
                    .key(),
            );
            set_attr(element, "fullscreen", yes(picture.fullscreen));
            set_attr(element, "vsync", if picture.vsync { "1" } else { "0" });
        }
        b"refreshRate" => set_attr(element, "rate", "60"),
        b"graphics_detail" => set_attr(element, "level", quality.key()),
        b"multisampling" => set_attr(element, "level", "4"),
        b"back_buffer" | b"colour_target" | b"minimap" | b"codriver_icon" => {
            set_attr(element, "level", "4")
        }
        b"dynamic_envmap" | b"static_envmap" | b"rear_view_mirror" => {
            set_attr(element, "level", "0")
        }
        b"wind" => {
            let enabled = quality != Quality::Medium;
            set_attr(element, "particles", yes(enabled));
            set_attr(element, "groundcover", yes(enabled));
        }
        b"shadows" => {
            set_attr(element, "enabled", "true");
            set_attr(
                element,
                "size",
                if quality == Quality::Medium {
                    "1024"
                } else {
                    "2048"
                },
            );
            set_attr(
                element,
                "maskQuality",
                if quality == Quality::Medium { "1" } else { "2" },
            );
        }
        b"particles" => {
            set_attr(element, "enabled", "true");
            set_attr(element, "collisions", "true");
            set_attr(element, "max", &particles.to_string());
        }
        b"crowd" => {
            set_attr(element, "enabled", "true");
            set_attr(element, "lodQuality", &crowd.to_string());
        }
        b"groundcover" => {
            set_attr(element, "enabled", "true");
            set_attr(element, "layers", &groundcover.to_string());
        }
        b"trees" => {
            set_attr(
                element,
                "visibility",
                if quality == Quality::Medium {
                    "1000"
                } else {
                    "2000"
                },
            );
            set_attr(element, "low", "180");
            set_attr(element, "high", "60");
            set_attr(element, "lodQuality", &tree_lod.to_string());
        }
        b"visibility" => {
            set_attr(element, "level", "high");
            set_attr(
                element,
                "clip",
                if quality == Quality::Ultra {
                    "25000"
                } else {
                    "15000"
                },
            );
            set_attr(
                element,
                "cull",
                if quality == Quality::Ultra {
                    "4000"
                } else {
                    "2000"
                },
            );
        }
        b"vehicles" => {
            set_attr(element, "characterQuality", &character.to_string());
            set_attr(element, "lodQuality", &vehicle_lod.to_string());
        }
        b"reflections" => {
            set_attr(
                element,
                "faces",
                if quality == Quality::Medium { "3" } else { "6" },
            );
            set_attr(
                element,
                "size",
                if quality == Quality::Ultra {
                    "512"
                } else {
                    "256"
                },
            );
            set_attr(element, "forceBilinear", yes(quality != Quality::Ultra));
        }
        b"mirrors" => set_attr(element, "enabled", "true"),
        b"skidmarks" | b"dynamic_ambient_occ" => set_attr(element, "enabled", "true"),
        b"textures" => set_attr(element, "lod", "0"),
        _ => {}
    }
}

pub fn configure_xml(
    input: &str,
    picture: &Picture,
    selection: WorkerSelection,
) -> Result<String, String> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Vec::new());
    loop {
        match reader
            .read_event()
            .map_err(|error| format!("invalid hardware settings XML: {error}"))?
        {
            Event::Eof => break,
            Event::Start(mut element) => {
                configure_tag(&mut element, picture, selection);
                writer
                    .write_event(Event::Start(element))
                    .map_err(|error| error.to_string())?;
            }
            Event::Empty(mut element) => {
                configure_tag(&mut element, picture, selection);
                writer
                    .write_event(Event::Empty(element))
                    .map_err(|error| error.to_string())?;
            }
            event => writer
                .write_event(event)
                .map_err(|error| error.to_string())?,
        }
    }
    String::from_utf8(writer.into_inner()).map_err(|error| error.to_string())
}

/// Writes `contents` as a whole-file swap (`rea_launcher::fsx::install`:
/// never through a hard link, nothing written when the bytes are already
/// there).
fn install_xml(path: &Path, contents: &str) -> Result<(), String> {
    fsx::install(path, contents)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

pub fn configure_runtime(
    runtime: &Path,
    picture: &Picture,
    workers: Workers,
    logical_cpus: usize,
) -> Result<WorkerSelection, String> {
    let selection = select_workers(workers, logical_cpus);
    let system = runtime.join("system");
    for cores in [8, 12] {
        install_xml(
            &system.join(format!("workerMap{cores}Core.xml")),
            &worker_map_xml(cores)?,
        )?;
    }
    install_xml(
        &system.join("hardware_settings_restrictions.xml"),
        &restrictions_xml(workers, logical_cpus),
    )?;
    let default = fs::read_to_string(system.join("hardware_settings_config.xml"))
        .unwrap_or_else(|_| DEFAULT_CONFIG.into());
    let configured = configure_xml(&default, picture, selection)?;
    install_xml(&system.join("hardware_settings_config.xml"), &configured)?;
    configure_user(
        &runtime
            .join("hardwaresettings")
            .join("hardware_settings_config.xml"),
        picture,
        selection,
    )?;
    Ok(selection)
}

pub fn user_config_path() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Documents")
        .join("Codemasters")
        .join("DiRT")
        .join("hardwaresettings")
        .join("hardware_settings_config.xml")
}

pub fn configure_user(
    path: &Path,
    picture: &Picture,
    selection: WorkerSelection,
) -> Result<(), String> {
    let current = fs::read_to_string(path).unwrap_or_else(|_| DEFAULT_CONFIG.into());
    let configured = configure_xml(&current, picture, selection)?;
    install_xml(path, &configured)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Aspect;

    fn picture() -> Picture {
        Picture {
            resolution: Some((3840, 2160)),
            aspect: Some(Aspect::Ratio169),
            quality: Quality::Ultra,
            fullscreen: true,
            vsync: true,
        }
    }

    #[test]
    fn auto_selects_only_complete_maps() {
        assert_eq!(select_workers(Workers::Auto, 32).configured_cores, 12);
        assert_eq!(select_workers(Workers::Auto, 10).configured_cores, 8);
        assert_eq!(select_workers(Workers::Auto, 6).configured_cores, 4);
        assert_eq!(select_workers(Workers::Auto, 2).configured_cores, 2);
    }

    #[test]
    fn generated_workers_never_reference_missing_cores() {
        for cores in [8, 12] {
            let xml = worker_map_xml(cores).unwrap();
            assert_eq!(xml.matches("<workerThread ").count(), cores as usize);
            for core in 0..cores {
                assert!(xml.contains(&format!("name=\"core_{core}\"")));
            }
            assert!(!xml.contains(&format!("name=\"core_{cores}\"")));
        }
    }

    #[test]
    fn six_core_rule_avoids_the_broken_map() {
        let xml = restrictions_xml(Workers::Auto, 6);
        assert!(xml.contains("value=\"6\""));
        assert!(!xml.contains("workerMap6Core.xml"));
        assert!(xml.contains("workerMap4Core.xml"));
    }

    #[test]
    fn config_is_4k_ultra_and_idempotent() {
        let selection = select_workers(Workers::Auto, 32);
        let once = configure_xml(DEFAULT_CONFIG, &picture(), selection).unwrap();
        assert!(once.contains("width=\"3840\""));
        assert!(once.contains("height=\"2160\""));
        assert!(once.contains("aspect=\"16:9\""));
        assert!(once.contains("level=\"ultra\""));
        assert!(once.contains("workerMapFile=\"system/workerMap12Core.xml\""));
        assert!(once.contains("size=\"512\""));
        let twice = configure_xml(&once, &picture(), selection).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn unrelated_player_settings_survive() {
        let input = DEFAULT_CONFIG
            .replace("level=\"1.0\"", "level=\"1.3\"")
            .replace("voices=\"40\"", "voices=\"80\"");
        let output = configure_xml(&input, &picture(), select_workers(Workers::Auto, 8)).unwrap();
        assert!(output.contains("gamma level=\"1.3\""));
        assert!(output.contains("workerMap8Core.xml"));
    }

    #[test]
    fn lower_presets_do_not_use_ultra_values() {
        let mut picture = picture();
        picture.quality = Quality::Medium;
        let output =
            configure_xml(DEFAULT_CONFIG, &picture, select_workers(Workers::Auto, 4)).unwrap();
        assert!(output.contains("graphics_detail level=\"medium\""));
        assert!(output.contains("max=\"2000\""));
        assert!(output.contains("clip=\"15000\""));
    }

    #[test]
    fn runtime_config_covers_the_games_local_cache() {
        let runtime =
            std::env::temp_dir().join(format!("dirt-hardware-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&runtime);
        configure_runtime(&runtime, &picture(), Workers::Auto, 32).unwrap();
        let cached = fs::read_to_string(
            runtime
                .join("hardwaresettings")
                .join("hardware_settings_config.xml"),
        )
        .unwrap();
        assert!(cached.contains("width=\"3840\""));
        assert!(cached.contains("workerMap12Core.xml"));
        fs::remove_dir_all(runtime).unwrap();
    }
}
