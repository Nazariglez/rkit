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

#[derive(Clone, Copy)]
enum Primitive {
    Node,
    Row,
    Column,
    Container,
    Text,
    RichText,
    Image,
}

enum Tag<'a> {
    Primitive(Primitive),
    Custom(&'a ExprPath),
}

struct Modifier {
    method: Ident,
    argument: TokenStream2,
}

struct WidgetProp {
    setter: Ident,
    value: TokenStream2,
}

struct AttributeValue {
    tokens: TokenStream2,
    span: proc_macro2::Span,
}

#[derive(Default)]
struct ElementAttributes {
    props: Option<AttributeValue>,
    children: Option<AttributeValue>,
    modifiers: Vec<Modifier>,
    widget_props: Vec<WidgetProp>,
}

enum ChildSource {
    None,
    Syntactic {
        children: Vec<TokenStream2>,
        span: proc_macro2::Span,
    },
    Explicit(AttributeValue),
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
    let tag = classify_tag(element.name())?;
    let attributes = element_attributes(element, &tag)?;
    match tag {
        Tag::Primitive(primitive) => lower_primitive(element, primitive, attributes, framework),
        Tag::Custom(path) => lower_custom(element, path, attributes, framework),
    }
}

fn lower_primitive(
    element: &NodeElement<rstml::node::Infallible>,
    primitive: Primitive,
    attributes: ElementAttributes,
    framework: &Path,
) -> syn::Result<TokenStream2> {
    let ElementAttributes {
        props,
        children,
        modifiers,
        widget_props: _,
    } = attributes;

    if matches!(primitive, Primitive::Text) && children.is_some() {
        return Err(Error::new_spanned(
            element.name(),
            "text does not accept a children attribute",
        ));
    }

    let scene = match primitive {
        Primitive::Node | Primitive::Row | Primitive::Column if props.is_some() => {
            return Err(Error::new_spanned(
                element.name(),
                "this primitive does not accept a props attribute",
            ));
        }
        Primitive::Node => quote!(#framework::ecs::ui::ui::node()),
        Primitive::Row => quote!(#framework::ecs::ui::ui::row()),
        Primitive::Column => quote!(#framework::ecs::ui::ui::column()),
        Primitive::Container => {
            let input = props.map_or_else(
                || quote!(#framework::ecs::ui::widgets::UIContainer::default()),
                |props| props.tokens,
            );
            quote!(#framework::ecs::ui::ui::container(#input))
        }
        Primitive::Text => {
            if props.is_some() {
                return Err(Error::new_spanned(
                    element.name(),
                    "text does not accept a props attribute",
                ));
            }
            let payload = text_payload(element)?;
            quote!(#framework::ecs::ui::ui::text(#payload))
        }
        Primitive::RichText => {
            let Some(input) = props else {
                return Err(Error::new_spanned(
                    element.name(),
                    "rich_text requires props={UIRichText expression}",
                ));
            };
            let input = input.tokens;
            quote!(#framework::ecs::ui::ui::rich_text(#input))
        }
        Primitive::Image => {
            let Some(input) = props else {
                return Err(Error::new_spanned(
                    element.name(),
                    "image requires props={UIImage expression}",
                ));
            };
            let input = input.tokens;
            quote!(#framework::ecs::ui::ui::image(#input))
        }
    };

    let scene = apply_modifiers(scene, modifiers);
    if matches!(primitive, Primitive::Text) {
        return Ok(scene);
    }

    Ok(apply_primitive_child_source(
        scene,
        child_source(element, children, framework)?,
    ))
}

fn lower_custom(
    element: &NodeElement<rstml::node::Infallible>,
    path: &ExprPath,
    attributes: ElementAttributes,
    framework: &Path,
) -> syn::Result<TokenStream2> {
    let ElementAttributes {
        props: _,
        children,
        modifiers,
        widget_props,
    } = attributes;
    let source = child_source(element, children, framework)?;
    let tag_span = element.name().span();
    let new_ident = Ident::new(UI_NEW, tag_span);
    let build_ident = Ident::new(UI_BUILD, tag_span);
    let builder = Ident::new("__rkit_rsx_builder", proc_macro2::Span::mixed_site());
    let result = Ident::new("__rkit_rsx_scene", proc_macro2::Span::mixed_site());
    let setters = widget_props.iter().map(|prop| {
        let setter = &prop.setter;
        let value = &prop.value;
        quote!(let #builder = #builder.#setter(#value);)
    });
    let attach_children = match source {
        ChildSource::None => quote!(),
        ChildSource::Syntactic { children, span } => {
            let children_ident = Ident::new(UI_CHILDREN, span);
            quote!(let #builder = #builder.#children_ident([#(#children),*]);)
        }
        ChildSource::Explicit(AttributeValue { tokens, span }) => {
            let children_ident = Ident::new(UI_CHILDREN, span);
            quote!(let #builder = #builder.#children_ident(#tokens);)
        }
    };
    let scene = quote!({
        let #builder = #path::#new_ident();
        #(#setters)*
        #attach_children
        let #result: #framework::ecs::ui::UIScene = #builder.#build_ident();
        #result
    });
    Ok(apply_modifiers(scene, modifiers))
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

fn classify_tag(name: &NodeName) -> syn::Result<Tag<'_>> {
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
    if path.path.segments.len() == 1 {
        let ident = &path.path.segments[0].ident;
        if let Some(primitive) = primitive_from_ident(ident) {
            return Ok(Tag::Primitive(primitive));
        }
    }
    Ok(Tag::Custom(path))
}

pub(crate) fn is_primitive_tag(ident: &Ident) -> bool {
    primitive_from_ident(ident).is_some()
}

fn primitive_from_ident(ident: &Ident) -> Option<Primitive> {
    match ident.unraw().to_string().as_str() {
        "node" => Some(Primitive::Node),
        "row" => Some(Primitive::Row),
        "column" => Some(Primitive::Column),
        "container" => Some(Primitive::Container),
        "text" => Some(Primitive::Text),
        "rich_text" => Some(Primitive::RichText),
        "image" => Some(Primitive::Image),
        _ => None,
    }
}

fn child_source(
    element: &NodeElement<rstml::node::Infallible>,
    explicit: Option<AttributeValue>,
    framework: &Path,
) -> syn::Result<ChildSource> {
    if let Some(children) = explicit {
        if let Some(child) = element.children.first() {
            return Err(Error::new_spanned(
                child,
                "children attribute cannot be combined with nested children",
            ));
        }
        return Ok(ChildSource::Explicit(children));
    }

    let mut children = Vec::with_capacity(element.children.len());
    let mut span = None;
    for child in &element.children {
        span.get_or_insert_with(|| child.span());
        children.push(lower_scene_node(child, framework)?);
    }
    match span {
        Some(span) => Ok(ChildSource::Syntactic { children, span }),
        None => Ok(ChildSource::None),
    }
}

fn apply_primitive_child_source(scene: TokenStream2, source: ChildSource) -> TokenStream2 {
    match source {
        ChildSource::None => scene,
        ChildSource::Syntactic { children, .. } => children
            .into_iter()
            .fold(scene, |scene, child| quote!(#scene.child(#child))),
        ChildSource::Explicit(AttributeValue { tokens, .. }) => quote!(#scene.children(#tokens)),
    }
}

fn apply_modifiers(mut scene: TokenStream2, modifiers: Vec<Modifier>) -> TokenStream2 {
    for Modifier { method, argument } in modifiers {
        scene = quote!(#scene.#method(#argument));
    }
    scene
}

fn element_attributes(
    element: &NodeElement<rstml::node::Infallible>,
    tag: &Tag<'_>,
) -> syn::Result<ElementAttributes> {
    let custom = matches!(tag, Tag::Custom(_));
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
            match name.unraw().to_string().as_str() {
                "props" if custom => {
                    return Err(Error::new_spanned(
                        key,
                        "custom widgets use prop:name attributes instead of props",
                    ));
                }
                "props" | "children" => {
                    let slot = if name.unraw() == "props" {
                        &mut attributes.props
                    } else {
                        &mut attributes.children
                    };
                    if slot.replace(value).is_some() {
                        return Err(Error::new_spanned(
                            key,
                            format!("duplicate {} attribute", name.unraw()),
                        ));
                    }
                }
                "insert" | "style" | "observe" | "on_click" => {
                    attributes.modifiers.push(Modifier {
                        method: name.clone(),
                        argument: value.tokens,
                    });
                }
                _ => {
                    let message = if custom {
                        "custom widget properties must use prop:name"
                    } else {
                        "unsupported primitive attribute"
                    };
                    return Err(Error::new_spanned(key, message));
                }
            }
            continue;
        }
        if !custom {
            return Err(Error::new_spanned(
                key,
                "primitive attributes cannot use namespaces",
            ));
        }
        let Some(name) = prop_attribute_name(key) else {
            return Err(Error::new_spanned(
                key,
                "unsupported rsx! attribute namespace",
            ));
        };
        if name.unraw() == "children" {
            return Err(Error::new_spanned(
                name,
                "children is a structural attribute, not a widget prop",
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

fn prop_attribute_name(name: &NodeName) -> Option<&Ident> {
    let NodeName::Punctuated(name) = name else {
        return None;
    };
    let mut pairs = name.pairs();
    let Pair::Punctuated(NodeNameFragment::Ident(namespace), punctuation) = pairs.next()? else {
        return None;
    };
    if namespace.unraw() != "prop" || punctuation.as_char() != ':' {
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

fn braced_attribute_value(attribute: &rstml::node::KeyedAttribute) -> syn::Result<AttributeValue> {
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
    Ok(AttributeValue {
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
