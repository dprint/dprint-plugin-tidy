use std::path::Path;

use crate::configuration::Configuration;

/// Gets the language tidy formats the file as, which is decided by its name.
pub fn language_of(file_path: &Path) -> Option<tidy::Language> {
  let file_name = file_path.file_name()?.to_str()?;
  tidy::Language::file(file_name)
}

/// Gets the section of the configuration that has the options of the language (ex. `js`).
pub fn section_of(language: tidy::Language) -> &'static str {
  tidy::schema::of(language)
}

/// The file extensions the plugin formats with the provided configuration.
pub fn file_extensions(config: &Configuration) -> Vec<String> {
  enabled_items(config, |section| section.extensions)
}

/// The names of the files the plugin formats regardless of their extension
/// with the provided configuration.
pub fn file_names(config: &Configuration) -> Vec<String> {
  enabled_items(config, |section| section.file_names)
}

fn enabled_items(config: &Configuration, items: impl Fn(&SectionFiles) -> &'static [&'static str]) -> Vec<String> {
  SECTION_FILES
    .iter()
    .filter(|section| config.is_section_enabled(section.name))
    .flat_map(|section| items(section).iter())
    .map(|item| item.to_string())
    .collect()
}

struct SectionFiles {
  name: &'static str,
  extensions: &'static [&'static str],
  file_names: &'static [&'static str],
}

// These are the files tidy formats (see `tidy::Language::file`), which come from
// Prettier's languages. dprint only matches the last extension of a file, so the
// ones with more than one (ex. `.json.example` or `.yaml.sed`) are left out and
// are only formatted when associated with the plugin. So are the names that
// tidy only matches with a suffix (ex. `Dockerfile.dev`).
const SECTION_FILES: &[SectionFiles] = &[
  SectionFiles {
    name: "html",
    extensions: &["html", "htm", "xht", "xhtml", "mjml"],
    file_names: &[],
  },
  SectionFiles {
    name: "vue",
    extensions: &["vue"],
    file_names: &[],
  },
  SectionFiles {
    name: "handlebars",
    extensions: &["hbs", "handlebars"],
    file_names: &[],
  },
  SectionFiles {
    name: "css",
    extensions: &["css", "wxss", "pcss", "postcss", "scss", "less"],
    file_names: &[],
  },
  SectionFiles {
    name: "json",
    extensions: &[
      "json",
      "avsc",
      "geojson",
      "gltf",
      "har",
      "ice",
      "importmap",
      "mcmeta",
      "sarif",
      "slnlaunch",
      "tact",
      "tfstate",
      "topojson",
      "webapp",
      "webmanifest",
      "yy",
      "yyp",
      "jsonc",
      "code-snippets",
      "code-workspace",
      "sublime-build",
      "sublime-color-scheme",
      "sublime-commands",
      "sublime-completions",
      "sublime-keymap",
      "sublime-macro",
      "sublime-menu",
      "sublime-mousemap",
      "sublime-project",
      "sublime-settings",
      "sublime-theme",
      "sublime-workspace",
      "sublime_metrics",
      "sublime_session",
      "json5",
    ],
    file_names: &[
      ".all-contributorsrc",
      ".arcconfig",
      ".auto-changelog",
      ".c8rc",
      ".htmlhintrc",
      ".imgbotconfig",
      ".nycrc",
      ".tern-config",
      ".tern-project",
      ".watchmanconfig",
      ".babelrc",
      ".jscsrc",
      ".jshintrc",
      ".jslintrc",
      ".swcrc",
    ],
  },
  SectionFiles {
    name: "yaml",
    extensions: &[
      "yml",
      "mir",
      "reek",
      "rviz",
      "sublime-syntax",
      "syntax",
      "yaml",
      "yaml-tmlanguage",
    ],
    file_names: &[
      ".clang-format",
      ".clang-tidy",
      ".clangd",
      ".gemrc",
      "CITATION.cff",
      "glide.lock",
      "pixi.lock",
      ".prettierrc",
      ".stylelintrc",
      ".lintstagedrc",
    ],
  },
  SectionFiles {
    name: "toml",
    extensions: &["toml"],
    file_names: &[],
  },
  SectionFiles {
    name: "xml",
    extensions: &[
      "adml",
      "admx",
      "ant",
      "axaml",
      "axml",
      "brd",
      "builds",
      "ccproj",
      "ccxml",
      "clixml",
      "cproject",
      "cscfg",
      "csdef",
      "csl",
      "csproj",
      "ct",
      "dae",
      "depproj",
      "dita",
      "ditamap",
      "ditaval",
      "dotsettings",
      "filters",
      "fsproj",
      "fxml",
      "glade",
      "gmx",
      "gpx",
      "grxml",
      "gst",
      "hzp",
      "iml",
      "inx",
      "ivy",
      "jelly",
      "jsproj",
      "kid",
      "kml",
      "launch",
      "lvclass",
      "lvlib",
      "lvproj",
      "mdpolicy",
      "mpl",
      "mps",
      "msd",
      "mxml",
      "natvis",
      "ndproj",
      "nproj",
      "nuspec",
      "odd",
      "osm",
      "owl",
      "pkgproj",
      "plist",
      "pluginspec",
      "proj",
      "props",
      "ps1xml",
      "psc1",
      "pt",
      "qhelp",
      "rdf",
      "resx",
      "rss",
      "runsettings",
      "sch",
      "scxml",
      "sfproj",
      "shproj",
      "slnx",
      "srdf",
      "sttheme",
      "storyboard",
      "sublime-snippet",
      "svg",
      "targets",
      "tmcommand",
      "tmlanguage",
      "tmpreferences",
      "tmsnippet",
      "tmtheme",
      "tml",
      "ui",
      "urdf",
      "ux",
      "vbproj",
      "vcxproj",
      "vsixmanifest",
      "vssettings",
      "vstemplate",
      "vxml",
      "wixproj",
      "workflow",
      "wsdl",
      "wsf",
      "wxi",
      "wxl",
      "x3d",
      "xacro",
      "xaml",
      "xib",
      "xlf",
      "xliff",
      "xmi",
      "xml",
      "xmp",
      "xpl",
      "xproc",
      "xproj",
      "xsd",
      "xsl",
      "xslt",
      "xsp-config",
      "xspec",
      "xul",
      "zcml",
    ],
    file_names: &[
      ".classpath",
      ".cproject",
      ".project",
      "app.config",
      "nuget.config",
      "options.stylecop",
      "web.debug.config",
      "web.release.config",
      "web.config",
      "packages.config",
    ],
  },
  SectionFiles {
    name: "markdown",
    extensions: &["md", "markdown"],
    file_names: &[],
  },
  SectionFiles {
    name: "dockerfile",
    extensions: &["dockerfile", "containerfile"],
    file_names: &["Dockerfile", "Containerfile"],
  },
  SectionFiles {
    name: "ini",
    extensions: &[
      "ini",
      "cfg",
      "cnf",
      "dof",
      "lektorproject",
      "prefs",
      "properties",
      "url",
      "sfv",
      "mc",
      "editorconfig",
      "gitconfig",
      "outjob",
      "pcbdoc",
      "prjpcb",
      "schdoc",
    ],
    file_names: &[
      ".coveragerc",
      ".flake8",
      ".pylintrc",
      "pylintrc",
      "buildozer.spec",
      "vlcrc",
      ".editorconfig",
      ".shellcheckrc",
      ".npmrc",
      ".gitconfig",
      ".gitmodules",
      "gitconfig",
    ],
  },
  SectionFiles {
    name: "graphql",
    extensions: &["graphql", "gql", "graphqls"],
    file_names: &[],
  },
  SectionFiles {
    name: "js",
    extensions: &[
      "js",
      "_js",
      "bones",
      "cjs",
      "es",
      "es6",
      "gs",
      "jake",
      "javascript",
      "jsb",
      "jscad",
      "jsfl",
      "jslib",
      "jsm",
      "jspre",
      "jss",
      "mjs",
      "njs",
      "pac",
      "sjs",
      "ssjs",
      "xsjs",
      "xsjslib",
      // tidy formats `.wxs` (WeChat) as JS rather than XML (WiX)
      "wxs",
      "jsx",
      "ts",
      "cts",
      "mts",
      "tsx",
    ],
    file_names: &["Jakefile", "start.frag", "end.frag"],
  },
];

#[cfg(test)]
mod test {
  use super::*;
  use crate::configuration::resolve_config;
  use dprint_core::configuration::ConfigKeyMap;
  use dprint_core::configuration::ConfigKeyValue;
  use dprint_core::configuration::GlobalConfiguration;

  #[test]
  fn sections_are_tidys() {
    let mut names = SECTION_FILES.iter().map(|section| section.name).collect::<Vec<_>>();
    let mut expected = tidy::schema::SECTIONS.to_vec();
    names.sort();
    expected.sort();
    assert_eq!(names, expected);
  }

  #[test]
  fn matched_files_are_formatted_by_their_section() {
    for section in SECTION_FILES {
      let paths = section
        .extensions
        .iter()
        .map(|ext| format!("file.{ext}"))
        .chain(section.file_names.iter().map(|name| name.to_string()));
      for path in paths {
        let language = language_of(Path::new(&path));
        assert_eq!(language.map(section_of), Some(section.name), "{path}");
      }
    }
  }

  #[test]
  fn matched_files_are_classified_case_insensitively() {
    // dprint matches extensions case insensitively, so tidy must too
    for section in SECTION_FILES {
      for ext in section.extensions {
        let path = format!("FILE.{}", ext.to_ascii_uppercase());
        let language = language_of(Path::new(&path));
        assert_eq!(language.map(section_of), Some(section.name), "{path}");
      }
    }
  }

  #[test]
  fn disabling_a_section_only_unmatches_its_files() {
    let all_extensions = file_extensions(&default_config());
    let all_names = file_names(&default_config());
    for section in SECTION_FILES {
      let mut config_map = ConfigKeyMap::new();
      config_map.insert(format!("{}.enabled", section.name), ConfigKeyValue::Bool(false));
      let config = resolve_config(config_map, &GlobalConfiguration::default()).config;
      let extensions = file_extensions(&config);
      let names = file_names(&config);
      for ext in &all_extensions {
        let is_section_ext = section.extensions.contains(&ext.as_str());
        assert_eq!(extensions.contains(ext), !is_section_ext, "{} {ext}", section.name);
      }
      for name in &all_names {
        let is_section_name = section.file_names.contains(&name.as_str());
        assert_eq!(names.contains(name), !is_section_name, "{} {name}", section.name);
      }
    }
  }

  fn default_config() -> Configuration {
    resolve_config(ConfigKeyMap::new(), &GlobalConfiguration::default()).config
  }
}
