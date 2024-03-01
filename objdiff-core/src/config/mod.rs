use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::Result;
use filetime::FileTime;
use globset::{Glob, GlobSet, GlobSetBuilder};

#[inline]
fn bool_true() -> bool { true }

#[derive(Default, Clone, serde::Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub min_version: Option<String>,
    #[serde(default)]
    pub custom_make: Option<String>,
    #[serde(default)]
    pub target_dir: Option<PathBuf>,
    #[serde(default)]
    pub base_dir: Option<PathBuf>,
    #[serde(default = "bool_true")]
    pub build_base: bool,
    #[serde(default)]
    pub build_target: bool,
    #[serde(default)]
    pub watch_patterns: Option<Vec<Glob>>,
    #[serde(default, alias = "units")]
    pub objects: Vec<ProjectObject>,
}

#[derive(Default, Clone, serde::Deserialize)]
pub struct ProjectObject {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub target_path: Option<PathBuf>,
    #[serde(default)]
    pub base_path: Option<PathBuf>,
    #[serde(default)]
    pub reverse_fn_order: Option<bool>,
    #[serde(default)]
    pub complete: Option<bool>,
    #[serde(default)]
    pub scratch: Option<ScratchConfig>,
}

impl ProjectObject {
    pub fn name(&self) -> &str {
        if let Some(name) = &self.name {
            name
        } else if let Some(path) = &self.path {
            path.to_str().unwrap_or("[invalid path]")
        } else {
            "[unknown]"
        }
    }

    pub fn resolve_paths(
        &mut self,
        project_dir: &Path,
        target_obj_dir: Option<&Path>,
        base_obj_dir: Option<&Path>,
    ) {
        if let (Some(target_obj_dir), Some(path), None) =
            (target_obj_dir, &self.path, &self.target_path)
        {
            self.target_path = Some(target_obj_dir.join(path));
        } else if let Some(path) = &self.target_path {
            self.target_path = Some(project_dir.join(path));
        }
        if let (Some(base_obj_dir), Some(path), None) = (base_obj_dir, &self.path, &self.base_path)
        {
            self.base_path = Some(base_obj_dir.join(path));
        } else if let Some(path) = &self.base_path {
            self.base_path = Some(project_dir.join(path));
        }
    }
}

#[derive(Default, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ScratchConfig {
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub compiler: Option<String>,
    #[serde(default)]
    pub c_flags: Option<String>,
    #[serde(default)]
    pub ctx_path: Option<PathBuf>,
    #[serde(default)]
    pub build_ctx: bool,
}

pub const CONFIG_FILENAMES: [&str; 3] = ["objdiff.yml", "objdiff.yaml", "objdiff.json"];

pub const DEFAULT_WATCH_PATTERNS: &[&str] = &[
    "*.c", "*.cp", "*.cpp", "*.cxx", "*.h", "*.hp", "*.hpp", "*.hxx", "*.s", "*.S", "*.asm",
    "*.inc", "*.py", "*.yml", "*.txt", "*.json",
];

#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub project_dir: Option<PathBuf>,
    pub custom_make: Option<String>,
    pub selected_wsl_distro: Option<String>,
}

impl BuildConfig {
    pub(crate) fn from_config(config: &AppConfig) -> Self {
        Self {
            project_dir: config.project_dir.clone(),
            custom_make: config.custom_make.clone(),
            selected_wsl_distro: config.selected_wsl_distro.clone(),
        }
    }
}


#[derive(Clone, Eq, PartialEq)]
pub struct ProjectConfigInfo {
    pub path: PathBuf,
    pub timestamp: FileTime,
}

pub fn try_project_config(dir: &Path) -> Option<(Result<ProjectConfig>, ProjectConfigInfo)> {
    for filename in CONFIG_FILENAMES.iter() {
        let config_path = dir.join(filename);
        let Ok(mut file) = File::open(&config_path) else {
            continue;
        };
        let metadata = file.metadata();
        if let Ok(metadata) = metadata {
            if !metadata.is_file() {
                continue;
            }
            let ts = FileTime::from_last_modification_time(&metadata);
            let config = match filename.contains("json") {
                true => read_json_config(&mut file),
                false => read_yml_config(&mut file),
            };
            return Some((config, ProjectConfigInfo { path: config_path, timestamp: ts }));
        }
    }
    None
}

fn read_yml_config<R: Read>(reader: &mut R) -> Result<ProjectConfig> {
    Ok(serde_yaml::from_reader(reader)?)
}

fn read_json_config<R: Read>(reader: &mut R) -> Result<ProjectConfig> {
    Ok(serde_json::from_reader(reader)?)
}

pub fn build_globset(vec: &[Glob]) -> std::result::Result<GlobSet, globset::Error> {
    let mut builder = GlobSetBuilder::new();
    for glob in vec {
        builder.add(glob.clone());
    }
    builder.build()
}

pub(crate) fn run_make(config: &BuildConfig, arg: &Path) -> BuildStatus {
    let Some(cwd) = &config.project_dir else {
        return BuildStatus {
            success: false,
            stderr: "Missing project dir".to_string(),
            ..Default::default()
        };
    };
    match run_make_cmd(config, cwd, arg) {
        Ok(status) => status,
        Err(e) => BuildStatus { success: false, stderr: e.to_string(), ..Default::default() },
    }
}

fn run_make_cmd(config: &BuildConfig, cwd: &Path, arg: &Path) -> Result<BuildStatus> {
    let make = config.custom_make.as_deref().unwrap_or("make");
    #[cfg(not(windows))]
    let mut command = {
        let mut command = Command::new(make);
        command.current_dir(cwd).arg(arg);
        command
    };
    #[cfg(windows)]
    let mut command = {
        use std::os::windows::process::CommandExt;

        use path_slash::PathExt;
        let mut command = if config.selected_wsl_distro.is_some() {
            Command::new("wsl")
        } else {
            Command::new(make)
        };
        if let Some(distro) = &config.selected_wsl_distro {
            command
                .arg("--cd")
                .arg(cwd)
                .arg("-d")
                .arg(distro)
                .arg("--")
                .arg(make)
                .arg(arg.to_slash_lossy().as_ref());
        } else {
            command.current_dir(cwd).arg(arg.to_slash_lossy().as_ref());
        }
        command.creation_flags(winapi::um::winbase::CREATE_NO_WINDOW);
        command
    };
    let mut cmdline = shell_escape::escape(command.get_program().to_string_lossy()).into_owned();
    for arg in command.get_args() {
        cmdline.push(' ');
        cmdline.push_str(shell_escape::escape(arg.to_string_lossy()).as_ref());
    }
    let output = command.output().context("Failed to execute build")?;
    let stdout = from_utf8(&output.stdout).context("Failed to process stdout")?;
    let stderr = from_utf8(&output.stderr).context("Failed to process stderr")?;
    Ok(BuildStatus {
        success: output.status.code().unwrap_or(-1) == 0,
        cmdline,
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
    })
}
