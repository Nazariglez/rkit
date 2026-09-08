use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    Error, FnArg, ItemFn, Pat, PatIdent, ReturnType, Type, TypeArray, TypeGroup, TypeParen,
    TypePath, TypePtr, TypeReference, TypeSlice, ext::IdentExt, spanned::Spanned, visit::Visit,
};

pub(crate) const UI_NEW: &str = "__ui_new";
pub(crate) const UI_CHILDREN: &str = "__ui_children";
pub(crate) const UI_BUILD: &str = "__ui_build";
pub(crate) const RESERVED_PREFIX: &str = "__ui_";

struct Prop {
    ident: Ident,
    ty: Type,
    kind: PropKind,
}

enum PropKind {
    Required(Ident),
    Optional(Box<Type>),
}

pub fn expand(attribute: TokenStream, item: TokenStream) -> TokenStream {
    match expand_inner(attribute.into(), item.into()) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_inner(attribute: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let tag = syn::parse2::<Ident>(attribute).map_err(|_| {
        Error::new(
            Span::call_site(),
            "ui_widget requires exactly one tag identifier",
        )
    })?;
    if crate::rsx::is_primitive_tag(&tag) {
        return Err(Error::new_spanned(
            tag,
            "ui_widget tag is a reserved primitive name",
        ));
    }

    let function = syn::parse2::<ItemFn>(item)?;
    let mut used_states = declaration_idents(&function, &tag)?;
    let (props, children) = validate_signature(&function, &mut used_states)?;
    Ok(generate(function, tag, props, children, used_states))
}

fn validate_signature(
    function: &ItemFn,
    used_states: &mut Vec<Ident>,
) -> syn::Result<(Vec<Prop>, bool)> {
    let signature = &function.sig;
    if signature.asyncness.is_some() {
        return Err(Error::new_spanned(
            signature,
            "ui_widget functions cannot be async",
        ));
    }
    if signature.unsafety.is_some() {
        return Err(Error::new_spanned(
            signature,
            "ui_widget functions must be safe",
        ));
    }
    if signature.constness.is_some() {
        return Err(Error::new_spanned(
            signature,
            "ui_widget functions cannot be const",
        ));
    }
    if signature.abi.is_some() {
        return Err(Error::new_spanned(
            signature,
            "ui_widget functions cannot use an external ABI",
        ));
    }
    if signature.variadic.is_some() {
        return Err(Error::new_spanned(
            signature,
            "ui_widget functions cannot be variadic",
        ));
    }
    if !signature.generics.params.is_empty() || signature.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &signature.generics,
            "ui_widget functions cannot have generics or a where clause",
        ));
    }
    if matches!(signature.output, ReturnType::Default) {
        return Err(Error::new_spanned(
            signature,
            "ui_widget functions require an explicit return type",
        ));
    }
    if function
        .attrs
        .iter()
        .any(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
    {
        return Err(Error::new_spanned(
            function,
            "place cfg or cfg_attr before ui_widget or gate the containing module",
        ));
    }

    let mut props = Vec::with_capacity(signature.inputs.len());
    let mut children = false;
    for (index, argument) in signature.inputs.iter().enumerate() {
        let FnArg::Typed(argument) = argument else {
            return Err(Error::new_spanned(
                argument,
                "ui_widget functions cannot have a receiver",
            ));
        };
        if !argument.attrs.is_empty() {
            return Err(Error::new_spanned(
                argument,
                "ui_widget parameters cannot have attributes",
            ));
        }
        let Pat::Ident(PatIdent {
            ident,
            by_ref: None,
            subpat: None,
            ..
        }) = argument.pat.as_ref()
        else {
            return Err(Error::new_spanned(
                &argument.pat,
                "ui_widget parameters must be simple identifier patterns",
            ));
        };
        if is_reserved_name(ident) {
            return Err(Error::new_spanned(
                ident,
                "ui_widget parameter names cannot start with __ui_",
            ));
        }
        if ident.unraw() == "children" {
            if index + 1 != signature.inputs.len() {
                return Err(Error::new_spanned(
                    ident,
                    "ui_widget children must be the final parameter",
                ));
            }
            if !is_children_type(&argument.ty) {
                return Err(Error::new_spanned(
                    &argument.ty,
                    "ui_widget children must be impl IntoIterator<Item = UIScene>",
                ));
            }
            children = true;
            continue;
        }
        if props
            .iter()
            .any(|prop: &Prop| prop.ident.unraw() == ident.unraw())
        {
            return Err(Error::new_spanned(
                ident,
                "ui_widget parameter names must be unique",
            ));
        }

        let ty = (*argument.ty).clone();
        validate_prop_type(&ty)?;
        let kind = if let Some(inner) = standard_option_inner(&ty) {
            PropKind::Optional(Box::new(inner.clone()))
        } else {
            let state = unique_state_ident(&format!("__UI{}", ident.unraw()), used_states);
            used_states.push(state.clone());
            PropKind::Required(state)
        };
        props.push(Prop {
            ident: ident.clone(),
            ty,
            kind,
        });
    }
    Ok((props, children))
}

fn is_children_type(ty: &Type) -> bool {
    let Type::ImplTrait(impl_trait) = ty else {
        return false;
    };
    if impl_trait.bounds.len() != 1 {
        return false;
    }
    let Some(syn::TypeParamBound::Trait(bound)) = impl_trait.bounds.first() else {
        return false;
    };
    if bound.lifetimes.is_some() || !matches!(bound.modifier, syn::TraitBoundModifier::None) {
        return false;
    }
    let Some(segment) = bound.path.segments.last() else {
        return false;
    };
    let plain_prefix = bound
        .path
        .segments
        .iter()
        .take(bound.path.segments.len() - 1)
        .all(|segment| matches!(segment.arguments, syn::PathArguments::None));
    if segment.ident.unraw() != "IntoIterator" || !plain_prefix {
        return false;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return false;
    };
    if arguments.args.len() != 1 {
        return false;
    }
    matches!(arguments.args.first(), Some(syn::GenericArgument::AssocType(item))
        if item.ident.unraw() == "Item" && item.generics.is_none())
}

fn validate_prop_type(ty: &Type) -> syn::Result<()> {
    match ty {
        Type::Infer(_) => Err(Error::new_spanned(
            ty,
            "ui_widget parameter types cannot be inferred",
        )),
        Type::ImplTrait(_) => Err(Error::new_spanned(
            ty,
            "ui_widget parameter types cannot use impl Trait",
        )),
        Type::Reference(TypeReference { lifetime, elem, .. }) => {
            let Some(lifetime) = lifetime else {
                return Err(Error::new_spanned(
                    ty,
                    "ui_widget reference parameters must be explicitly 'static",
                ));
            };
            if lifetime.ident != "static" {
                return Err(Error::new_spanned(
                    lifetime,
                    "ui_widget reference parameters must be explicitly 'static",
                ));
            }
            validate_prop_type(elem)
        }
        Type::Array(TypeArray { elem, .. })
        | Type::Group(TypeGroup { elem, .. })
        | Type::Paren(TypeParen { elem, .. })
        | Type::Ptr(TypePtr { elem, .. })
        | Type::Slice(TypeSlice { elem, .. }) => validate_prop_type(elem),
        Type::Path(TypePath { path, .. }) => {
            for segment in &path.segments {
                if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                    for argument in &arguments.args {
                        match argument {
                            syn::GenericArgument::Type(ty) => validate_prop_type(ty)?,
                            syn::GenericArgument::AssocType(ty) => validate_prop_type(&ty.ty)?,
                            _ => {}
                        }
                    }
                }
            }
            Ok(())
        }
        Type::Tuple(tuple) => {
            for element in &tuple.elems {
                validate_prop_type(element)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub(crate) fn is_reserved_name(ident: &Ident) -> bool {
    ident.unraw().to_string().starts_with(RESERVED_PREFIX)
}

fn standard_option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    if path.qself.is_some() {
        return None;
    }
    let segments = &path.path.segments;
    let option = match segments.len() {
        1 if path.path.leading_colon.is_none() && segments[0].ident.unraw() == "Option" => {
            &segments[0]
        }
        3 if (segments[0].ident.unraw() == "std" || segments[0].ident.unraw() == "core")
            && segments[1].ident.unraw() == "option"
            && segments[2].ident.unraw() == "Option" =>
        {
            &segments[2]
        }
        _ => return None,
    };
    let syn::PathArguments::AngleBracketed(arguments) = &option.arguments else {
        return None;
    };
    let mut arguments = arguments.args.iter();
    let Some(syn::GenericArgument::Type(inner)) = arguments.next() else {
        return None;
    };
    if arguments.next().is_some() {
        return None;
    }
    Some(inner)
}

fn generate(
    function: ItemFn,
    tag: Ident,
    props: Vec<Prop>,
    children: bool,
    used_states: Vec<Ident>,
) -> TokenStream2 {
    let visibility = &function.vis;
    let function_name = &function.sig.ident;
    let return_type = match &function.sig.output {
        ReturnType::Type(_, ty) => ty,
        ReturnType::Default => unreachable!("validated explicit return type"),
    };
    let required: Vec<_> = props
        .iter()
        .filter_map(|prop| match &prop.kind {
            PropKind::Required(state) => Some((prop, state)),
            PropKind::Optional(_) => None,
        })
        .collect();
    let state_idents: Vec<_> = required.iter().map(|(_, state)| (*state).clone()).collect();
    let child_state = children.then(|| unique_state_ident("__UIChildren", &used_states));
    let new_ident = Ident::new(UI_NEW, Span::mixed_site());
    let children_ident = Ident::new(UI_CHILDREN, Span::mixed_site());
    let build_ident = Ident::new(UI_BUILD, Span::mixed_site());

    let mut struct_fields: Vec<_> = props
        .iter()
        .map(|prop| {
            let ident = &prop.ident;
            match &prop.kind {
                PropKind::Required(state) => quote!(#ident: #state),
                PropKind::Optional(_) => {
                    let ty = &prop.ty;
                    quote!(#ident: #ty)
                }
            }
        })
        .collect();
    if let Some(child_state) = &child_state {
        struct_fields.push(quote!(children: #child_state));
    }
    let mut generic_params: Vec<_> = state_idents.iter().map(|state| quote!(#state)).collect();
    if let Some(child_state) = &child_state {
        generic_params.push(quote!(#child_state));
    }
    let definition = if generic_params.is_empty() {
        quote!(#visibility struct #tag { #(#struct_fields,)* })
    } else {
        quote!(#visibility struct #tag<#(#generic_params),*> { #(#struct_fields,)* })
    };

    let initial_states: Vec<_> = required.iter().map(|_| quote!(())).collect();
    let initial_child = children.then(|| quote!(::core::iter::Empty<#return_type>));
    let initial_type = tag_type(&tag, &initial_states, initial_child);
    let mut initial_fields: Vec<_> = props
        .iter()
        .map(|prop| {
            let ident = &prop.ident;
            match prop.kind {
                PropKind::Required(_) => quote!(#ident: ()),
                PropKind::Optional(_) => quote!(#ident: ::core::option::Option::None),
            }
        })
        .collect();
    if children {
        initial_fields.push(quote!(children: ::core::iter::empty::<#return_type>()));
    }
    let final_states: Vec<_> = required
        .iter()
        .map(|(prop, _)| {
            let ty = &prop.ty;
            quote!((#ty,))
        })
        .collect();
    let final_child = child_state.as_ref().map(|state| quote!(#state));
    let final_type = tag_type(&tag, &final_states, final_child);
    let mut arguments: Vec<_> = props
        .iter()
        .map(|prop| {
            let ident = &prop.ident;
            match prop.kind {
                PropKind::Required(_) => quote!(self.#ident.0),
                PropKind::Optional(_) => quote!(self.#ident),
            }
        })
        .collect();
    if children {
        arguments.push(quote!(self.children));
    }

    let setters = props.iter().map(|prop| {
        let setter = &prop.ident;
        let input_ty = match &prop.kind {
            PropKind::Required(_) => &prop.ty,
            PropKind::Optional(inner) => inner.as_ref(),
        };
        let input = format_ident!("__ui_value", span = Span::mixed_site());
        let target_state = match &prop.kind {
            PropKind::Required(state) => Some(state),
            PropKind::Optional(_) => None,
        };
        let mut setter_generics: Vec<_> = required
            .iter()
            .filter(|(_, required_state)| {
                target_state
                    .as_ref()
                    .is_none_or(|state| *state != *required_state)
            })
            .map(|(_, state)| (*state).clone())
            .collect();
        if let Some(child_state) = &child_state {
            setter_generics.push(child_state.clone());
        }
        let self_states: Vec<_> = required
            .iter()
            .map(|(_, required_state)| {
                if target_state
                    .as_ref()
                    .is_some_and(|state| *state == *required_state)
                {
                    quote!(())
                } else {
                    quote!(#required_state)
                }
            })
            .collect();
        let result_states: Vec<_> = required
            .iter()
            .map(|(required_prop, required_state)| {
                if target_state
                    .as_ref()
                    .is_some_and(|state| *state == *required_state)
                {
                    let ty = &required_prop.ty;
                    quote!((#ty,))
                } else {
                    quote!(#required_state)
                }
            })
            .collect();
        let child_type = child_state.as_ref().map(|state| quote!(#state));
        let self_type = tag_type(&tag, &self_states, child_type.clone());
        let result_type = tag_type(&tag, &result_states, child_type);
        let mut fields: Vec<_> = props
            .iter()
            .map(|other| {
                let ident = &other.ident;
                if ident == setter {
                    match other.kind {
                        PropKind::Required(_) => quote!(#ident: (#input,)),
                        PropKind::Optional(_) => {
                            quote!(#ident: ::core::option::Option::Some(#input))
                        }
                    }
                } else {
                    quote!(#ident: self.#ident)
                }
            })
            .collect();
        if children {
            fields.push(quote!(children: self.children));
        }
        let impl_generics = if setter_generics.is_empty() {
            quote!()
        } else {
            quote!(<#(#setter_generics),*>)
        };
        quote! {
            impl #impl_generics #self_type {
                #[doc(hidden)]
                pub fn #setter(self, #input: #input_ty) -> #result_type {
                    #tag { #(#fields,)* }
                }
            }
        }
    });

    let children_impl = child_state.as_ref().map(|child_state| {
        let mut child_used_states = used_states.clone();
        child_used_states.push(child_state.clone());
        let child_source = unique_state_ident("__UIChildSource", &child_used_states);
        let mut impl_generics: Vec<_> = state_idents.iter().map(|state| quote!(#state)).collect();
        impl_generics.push(quote!(#child_state));
        let states: Vec<_> = state_idents.iter().map(|state| quote!(#state)).collect();
        let self_type = tag_type(&tag, &states, Some(quote!(#child_state)));
        let result_type = tag_type(&tag, &states, Some(quote!(#child_source)));
        let mut fields: Vec<_> = props
            .iter()
            .map(|prop| {
                let ident = &prop.ident;
                quote!(#ident: self.#ident)
            })
            .collect();
        fields.push(quote!(children: #children_ident));
        quote! {
            impl<#(#impl_generics),*> #self_type {
                #[doc(hidden)]
                pub fn #children_ident<#child_source>(self, #children_ident: #child_source) -> #result_type
                where
                    #child_source: ::core::iter::IntoIterator<Item = #return_type>,
                {
                    #tag { #(#fields,)* }
                }
            }
        }
    });

    let build_impl_generics = child_state
        .as_ref()
        .map(|state| quote!(<#state>))
        .unwrap_or_default();
    let build_where = child_state
        .as_ref()
        .map(|state| quote!(where #state: ::core::iter::IntoIterator<Item = #return_type>));

    quote! {
        #function

        #[doc(hidden)]
        #definition

        impl #initial_type {
            #[doc(hidden)]
            pub fn #new_ident() -> Self {
                Self { #(#initial_fields,)* }
            }
        }

        #(#setters)*

        #children_impl

        impl #build_impl_generics #final_type #build_where {
            #[doc(hidden)]
            pub fn #build_ident(self) -> #return_type {
                #function_name(#(#arguments),*)
            }
        }
    }
}

fn tag_type(tag: &Ident, states: &[TokenStream2], child: Option<TokenStream2>) -> TokenStream2 {
    let mut parameters: Vec<_> = states.to_vec();
    if let Some(child) = child {
        parameters.push(child);
    }
    if parameters.is_empty() {
        quote!(#tag)
    } else {
        quote!(#tag<#(#parameters),*>)
    }
}

fn declaration_idents(function: &ItemFn, tag: &Ident) -> syn::Result<Vec<Ident>> {
    #[derive(Default)]
    struct Names {
        idents: Vec<Ident>,
        macro_span: Option<Span>,
    }

    impl<'ast> Visit<'ast> for Names {
        fn visit_ident(&mut self, ident: &'ast Ident) {
            self.idents.push(ident.clone());
        }

        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            self.macro_span.get_or_insert_with(|| mac.span());
        }
    }

    let mut names = Names::default();
    names.idents.push(tag.clone());
    names.visit_signature(&function.sig);
    if let Some(span) = names.macro_span {
        return Err(Error::new(
            span,
            "ui_widget signature macros must be moved into a type alias or constant",
        ));
    }
    Ok(names.idents)
}

fn unique_state_ident(prefix: &str, used: &[Ident]) -> Ident {
    let mut suffix = 0;
    loop {
        let name = if suffix == 0 {
            prefix.to_owned()
        } else {
            format!("{prefix}{suffix}")
        };
        let ident = Ident::new(&name, Span::mixed_site());
        if !used.iter().any(|used| used.unraw() == ident.unraw()) {
            return ident;
        }
        suffix += 1;
    }
}
