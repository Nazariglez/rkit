use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    Error, FnArg, GenericParam, ItemFn, Lifetime, LifetimeParam, Pat, PatIdent, ReturnType, Type,
    TypeArray, TypeGroup, TypeParen, TypePath, TypePtr, TypeReference, TypeSlice, WherePredicate,
    ext::IdentExt, spanned::Spanned, visit::Visit, visit_mut::VisitMut,
};

pub(crate) const UI_NEW: &str = "__ui_new";
pub(crate) const UI_CHILDREN: &str = "__ui_children";
pub(crate) const UI_BUILD: &str = "__ui_build";
pub(crate) const RESERVED_PREFIX: &str = "__ui_";

struct WidgetSignature {
    props: Vec<Prop>,
    children: bool,
    lifetimes: Vec<LifetimeParam>,
    concrete_field_lifetimes: Vec<Ident>,
    lifetime_where: Vec<WherePredicate>,
}

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
    if crate::rsx::is_core_tag(&tag) {
        return Err(Error::new_spanned(
            tag,
            "ui_widget tag is a reserved core name",
        ));
    }

    let function = syn::parse2::<ItemFn>(item)?;
    let mut used_states = declaration_idents(&function, &tag)?;
    let signature = validate_signature(&function, &mut used_states)?;
    Ok(generate(function, tag, signature, used_states))
}

fn validate_signature(
    function: &ItemFn,
    used_states: &mut Vec<Ident>,
) -> syn::Result<WidgetSignature> {
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
    let (mut lifetimes, lifetime_where) = validate_lifetime_generics(&signature.generics)?;
    let mut normalizer = LifetimeNormalizer::new(signature);
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

        let mut ty = (*argument.ty).clone();
        validate_prop_type(&ty)?;
        let concrete_field = standard_option_inner(&ty).is_some();
        normalizer.visit_adapter_type(&mut ty, concrete_field);
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
    let (normalized_lifetimes, concrete_field_lifetimes) = normalizer.into_parts();
    lifetimes.extend(normalized_lifetimes);
    Ok(WidgetSignature {
        props,
        children,
        lifetimes,
        concrete_field_lifetimes,
        lifetime_where,
    })
}

fn validate_lifetime_generics(
    generics: &syn::Generics,
) -> syn::Result<(Vec<LifetimeParam>, Vec<WherePredicate>)> {
    let mut lifetimes = Vec::new();
    for parameter in &generics.params {
        let GenericParam::Lifetime(lifetime) = parameter else {
            return Err(Error::new_spanned(
                parameter,
                "ui_widget functions may only have lifetime generics",
            ));
        };
        lifetimes.push(lifetime.clone());
    }

    let mut lifetime_where = Vec::new();
    if let Some(where_clause) = &generics.where_clause {
        for predicate in &where_clause.predicates {
            if !matches!(predicate, WherePredicate::Lifetime(_)) {
                return Err(Error::new_spanned(
                    predicate,
                    "ui_widget function where clauses may only contain lifetime bounds",
                ));
            }
            lifetime_where.push(predicate.clone());
        }
    }
    Ok((lifetimes, lifetime_where))
}

struct LifetimeNormalizer {
    used: Vec<Ident>,
    lifetimes: Vec<LifetimeParam>,
    concrete_field_lifetimes: Vec<Ident>,
    bound_depth: usize,
    collecting_field_lifetimes: bool,
}

impl LifetimeNormalizer {
    fn new(signature: &syn::Signature) -> Self {
        #[derive(Default)]
        struct Names {
            idents: Vec<Ident>,
        }

        impl<'ast> Visit<'ast> for Names {
            fn visit_lifetime(&mut self, lifetime: &'ast Lifetime) {
                if lifetime.ident != "_" {
                    self.idents.push(lifetime.ident.clone());
                }
            }
        }

        let mut names = Names::default();
        names.visit_signature(signature);
        Self {
            used: names.idents,
            lifetimes: Vec::new(),
            concrete_field_lifetimes: Vec::new(),
            bound_depth: 0,
            collecting_field_lifetimes: false,
        }
    }

    fn into_parts(self) -> (Vec<LifetimeParam>, Vec<Ident>) {
        (self.lifetimes, self.concrete_field_lifetimes)
    }

    fn visit_adapter_type(&mut self, ty: &mut Type, concrete_field: bool) {
        let was_collecting =
            std::mem::replace(&mut self.collecting_field_lifetimes, concrete_field);
        self.visit_type_mut(ty);
        self.collecting_field_lifetimes = was_collecting;
    }

    fn next_lifetime(&mut self) -> Lifetime {
        let ident = unique_state_ident("__ui_lifetime", &self.used);
        self.used.push(ident.clone());
        let lifetime = Lifetime::new(&format!("'{ident}"), Span::mixed_site());
        self.lifetimes.push(syn::parse_quote!(#lifetime));
        lifetime
    }

    fn normalize_lifetime(&mut self, lifetime: &mut Lifetime) {
        if lifetime.ident == "_" {
            if self.bound_depth != 0 {
                return;
            }
            *lifetime = self.next_lifetime();
        }
        if self.collecting_field_lifetimes
            && !self
                .concrete_field_lifetimes
                .iter()
                .any(|used| used.unraw() == lifetime.ident.unraw())
        {
            self.concrete_field_lifetimes.push(lifetime.ident.clone());
        }
    }
}

impl VisitMut for LifetimeNormalizer {
    fn visit_type_bare_fn_mut(&mut self, function: &mut syn::TypeBareFn) {
        self.bound_depth += 1;
        syn::visit_mut::visit_type_bare_fn_mut(self, function);
        self.bound_depth -= 1;
    }

    fn visit_trait_bound_mut(&mut self, bound: &mut syn::TraitBound) {
        let callback = bound.path.segments.last().is_some_and(|segment| {
            matches!(&segment.arguments, syn::PathArguments::Parenthesized(_))
        });
        if callback {
            self.bound_depth += 1;
        }
        syn::visit_mut::visit_trait_bound_mut(self, bound);
        if callback {
            self.bound_depth -= 1;
        }
    }

    fn visit_bound_lifetimes_mut(&mut self, lifetimes: &mut syn::BoundLifetimes) {
        self.bound_depth += 1;
        syn::visit_mut::visit_bound_lifetimes_mut(self, lifetimes);
        self.bound_depth -= 1;
    }

    fn visit_type_reference_mut(&mut self, reference: &mut TypeReference) {
        match &mut reference.lifetime {
            Some(lifetime) => self.normalize_lifetime(lifetime),
            None if self.bound_depth == 0 => {
                let mut lifetime = self.next_lifetime();
                self.normalize_lifetime(&mut lifetime);
                reference.lifetime = Some(lifetime);
            }
            None => {}
        }
        syn::visit_mut::visit_type_reference_mut(self, reference);
    }

    fn visit_generic_argument_mut(&mut self, argument: &mut syn::GenericArgument) {
        if let syn::GenericArgument::Lifetime(lifetime) = argument {
            self.normalize_lifetime(lifetime);
        }
        syn::visit_mut::visit_generic_argument_mut(self, argument);
    }

    fn visit_type_param_bound_mut(&mut self, bound: &mut syn::TypeParamBound) {
        if let syn::TypeParamBound::Lifetime(lifetime) = bound {
            self.normalize_lifetime(lifetime);
        }
        syn::visit_mut::visit_type_param_bound_mut(self, bound);
    }

    fn visit_expr_mut(&mut self, _: &mut syn::Expr) {}
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
        Type::Reference(TypeReference { elem, .. }) => validate_prop_type(elem),
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

fn is_standard_string(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    if path.qself.is_some() {
        return false;
    }
    let segments = &path.path.segments;
    let matches_path = match segments.len() {
        1 => path.path.leading_colon.is_none() && segments[0].ident.unraw() == "String",
        3 => {
            (segments[0].ident.unraw() == "std" || segments[0].ident.unraw() == "alloc")
                && segments[1].ident.unraw() == "string"
                && segments[2].ident.unraw() == "String"
        }
        _ => false,
    };
    matches_path
        && segments
            .iter()
            .all(|segment| matches!(segment.arguments, syn::PathArguments::None))
}

fn generate(
    function: ItemFn,
    tag: Ident,
    signature: WidgetSignature,
    used_states: Vec<Ident>,
) -> TokenStream2 {
    let WidgetSignature {
        props,
        children,
        lifetimes,
        concrete_field_lifetimes,
        lifetime_where,
    } = signature;
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
    let has_optional_string = props
        .iter()
        .any(|prop| matches!(&prop.kind, PropKind::Optional(inner) if is_standard_string(inner)));
    let mut helper_names = used_states.clone();
    helper_names.extend(
        lifetimes
            .iter()
            .map(|lifetime| lifetime.lifetime.ident.clone()),
    );
    if let Some(child_state) = &child_state {
        helper_names.push(child_state.clone());
    }
    let optional_string_trait = unique_state_ident("__UIOptionalString", &helper_names);
    helper_names.push(optional_string_trait.clone());
    let optional_string_kind = unique_state_ident("__UIStringKind", &helper_names);
    helper_names.push(optional_string_kind.clone());
    let optional_string_bare = unique_state_ident("__UIStringBare", &helper_names);
    helper_names.push(optional_string_bare.clone());
    let optional_string_option = unique_state_ident("__UIStringOption", &helper_names);

    let lifetime_names: Vec<_> = lifetimes
        .iter()
        .map(|lifetime| &lifetime.lifetime)
        .collect();
    let lifetime_witness_names: Vec<_> = lifetime_names
        .iter()
        .copied()
        .filter(|lifetime| {
            !concrete_field_lifetimes
                .iter()
                .any(|used| used.unraw() == lifetime.ident.unraw())
        })
        .collect();
    let lifetime_witness = (!lifetime_witness_names.is_empty()).then(|| {
        quote!(__ui_lifetimes: ::core::marker::PhantomData<fn() -> (#(&#lifetime_witness_names (),)* )>)
    });
    let initial_lifetime_witness = lifetime_witness
        .is_some()
        .then(|| quote!(__ui_lifetimes: ::core::marker::PhantomData,));
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
    if let Some(lifetime_witness) = &lifetime_witness {
        struct_fields.push(lifetime_witness.clone());
    }
    let mut generic_params: Vec<_> = lifetimes.iter().map(|lifetime| quote!(#lifetime)).collect();
    generic_params.extend(state_idents.iter().map(|state| quote!(#state)));
    if let Some(child_state) = &child_state {
        generic_params.push(quote!(#child_state));
    }
    let lifetime_where_clause =
        (!lifetime_where.is_empty()).then(|| quote!(where #(#lifetime_where),*));
    let definition = if generic_params.is_empty() {
        quote!(#visibility struct #tag { #(#struct_fields,)* })
    } else {
        quote!(
            #visibility struct #tag<#(#generic_params),*> #lifetime_where_clause {
                #(#struct_fields,)*
            }
        )
    };

    let initial_states: Vec<_> = required.iter().map(|_| quote!(())).collect();
    let initial_child = children.then(|| quote!(::core::iter::Empty<#return_type>));
    let initial_type = tag_type(&tag, &lifetime_names, &initial_states, initial_child);
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
    let final_type = tag_type(&tag, &lifetime_names, &final_states, final_child);
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
        let string_input = is_standard_string(input_ty);
        let optional_input = matches!(&prop.kind, PropKind::Optional(_));
        let optional_string_input = optional_input && string_input;
        let target_state = match &prop.kind {
            PropKind::Required(state) => Some(state),
            PropKind::Optional(_) => None,
        };
        let mut setter_generics: Vec<_> =
            lifetimes.iter().map(|lifetime| quote!(#lifetime)).collect();
        setter_generics.extend(
            required
                .iter()
                .filter(|(_, required_state)| {
                    target_state
                        .as_ref()
                        .is_none_or(|state| *state != *required_state)
                })
                .map(|(_, state)| quote!(#state)),
        );
        if let Some(child_state) = &child_state {
            setter_generics.push(quote!(#child_state));
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
        let self_type = tag_type(&tag, &lifetime_names, &self_states, child_type.clone());
        let result_type = tag_type(&tag, &lifetime_names, &result_states, child_type);
        let mut fields: Vec<_> = props
            .iter()
            .map(|other| {
                let ident = &other.ident;
                if ident == setter {
                    match other.kind {
                        PropKind::Required(_) => quote!(#ident: (#input,)),
                        PropKind::Optional(_) => quote!(#ident: #input)
                    }
                } else {
                    quote!(#ident: self.#ident)
                }
            })
            .collect();
        if children {
            fields.push(quote!(children: self.children));
        }
        if lifetime_witness.is_some() {
            fields.push(quote!(__ui_lifetimes: self.__ui_lifetimes));
        }
        let impl_generics = if setter_generics.is_empty() {
            quote!()
        } else {
            quote!(<#(#setter_generics),*>)
        };
        let method_input = if optional_string_input {
            quote!(#input: impl #optional_string_trait<#optional_string_kind>)
        } else if string_input {
            quote!(#input: impl ::core::convert::Into<#input_ty>)
        } else if optional_input {
            quote!(#input: impl ::core::convert::Into<::core::option::Option<#input_ty>>)
        } else {
            quote!(#input: #input_ty)
        };
        let conversion = if optional_string_input {
            quote!(
                let #input: ::core::option::Option<#input_ty> =
                    <_ as #optional_string_trait<#optional_string_kind>>::__ui_optional_string(#input);
            )
        } else if string_input {
            quote!(let #input: #input_ty = #input.into();)
        } else if optional_input {
            quote!(let #input: ::core::option::Option<#input_ty> = #input.into();)
        } else {
            quote!()
        };
        let method_generics = optional_string_input.then(|| quote!(<#optional_string_kind>));
        quote! {
            impl #impl_generics #self_type #lifetime_where_clause {
                #[doc(hidden)]
                pub fn #setter #method_generics(self, #method_input) -> #result_type {
                    #conversion
                    #tag { #(#fields,)* }
                }
            }
        }
    });
    let setters = if has_optional_string {
        quote! {
            const _: () = {
                #[doc(hidden)]
                pub enum #optional_string_bare {}

                #[doc(hidden)]
                pub enum #optional_string_option {}

                #[doc(hidden)]
                pub trait #optional_string_trait<#optional_string_kind> {
                    fn __ui_optional_string(self) -> ::core::option::Option<::std::string::String>;
                }

                impl<#optional_string_kind: ::core::convert::Into<::std::string::String>>
                    #optional_string_trait<#optional_string_bare> for #optional_string_kind
                {
                    fn __ui_optional_string(self) -> ::core::option::Option<::std::string::String> {
                        ::core::option::Option::Some(self.into())
                    }
                }

                impl #optional_string_trait<#optional_string_option>
                    for ::core::option::Option<::std::string::String>
                {
                    fn __ui_optional_string(self) -> ::core::option::Option<::std::string::String> {
                        self
                    }
                }

                #(#setters)*
            };
        }
    } else {
        quote!(#(#setters)*)
    };

    let children_impl = child_state.as_ref().map(|child_state| {
        let mut child_used_states = used_states.clone();
        child_used_states.push(child_state.clone());
        let child_source = unique_state_ident("__UIChildSource", &child_used_states);
        let mut impl_generics: Vec<_> = lifetimes.iter().map(|lifetime| quote!(#lifetime)).collect();
        impl_generics.extend(state_idents.iter().map(|state| quote!(#state)));
        impl_generics.push(quote!(#child_state));
        let states: Vec<_> = state_idents.iter().map(|state| quote!(#state)).collect();
        let self_type = tag_type(&tag, &lifetime_names, &states, Some(quote!(#child_state)));
        let result_type = tag_type(&tag, &lifetime_names, &states, Some(quote!(#child_source)));
        let mut fields: Vec<_> = props
            .iter()
            .map(|prop| {
                let ident = &prop.ident;
                quote!(#ident: self.#ident)
            })
            .collect();
        fields.push(quote!(children: #children_ident));
        if lifetime_witness.is_some() {
            fields.push(quote!(__ui_lifetimes: self.__ui_lifetimes));
        }
        quote! {
            impl<#(#impl_generics),*> #self_type #lifetime_where_clause {
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

    let mut build_generics: Vec<_> = lifetimes.iter().map(|lifetime| quote!(#lifetime)).collect();
    if let Some(child_state) = &child_state {
        build_generics.push(quote!(#child_state));
    }
    let build_impl_generics = if build_generics.is_empty() {
        quote!()
    } else {
        quote!(<#(#build_generics),*>)
    };
    let build_where = match child_state.as_ref() {
        Some(state) => quote!(
            where
                #(#lifetime_where,)*
                #state: ::core::iter::IntoIterator<Item = #return_type>
        ),
        None => lifetime_where_clause.clone().unwrap_or_default(),
    };
    let initial_impl_generics = if lifetimes.is_empty() {
        quote!()
    } else {
        quote!(<#(#lifetimes),*>)
    };

    quote! {
        #function

        #[doc(hidden)]
        #definition

        impl #initial_impl_generics #initial_type #lifetime_where_clause {
            #[doc(hidden)]
            pub fn #new_ident() -> Self {
                Self {
                    #(#initial_fields,)*
                    #initial_lifetime_witness
                }
            }
        }

        #setters

        #children_impl

        impl #build_impl_generics #final_type #build_where {
            #[doc(hidden)]
            pub fn #build_ident(self) -> #return_type {
                #function_name(#(#arguments),*)
            }
        }
    }
}

fn tag_type(
    tag: &Ident,
    lifetimes: &[&Lifetime],
    states: &[TokenStream2],
    child: Option<TokenStream2>,
) -> TokenStream2 {
    let mut parameters: Vec<_> = lifetimes.iter().map(|lifetime| quote!(#lifetime)).collect();
    parameters.extend(states.iter().cloned());
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
