use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream as TokenStream2, TokenTree};
use quote::{ToTokens, quote};
use rstml::node::{Node, NodeAttribute, NodeBlock, NodeElement, NodeName, NodeNameFragment};
use syn::{
    Error, Expr, ExprPath, Path, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    punctuated::Pair,
    spanned::Spanned,
};

use crate::ui_widget::{UI_BUILD, UI_CHILDREN, UI_NEW, is_reserved_name};

struct RsxInput {
    framework: Path,
    tokens: TokenStream2,
}

struct Modifier {
    method: Ident,
    argument: TokenStream2,
}

struct WidgetProp {
    setter: Ident,
    value: TokenStream2,
}

struct SpannedTokens {
    tokens: TokenStream2,
    span: Span,
}

#[derive(Default)]
struct ElementAttributes {
    children: Option<SpannedTokens>,
    entity: bool,
    modifiers: Vec<Modifier>,
    widget_props: Vec<WidgetProp>,
}

impl Parse for RsxInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let framework = input.parse()?;
        input.parse::<Token![;]>()?;
        let tokens = input.parse()?;
        Ok(Self { framework, tokens })
    }
}

pub fn expand(input: TokenStream) -> TokenStream {
    match syn::parse::<RsxInput>(input).and_then(lower) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn lower(input: RsxInput) -> syn::Result<TokenStream2> {
    let nodes = rstml::Parser::new(rstml::ParserConfig::new().number_of_top_level_nodes(1))
        .parse_simple(input.tokens)?;
    let [root] = nodes.as_slice() else {
        let span = nodes
            .first()
            .map(Spanned::span)
            .unwrap_or_else(proc_macro2::Span::call_site);
        return Err(Error::new(span, "rsx! requires exactly one scene root"));
    };

    let scene = lower_scene_node(root, &input.framework)?;
    let framework = input.framework;
    let result = Ident::new("__rkit_rsx_scene", proc_macro2::Span::mixed_site());
    Ok(quote! {{
        let #result: #framework::ecs::ui::UIScene = #scene;
        #result
    }})
}

fn lower_element(
    element: &NodeElement<rstml::node::Infallible>,
    framework: &Path,
) -> syn::Result<TokenStream2> {
    reject_generics(element)?;
    let tag = tag_path(element.name())?;
    let core_adapter = (tag.path.segments.len() == 1)
        .then(|| core_adapter_name(&tag.path.segments[0].ident))
        .flatten();
    let is_text = core_adapter == Some("Text");
    let mut attributes = element_attributes(element)?;

    let source = if is_text {
        if attributes.children.is_some() {
            return Err(Error::new_spanned(
                element.name(),
                "text does not accept ui:children",
            ));
        }
        if attributes
            .widget_props
            .iter()
            .any(|prop| prop.setter.unraw() == "text")
        {
            return Err(Error::new_spanned(
                element.name(),
                "text content belongs in its braced payload, not a text attribute",
            ));
        }
        attributes.widget_props.push(WidgetProp {
            setter: Ident::new("text", element.name().span()),
            value: text_payload(element)?,
        });
        None
    } else {
        child_source(element, attributes.children.take(), framework)?
    };

    let path = match core_adapter {
        Some(adapter) => {
            let adapter = Ident::new(adapter, Span::mixed_site());
            quote!(#framework::ecs::ui::rsx_widgets::#adapter)
        }
        None => quote!(#tag),
    };
    let tag_span = element.name().span();
    let new_ident = Ident::new(UI_NEW, tag_span);
    let build_ident = Ident::new(UI_BUILD, tag_span);
    let builder = Ident::new("__rkit_rsx_builder", proc_macro2::Span::mixed_site());
    let result = Ident::new("__rkit_rsx_scene", proc_macro2::Span::mixed_site());
    let setters = attributes.widget_props.iter().map(|prop| {
        let setter = &prop.setter;
        let value = &prop.value;
        quote!(let #builder = #builder.#setter(#value);)
    });
    let attach_children = source.map(|SpannedTokens { tokens, span }| {
        let children_ident = Ident::new(UI_CHILDREN, span);
        quote!(let #builder = #builder.#children_ident(#tokens);)
    });
    let scene = quote!({
        let #builder = #path::#new_ident();
        #(#setters)*
        #attach_children
        let #result: #framework::ecs::ui::UIScene = #builder.#build_ident();
        #result
    });
    Ok(apply_modifiers(scene, attributes.modifiers))
}

fn reject_generics(element: &NodeElement<rstml::node::Infallible>) -> syn::Result<()> {
    reject_tag_generics(&element.open_tag.generics)?;
    if let Some(close_tag) = &element.close_tag {
        reject_tag_generics(&close_tag.generics)?;
    }
    Ok(())
}

fn reject_tag_generics(generics: &syn::Generics) -> syn::Result<()> {
    if let Some(lt_token) = &generics.lt_token {
        return Err(Error::new(lt_token.span, "rsx! tags cannot have generics"));
    }
    if !generics.params.is_empty() || generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            generics,
            "rsx! tags cannot have generics",
        ));
    }
    Ok(())
}

fn tag_path(name: &NodeName) -> syn::Result<&ExprPath> {
    let NodeName::Path(path) = name else {
        return Err(Error::new_spanned(name, "rsx! tags must be Rust paths"));
    };
    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || path
            .path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
    {
        return Err(Error::new_spanned(
            name,
            "rsx! tags must be non-generic Rust paths",
        ));
    }
    Ok(path)
}

pub(crate) fn is_core_tag(ident: &Ident) -> bool {
    core_adapter_name(ident).is_some()
}

fn core_adapter_name(ident: &Ident) -> Option<&'static str> {
    match ident.unraw().to_string().as_str() {
        "node" => Some("Node"),
        "row" => Some("Row"),
        "column" => Some("Column"),
        "container" => Some("Container"),
        "text" => Some("Text"),
        "rich_text" => Some("RichText"),
        "image" => Some("Image"),
        _ => None,
    }
}

fn child_source(
    element: &NodeElement<rstml::node::Infallible>,
    explicit: Option<SpannedTokens>,
    framework: &Path,
) -> syn::Result<Option<SpannedTokens>> {
    let Some(first) = element.children.first() else {
        return Ok(explicit);
    };
    if explicit.is_some() {
        return Err(Error::new_spanned(
            first,
            "ui:children cannot be combined with nested children",
        ));
    }

    let children = element
        .children
        .iter()
        .map(|child| lower_scene_node(child, framework))
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(Some(SpannedTokens {
        tokens: quote!([#(#children),*]),
        span: first.span(),
    }))
}

fn apply_modifiers(mut scene: TokenStream2, modifiers: Vec<Modifier>) -> TokenStream2 {
    for Modifier { method, argument } in modifiers {
        scene = quote!(#scene.#method(#argument));
    }
    scene
}

fn element_attributes(
    element: &NodeElement<rstml::node::Infallible>,
) -> syn::Result<ElementAttributes> {
    let mut attributes = ElementAttributes::default();
    for attribute in element.attributes() {
        let NodeAttribute::Attribute(attribute) = attribute else {
            return Err(Error::new_spanned(
                attribute,
                "rsx! attributes must be named and braced",
            ));
        };
        let key = &attribute.key;
        let value = braced_attribute_value(attribute)?;
        if let Some(name) = plain_attribute_name(key) {
            if name.unraw() == "children" {
                return Err(Error::new_spanned(
                    name,
                    "use ui:children for an explicit child source",
                ));
            }
            if is_reserved_name(name) {
                return Err(Error::new_spanned(
                    name,
                    "widget property names cannot start with __ui_",
                ));
            }
            if attributes
                .widget_props
                .iter()
                .any(|prop| prop.setter.unraw() == name.unraw())
            {
                return Err(Error::new_spanned(name, "duplicate widget prop attribute"));
            }
            attributes.widget_props.push(WidgetProp {
                setter: name.clone(),
                value: value.tokens,
            });
            continue;
        }

        let Some(name) = control_attribute_name(key) else {
            return Err(Error::new_spanned(
                key,
                "unsupported rsx! attribute namespace",
            ));
        };
        match name.unraw().to_string().as_str() {
            "insert" | "style" | "observe" | "on_click" => attributes.modifiers.push(Modifier {
                method: name.clone(),
                argument: value.tokens,
            }),
            "entity" => {
                if std::mem::replace(&mut attributes.entity, true) {
                    return Err(Error::new_spanned(name, "duplicate ui:entity attribute"));
                }
                attributes.modifiers.push(Modifier {
                    method: Ident::new("entity", name.span()),
                    argument: value.tokens,
                });
            }
            "children" => {
                if attributes.children.replace(value).is_some() {
                    return Err(Error::new_spanned(name, "duplicate ui:children attribute"));
                }
            }
            _ => {
                return Err(Error::new_spanned(name, "unknown ui: control"));
            }
        }
    }
    Ok(attributes)
}

fn plain_attribute_name(name: &NodeName) -> Option<&Ident> {
    let NodeName::Path(path) = name else {
        return None;
    };
    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || path.path.segments.len() != 1
        || !matches!(path.path.segments[0].arguments, syn::PathArguments::None)
    {
        return None;
    }
    Some(&path.path.segments[0].ident)
}

fn control_attribute_name(name: &NodeName) -> Option<&Ident> {
    let NodeName::Punctuated(name) = name else {
        return None;
    };
    let mut pairs = name.pairs();
    let Pair::Punctuated(NodeNameFragment::Ident(namespace), punctuation) = pairs.next()? else {
        return None;
    };
    if namespace.unraw() != "ui" || punctuation.as_char() != ':' {
        return None;
    }
    let Pair::End(NodeNameFragment::Ident(name)) = pairs.next()? else {
        return None;
    };
    if pairs.next().is_some() {
        return None;
    }
    Some(name)
}

fn braced_attribute_value(attribute: &rstml::node::KeyedAttribute) -> syn::Result<SpannedTokens> {
    let Some(value) = attribute.value() else {
        return Err(Error::new_spanned(
            attribute,
            "rsx! attribute values must be braced Rust expressions",
        ));
    };
    let Expr::Block(block) = value else {
        return Err(Error::new_spanned(
            value,
            "rsx! attribute values must be braced Rust expressions",
        ));
    };
    if block.label.is_some() || !block.attrs.is_empty() {
        return Err(Error::new_spanned(
            block,
            "rsx! attribute values cannot use outer labels or attributes",
        ));
    }
    Ok(SpannedTokens {
        tokens: quote_block_expression(&block.block),
        span: block.block.span(),
    })
}

fn text_payload(element: &NodeElement<rstml::node::Infallible>) -> syn::Result<TokenStream2> {
    let [Node::Block(block)] = element.children.as_slice() else {
        return Err(Error::new_spanned(
            element.name(),
            "text requires exactly one braced Rust payload",
        ));
    };
    let Some(block) = block.try_block() else {
        return Err(Error::new_spanned(block, "text payload must be valid Rust"));
    };
    Ok(quote_block_expression(block))
}

fn lower_scene_node(node: &Node, framework: &Path) -> syn::Result<TokenStream2> {
    match node {
        Node::Element(element) => lower_element(element, framework),
        Node::Block(block) => lower_block_child(block),
        _ => Err(Error::new_spanned(
            node,
            "rsx! nodes must be elements or braced scene expressions",
        )),
    }
}

fn lower_block_child(block: &NodeBlock) -> syn::Result<TokenStream2> {
    let Some(block) = block.try_block() else {
        return Err(Error::new_spanned(
            block,
            "rsx! scene expression must be valid Rust",
        ));
    };
    Ok(quote_block_expression(block))
}

fn quote_block_expression(block: &syn::Block) -> TokenStream2 {
    let mut contents = TokenStream2::new();
    for statement in &block.stmts {
        statement.to_tokens(&mut contents);
    }
    let mut group = Group::new(Delimiter::Brace, contents);
    group.set_span(Span::mixed_site());
    TokenTree::Group(group).into()
}
