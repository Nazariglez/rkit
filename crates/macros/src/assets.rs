use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};
use syn::{ExprArray, ExprPath, ItemMod, LitStr, Token, Type, parse::Parser, parse_macro_input};

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

const ROOT_ATTR: &str = "root";
const SKIP_ATTR: &str = "skip";
const TYPES_ATTR: &str = "types";
const WITH_EXT_ATTR: &str = "with_ext";
const EMBED_ATTR: &str = "embed";
const LIST_ID_ATTR: &str = "id";

pub fn assets(attr: TokenStream, item: TokenStream) -> TokenStream {
    // #[assets(root="...", skip=[...],
    //          types(ogg = Sound, png = Texture),
    //          with_ext = false
    //          embed = false,
    //          id = "lit")]
    let mut root_rel = None;
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
                // ext : Type
                let ext_ident = entry
                    .path
                    .get_ident()
                    .ok_or_else(|| entry.error("expected an extension like `ogg`"))?
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
            Err(meta.error(format!(
                "unknown key; allowed: `{}`, `{}`, `{}(...)`, `{}`, `{}`, `{}`",
                ROOT_ATTR, SKIP_ATTR, TYPES_ATTR, WITH_EXT_ATTR, EMBED_ATTR, LIST_ID_ATTR
            )))
        }
    });

    if let Err(e) = parser.parse(attr) {
        return e.to_compile_error().into();
    }

    let mut module = parse_macro_input!(item as ItemMod);

    let root_rel = match root_rel {
        Some(s) => s,
        None => {
            return compile_err(&format!(
                "missing `{ROOT_ATTR} = \"...\"` in #[assets(...)]"
            ));
        }
    };

    // Must be an inline module
    let items = match &mut module.content {
        Some((_brace, items)) => items,
        None => {
            return compile_err("assets requires an inline module: `pub mod my_assets {}`");
        }
    };

    // Resolve absolute root path from caller crate
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let root_abs = PathBuf::from(&manifest_dir).join(&root_rel);
    if !root_abs.is_dir() {
        return compile_err(&format!(
            "assets root does not exist or is not a directory: {}",
            root_abs.display()
        ));
    }

    // Build skip globset (defaults + user)
    let skipset = match build_skipset(user_skip) {
        Ok(s) => s,
        Err(e) => return compile_err(&e),
    };

    // Collect files honoring .gitignore/.ignore
    let files = match collect_files_with_gitignore(&root_abs, &skipset) {
        Ok(v) => v,
        Err(e) => return compile_err(&e),
    };

    // Build directory tree
    let tree = DirNode::from_files(&files);

    // Check collisions and aggregate errors (single compile_error!)
    let mut errs: Vec<String> = Vec::new();
    tree.collect_collisions(&mut errs, with_ext);
    if !errs.is_empty() {
        let msg = errs.join("\\n");
        return compile_err(&msg);
    }

    // Auto-name root struct from module ident (e.g., my_assets -> MyAssets)
    let root_struct_ident = pascal_from_ident(&module.ident);

    // Generate structs + PATHS + paths() + load_list()
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

// ---------- helpers ----------

fn compile_err(msg: &str) -> TokenStream {
    TokenStream::from(quote! { compile_error!(#msg); })
}

/// Default skips + user wildcards
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
        if pat.contains('/') {
            pats.push(pat);
        } else {
            pats.push(format!("**/{}", pat));
        }
    }

    let mut b = GlobSetBuilder::new();
    for p in pats {
        let g = Glob::new(&p).map_err(|e| format!("invalid glob in skip: `{}` ({e})", p))?;
        b.add(g);
    }
    b.build()
        .map_err(|e| format!("failed to build skip set: {e}"))
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

    let mut out = Vec::<PathBuf>::new();
    for dent in builder.build() {
        let dent = dent.map_err(|e| e.to_string())?;
        let path = dent.path();
        if path == root_abs {
            continue;
        }
        let Some(ft) = dent.file_type() else {
            continue;
        };
        if ft.is_dir() {
            continue;
        }

        let rel = path.strip_prefix(root_abs).map_err(|e| e.to_string())?;
        let s = rel.to_string_lossy().replace('\\', "/");
        if skip.is_match(&s) {
            continue;
        }

        out.push(rel.to_path_buf());
    }
    out.sort();
    Ok(out)
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

    /// Duplicate basenames per directory (file/file or file/dir)
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
            let key = to_snake(&name);
            if let Some(prev) = seen.get(&key) {
                errs.push(format!(
                    "name collision in `{}`: `{}` and `{}` both map to field `{}`.",
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
            let key = to_snake(dir_name);
            if let Some(conf) = seen.get(&key) {
                errs.push(format!(
                    "name collision in `{}` between subdir `{}` and file `{}` (both map to `{}`).",
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

// ---------- codegen (structs + PATHS + paths + load_list) ----------

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

    // define paths and data depending on the embed flag
    let all_paths = gather_full_paths(root, root_rel);
    let mut path_to_idx = std::collections::BTreeMap::<String, usize>::new();
    for (i, s) in all_paths.iter().enumerate() {
        path_to_idx.insert(s.clone(), i);
    }
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

    // ---- choose the list id expression (single block) ----
    let list_id_expr: TokenStream2 = if let Some(id_str) = id {
        let lit = LitStr::new(&id_str, Span::call_site());
        quote!( #lit )
    } else {
        quote!(::core::any::type_name::<Self>())
    };

    if embed {
        defs.extend(quote! {
            impl #root_struct_ident {
                pub const DATA: &'static [(&'static str, &'static [u8])] = &[ #data_elems ];
                fn to_load_list() -> LoadList {
                    Self::DATA.into()
                }
            }
        });
    } else {
        defs.extend(quote! {
            impl #root_struct_ident {
                pub const PATHS: &'static [&'static str] = &[ #( #path_lits ),* ];
                fn to_load_list() -> LoadList {
                    Self::PATHS.into()
                }
            }
        });
    }

    // build expression that constructs the full struct by loader.take::<T>(id)
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
            fn list_id() -> &'static str {
                #list_id_expr
            }

            fn load_list() -> LoadList {
                Self::to_load_list()
            }

            fn parse_list(loader: &mut AssetLoader) -> Result<Option<Self>, String> {
                let value = #build_expr;
                Ok(Some(value))
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
        type_ident_from_dir(&node.rel_dir)
    };

    let mut field_idents: Vec<syn::Ident> = Vec::new();
    let mut field_types: Vec<TokenStream2> = Vec::new();

    // subdirs
    for (dir_name, child) in &node.dirs {
        let field_ident = syn::Ident::new(&to_snake(dir_name), Span::call_site());
        let field_ty = if child.rel_dir.as_os_str().is_empty() {
            root_struct_ident.clone()
        } else {
            type_ident_from_dir(&child.rel_dir)
        };
        field_idents.push(field_ident.clone());
        field_types.push(quote!(#field_ty));
        gen_dir_node(child, parsers, root_struct_ident, defs, with_ext);
    }

    // files -> Type from map or Vec<u8>
    for f in &node.files {
        let field_ident = field_ident_from_filename(f, with_ext);
        let ext = f
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if let Some(ty) = parsers.get(&ext) {
            field_idents.push(field_ident);
            field_types.push(quote!(#ty));
        } else {
            field_idents.push(field_ident);
            field_types.push(quote!(::std::vec::Vec<u8>));
        }
    }

    defs.extend(quote! {
        #[derive(Resource, Clone)]
        pub struct #ty_ident {
            #(pub #field_idents: #field_types,)*
        }
    });
}

// ----- naming helpers -----

fn field_ident_from_filename(path_rel: &Path, with_ext: bool) -> syn::Ident {
    let stem = path_rel
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let base = if with_ext {
        match path_rel.extension().and_then(|s| s.to_str()) {
            Some(ext) if !ext.is_empty() => format!("{}_{}", stem, ext.to_ascii_lowercase()),
            _ => stem.to_string(),
        }
    } else {
        stem.to_string()
    };
    syn::Ident::new(&to_snake(&base), Span::call_site())
}

fn type_ident_from_dir(rel_dir: &Path) -> syn::Ident {
    let mut parts: Vec<String> = Vec::new();
    for comp in rel_dir.iter() {
        let x = comp.to_string_lossy();
        let mut part = String::new();
        let mut up = true;
        for ch in x.chars() {
            if ch.is_ascii_alphanumeric() {
                if up {
                    part.push(ch.to_ascii_uppercase());
                    up = false;
                } else {
                    part.push(ch);
                }
            } else {
                up = true;
            }
        }
        if part.is_empty() {
            part.push_str("Dir");
        }
        parts.push(part);
    }
    let name = format!("Dir_{}", parts.join("_"));
    format_ident!("{}", name)
}

fn pascal_from_ident(ident: &syn::Ident) -> syn::Ident {
    let raw = ident.to_string();
    let mut out = String::new();
    let mut up = true;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            if up {
                out.push(ch.to_ascii_uppercase());
                up = false;
            } else {
                out.push(ch);
            }
        } else {
            up = true;
        }
    }
    if out.is_empty() {
        out.push_str("Assets");
    }
    format_ident!("{}", out)
}

fn to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_underscore = false;
    for ch in s.chars() {
        let c = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '_'
        };
        if c == '_' {
            if !prev_underscore {
                out.push(c);
            }
            prev_underscore = true;
        } else {
            out.push(c);
            prev_underscore = false;
        }
    }
    if out.is_empty() || out.chars().next().unwrap().is_ascii_digit() {
        out.insert(0, '_');
    }
    out
}

// ----- path list helper -----

fn gather_full_paths(root: &DirNode, root_rel: &str) -> Vec<String> {
    fn rec(node: &DirNode, acc: &mut Vec<String>, base: &str) {
        for f in &node.files {
            let rel = f.to_string_lossy().replace('\\', "/");
            if base.is_empty() {
                acc.push(rel);
            } else {
                // normalize base like "./assets" or "assets/"
                let mut b = base.replace('\\', "/");
                while b.ends_with('/') {
                    b.pop();
                }
                acc.push(format!("{}/{}", b, rel));
            }
        }
        for child in node.dirs.values() {
            rec(child, acc, base);
        }
    }
    let mut out = Vec::new();
    rec(root, &mut out, root_rel);
    out
}

fn gen_init_expr(
    node: &DirNode,
    root_struct_ident: &syn::Ident,
    root_rel: &str,
    with_ext: bool,
    path_to_idx: &std::collections::BTreeMap<String, usize>,
) -> TokenStream2 {
    let this_ty = if node.rel_dir.as_os_str().is_empty() {
        root_struct_ident.clone()
    } else {
        type_ident_from_dir(&node.rel_dir)
    };

    let mut field_inits: Vec<TokenStream2> = Vec::new();

    // subdirs
    for (dir_name, child) in &node.dirs {
        let field_ident = syn::Ident::new(&to_snake(dir_name), Span::call_site());
        let child_expr = gen_init_expr(child, root_struct_ident, root_rel, with_ext, path_to_idx);
        field_inits.push(quote!( #field_ident: #child_expr ));
    }

    // files -> look up index in PATHS
    for f in &node.files {
        let field_ident = field_ident_from_filename(f, with_ext);

        // reproduce gather_full_paths' normalization
        let rel = f.to_string_lossy().replace('\\', "/");
        let mut base = root_rel.replace('\\', "/");
        while base.ends_with('/') {
            base.pop();
        }
        let full = if base.is_empty() {
            rel
        } else {
            format!("{}/{}", base, rel)
        };

        let idx = *path_to_idx
            .get(&full)
            .expect("path must exist in PATHS mapping");
        let idx_lit = syn::LitInt::new(&idx.to_string(), Span::call_site());

        field_inits.push(quote!( #field_ident: list.get(Self::PATHS[#idx_lit])? ));
    }

    quote!( #this_ty { #( #field_inits, )* } )
}

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
        type_ident_from_dir(&node.rel_dir)
    };

    let mut pre_lets: Vec<TokenStream2> = Vec::new();
    let mut field_inits: Vec<TokenStream2> = Vec::new();

    // subdirs first: each becomes a nested block that returns its struct
    for (dir_name, child) in &node.dirs {
        let field_ident = syn::Ident::new(&to_snake(dir_name), Span::call_site());
        let child_expr = gen_parse_expr(
            child,
            root_struct_ident,
            root_rel,
            with_ext,
            path_to_idx,
            embed,
            parsers,
        );
        pre_lets.push(quote! {
            let #field_ident = #child_expr;
        });
        field_inits.push(quote!( #field_ident: #field_ident ));
    }

    // files -> compute type, id expr, then take::<T>(id)
    for f in &node.files {
        let field_ident = field_ident_from_filename(f, with_ext);
        let ext = f
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        // choose the type for this field
        let field_ty: TokenStream2 = if let Some(ty) = parsers.get(&ext) {
            quote!(#ty)
        } else {
            quote!(::std::vec::Vec<u8>)
        };

        // reproduce gather_full_paths' normalization to get the full path key
        let rel = f.to_string_lossy().replace('\\', "/");
        let mut base = root_rel.replace('\\', "/");
        while base.ends_with('/') {
            base.pop();
        }
        let full = if base.is_empty() {
            rel
        } else {
            format!("{}/{}", base, rel)
        };

        let idx = *path_to_idx
            .get(&full)
            .expect("path must exist in PATHS/DATA mapping");
        let idx_lit = syn::LitInt::new(&idx.to_string(), Span::call_site());

        let id_expr = if embed {
            quote!( Self::DATA[#idx_lit].0 )
        } else {
            quote!( Self::PATHS[#idx_lit] )
        };

        pre_lets.push(quote! {
            let #field_ident: #field_ty = match loader.take::<#field_ty>(#id_expr) {
                Some(v) => v,
                None => return Err(::std::format!(
                    "asset '{}' missing or wrong type; expected {}",
                    #id_expr,
                    ::core::any::type_name::<#field_ty>(),
                )),
            };
        });
        field_inits.push(quote!( #field_ident: #field_ident ));
    }

    quote! {{
        #(#pre_lets)*
        #this_ty { #( #field_inits, )* }
    }}
}
