//! A TypeScript project loaded through tsrs's crates, with what vile's type stage needs from it.

use std::sync::Arc;

use tsrs_ast::{Node, SourceFile, is_function_like};
use tsrs_compiler::{CompilerHost, Program, ProgramOptions, new_compiler_host, new_program};
use tsrs_core::context::Context;
use tsrs_core::text::TextPos;
use tsrs_core::tspath;
use tsrs_core::{P, ensure_script_kind_from_file_name};
use tsrs_lsproto::PositionEncodingKind;
use tsrs_project::{FsRef, OverlayMap, ToPath, new_overlay, new_overlay_fs};
use tsrs_tsoptions::{ParseConfigHost, get_parsed_command_line_of_config_file};
use tsrs_vfs::{FS, bundled, cachedvfs, osvfs};
use vile_core::patch::{Patch, PatchError};
use vile_core::span::Span;

/// Why a project can't be loaded.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("the config file has errors: {0:?}")]
    Config(Vec<String>),
}

/// Why a patch can't be checked.
#[derive(Debug, thiserror::Error)]
pub enum CheckError {
    #[error("the file isn't part of the project")]
    NotInProject,
    #[error(transparent)]
    Patch(#[from] PatchError),
}

/// A diagnostic from the type check, in the file that was checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: i32,
    /// `None` for a diagnostic about the whole program rather than a place in the file.
    pub span: Option<Span>,
    pub message: String,
}

/// The result of checking one patch.
#[derive(Debug)]
pub struct Checked {
    pub diagnostics: Vec<Diagnostic>,
    /// Whether tsrs kept the other files' trees, or had to build the program again.
    pub is_reused: bool,
}

/// A function with a body, as body replacement needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    /// The body, braces included.
    pub body: Span,
    /// The return type, as TypeScript prints it.
    pub return_type: String,
}

/// The file system and current directory that tsrs reads the config through.
struct ConfigHost {
    fs: Arc<dyn FS>,
    current_directory: String,
}

impl ParseConfigHost for ConfigHost {
    fn fs(&self) -> &dyn FS {
        &*self.fs
    }

    fn get_current_directory(&self) -> &str {
        &self.current_directory
    }
}

/// The byte offset of a parsed node's position; synthetic nodes have none.
const fn offset(position: TextPos) -> Option<u32> {
    match position {
        TextPos::Source(position) => Some(position.get()),
        TextPos::Synthetic => None,
    }
}

fn contains(node: P<Node>, position: u32) -> bool {
    matches!((offset(node.pos()), offset(node.end())), (Some(start), Some(end)) if start <= position && position < end)
}

/// The innermost function with a body that contains the offset.
fn find_innermost_function(
    node: P<Node>,
    position: u32,
    outer: Option<P<Node>>,
) -> Option<P<Node>> {
    let mut found = outer;
    node.for_each_child(&mut |child| {
        if !contains(child, position) {
            return false;
        }
        let has_body =
            is_function_like(child) && child.body().is_some_and(|body| contains(body, position));
        found =
            find_innermost_function(child, position, if has_body { Some(child) } else { found });
        true
    });
    found
}

/// Every function with a body, outermost first.
fn collect_functions(node: P<Node>, functions: &mut Vec<P<Node>>) {
    node.for_each_child(&mut |child| {
        if is_function_like(child) && child.body().is_some() {
            functions.push(child);
        }
        collect_functions(child, functions);
        false
    });
}

fn convert(diagnostic: P<tsrs_ast::Diagnostic>) -> Diagnostic {
    let span = diagnostic
        .pos()
        .zip(diagnostic.end())
        .and_then(|(start, end)| Span::new(start, end).ok());
    Diagnostic {
        code: diagnostic.code(),
        span,
        message: diagnostic.localize(),
    }
}

/// A project as tsrs sees it after loading its config.
pub struct Project {
    program: &'static Program,
    fs: Arc<dyn FS>,
    current_directory: String,
}

impl std::fmt::Debug for Project {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Project")
            .field("current_directory", &self.current_directory)
            .finish_non_exhaustive()
    }
}

impl Project {
    /// Loads the project of a config file, given as an absolute path, and parses and binds its files.
    ///
    /// # Errors
    ///
    /// [`LoadError::Config`] when tsrs reports errors in the config.
    pub fn load(config_file: &str) -> Result<Self, LoadError> {
        let current_directory = tspath::get_directory_path(config_file);
        let fs: Arc<dyn FS> = Arc::new(cachedvfs::from(bundled::wrap_fs(osvfs::fs())));
        let config_host = ConfigHost {
            fs: Arc::clone(&fs),
            current_directory: current_directory.clone(),
        };
        let (config, diagnostics) =
            get_parsed_command_line_of_config_file(config_file, None, None, &config_host, None);
        let messages: Vec<String> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.localize())
            .collect();
        let Some(config) = config.filter(|_| messages.is_empty()) else {
            return Err(LoadError::Config(messages));
        };
        let host = Self::create_host(&current_directory, Arc::clone(&fs));
        let program = new_program(ProgramOptions::new(P::new(config), host));
        program.bind_source_files();
        Ok(Self {
            program,
            fs,
            current_directory,
        })
    }

    fn create_host(current_directory: &str, fs: Arc<dyn FS>) -> Arc<dyn CompilerHost> {
        new_compiler_host(
            current_directory,
            fs,
            &bundled::lib_path(),
            None,
            None,
            None,
        )
    }

    fn source_file(&self, file_name: &str) -> Option<P<SourceFile>> {
        self.program.get_source_file(file_name)
    }

    /// The project's own files, without libraries and declaration files.
    #[must_use]
    pub fn file_names(&self) -> Vec<String> {
        self.program
            .get_source_files()
            .iter()
            .filter(|file| {
                !file.is_declaration_file()
                    && !self.program.is_source_file_from_external_library(**file)
            })
            .map(|file| file.file_name().to_string())
            .collect()
    }

    /// The text of a file of the project.
    #[must_use]
    pub fn text(&self, file_name: &str) -> Option<&'static str> {
        self.source_file(file_name).map(|file| file.text())
    }

    fn describe(&self, file: P<SourceFile>, function: P<Node>) -> Option<Function> {
        let body = function.body()?;
        let start = tsrs_scanner::skip_trivia(file.text(), offset(body.pos())?);
        let body = Span::new(start, offset(body.end())?).ok()?;
        // The handle locks the checker until it's dropped.
        let return_type = {
            let mut checker = self
                .program
                .get_type_checker_for_file(&Context::background(), file);
            let signature = checker.get_signature_from_declaration_exported(function);
            let return_type = checker.get_return_type_of_signature_exported(signature);
            checker.type_to_string_exported(return_type)
        };
        Some(Function { body, return_type })
    }

    /// The innermost function whose body contains the byte offset.
    #[must_use]
    pub fn find_function(&self, file_name: &str, position: u32) -> Option<Function> {
        let file = self.source_file(file_name)?;
        let function = find_innermost_function(file.as_node(), position, None)?;
        self.describe(file, function)
    }

    /// Every function with a body in the file.
    #[must_use]
    pub fn functions(&self, file_name: &str) -> Vec<Function> {
        let Some(file) = self.source_file(file_name) else {
            return Vec::new();
        };
        let mut functions = Vec::new();
        collect_functions(file.as_node(), &mut functions);
        functions
            .into_iter()
            .filter_map(|function| self.describe(file, function))
            .collect()
    }

    /// Checks the file with the patch applied, in memory, and returns its diagnostics.
    ///
    /// Only the patched file is checked: an error the patch causes in another file isn't found.
    /// Diagnostics about unused declarations are left out.
    ///
    /// # Errors
    ///
    /// [`CheckError::NotInProject`] for a file outside the project, and [`CheckError::Patch`] when
    /// the patch doesn't fit the file.
    pub fn check(&self, file_name: &str, patch: &Patch) -> Result<Checked, CheckError> {
        let file = self
            .source_file(file_name)
            .ok_or(CheckError::NotInProject)?;
        let text = patch.apply(file.text())?;
        let kind = ensure_script_kind_from_file_name(file_name);
        let overlay = new_overlay(file_name.to_string(), text, 1, kind);
        let overlays: OverlayMap =
            Arc::new(std::iter::once((file.path().clone(), Arc::new(overlay))).collect());
        let current_directory = self.current_directory.clone();
        let case_sensitive = self.fs.use_case_sensitive_file_names();
        let to_path: ToPath =
            Arc::new(move |name: &str| tspath::to_path(name, &current_directory, case_sensitive));
        let fs = new_overlay_fs(
            FsRef::Host(Arc::clone(&self.fs)),
            overlays,
            PositionEncodingKind::UTF8,
            to_path,
        );
        let host = Self::create_host(&self.current_directory, Arc::new(fs));
        let (program, patched, is_reused) =
            self.program.update_program(file.path(), host, None, None);
        let patched = patched.ok_or(CheckError::NotInProject)?;
        let context = Context::background();
        let mut diagnostics = program.get_syntactic_diagnostics(&context, Some(patched));
        if diagnostics.is_empty() {
            diagnostics = program.get_semantic_diagnostics(&context, Some(patched));
        }
        // An unused declaration (noUnusedLocals, noUnusedParameters) doesn't stop the mutant running.
        let diagnostics = diagnostics
            .into_iter()
            .filter(|diagnostic| !diagnostic.reports_unnecessary())
            .map(convert)
            .collect();
        Ok(Checked {
            diagnostics,
            is_reused,
        })
    }
}
