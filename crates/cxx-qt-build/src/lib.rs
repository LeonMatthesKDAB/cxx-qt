// SPDX-FileCopyrightText: 2021 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
// SPDX-FileContributor: Be Wilson <be.wilson@kdab.com>
// SPDX-FileContributor: Gerhard de Clercq <gerhard.declercq@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![deny(missing_docs)]

//! This crate provides a builder which parses given Rust source code to search
//! for CXX-Qt or CXX macros and generate any resulting C++ code. It also builds
//! the C++ code into a binary with any cxx-qt-lib code and Qt linked.

mod cfg_evaluator;

mod diagnostics;
use diagnostics::{Diagnostic, GeneratedError};

mod opts;
pub use opts::CxxQtBuildersOpts;
pub use opts::QObjectHeaderOpts;

mod qml_modules;
use qml_modules::OwningQmlModule;
pub use qml_modules::QmlModule;

pub use qt_build_utils::MocArguments;
use qt_build_utils::SemVer;
use quote::ToTokens;
use std::{
    collections::HashSet,
    env,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use cxx_qt_gen::{
    parse_qt_file, write_cpp, write_rust, CppFragment, CxxQtItem, GeneratedCppBlocks,
    GeneratedRustBlocks, Parser,
};

// TODO: we need to eventually support having multiple modules defined in a single file. This
// is currently an issue because we are using the Rust file name to derive the cpp file name
// and are blindly re-writing files.
//
// As we use struct names for the QObject files, we should actually be able to support multiple
// QObject macros and at most one "raw CXX" macro per file already. For now this remains a TODO
// as to keep things simpler. We also want to able to warn users about duplicate names eventually.

struct GeneratedCppFilePaths {
    plain_cpp: PathBuf,
    qobject: Option<PathBuf>,
    qobject_header: Option<PathBuf>,
}

struct GeneratedCpp {
    cxx_qt: Option<CppFragment>,
    cxx: cxx_gen::GeneratedCode,
    file_ident: String,
}

impl GeneratedCpp {
    /// Generate QObject and cxx header/source C++ file contents
    pub fn new(rust_file_path: impl AsRef<Path>) -> Result<Self, Diagnostic> {
        let to_diagnostic = |err| Diagnostic::new(rust_file_path.as_ref().to_owned(), err);

        let rust_file_path = rust_file_path.as_ref();

        let file = parse_qt_file(rust_file_path)
            .map_err(GeneratedError::from)
            .map_err(to_diagnostic)?;

        let mut cxx_qt = None;
        let mut file_ident: String = "".to_owned();
        let mut tokens = proc_macro2::TokenStream::new();

        // Add any attributes in the file into the tokenstream
        for attr in &file.attrs {
            tokens.extend(attr.into_token_stream());
        }

        // Loop through the items looking for any CXX or CXX-Qt blocks
        for item in &file.items {
            match item {
                CxxQtItem::Cxx(m) => {
                    // TODO: later we will allow for multiple CXX or CXX-Qt blocks in one file
                    if !file_ident.is_empty() {
                        panic!(
                            "Unfortunately only files with either a single cxx or a single cxx_qt module are currently supported.
                            The file {} has more than one of these.",
                            rust_file_path.display());
                    }

                    // Match upstream where they use the file name as the ident
                    //
                    // TODO: what happens if there are folders?
                    //
                    // TODO: ideally CXX-Qt would also use the file name
                    // https://github.com/KDAB/cxx-qt/pull/200/commits/4861c92e66c3a022d3f0dedd9f8fd20db064b42b
                    rust_file_path
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .clone_into(&mut file_ident);
                    tokens.extend(m.into_token_stream());
                }
                CxxQtItem::CxxQt(m) => {
                    // TODO: later we will allow for multiple CXX or CXX-Qt blocks in one file
                    if !file_ident.is_empty() {
                        panic!(
                            "Unfortunately only files with either a single cxx or a single cxx_qt module are currently supported.
                            The file {} has more than one of these.",
                            rust_file_path.display());
                    }

                    let parser = Parser::from(m.clone())
                        .map_err(GeneratedError::from)
                        .map_err(to_diagnostic)?;
                    let generated_cpp = GeneratedCppBlocks::from(&parser)
                        .map_err(GeneratedError::from)
                        .map_err(to_diagnostic)?;
                    // TODO: we'll have to extend the C++ data here rather than overwriting
                    // assuming we share the same file
                    cxx_qt = Some(write_cpp(&generated_cpp));

                    let generated_rust = GeneratedRustBlocks::from(&parser)
                        .map_err(GeneratedError::from)
                        .map_err(to_diagnostic)?;
                    let rust_tokens = write_rust(&generated_rust);
                    file_ident.clone_from(&parser.cxx_file_stem);

                    // We need to do this and can't rely on the macro, as we need to generate the
                    // CXX bridge Rust code that is then fed into the cxx_gen generation.
                    tokens.extend(rust_tokens);
                }
                CxxQtItem::Item(item) => {
                    tokens.extend(item.into_token_stream());
                }
            }
        }

        let mut opt = cxx_gen::Opt::default();
        opt.cfg_evaluator = Box::new(cfg_evaluator::CargoEnvCfgEvaluator);
        let cxx = cxx_gen::generate_header_and_cc(tokens, &opt)
            .map_err(GeneratedError::from)
            .map_err(to_diagnostic)?;

        Ok(GeneratedCpp {
            cxx_qt,
            cxx,
            file_ident,
        })
    }

    /// Write generated .cpp and .h files to specified directories. Returns the paths of all files written.
    pub fn write_to_directories(
        self,
        cpp_directory: impl AsRef<Path>,
        header_directory: impl AsRef<Path>,
    ) -> GeneratedCppFilePaths {
        let cpp_directory = cpp_directory.as_ref();
        let header_directory = header_directory.as_ref();
        for directory in [cpp_directory, header_directory] {
            std::fs::create_dir_all(directory)
                .expect("Could not create directory to write cxx-qt generated files");
        }

        let mut cpp_file_paths = GeneratedCppFilePaths {
            plain_cpp: PathBuf::new(),
            qobject: None,
            qobject_header: None,
        };
        if let Some(cxx_qt_generated) = &self.cxx_qt {
            let header_path = PathBuf::from(format!(
                "{}/{}.cxxqt.h",
                header_directory.display(),
                self.file_ident
            ));
            let mut header =
                File::create(&header_path).expect("Could not create cxx-qt header file");
            let header_generated = match cxx_qt_generated {
                CppFragment::Pair { header, source: _ } => header,
                CppFragment::Header(header) => header,
                CppFragment::Source(_) => panic!("Unexpected call for source fragment."),
            };
            header
                .write_all(header_generated.as_bytes())
                .expect("Could not write cxx-qt header file");
            cpp_file_paths.qobject_header = Some(header_path);

            let cpp_path = PathBuf::from(format!(
                "{}/{}.cxxqt.cpp",
                cpp_directory.display(),
                self.file_ident
            ));
            let mut cpp = File::create(&cpp_path).expect("Could not create cxx-qt source file");
            let source_generated = match cxx_qt_generated {
                CppFragment::Pair { header: _, source } => source,
                CppFragment::Header(_) => panic!("Unexpected call for header fragment."),
                CppFragment::Source(source) => source,
            };
            cpp.write_all(source_generated.as_bytes())
                .expect("Could not write cxx-qt source file");
            cpp_file_paths.qobject = Some(cpp_path);
        }

        let header_path = PathBuf::from(format!(
            "{}/{}.cxx.h",
            header_directory.display(),
            self.file_ident
        ));
        let mut header = File::create(header_path).expect("Could not create cxx header file");
        header
            .write_all(&self.cxx.header)
            .expect("Could not write cxx header file");

        let cpp_path = PathBuf::from(format!(
            "{}/{}.cxx.cpp",
            cpp_directory.display(),
            self.file_ident
        ));
        let mut cpp = File::create(&cpp_path).expect("Could not create cxx source file");
        cpp.write_all(&self.cxx.implementation)
            .expect("Could not write cxx source file");
        cpp_file_paths.plain_cpp = cpp_path;

        cpp_file_paths
    }
}

/// Generate C++ files from a given list of Rust files, returning the generated paths
fn generate_cxxqt_cpp_files(
    rs_source: &[impl AsRef<Path>],
    header_dir: impl AsRef<Path>,
) -> Vec<GeneratedCppFilePaths> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let mut generated_file_paths: Vec<GeneratedCppFilePaths> = Vec::with_capacity(rs_source.len());
    for rs_path in rs_source {
        let cpp_directory = out_dir().join("cxx-qt-gen/src");
        let path = manifest_dir.join(rs_path);
        println!("cargo:rerun-if-changed={}", path.to_string_lossy());

        let generated_code = match GeneratedCpp::new(&path) {
            Ok(v) => v,
            Err(diagnostic) => {
                diagnostic.report();
                std::process::exit(1);
            }
        };
        generated_file_paths.push(generated_code.write_to_directories(cpp_directory, &header_dir));
    }

    generated_file_paths
}

/// The export directory, if one was specified through the environment.
/// Note that this is not namspaced by crate.
fn export_dir() -> Option<PathBuf> {
    env::var("CXXQT_EXPORT_DIR").ok().map(PathBuf::from)
}

fn out_dir() -> PathBuf {
    env::var("OUT_DIR").unwrap().into()
}

fn plugin_name_from_uri(plugin_uri: &str) -> String {
    plugin_uri.replace('.', "_")
}

/// The include directory needs to be namespaced by crate name when exporting for a C++ build system,
/// but for using cargo build without a C++ build system, OUT_DIR is already namespaced by crate name.
fn header_root() -> PathBuf {
    export_dir()
        .unwrap_or_else(|| PathBuf::from(env::var("OUT_DIR").unwrap()))
        .join("include")
        .join(crate_name())
}

fn crate_name() -> String {
    env::var("CARGO_PKG_NAME").unwrap()
}

fn static_lib_name() -> String {
    format!("{}-cxxqt-generated", crate_name())
}

fn panic_duplicate_file_and_qml_module(
    path: impl AsRef<Path>,
    uri: &str,
    version_major: usize,
    version_minor: usize,
) {
    panic!("CXX-Qt bridge Rust file {} specified in QML module {uri} (version {version_major}.{version_minor}), but also specified via CxxQtBuilder::file. Bridge files must be specified via CxxQtBuilder::file or CxxQtBuilder::qml_module, but not both.", path.as_ref().display());
}

/// Run cxx-qt's C++ code generator on Rust modules marked with the `cxx_qt::bridge` macro, compile
/// the code, and link to Qt. This is the complement of the `cxx_qt::bridge` macro, which the Rust
/// compiler uses to generate the corresponding Rust code. No dependencies besides Qt, a C++17 compiler,
/// and Rust toolchain are required.
///
/// For example, if your `cxx_qt::bridge` module is in a file called `src/lib.rs` within your crate,
/// put this in your [build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html):
///
/// ```no_run
/// use cxx_qt_build::CxxQtBuilder;
///
/// CxxQtBuilder::new()
///     .file("src/lib.rs")
///     .build();
/// ```
///
/// If you have multiple major versions of Qt installed (for example, 5 and 6), you can tell
/// [CxxQtBuilder] which one to use by setting the `QT_VERSION_MAJOR` environment variable to when
/// running `cargo build`. Otherwise [CxxQtBuilder] prefers the newer version by default.
///
/// To use [CxxQtBuilder] for a library to link with a C++ application, specify a directory to output
/// cxx-qt's autogenerated headers by having the C++ build system set the `CXXQT_EXPORT_DIR`
/// environment variable before calling `cargo build`. Then, add the same directory path to the C++
/// include paths. Also, set the `QMAKE` environment variable to the path of the `qmake` executable
/// for the Qt installation found by the C++ build system. This ensures that the C++ build system and
/// [CxxQtBuilder] link to the same installation of Qt.
///
/// Under the hood, [CxxQtBuilder] uses [cc::Build], which allows compiling aditional C++ files as well.
/// Refer to [CxxQtBuilder::cc_builder] for details.
///
/// In addition to autogenerating and building QObject C++ subclasses, manually written QObject
/// subclasses can be parsed by moc and built using [CxxQtBuilder::qobject_header].
#[derive(Default)]
pub struct CxxQtBuilder {
    rust_sources: Vec<PathBuf>,
    qobject_headers: Vec<QObjectHeaderOpts>,
    qrc_files: Vec<PathBuf>,
    qt_modules: HashSet<String>,
    qml_modules: Vec<OwningQmlModule>,
    cc_builder: cc::Build,
    extra_defines: HashSet<String>,
    initializers: Vec<String>,
}

impl CxxQtBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        let mut qt_modules = HashSet::new();
        qt_modules.insert("Core".to_owned());
        Self {
            rust_sources: vec![],
            qobject_headers: vec![],
            qrc_files: vec![],
            qt_modules,
            qml_modules: vec![],
            cc_builder: cc::Build::new(),
            extra_defines: HashSet::new(),
            initializers: Vec::new(),
        }
    }

    /// Specify rust file paths to parse through the cxx-qt marco
    /// Relative paths are treated as relative to the path of your crate's Cargo.toml file
    pub fn file(mut self, rust_source: impl AsRef<Path>) -> Self {
        let rust_source = rust_source.as_ref().to_path_buf();
        for qml_module in &self.qml_modules {
            if qml_module.rust_files.contains(&rust_source) {
                panic_duplicate_file_and_qml_module(
                    &rust_source,
                    &qml_module.uri,
                    qml_module.version_major,
                    qml_module.version_minor,
                );
            }
        }
        println!("cargo:rerun-if-changed={}", rust_source.display());
        self.rust_sources.push(rust_source);
        self
    }

    /// Include files listed in a .qrc file into the binary
    /// with [Qt's resource system](https://doc.qt.io/qt-6/resources.html).
    /// ```no_run
    /// # use cxx_qt_build::CxxQtBuilder;
    /// CxxQtBuilder::new()
    ///     .file("src/cxxqt_module.rs")
    ///     .qrc("src/my_resources.qrc")
    ///     .build();
    /// ```
    ///
    /// ⚠️  In CMake projects, the .qrc file is typically added to the `SOURCES` of the target.
    /// Prefer this to adding the qrc file through cxx-qt-build.
    /// When using CMake, the qrc file will **not** be built by cxx-qt-build!
    pub fn qrc(mut self, qrc_file: impl AsRef<Path>) -> Self {
        let qrc_file = qrc_file.as_ref();
        self.qrc_files.push(qrc_file.to_path_buf());
        println!("cargo:rerun-if-changed={}", qrc_file.display());
        self
    }

    /// Link additional [Qt modules](https://doc.qt.io/qt-6/qtmodules.html).
    /// Specify their names without the `Qt` prefix, for example `"Widgets"`.
    /// The `Core` module and any modules from [CxxQtBuildersOpts] are linked automatically; there is no need to specify them.
    pub fn qt_module(mut self, module: &str) -> Self {
        self.qt_modules.insert(module.to_owned());
        self
    }

    /// Register a QML module at build time. The `rust_files` of the [QmlModule] struct
    /// should contain `#[cxx_qt::bridge]` modules with QObject types annotated with `#[qml_element]`.
    ///
    /// The QmlModule struct's `qml_files` are registered with the [Qt Resource System](https://doc.qt.io/qt-6/resources.html) in
    /// the [default QML import path](https://doc.qt.io/qt-6/qtqml-syntax-imports.html#qml-import-path) `qrc:/qt/qml/uri/of/module/`.
    /// Additional resources such as images can be added to the Qt resources for the QML module by specifying
    /// the `qrc_files` field.
    ///
    /// When using Qt 6, this will [run qmlcachegen](https://doc.qt.io/qt-6/qtqml-qtquick-compiler-tech.html)
    /// to compile the specified `.qml` files ahead-of-time.
    ///
    /// ```no_run
    /// use cxx_qt_build::{CxxQtBuilder, QmlModule};
    ///
    /// CxxQtBuilder::new()
    ///     .qml_module(QmlModule {
    ///         uri: "com.kdab.cxx_qt.demo",
    ///         rust_files: &["src/cxxqt_object.rs"],
    ///         qml_files: &["qml/main.qml"],
    ///         ..Default::default()
    ///     })
    ///     .build();
    /// ```
    pub fn qml_module<A: AsRef<Path>, B: AsRef<Path>>(
        mut self,
        qml_module: QmlModule<A, B>,
    ) -> CxxQtBuilder {
        let qml_module = OwningQmlModule::from(qml_module);
        for path in &qml_module.rust_files {
            if self.rust_sources.contains(path) {
                panic_duplicate_file_and_qml_module(
                    path,
                    &qml_module.uri,
                    qml_module.version_major,
                    qml_module.version_minor,
                );
            }
        }
        self.qml_modules.push(qml_module);
        self
    }

    /// Specify a C++ header containing a Q_OBJECT macro to run [moc](https://doc.qt.io/qt-6/moc.html) on.
    /// This allows building QObject C++ subclasses besides the ones autogenerated by cxx-qt.
    pub fn qobject_header(mut self, opts: impl Into<QObjectHeaderOpts>) -> Self {
        let opts = opts.into();
        println!("cargo:rerun-if-changed={}", opts.path.display());
        self.qobject_headers.push(opts);
        self
    }

    /// Use a closure to run additional customization on [CxxQtBuilder]'s internal [cc::Build]
    /// before calling [CxxQtBuilder::build]. This allows to add extra include paths, compiler flags,
    /// or anything else available via [cc::Build]'s API. For example, to add an include path for
    /// manually written C++ headers located in a directory called `include` within your crate:
    ///
    /// ```no_run
    /// # use cxx_qt_build::CxxQtBuilder;
    ///
    /// CxxQtBuilder::new()
    ///     .file("src/lib.rs")
    ///     .cc_builder(|cc| {
    ///         cc.include("include");
    ///     })
    ///     .build();
    /// ```
    pub fn cc_builder(mut self, mut callback: impl FnMut(&mut cc::Build)) -> Self {
        callback(&mut self.cc_builder);
        self
    }

    /// Build with the given extra options
    pub fn with_opts(mut self, opts: CxxQtBuildersOpts) -> Self {
        let header_root = header_root();
        for (file_contents, dir_name, file_name) in opts.headers {
            let directory = if dir_name.is_empty() {
                header_root.clone()
            } else {
                header_root.join(dir_name)
            };
            std::fs::create_dir_all(directory.clone())
                .expect("Could not create {directory} header directory");

            let h_path = directory.join(file_name);
            std::fs::write(&h_path, file_contents).unwrap_or_else(|_| {
                panic!(
                    "Could not write header: {h_path}",
                    h_path = h_path.to_string_lossy()
                )
            });
        }

        // Add any of the defines
        self.extra_defines.extend(opts.defines);

        self.initializers.extend(opts.initializers);

        // Add any of the Qt modules
        self.qt_modules.extend(opts.qt_modules);

        self
    }

    fn define_cfg_variable(key: String, value: Option<&str>) {
        if let Some(value) = value {
            println!("cargo:rustc-cfg={key}=\"{value}\"");
        } else {
            println!("cargo:rustc-cfg={key}");
        }
        let variable_cargo = format!("CARGO_CFG_{}", key);
        env::set_var(variable_cargo, value.unwrap_or("true"));
    }

    fn define_cfg_check_variable(key: String, values: Option<Vec<&str>>) {
        if let Some(values) = values {
            let values = values
                .iter()
                // Escape and add quotes
                .map(|value| format!("\"{}\"", value.escape_default()))
                .collect::<Vec<_>>()
                .join(", ");
            println!("cargo:rustc-check-cfg=cfg({key}, values({values}))");
        } else {
            println!("cargo:rustc-check-cfg=cfg({key})");
        }
    }

    fn define_qt_version_cfg_variables(version: &SemVer) {
        // Allow for Qt 5 or Qt 6 as valid values
        CxxQtBuilder::define_cfg_check_variable(
            "cxxqt_qt_version_major".to_owned(),
            Some(vec!["5", "6"]),
        );
        // Find the Qt version and tell the Rust compiler
        // this allows us to have conditional Rust code
        CxxQtBuilder::define_cfg_variable(
            "cxxqt_qt_version_major".to_string(),
            Some(version.major.to_string().as_str()),
        );

        // Seed all values from Qt 5.0 through to Qt 7.99
        for major in 5..=7 {
            CxxQtBuilder::define_cfg_check_variable(
                format!("cxxqt_qt_version_at_least_{major}"),
                None,
            );

            for minor in 0..=99 {
                CxxQtBuilder::define_cfg_check_variable(
                    format!("cxxqt_qt_version_at_least_{major}_{minor}"),
                    None,
                );
            }
        }

        for minor in 0..=version.minor {
            let qt_version_at_least =
                format!("cxxqt_qt_version_at_least_{}_{}", version.major, minor);
            CxxQtBuilder::define_cfg_variable(qt_version_at_least.to_string(), None);
        }

        // We don't support Qt < 5
        for major in 5..=version.major {
            let at_least_qt_major_version = format!("cxxqt_qt_version_at_least_{}", major);
            CxxQtBuilder::define_cfg_variable(at_least_qt_major_version, None);
        }
    }

    fn write_common_headers() {
        let header_root = header_root();
        // Write cxx-qt and cxx headers
        cxx_qt::write_headers(header_root.join("cxx-qt"));
        std::fs::create_dir_all(header_root.join("rust"))
            .expect("Could not create cxx header directory");
        let h_path = header_root.join("rust").join("cxx.h");
        // Wrap the File in a block scope so the file is closed before the compiler is run.
        // Otherwise MSVC fails to open cxx.h because the process for this build script already has it open.
        {
            std::fs::write(h_path, cxx_gen::HEADER).expect("Failed to write cxx.h");
        }
    }

    fn setup_cc_builder<'a>(
        builder: &mut cc::Build,
        include_paths: &[impl AsRef<Path>],
        defines: impl Iterator<Item = &'a str>,
    ) {
        // Note, ensure our settings stay in sync across cxx-qt, cxx-qt-build, and cxx-qt-lib
        builder.cpp(true);
        builder.std("c++17");
        // MSVC
        builder.flag_if_supported("/Zc:__cplusplus");
        builder.flag_if_supported("/permissive-");
        builder.flag_if_supported("/bigobj");
        // MinGW requires big-obj otherwise debug builds fail
        builder.flag_if_supported("-Wa,-mbig-obj");

        // Enable any extra defines
        for define in defines {
            builder.define(define, None);
        }

        for include_path in include_paths {
            builder.include(include_path);
        }
    }

    fn moc_qobject_headers(&mut self, qtbuild: &mut qt_build_utils::QtBuild) {
        for QObjectHeaderOpts {
            path,
            moc_arguments,
        } in &self.qobject_headers
        {
            let moc_products = qtbuild.moc(path, moc_arguments.clone());
            self.cc_builder.file(moc_products.cpp);
        }
    }

    fn generate_cpp_files_from_cxxqt_bridges(&mut self, header_dir: impl AsRef<Path>) {
        for files in generate_cxxqt_cpp_files(&self.rust_sources, &header_dir) {
            self.cc_builder.file(files.plain_cpp);
            if let (Some(qobject), Some(qobject_header)) = (files.qobject, files.qobject_header) {
                self.cc_builder.file(&qobject);
                self.qobject_headers.push(qobject_header.into());
            }
        }
    }

    fn build_object_file(
        builder: &cc::Build,
        file_path: impl AsRef<Path>,
        export_path: Option<(&str, &str)>,
    ) {
        let mut obj_builder = builder.clone();
        obj_builder.file(file_path);
        let obj_files = obj_builder.compile_intermediates();

        // We only expect a single file, so destructure the vec.
        // If there's 0 or > 1 file, we panic in the `else` branch, because then the builder is
        // probably not correctly configured.
        if let [obj_file] = &obj_files[..] {
            if let Some(export_dir) = export_dir() {
                if let Some((out_directory, out_file_name)) = export_path {
                    let obj_dir = export_dir.join(out_directory);
                    std::fs::create_dir_all(&obj_dir).unwrap_or_else(|_| {
                        panic!(
                            "Could not create directory for object file: {}",
                            obj_dir.to_string_lossy()
                        )
                    });
                    let obj_path = obj_dir.join(out_file_name);
                    std::fs::copy(obj_file, &obj_path).unwrap_or_else(|_| {
                        panic!(
                            "Failed to move object file to {obj_path}!",
                            obj_path = obj_path.to_string_lossy()
                        )
                    });
                }
            } else {
                println!("cargo::rustc-link-arg={}", obj_file.to_string_lossy());
                // The linker argument order matters!
                // We need to link the object file first, then link the static library.
                // Otherwise, the linker will be unable to find the symbols in the static library file.
                // See also: https://stackoverflow.com/questions/45135/why-does-the-order-in-which-libraries-are-linked-sometimes-cause-errors-in-gcc
                println!("cargo::rustc-link-arg=-l{}", static_lib_name());
            }
        } else {
            panic!(
                "CXX-Qt internal error: Expected only one object file out of cc::Build! Got {}",
                obj_files.len()
            );
        }
    }

    fn build_qml_modules(
        &mut self,
        init_builder: &cc::Build,
        qtbuild: &mut qt_build_utils::QtBuild,
        generated_header_dir: impl AsRef<Path>,
    ) {
        for qml_module in &self.qml_modules {
            let mut qml_metatypes_json = Vec::new();

            for files in generate_cxxqt_cpp_files(&qml_module.rust_files, &generated_header_dir) {
                self.cc_builder.file(files.plain_cpp);
                if let (Some(qobject), Some(qobject_header)) = (files.qobject, files.qobject_header)
                {
                    self.cc_builder.file(&qobject);
                    let moc_products = qtbuild.moc(
                        qobject_header,
                        MocArguments::default().uri(qml_module.uri.clone()),
                    );
                    self.cc_builder.file(moc_products.cpp);
                    qml_metatypes_json.push(moc_products.metatypes_json);
                }
            }

            let qml_module_registration_files = qtbuild.register_qml_module(
                &qml_metatypes_json,
                &qml_module.uri,
                qml_module.version_major,
                qml_module.version_minor,
                // TODO: This will be passed to the `optional plugin ...` part of the qmldir
                // We don't load any shared libraries, so the name shouldn't matter
                // But make sure it still works
                &plugin_name_from_uri(&qml_module.uri),
                &qml_module.qml_files,
                &qml_module.qrc_files,
            );
            self.cc_builder
                .file(qml_module_registration_files.qmltyperegistrar)
                .file(qml_module_registration_files.plugin)
                // In comparison to the other RCC files, we don't need to link this with whole-archive or
                // anything like that.
                // The plugin_init file already takes care of loading the resources associated with this
                // RCC file.
                .file(qml_module_registration_files.rcc);

            for qmlcachegen_file in qml_module_registration_files.qmlcachegen {
                self.cc_builder.file(qmlcachegen_file);
            }
            // This is required, as described here: plugin_builder
            self.cc_builder.define("QT_STATICPLUGIN", None);

            // If any of the files inside the qml module change, then trigger a rerun
            for path in qml_module.qml_files.iter().chain(
                qml_module
                    .rust_files
                    .iter()
                    .chain(qml_module.qrc_files.iter()),
            ) {
                println!("cargo:rerun-if-changed={}", path.display());
            }

            // Now all necessary symbols should be included in the cc_builder.
            // However, the plugin needs to be initialized at runtime.
            // This is done through the plugin_init file.
            // It needs to be linked as an object file, to ensure that the linker doesn't throw away
            // the static initializers in this file.
            // For CMake builds, we export this file to then later include it as an object library in
            // CMake.
            // In cargo builds, add the object file as a direct argument to the linker.
            Self::build_object_file(
                init_builder,
                &qml_module_registration_files.plugin_init,
                Some((
                    &format!("plugins/{}", plugin_name_from_uri(&qml_module.uri)),
                    "plugin_init.o",
                )),
            );
        }
    }

    fn setup_qt5_compatibility(&mut self, qtbuild: &qt_build_utils::QtBuild) {
        // If we are using Qt 5 then write the std_types source
        // This registers std numbers as a type for use in QML
        //
        // Note that we need this to be compiled into an object file
        // as they are stored in statics in the source.
        //
        // TODO: Can we move this into cxx-qt so that it's only built
        // once rather than for every cxx-qt-build? When we do this
        // ensure that in a multi project that numbers work everywhere.
        //
        // Also then it should be possible to use CARGO_MANIFEST_DIR/src/std_types_qt5.cpp
        // as path for cc::Build rather than copying the .cpp file
        //
        // https://github.com/rust-lang/rust/issues/108081
        // https://github.com/KDAB/cxx-qt/pull/598
        if qtbuild.version().major == 5 {
            self.initializers
                .push(include_str!("std_types_qt5.cpp").to_owned());
        }
    }

    fn build_initializers(&mut self, init_builder: &cc::Build) {
        let initializers_path = out_dir().join("cxx-qt-build").join("initializers");
        std::fs::create_dir_all(&initializers_path).expect("Failed to create initializers path!");

        let initializers_path = initializers_path.join(format!("{}.cpp", crate_name()));
        std::fs::write(&initializers_path, self.initializers.join("\n"))
            .expect("Could not write initializers file");
        Self::build_object_file(
            init_builder,
            initializers_path,
            Some(("initializers", &format!("{}.o", crate_name()))),
        );
    }

    fn build_qrc_files(&mut self, init_builder: &cc::Build, qtbuild: &mut qt_build_utils::QtBuild) {
        for qrc_file in &self.qrc_files {
            // We need to link this using an obect file or +whole-achive, the static initializer of
            // the qrc file isn't lost.
            Self::build_object_file(init_builder, qtbuild.qrc(&qrc_file), None);

            // Also ensure that each of the files in the qrc can cause a change
            for qrc_inner_file in qtbuild.qrc_list(&qrc_file) {
                println!("cargo:rerun-if-changed={}", qrc_inner_file.display());
            }
        }
    }

    /// Generate and compile cxx-qt C++ code, as well as compile any additional files from
    /// [CxxQtBuilder::qobject_header] and [CxxQtBuilder::cc_builder].
    pub fn build(mut self) {
        // Ensure that the linker is setup correctly for Cargo builds
        qt_build_utils::setup_linker();

        let header_root = header_root();
        let generated_header_dir = header_root.join("cxx-qt-gen/");

        let mut qtbuild = qt_build_utils::QtBuild::new(self.qt_modules.drain().collect())
            .expect("Could not find Qt installation");
        qtbuild.cargo_link_libraries(&mut self.cc_builder);
        Self::define_qt_version_cfg_variables(qtbuild.version());

        Self::write_common_headers();

        // Setup compilers
        // Static QML plugin and Qt resource initializers need to be linked as their own separate
        // object files because they use static variables which need to be initialized before main
        // (regardless of whether main is in Rust or C++). Normally linkers only copy symbols referenced
        // from within main when static linking, which would result in discarding those static variables.
        // Use a separate cc::Build for the little amount of code that needs to be built & linked this way.
        let mut init_builder = cc::Build::new();
        let mut include_paths = qtbuild.include_paths();
        include_paths.extend([header_root.clone(), generated_header_dir.clone()]);

        Self::setup_cc_builder(
            &mut self.cc_builder,
            &include_paths,
            self.extra_defines.iter().map(String::as_str),
        );
        Self::setup_cc_builder(
            &mut init_builder,
            &include_paths,
            self.extra_defines.iter().map(String::as_str),
        );
        // Note: From now on the init_builder is correctly configured.
        // When building object files with this builder, we always need to copy it first.
        // So remove `mut` to ensure that we can't accidentally change the configuration or add
        // files.
        let init_builder = init_builder;

        // Generate files
        self.generate_cpp_files_from_cxxqt_bridges(&generated_header_dir);

        self.moc_qobject_headers(&mut qtbuild);

        // Bridges for QML modules are handled separately because
        // the metatypes_json generated by moc needs to be passed to qmltyperegistrar
        self.build_qml_modules(&init_builder, &mut qtbuild, &generated_header_dir);

        self.build_qrc_files(&init_builder, &mut qtbuild);

        self.setup_qt5_compatibility(&qtbuild);

        self.build_initializers(&init_builder);

        // Only compile if we have added files to the builder
        // otherwise we end up with no static library but ask cargo to link to it which causes an error
        if self.cc_builder.get_files().count() > 0 {
            self.cc_builder.compile(&static_lib_name());
        }
    }
}
