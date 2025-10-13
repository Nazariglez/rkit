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

use proc_macro_error::abort_call_site;

const ROOT_ATTR: &str = "root";
const SKIP_ATTR: &str = "skip";
const TYPES_ATTR: &str = "types";
const WITH_EXT_ATTR: &str = "with_ext";
const EMBED_ATTR: &str = "embed";
const LIST_ID_ATTR: &str = "id";

pub fn assets(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut root_rel: Option<String> = None;
    let mut user_skip = Vec::new();
    let mut with_ext = false;
    let mut embed = false;
    let mut list_id: Option<String> = None;
    let mut parser_map: HashMap<String, Type> = HashMap::new();

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
        } else {
            Err(meta
                .error("unknown attribute; use: root, skip, types(...), with_ext, embed, or id"))
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
        Err(e) => abort_call_site!("{}", e),
    };

    let files = match collect_files_with_gitignore(&root_abs, &skipset) {
        Ok(v) => v,
        Err(e) => abort_call_site!("{}", e),
    };

    let tree = DirNode::from_files(&files);

    let mut errs: Vec<String> = Vec::new();
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
    );

    items.push(syn::Item::Verbatim(generated));
    TokenStream::from(quote! { #module })
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

    let mut files = Vec::new();
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
}

impl DirNode {
    fn from_files(files: &[PathBuf]) -> Self {
        let mut root = DirNode {
            rel_dir: PathBuf::new(),
            files: Vec::new(),
            dirs: BTreeMap::new(),
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
                    files: Vec::new(),
                    dirs: BTreeMap::new(),
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

    /// Detect naming collisions by simulating field name generation (critical for catching compile errors early).
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

        for dir_name in self.dirs.keys() {
            let key = snake_with_digit_boundaries(dir_name);
            if let Some(conf) = seen.get(&key) {
                errs.push(format!(
                    "collision in '{}': dir '{}' and file '{}' -> '{}'",
                    display_rel(&self.rel_dir),
                    dir_name,
                    conf.display(),
                    key
                ));
            }
        }

        for child in self.dirs.values() {
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
    );

    defs.extend(quote! {
        impl AutoLoad for #root_struct_ident {
            fn list_id() -> &'static str { #list_id_expr }
            fn load_list() -> LoadList { Self::to_load_list() }
            fn parse_list(loader: &mut AssetLoader) -> Result<Option<Self>, String> {
                Ok(Some(#build_expr))
            }
        }
    });

    gen_dir_node(root, parsers, root_struct_ident, &mut defs, with_ext);
    defs
}

fn gen_dir_node(
    node: &DirNode,
    parsers: &HashMap<String, Type>,
    root_struct_ident: &syn::Ident,
    defs: &mut TokenStream2,
    with_ext: bool,
) {
    let ty_ident = if node.rel_dir.as_os_str().is_empty() {
        root_struct_ident.clone()
    } else {
        make_pascal_dir_ident(&node.rel_dir)
    };

    let mut field_idents = Vec::new();
    let mut field_types = Vec::new();

    for (dir_name, child) in &node.dirs {
        let field_ident = make_snake_ident(dir_name);
        let field_ty = if child.rel_dir.as_os_str().is_empty() {
            root_struct_ident.clone()
        } else {
            make_pascal_dir_ident(&child.rel_dir)
        };
        field_idents.push(field_ident);
        field_types.push(quote!(#field_ty));
        gen_dir_node(child, parsers, root_struct_ident, defs, with_ext);
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
            rec(child, acc, base);
        }
    }
    let mut out = Vec::new();
    rec(root, &mut out, root_rel);
    out
}

/// Generate code that extracts assets from loader and constructs the typed tree.
fn gen_parse_expr(
    node: &DirNode,
    root_struct_ident: &syn::Ident,
    root_rel: &str,
    with_ext: bool,
    path_to_idx: &std::collections::BTreeMap<String, usize>,
    embed: bool,
    parsers: &HashMap<String, Type>,
) -> TokenStream2 {
    let this_ty = if node.rel_dir.as_os_str().is_empty() {
        root_struct_ident.clone()
    } else {
        make_pascal_dir_ident(&node.rel_dir)
    };

    let mut pre_lets = Vec::new();
    let mut field_inits = Vec::new();

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
}
