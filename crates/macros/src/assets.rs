use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};
use syn::{ExprArray, ItemMod, LitStr, Token, Type, parse::Parser, parse_macro_input};

use globset::{Glob, GlobSet, GlobSetBuilder};
use heck::{ToSnakeCase, ToUpperCamelCase};
use ignore::WalkBuilder;
use unicode_ident::{is_xid_continue, is_xid_start};

use proc_macro_error2::abort_call_site;

const ROOT_ATTR: &str = "root";
const SKIP_ATTR: &str = "skip";
const TYPES_ATTR: &str = "types";
const WITH_EXT_ATTR: &str = "with_ext";
const EMBED_ATTR: &str = "embed";
const LIST_ID_ATTR: &str = "id";
const CUSTOM_ATTR: &str = "custom";
const ATLAS_ATTR: &str = "atlas";

struct CustomField {
    name: String,
    ty: Type,
    parser: syn::Path,
}

#[derive(serde::Deserialize)]
struct MacroAtlasRoot {
    frames: Vec<MacroAtlasFrame>,
    meta: MacroAtlasMeta,
}

#[derive(serde::Deserialize)]
struct MacroAtlasFrame {
    filename: String,
}

#[derive(serde::Deserialize)]
struct MacroAtlasMeta {
    image: Option<String>,
}

#[derive(Debug)]
struct AtlasInfo {
    json_rel: String,
    parent_dir: PathBuf,
    namespace: String,
    image_rel: String,
    image_already_exists: bool,
    frame_names: Vec<String>,
}

pub fn assets(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut root_rel: Option<String> = None;
    let mut user_skip = vec![];
    let mut with_ext = false;
    let mut embed = false;
    let mut list_id: Option<String> = None;
    let mut parser_map: HashMap<String, Type> = HashMap::new();
    let mut custom_fields: Vec<CustomField> = vec![];
    let mut atlas_paths: Vec<String> = vec![];

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident(ROOT_ATTR) {
            let s: LitStr = meta.value()?.parse()?;
            root_rel = Some(s.value());
            Ok(())
        } else if meta.path.is_ident(SKIP_ATTR) {
            let arr: ExprArray = meta.value()?.parse()?;
            for e in arr.elems.iter() {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = e
                {
                    user_skip.push(s.value());
                } else {
                    return Err(meta.error(
                        "skip expects string literals, e.g. skip = [\"*.psd\", \"whatever\"]",
                    ));
                }
            }
            Ok(())
        } else if meta.path.is_ident(TYPES_ATTR) {
            meta.parse_nested_meta(|entry| {
                let ext_ident = entry
                    .path
                    .get_ident()
                    .ok_or_else(|| entry.error("expected extension like 'ogg'"))?
                    .to_string()
                    .to_ascii_lowercase();

                entry.input.parse::<Token![:]>()?;

                let ty: Type = entry.input.parse()?;
                parser_map.insert(ext_ident, ty);
                Ok(())
            })
        } else if meta.path.is_ident(WITH_EXT_ATTR) {
            let lit: syn::LitBool = meta.value()?.parse()?;
            with_ext = lit.value();
            Ok(())
        } else if meta.path.is_ident(EMBED_ATTR) {
            let lit: syn::LitBool = meta.value()?.parse()?;
            embed = lit.value();
            Ok(())
        } else if meta.path.is_ident(LIST_ID_ATTR) {
            let s: LitStr = meta.value()?.parse()?;
            list_id = Some(s.value());
            Ok(())
        } else if meta.path.is_ident(CUSTOM_ATTR) {
            meta.parse_nested_meta(|entry| {
                let field_name = entry
                    .path
                    .get_ident()
                    .ok_or_else(|| entry.error("expected field name"))?
                    .to_string();

                entry.input.parse::<Token![:]>()?;
                let field_ty: Type = entry.input.parse()?;
                entry.input.parse::<Token![=]>()?;
                let parser_path: syn::Path = entry.input.parse()?;

                custom_fields.push(CustomField {
                    name: field_name,
                    ty: field_ty,
                    parser: parser_path,
                });
                Ok(())
            })
        } else if meta.path.is_ident(ATLAS_ATTR) {
            let arr: ExprArray = meta.value()?.parse()?;
            for e in arr.elems.iter() {
                match e {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) => {
                        atlas_paths.push(s.value());
                    }
                    _ => {
                        return Err(meta.error(
                            "atlas expects string literals, e.g. atlas = [\"img/sprites.json\"]",
                        ));
                    }
                }
            }
            Ok(())
        } else {
            Err(meta
                .error("unknown attribute; use: root, skip, types(...), with_ext, embed, id, custom(...), or atlas"))
        }
    });

    if let Err(e) = parser.parse(attr) {
        return e.to_compile_error().into();
    }

    let mut module = parse_macro_input!(item as ItemMod);

    let root_rel = match root_rel {
        Some(s) => s,
        None => abort_call_site!("missing `root = \"...\"` in #[assets(...)]"),
    };

    let items = match &mut module.content {
        Some((_brace, items)) => items,
        None => abort_call_site!("requires inline module: `pub mod my_assets {}`"),
    };

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let root_abs = PathBuf::from(&manifest_dir).join(&root_rel);
    if !root_abs.is_dir() {
        abort_call_site!("directory not found: {}", root_abs.display());
    }

    let skipset = match build_skipset(user_skip) {
        Ok(s) => s,
        Err(e) => abort_call_site!("{e}"),
    };

    let files = match collect_files_with_gitignore(&root_abs, &skipset) {
        Ok(v) => v,
        Err(e) => abort_call_site!("{e}"),
    };

    let mut tree = DirNode::from_files(&files);

    // process atlas files
    let atlas_infos = match process_atlases(&atlas_paths, &root_abs, &root_rel, &tree) {
        Ok(infos) => infos,
        Err(e) => abort_call_site!("{e}"),
    };

    // inject atlas into the tree
    for atlas_info in &atlas_infos {
        if let Err(e) = inject_atlas_into_tree(&mut tree, atlas_info) {
            abort_call_site!("{e}");
        }
    }

    let mut errs: Vec<String> = vec![];
    tree.collect_collisions(&mut errs, with_ext);
    if !errs.is_empty() {
        abort_call_site!("{}", errs.join("\n"));
    }

    let root_struct_ident = pascal_from_ident(&module.ident);

    let generated = generate_structs(
        &tree,
        &root_rel,
        &parser_map,
        &root_struct_ident,
        list_id,
        with_ext,
        embed,
        &custom_fields,
    );

    items.push(syn::Item::Verbatim(generated));
    TokenStream::from(quote! { #module })
}

fn process_atlases(
    atlas_paths: &[String],
    root_abs: &Path,
    root_rel: &str,
    tree: &DirNode,
) -> Result<Vec<AtlasInfo>, String> {
    use std::collections::HashSet;

    let mut atlas_infos = vec![];
    let mut existing_files: HashSet<String> = HashSet::new();

    // collect all existing files in tree
    fn collect_files(node: &DirNode, existing: &mut HashSet<String>, base: &str) {
        for f in &node.files {
            let rel = f.to_string_lossy().replace('\\', "/");
            let full = if base.is_empty() {
                rel
            } else {
                format!("{}/{}", base, rel)
            };
            existing.insert(full);
        }
        for child in node.dirs.values() {
            collect_files(child, existing, base);
        }
    }
    collect_files(tree, &mut existing_files, root_rel);

    for atlas_path in atlas_paths {
        // resolve relative to root
        let atlas_abs = root_abs.join(atlas_path);

        let json_bytes = std::fs::read(&atlas_abs)
            .map_err(|e| format!("Cannot read atlas '{}': {}", atlas_path, e))?;

        let atlas_root: MacroAtlasRoot = serde_json::from_slice(&json_bytes)
            .map_err(|e| format!("Cannot parse atlas '{}': {}", atlas_path, e))?;

        if atlas_root.frames.is_empty() {
            return Err(format!("Atlas '{}' has no frames", atlas_path));
        }

        // extract parent directory and namespace
        let atlas_pathbuf = PathBuf::from(atlas_path);
        let parent_dir = atlas_pathbuf
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::new());

        let namespace = atlas_pathbuf
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("Invalid atlas filename: {}", atlas_path))?
            .to_string();

        let image_filename = atlas_root
            .meta
            .image
            .unwrap_or_else(|| format!("{}.png", namespace));

        let image_rel = if parent_dir.as_os_str().is_empty() {
            image_filename.clone()
        } else {
            format!(
                "{}/{}",
                parent_dir.to_string_lossy().replace('\\', "/"),
                image_filename
            )
        };

        // check if image already exists in tree
        let full_image_path = if root_rel.is_empty() {
            image_rel.clone()
        } else {
            format!("{}/{}", root_rel, image_rel)
        };
        let image_already_exists = existing_files.contains(&full_image_path);

        let frame_names: Vec<String> = atlas_root
            .frames
            .iter()
            .map(|f| f.filename.clone())
            .collect();

        atlas_infos.push(AtlasInfo {
            json_rel: atlas_path.clone(),
            parent_dir,
            namespace,
            image_rel,
            image_already_exists,
            frame_names,
        });
    }

    Ok(atlas_infos)
}

fn inject_atlas_into_tree(tree: &mut DirNode, atlas_info: &AtlasInfo) -> Result<(), String> {
    let mut node = tree;
    for comp in atlas_info.parent_dir.iter() {
        let name = comp.to_string_lossy().to_string();
        node = node.dirs.get_mut(&name).ok_or_else(|| {
            format!(
                "Parent directory '{}' not found for atlas",
                atlas_info.parent_dir.display()
            )
        })?;
    }

    // remove JSON file from files list
    let json_filename = PathBuf::from(&atlas_info.json_rel)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("Invalid JSON filename: {}", atlas_info.json_rel))?
        .to_string();

    node.files
        .retain(|f| f.file_name().and_then(|s| s.to_str()) != Some(&json_filename));

    // remove image file from files list if it exists
    let image_filename = PathBuf::from(&atlas_info.image_rel)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("Invalid image filename: {}", atlas_info.image_rel))?
        .to_string();

    node.files
        .retain(|f| f.file_name().and_then(|s| s.to_str()) != Some(&image_filename));

    let namespace_path = if atlas_info.parent_dir.as_os_str().is_empty() {
        PathBuf::from(&atlas_info.namespace)
    } else {
        atlas_info.parent_dir.join(&atlas_info.namespace)
    };

    let namespace_node = DirNode {
        rel_dir: namespace_path,
        files: vec![],
        dirs: BTreeMap::new(),
        atlas_info: Some(AtlasInfo {
            json_rel: atlas_info.json_rel.clone(),
            parent_dir: atlas_info.parent_dir.clone(),
            namespace: atlas_info.namespace.clone(),
            image_rel: atlas_info.image_rel.clone(),
            image_already_exists: atlas_info.image_already_exists,
            frame_names: atlas_info.frame_names.clone(),
        }),
    };

    // insert into parent's dirs
    node.dirs
        .insert(atlas_info.namespace.clone(), namespace_node);

    Ok(())
}

fn build_skipset(user_skip: Vec<String>) -> Result<GlobSet, String> {
    let mut pats: Vec<String> = vec![
        "**/.DS_Store".into(),
        "**/Thumbs.db".into(),
        "**/__MACOSX/**".into(),
        "**/.git/**".into(),
        "**/.svn/**".into(),
        "**/.hg/**".into(),
        "**/*.tmp".into(),
        "**/*~".into(),
        "**/#*#".into(),
        "**/.#*".into(),
    ];
    for pat in user_skip {
        pats.push(if pat.contains('/') {
            pat
        } else {
            format!("**/{}", pat)
        });
    }

    let mut builder = GlobSetBuilder::new();
    for p in pats {
        let glob = Glob::new(&p).map_err(|e| format!("invalid glob: `{p}` ({e})"))?;
        builder.add(glob);
    }
    builder.build().map_err(|e| format!("skip set error: {e}"))
}

fn collect_files_with_gitignore(root_abs: &Path, skip: &GlobSet) -> Result<Vec<PathBuf>, String> {
    let mut builder = WalkBuilder::new(root_abs);
    builder
        .git_ignore(true)
        .ignore(true)
        .git_global(true)
        .hidden(false)
        .parents(true)
        .follow_links(false)
        .sort_by_file_name(|a, b| a.cmp(b));

    let mut files = vec![];
    for entry in builder.build() {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path == root_abs || entry.file_type().map_or(true, |ft| ft.is_dir()) {
            continue;
        }

        let rel = path.strip_prefix(root_abs).map_err(|e| e.to_string())?;
        if !skip.is_match(&rel.to_string_lossy().replace('\\', "/")) {
            files.push(rel.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

/// Converts to snake_case with extra splits at alpha/digit boundaries.
/// Example: 'powerUp12' -> 'power_up_12', 'http2Server' -> 'http_2_server'.
fn snake_with_digit_boundaries(s: &str) -> String {
    let base = s.to_snake_case();
    let mut out = String::with_capacity(base.len() + 4);
    let mut prev: Option<char> = None;

    for c in base.chars() {
        if let Some(p) = prev {
            let alpha_to_digit = p.is_ascii_alphabetic() && c.is_ascii_digit();
            let digit_to_alpha = p.is_ascii_digit() && c.is_ascii_alphabetic();
            if (alpha_to_digit || digit_to_alpha) && p != '_' && c != '_' {
                out.push('_');
            }
        }
        out.push(c);
        prev = Some(c);
    }

    out
}

/// Detect Rust keywords and automatically use raw identifier (r#keyword) if needed.
/// Uses a parse probe: if 'struct <ident>;' fails, it's a keyword.
fn keyword_guard_ident(s: &str) -> syn::Ident {
    let ident = format_ident!("{}", s, span = Span::call_site());
    let probe: proc_macro2::TokenStream = quote! { struct #ident; };
    if syn::parse2::<syn::ItemStruct>(probe).is_ok() {
        ident
    } else {
        format_ident!("r#{}", s, span = Span::call_site())
    }
}

fn make_snake_ident<S: AsRef<str>>(s: S) -> syn::Ident {
    // 1) Convert to snake_case with digit-boundary splits
    let out = snake_with_digit_boundaries(s.as_ref());

    // 2) Sanitize to valid unicode identifier characters
    let mut fixed = String::with_capacity(out.len() + 1);
    for (i, ch) in out.chars().enumerate() {
        let ok = if i == 0 {
            is_xid_start(ch) || ch == '_' || ch.is_ascii_digit()
        } else {
            is_xid_continue(ch) || ch == '_'
        };
        fixed.push(if ok { ch } else { '_' });
    }

    if fixed.is_empty() {
        fixed.push('_');
    }

    // 3) Prefix with '_' if starts with digit
    if fixed.chars().next().unwrap().is_ascii_digit() {
        fixed.insert(0, '_');
    }

    // 4) Handle keywords by using raw identifiers (r#type, etc.)
    keyword_guard_ident(&fixed)
}

/// Generate PascalCase type name for a directory: Dir_Foo_BarBaz.
fn make_pascal_dir_ident(rel_dir: &Path) -> syn::Ident {
    let parts = rel_dir
        .iter()
        .map(|c| c.to_string_lossy().to_string().to_upper_camel_case())
        .collect::<Vec<_>>()
        .join("_");
    let name = if parts.is_empty() {
        "Dir_".to_string()
    } else {
        format!("Dir_{}", parts)
    };
    keyword_guard_ident(&name)
}

/// Generate PascalCase root struct name from the module identifier.
fn pascal_from_ident(ident: &syn::Ident) -> syn::Ident {
    let name = {
        let n = ident.to_string().to_upper_camel_case();
        if n.is_empty() { "Assets".into() } else { n }
    };
    keyword_guard_ident(&name)
}

#[derive(Debug)]
struct DirNode {
    rel_dir: PathBuf,
    files: Vec<PathBuf>,
    dirs: BTreeMap<String, DirNode>,
    atlas_info: Option<AtlasInfo>,
}

impl DirNode {
    fn from_files(files: &[PathBuf]) -> Self {
        let mut root = DirNode {
            rel_dir: PathBuf::new(),
            files: vec![],
            dirs: BTreeMap::new(),
            atlas_info: None,
        };
        for f in files {
            root.insert_file(f);
        }
        root.sort_files_recursive();
        root
    }

    /// Walk down directory path, creating intermediate DirNodes as needed, then add file to leaf node.
    fn insert_file(&mut self, file_rel: &Path) {
        let mut node = self;
        let mut cur_rel = PathBuf::new();
        if let Some(parent) = file_rel.parent() {
            for comp in parent.iter() {
                let name = comp.to_string_lossy().to_string();
                cur_rel.push(&name);
                node = node.dirs.entry(name).or_insert_with(|| DirNode {
                    rel_dir: cur_rel.clone(),
                    files: vec![],
                    dirs: BTreeMap::new(),
                    atlas_info: None,
                });
            }
        }
        node.files.push(file_rel.to_path_buf());
    }

    fn sort_files_recursive(&mut self) {
        self.files.sort();
        for child in self.dirs.values_mut() {
            child.sort_files_recursive();
        }
    }

    fn collect_collisions(&self, errs: &mut Vec<String>, with_ext: bool) {
        use std::collections::HashMap;
        let mut seen: HashMap<String, PathBuf> = HashMap::new();

        for f in &self.files {
            let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let name = if with_ext {
                match f.extension().and_then(|s| s.to_str()) {
                    Some(ext) if !ext.is_empty() => {
                        format!("{}_{}", stem, ext.to_ascii_lowercase())
                    }
                    _ => stem.to_string(),
                }
            } else {
                stem.to_string()
            };
            let key = snake_with_digit_boundaries(&name);
            if let Some(prev) = seen.get(&key) {
                errs.push(format!(
                    "collision in '{}': '{}' and '{}' -> '{}'",
                    display_rel(&self.rel_dir),
                    prev.display(),
                    f.display(),
                    key
                ));
            } else {
                seen.insert(key, f.clone());
            }
        }

        for (dir_name, child) in &self.dirs {
            let key = snake_with_digit_boundaries(dir_name);
            if let Some(conf) = seen.get(&key) {
                let is_atlas = child.atlas_info.is_some();
                let entity_type = if is_atlas { "atlas" } else { "dir" };
                errs.push(format!(
                    "collision in '{}': {} '{}' and file '{}' -> '{}'",
                    display_rel(&self.rel_dir),
                    entity_type,
                    dir_name,
                    conf.display(),
                    key
                ));
            }
        }

        // Check atlas frame name collisions
        for child in self.dirs.values() {
            if let Some(ref atlas_info) = child.atlas_info {
                let mut frame_seen: HashMap<String, String> = HashMap::new();
                for frame_name in &atlas_info.frame_names {
                    let key = make_snake_ident(frame_name).to_string();
                    if let Some(prev) = frame_seen.get(&key) {
                        errs.push(format!(
                            "collision in atlas '{}': frames '{}' and '{}' -> '{}'",
                            atlas_info.json_rel, prev, frame_name, key
                        ));
                    } else {
                        frame_seen.insert(key, frame_name.clone());
                    }
                }
            }
            child.collect_collisions(errs, with_ext);
        }
    }
}

fn display_rel(p: &Path) -> String {
    let s = p.to_string_lossy();
    if s.is_empty() {
        ".".to_string()
    } else {
        s.into_owned()
    }
}

fn generate_structs(
    root: &DirNode,
    root_rel: &str,
    parsers: &HashMap<String, Type>,
    root_struct_ident: &syn::Ident,
    id: Option<String>,
    with_ext: bool,
    embed: bool,
    custom_fields: &[CustomField],
) -> TokenStream2 {
    let mut defs = TokenStream2::new();
    defs.extend(quote! {
        #[allow(unused_imports)]
        use super::*;
    });

    // Build index mapping for PATHS/DATA array access
    let all_paths = gather_full_paths(root, root_rel);
    let path_to_idx: std::collections::BTreeMap<_, _> = all_paths
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i))
        .collect();

    let mut path_lits = vec![];
    let mut data_elems = TokenStream2::new();
    for p in &all_paths {
        let rel_lit = LitStr::new(p, Span::call_site());
        if embed {
            data_elems.extend(quote! {
                (#rel_lit, include_bytes!(::core::concat!(::core::env!("CARGO_MANIFEST_DIR"), "/", #rel_lit))),
            });
        }
        path_lits.push(rel_lit);
    }

    let list_id_expr: TokenStream2 = id
        .map(|s| {
            let lit = LitStr::new(&s, Span::call_site());
            quote!(#lit)
        })
        .unwrap_or_else(|| quote!(::core::any::type_name::<Self>()));

    if embed {
        defs.extend(quote! {
            impl #root_struct_ident {
                pub const DATA: &'static [(&'static str, &'static [u8])] = &[ #data_elems ];
                fn to_load_list() -> LoadList { Self::DATA.into() }
            }
        });
    } else {
        defs.extend(quote! {
            impl #root_struct_ident {
                pub const PATHS: &'static [&'static str] = &[ #( #path_lits ),* ];
                fn to_load_list() -> LoadList { Self::PATHS.into() }
            }
        });
    }

    let build_expr = gen_parse_expr(
        root,
        root_struct_ident,
        root_rel,
        with_ext,
        &path_to_idx,
        embed,
        parsers,
        custom_fields,
    );

    defs.extend(quote! {
        impl AutoLoad for #root_struct_ident {
            fn list_id() -> &'static str { #list_id_expr }
            fn load_list() -> LoadList { Self::to_load_list() }
            fn parse_list(world: &mut World, loader: &mut AssetLoader) -> Result<Option<Self>, String> {
                Ok(Some(#build_expr))
            }
        }
    });

    gen_dir_node(
        root,
        parsers,
        root_struct_ident,
        &mut defs,
        with_ext,
        custom_fields,
    );
    defs
}

fn gen_dir_node(
    node: &DirNode,
    parsers: &HashMap<String, Type>,
    root_struct_ident: &syn::Ident,
    defs: &mut TokenStream2,
    with_ext: bool,
    custom_fields: &[CustomField],
) {
    let ty_ident = if node.rel_dir.as_os_str().is_empty() {
        root_struct_ident.clone()
    } else {
        make_pascal_dir_ident(&node.rel_dir)
    };

    let mut field_idents = vec![];
    let mut field_types = vec![];

    if let Some(ref atlas_info) = node.atlas_info {
        for frame_name in &atlas_info.frame_names {
            let field_ident = make_snake_ident(frame_name);
            field_idents.push(field_ident);
            field_types.push(quote!(Sprite));
        }
    } else {
        for (dir_name, child) in &node.dirs {
            let field_ident = make_snake_ident(dir_name);
            let field_ty = if child.rel_dir.as_os_str().is_empty() {
                root_struct_ident.clone()
            } else {
                make_pascal_dir_ident(&child.rel_dir)
            };
            field_idents.push(field_ident);
            field_types.push(quote!(#field_ty));
            gen_dir_node(
                child,
                parsers,
                root_struct_ident,
                defs,
                with_ext,
                custom_fields,
            );
        }

        for f in &node.files {
            let field_ident = field_ident_from_filename(f, with_ext);
            let ext = f
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let field_ty = parsers
                .get(&ext)
                .map_or_else(|| quote!(::std::vec::Vec<u8>), |ty| quote!(#ty));
            field_idents.push(field_ident);
            field_types.push(field_ty);
        }

        if node.rel_dir.as_os_str().is_empty() {
            for custom_field in custom_fields {
                let field_ident = make_snake_ident(&custom_field.name);
                let field_ty = &custom_field.ty;
                field_idents.push(field_ident);
                field_types.push(quote!(#field_ty));
            }
        }
    }

    defs.extend(quote! {
        #[derive(Resource, Clone)]
        pub struct #ty_ident {
            #(pub #field_idents: #field_types,)*
        }
    });
}

fn field_ident_from_filename(path_rel: &Path, with_ext: bool) -> syn::Ident {
    let stem = path_rel
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let name = if with_ext {
        path_rel
            .extension()
            .and_then(|s| s.to_str())
            .filter(|e| !e.is_empty())
            .map(|e| format!("{}_{}", stem, e.to_ascii_lowercase()))
            .unwrap_or_else(|| stem.to_string())
    } else {
        stem.to_string()
    };
    make_snake_ident(name)
}

fn normalize_path(base: &str) -> String {
    let mut normalized = base.replace('\\', "/");
    while normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}

fn build_full_path(base: &str, rel: &str) -> String {
    let norm = normalize_path(base);
    if norm.is_empty() {
        rel.to_string()
    } else {
        format!("{}/{}", norm, rel)
    }
}

fn gather_full_paths(root: &DirNode, root_rel: &str) -> Vec<String> {
    fn rec(node: &DirNode, acc: &mut Vec<String>, base: &str) {
        for f in &node.files {
            let rel = f.to_string_lossy().replace('\\', "/");
            acc.push(build_full_path(base, &rel));
        }
        for child in node.dirs.values() {
            // if this is an atlas namespace add its JSON and image paths
            if let Some(ref atlas_info) = child.atlas_info {
                acc.push(build_full_path(base, &atlas_info.json_rel));
                acc.push(build_full_path(base, &atlas_info.image_rel));
            }
            rec(child, acc, base);
        }
    }
    let mut out = vec![];
    rec(root, &mut out, root_rel);
    out
}

/// generate code that extracts assets from loader and constructs the typed tree.
fn gen_parse_expr(
    node: &DirNode,
    root_struct_ident: &syn::Ident,
    root_rel: &str,
    with_ext: bool,
    path_to_idx: &std::collections::BTreeMap<String, usize>,
    embed: bool,
    parsers: &HashMap<String, Type>,
    custom_fields: &[CustomField],
) -> TokenStream2 {
    let this_ty = if node.rel_dir.as_os_str().is_empty() {
        root_struct_ident.clone()
    } else {
        make_pascal_dir_ident(&node.rel_dir)
    };

    let mut pre_lets = vec![];
    let mut field_inits = vec![];

    // check if this is an atlas namespac
    if let Some(ref atlas_info) = node.atlas_info {
        let json_full = build_full_path(root_rel, &atlas_info.json_rel);
        let json_idx = *path_to_idx
            .get(&json_full)
            .expect("atlas JSON must exist in PATHS/DATA mapping");
        let json_idx_lit = syn::LitInt::new(&json_idx.to_string(), Span::call_site());

        let image_full = build_full_path(root_rel, &atlas_info.image_rel);
        let image_idx = *path_to_idx
            .get(&image_full)
            .expect("atlas image must exist in PATHS/DATA mapping");
        let image_idx_lit = syn::LitInt::new(&image_idx.to_string(), Span::call_site());

        let json_id_expr = if embed {
            quote!(Self::DATA[#json_idx_lit].0)
        } else {
            quote!(Self::PATHS[#json_idx_lit])
        };

        let image_id_expr = if embed {
            quote!(Self::DATA[#image_idx_lit].0)
        } else {
            quote!(Self::PATHS[#image_idx_lit])
        };

        let json_lit = LitStr::new(&json_full, Span::call_site());
        let image_lit = LitStr::new(&image_full, Span::call_site());

        pre_lets.push(quote! {
            let __atlas_base: Sprite = match loader.take::<Sprite>(#image_id_expr) {
                Some(v) => v,
                None => return Err(::std::format!(
                    "atlas base image '{}' missing or wrong type",
                    #image_lit,
                )),
            };
        });

        pre_lets.push(quote! {
            let __atlas_json: ::std::vec::Vec<u8> = match loader.take::<::std::vec::Vec<u8>>(#json_id_expr) {
                Some(v) => v,
                None => return Err(::std::format!(
                    "atlas json '{}' missing",
                    #json_lit,
                )),
            };
        });

        pre_lets.push(quote! {
            let mut __atlas_map = create_sprites_from_spritesheet(&__atlas_json, &__atlas_base)
                .map_err(|e| ::std::format!("atlas '{}' parse error: {}", #json_lit, e))?;
        });

        // extract each frame
        for frame_name in &atlas_info.frame_names {
            let field_ident = make_snake_ident(frame_name);
            let frame_lit = LitStr::new(frame_name, Span::call_site());

            pre_lets.push(quote! {
                let #field_ident: Sprite = __atlas_map.remove(#frame_lit)
                    .ok_or_else(|| ::std::format!(
                        "atlas frame '{}' not found in '{}'",
                        #frame_lit,
                        #json_lit,
                    ))?;
            });
            field_inits.push(quote!(#field_ident: #field_ident));
        }
    } else {
        // normal directory node processing
        for (dir_name, child) in &node.dirs {
            let field_ident = make_snake_ident(dir_name);
            let child_expr = gen_parse_expr(
                child,
                root_struct_ident,
                root_rel,
                with_ext,
                path_to_idx,
                embed,
                parsers,
                custom_fields,
            );
            pre_lets.push(quote! { let #field_ident = #child_expr; });
            field_inits.push(quote!(#field_ident: #field_ident));
        }

        for f in &node.files {
            let field_ident = field_ident_from_filename(f, with_ext);
            let ext = f
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let field_ty: TokenStream2 = parsers
                .get(&ext)
                .map_or_else(|| quote!(::std::vec::Vec<u8>), |ty| quote!(#ty));

            let rel = f.to_string_lossy().replace('\\', "/");
            let full = build_full_path(root_rel, &rel);
            let idx = *path_to_idx
                .get(&full)
                .expect("path must exist in PATHS/DATA mapping");
            let idx_lit = syn::LitInt::new(&idx.to_string(), Span::call_site());

            let id_expr = if embed {
                quote!(Self::DATA[#idx_lit].0)
            } else {
                quote!(Self::PATHS[#idx_lit])
            };

            pre_lets.push(quote! {
                let #field_ident: #field_ty = match loader.take::<#field_ty>(#id_expr) {
                    Some(v) => v,
                    None => return Err(::std::format!(
                        "asset '{}' missing or wrong type (expected {})",
                        #id_expr,
                        ::core::any::type_name::<#field_ty>(),
                    )),
                };
            });
            field_inits.push(quote!(#field_ident: #field_ident));
        }

        if node.rel_dir.as_os_str().is_empty() {
            for custom_field in custom_fields {
                let field_ident = make_snake_ident(&custom_field.name);
                let field_ty = &custom_field.ty;
                let parser_fn = &custom_field.parser;

                pre_lets.push(quote! {
                    let #field_ident: #field_ty = match #parser_fn(world, loader) {
                        Ok(Some(v)) => v,
                        Ok(None) => return Ok(None),
                        Err(e) => return Err(::std::format!(
                            "custom field '{}' parser failed: {}",
                            stringify!(#field_ident),
                            e
                        )),
                    };
                });
                field_inits.push(quote!(#field_ident: #field_ident));
            }
        }
    }

    quote! {{
        #(#pre_lets)*
        #this_ty { #( #field_inits, )* }
    }}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snake_splits_digit_boundaries() {
        assert_eq!(snake_with_digit_boundaries("powerUp12"), "power_up_12");
        assert_eq!(snake_with_digit_boundaries("HTTP2Server"), "http_2_server");
        assert_eq!(snake_with_digit_boundaries("foo12bar"), "foo_12_bar");
        assert_eq!(
            snake_with_digit_boundaries("foo_bar_99baz"),
            "foo_bar_99_baz"
        );
    }

    #[test]
    fn make_snake_ident_handles_digits_and_keywords() {
        assert_eq!(make_snake_ident("powerUp12").to_string(), "power_up_12");
        // keyword becomes raw ident automatically
        assert_eq!(make_snake_ident("type").to_string(), "r#type");
        // leading digit gets prefixed
        assert_eq!(make_snake_ident("2dTexture").to_string(), "_2_d_texture");
    }

    #[test]
    fn field_ident_with_ext_suffix() {
        use std::path::PathBuf;
        let id = super::field_ident_from_filename(&PathBuf::from("powerUp12.ogg"), true);
        assert_eq!(id.to_string(), "power_up_12_ogg");
    }

    #[test]
    fn dir_and_root_pascal_names() {
        let p = PathBuf::from("Snd/FX");
        assert_eq!(make_pascal_dir_ident(&p).to_string(), "Dir_Snd_Fx");

        let m = syn::Ident::new("raw_assets", Span::call_site());
        assert_eq!(pascal_from_ident(&m).to_string(), "RawAssets");
    }

    #[test]
    fn collision_keys_match_generation() {
        // What we use in collision checks equals what we generate for fields
        let key = snake_with_digit_boundaries("powerUp12_ogg");
        assert_eq!(key, "power_up_12_ogg");
        let field = make_snake_ident("powerUp12_ogg").to_string();
        assert_eq!(field, "power_up_12_ogg");
    }

    #[test]
    fn atlas_frame_name_normalization() {
        assert_eq!(
            make_snake_ident("player_idle.png").to_string(),
            "player_idle_png"
        );
        assert_eq!(
            make_snake_ident("PlayerRun.png").to_string(),
            "player_run_png"
        );
        assert_eq!(
            make_snake_ident("ui/button.png").to_string(),
            "ui_button_png"
        );
        assert_eq!(make_snake_ident("Icon2D.png").to_string(), "icon_2_d_png");
    }

    #[test]
    fn atlas_frame_collision_detection() {
        let frame1 = make_snake_ident("Idle.png").to_string();
        let frame2 = make_snake_ident("idle.png").to_string();
        assert_eq!(frame1, frame2, "These should collide");
        assert_eq!(frame1, "idle_png");
    }
}
