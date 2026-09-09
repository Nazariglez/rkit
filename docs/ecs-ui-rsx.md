# ECS UI RSX authoring

Enable RSX authoring explicitly:

```toml
[dependencies]
rkit = { path = "../rkit", features = ["ecs-ui-rsx"] }
```

Import the macros from `ecs::ui`. `rsx!` constructs one owned `UIScene`, so spawning remains an ordinary explicit command.

```rust
use rkit::{
    ecs::ui::{rsx, ui, UIScene},
    gfx::Color,
};

fn menu(title: String, items: impl IntoIterator<Item = UIScene>) -> UIScene {
    rsx! {
        <column>
            <container bg_color={Color::rgb(0.1, 0.16, 0.28)}>
                <text>{title}</text>
                <column ui:children={items.into_iter()}/>
            </container>
        </column>
    }
}
```

RSX is a one-shot scene construction syntax. It is not reactive: expressions run when the scene is constructed, not when application state changes. Use normal ECS systems, components, and commands for later updates. Ordinary `ui` constructors and functions returning `UIScene` remain available and can be used in scene-expression holes such as `{make_scene()}`.

## Scoped entity links

Use `ui::with_entities` when one owned scene needs to store IDs for its own nodes in ordinary ECS components. The factory receives a short-lived `UIEntityScope`; `reserve()` creates an ID that exactly one node in that spawn invocation must claim with `.entity(id)` or `ui:entity={id}`. It is not an allocator for arbitrary ECS work, an existing-entity adoption API, or a persistent reference registry.

These ordinary Rust and RSX versions build the same root/child link. The factory captures `label` by value because it runs after `linked_button` returns.

```rust
use rkit::{
    ecs::ui::{ui, UIScene},
    prelude::{bevy_ecs, Component, Entity},
};

#[derive(Component)]
struct TextLink(Entity);

fn linked_button(label: String) -> UIScene {
    ui::with_entities(move |scope| {
        let text = scope.reserve();
        ui::node()
            .insert(TextLink(text))
            .child(ui::text(label).entity(text))
    })
}
```

```rust
use rkit::{
    ecs::ui::{rsx, ui, UIScene},
    prelude::{bevy_ecs, Component, Entity},
};

#[derive(Component)]
struct TextLink(Entity);

fn linked_button(label: String) -> UIScene {
    ui::with_entities(move |scope| {
        let text = scope.reserve();
        rsx! {
            <node ui:insert={TextLink(text)}>
                <text ui:entity={text}>{label}</text>
            </node>
        }
    })
}
```

The full `app_ecs_ui_rsx_entities` example uses this pattern for two clickable `LinkedButton` instances. Its click system follows a `TextLink` to update only that instance's `UIText`; it does not search the hierarchy or create repair entities.

### Allocation and construction timing

Calling `ui::with_entities` only stores the factory. Dropping that unspawned scene drops its captures and does not run the factory or reserve IDs. `spawn_ui` and `spawn_ui_children` expand factories synchronously while building their spawn plan, so the returned root IDs are known when those methods return. Materialization remains deferred until the command queue applies.

Factories run once, even if later plan validation rejects the spawn. Capture owned strings, scenes, sprites, and component inputs. A factory is `Send + 'static` but need not be `Sync`; the scope cannot escape its callback.

Borrowed widget props are construction-only. A `#[ui_widget]` function runs synchronously while its adapter builds the scene, so it may read local borrowed inputs and resolve owned scene data. The returned `UIScene` has no borrow lifetime and can outlive those locals. This does not relax deferred APIs: a widget must resolve owned values before passing them to `ui::with_entities`, `.style`, `.observe`, `.on_click`, or a component.

A widget whose children iterator is borrowed or otherwise unsuitable for the factory should consume it into an owned scene before capture:

```rust
fn framed(children: impl IntoIterator<Item = UIScene>) -> UIScene {
    let content = ui::column().children(children);
    ui::with_entities(move |scope| {
        let child = scope.reserve();
        ui::node().child(content.entity(child))
    })
}
```

Nested factories share one plan-local allocation ledger. A reservation is eligible only in the `spawn_ui` call, or complete `spawn_ui_children` batch, that expanded its factory. `.entity(id)` assigns a planned node; it neither spawns, adopts, reparents, nor modifies an arbitrary existing entity. IDs from another invocation, a normal `commands.spawn_empty`, or a stale entity are rejected.

### Root decoration and discovery

A scoped scene has the same root decorators as any other `UIScene`: `.insert`, `.style`, `.observe`, `.on_click`, `.child`, `.children`, and `.entity`. They decorate the factory's eventual root without adding a wrapper. Factory-root operations and children come before operations and children appended outside the factory. Consequently, inner style patches run before outer style patches when the deferred plan materializes.

RSX applies `ui:` controls to the scene returned by that tag. For a compound widget, `<LinkedButton ui:insert={Marker}/>` marks its returned root, not an implicit internal text node. Apply a marker or an application instance-key component to the actual internal node when outside systems must find it:

```rust
use rkit::prelude::{bevy_ecs, Component};

#[derive(Component)]
struct WidgetText;

#[derive(Component)]
struct WidgetInstance(u64);

// Inside the widget factory:
// <text ui:entity={text} ui:insert={(WidgetText, WidgetInstance(instance))}>{label}</text>
```

Those components are queryable after deferred commands apply. They are discovery mechanisms, not immediate ID exports: spawning returns only the scene root, and this API provides no scene result object, persistent UI reference, or synchronous descendant-ID export.

Assign `ui:entity` at most once per tag. An effective root may receive only one assignment across factory and outer decoration, even if both IDs are equal. Reusing an assigned ID for another node, assigning a foreign ID, or leaving a reservation unused rejects the whole plan.

### Rejection and panic limits

Scene validation occurs before the plan inserts scene bundles, styles, links nodes, or installs local observers. Missing layouts, invalid parents, runtime-owned component insertion, foreign or duplicate bindings, and unused reservations reject the plan and clean all IDs owned by that invocation; valid earlier roots in a rejected batch do not materialize. Rejection is reported as `UIRuntimeError::MissingLayout` or `UIRuntimeError::InvalidScene`, not returned by `spawn_ui`. A root ID returned before command application is therefore not proof that a live UI node exists.

A factory panic is not converted into a scene error and is not swallowed. The lowering cleanup path queues removal for IDs it had reserved before unwinding; if a caught panic's retained command queue is then applied, those owned empty entities do not remain. Do not rely on this for application recovery: factory side effects cannot be rolled back, and a command queue that is never applied does not materialize or clean deferred commands.

## Core tags

The only reserved unqualified tags are `node`, `row`, `column`, `container`, `text`, `rich_text`, and `image`. They use generated adapters over the existing `ui` constructors.

| Tag | Inputs | Children |
| --- | --- | --- |
| `node` | None | Yes |
| `row` | None; selects horizontal flex layout | Yes |
| `column` | None; selects vertical flex layout | Yes |
| `container` | Optional `bg_color: Color`, `border_color: Color`, `border_size: f32`, `corner_radius: f32` | Yes |
| `text` | One required braced text payload; optional text fields below | No |
| `rich_text` | Required `layout: RichTextLayout`; optional `shadow_color: Color`, `shadow_offset: Vec2` | Yes |
| `image` | Required `sprite: Sprite`; optional `tint: Color` | Yes |

`text` also accepts optional `font: Font`, `color: Color`, `size: f32`, `h_align: HAlign`, `line_height: f32`, `shadow_color: Color`, `shadow_offset: Vec2`, `color_tags: bool`, `outline_color: Color`, and `outline_width: u16`. Its payload implements `Into<String>` and is not a scene child.

All non-text core tags may be self-closing or use matching opening and closing tags. Nested content is either a sequence of tags and braced expressions that each yield a `UIScene` or `IntoIterator<Item = UIScene>`, or one `ui:children={iterator}` source. Do not combine the two forms. Nested expressions flatten one level only: `Option<UIScene>`, arrays, vectors, and mapped iterators contribute their owned scenes in order, while nested collections, references, strings, and unrelated values are rejected. Fixed nested content is eagerly collected in source order; an explicit `ui:children` iterator is passed to the existing `.children(...)` API without collection.

### Components, style, and defaults

Core attributes configure existing components; they are not a CSS or HTML vocabulary. `container` is one node with `UIContainer`, not a `div` or wrapper. `row` and `column` only select flex direction, so they have no appearance or layout attributes. Use a container with `ui:style` when both are needed.

Padding, gaps, dimensions, and alignment belong only in `ui:style`. In particular, `size` on `text` is font size, while layout size is `ui:style={|style| style.size(width, height)}`. Core `padding`, `gap`, `width`, and alignment attributes are rejected rather than becoming style shortcuts.

Omission preserves component-owned defaults: `UIContainer::default()`, `UIText::default()`, `UIImage::new(sprite)`, and `UIRichText::new(layout)`. Omitted container colors draw nothing. `rich_text` still requires a prepared `RichTextLayout`; RSX does not parse markup or convert strings into layouts.

Attributes always take braced Rust expressions without outer labels or attributes. RSX does not accept HTML attributes, quoted text nodes, fragments, or layout/style component fields. Generated widget setters declared with a recognized owned `String` type accept `Into<String>` inputs. Optional props accept either their inner value or an existing option; optional strings additionally require existing options to be exact `Option<String>` values.

## Reusable widgets

Annotate a free function returning `UIScene` with `#[ui_widget(Tag)]`. The function stays callable as normal Rust and the macro generates the explicitly named tag adapter beside it.

```rust
use rkit::{
    ecs::ui::{rsx, ui, ui_widget, widgets::UIContainer, UIScene},
    gfx::Color,
};

#[ui_widget(Panel)]
fn panel(
    title: String,
    style: Option<String>,
    subtitle: Option<String>,
    children: impl IntoIterator<Item = UIScene>,
) -> UIScene {
    let mut scene = ui::container(UIContainer {
        bg_color: Some(Color::rgb(0.16, 0.24, 0.38)),
        ..Default::default()
    })
    .child(ui::text(title));
    if style.as_deref() == Some("muted") {
        scene = scene.style(|style| style.opacity(0.7));
    }
    if let Some(subtitle) = subtitle {
        scene = scene.child(ui::text(subtitle));
    }
    scene.children(children)
}

fn screen(rows: impl IntoIterator<Item = UIScene>) -> UIScene {
    rsx! {
        <column>
            <Panel
                title={"Welcome"}
                style={"muted"}
                subtitle={"Optional strings accept borrowed inputs."}
                ui:style={|style| style.padding(12.0)}
            />
            <Panel title={"Configured"} ui:children={rows.into_iter()}/>
        </column>
    }
}
```

Tags are explicit Rust paths, not function-name lookup. A normal qualified path and an imported alias both work:

```rust
use rkit::ecs::ui::{rsx, ui, ui_widget, UIScene};

mod widgets {
    use super::*;

    #[ui_widget(Notice)]
    pub fn notice(message: String) -> UIScene {
        ui::text(message)
    }
}

use widgets::Notice as AppNotice;

fn qualified_notice() -> UIScene {
    rsx! { <widgets::Notice message={"Qualified"}/> }
}

fn aliased_notice() -> UIScene {
    rsx! { <AppNotice message={"Aliased"}/> }
}
```

The annotation supports synchronous, safe free functions with an explicit return type. Parameters must be simple identifier patterns with explicit field-compatible types. Construction props may contain shared or mutable references, including elided, named, or `'_` lifetimes, when the function resolves them into owned scene data before it returns. Lifetime-only generics and lifetime outlives bounds are supported; type and const generics, and type predicates in `where` clauses, remain rejected. Lifetimes bound inside function-pointer or callback types remain local to those types. Argument-position `impl Trait` is only supported for the final children parameter. Macro invocations in the signature must be moved into a type alias or constant so generated adapter parameters cannot capture names hidden inside the expansion. `async`, `const`, `unsafe`, ABI, variadic, receiver, destructuring, and parameter attributes are rejected. Put `cfg` or `cfg_attr` before `ui_widget`, or gate the containing module: a false conditional below `ui_widget` can make rustc remove the entire declaration before the macro runs, so it emits neither the function nor its adapter. The seven primitive names and parameter names beginning with `__ui_` are reserved.

```rust
use rkit::ecs::ui::{rsx, ui, ui_widget, UIScene};

#[ui_widget(BorrowedLabel)]
fn borrowed_label(text: &str) -> UIScene {
    ui::text(text)
}

fn screen() -> UIScene {
    let text = String::from("Built before `screen` returns");
    rsx! { <BorrowedLabel text={text.as_str()}/> }
}
```

`text` drops when `screen` returns, while the returned scene remains valid because `ui::text` owns the resolved string. The annotated function also remains directly callable with its original signature.

## Props and children

Every non-children function parameter is a bare named attribute. Required props must be supplied. The `Panel` example deliberately has a bare `style` prop beside `ui:style`: the first is passed to `panel`, while the second patches the returned scene root. Only a syntactic outer `Option<T>`, `std::option::Option<T>`, or `core::option::Option<T>` is optional. For a declared `color: Option<Color>`:

| RSX input | Function receives |
| --- | --- |
| Prop omitted | `None` |
| `color={color}` with `color: Color` | `Some(color)` |
| `color={Some(color)}` | `Some(color)` |
| `color={maybe_color}` with `maybe_color: Option<Color>` | `maybe_color` unchanged |
| `color={None}` | `None` |

Defaults remain in the function, for example `color.unwrap_or(default_color)`. Non-string optional setters convert once through `Into<Option<T>>`; this forwards an existing option instead of creating a nested option. Aliases of `Option` are required props because only the outer syntactic standard `Option` form is recognized.

A prop declared exactly as `String`, `std::string::String`, or `alloc::string::String` accepts an `Into<String>` input and converts it once in its generated setter. An optional syntactic `Option<String>` accepts the same bare inputs and an exact `Option<String>`, including inferred `None` and `Some(String)`. It does not convert optional elements: `Option<&str>` and `Option<Custom>` are not accepted for `Option<String>` even when their elements implement `Into<String>`; use `.map(str::to_owned)` or construct an `Option<String>`. Prefer string literals directly over a bare `.into()` expression when the conversion target would otherwise be ambiguous. Direct calls to the annotated function still require its declared `String` type.

Nested options are not flattened. For `color: Option<Option<Color>>`, `None::<Option<Color>>` is outer absence, while `None::<Color>` becomes `Some(None)`; an existing `Option<Option<Color>>` passes through unchanged. Bare `None` is ambiguous because both inputs are valid, so migrate it to an explicitly typed `None` rather than relying on an implicit meaning.

A final `children: impl IntoIterator<Item = UIScene>` parameter opts a widget into content. Omitting content passes an empty iterator; nested scenes and one-level nested scene collections, or `ui:children={items}`, attach the supplied source. The function decides where to consume that iterator. A widget without that final parameter is a leaf: a self-closing tag or empty pair is valid, while any nested child or explicit `ui:children={...}` is an error.

Mixed content does not need a grouping tag. A conditional scene and mapped buttons can sit directly between ordinary siblings:

```rust
fn action_list(hint: Option<String>, actions: impl IntoIterator<Item = String>) -> UIScene {
    rsx! {
        <column>
            <text>{"Each button shares one click observer."}</text>
            {hint.map(ui::text)}
            {actions.into_iter().map(ui::text)}
            <text>{"Click an action button"}</text>
        </column>
    }
}
```

An absent `hint` contributes no node. Use `ui:children={items}` instead when the widget must control when it consumes one iterator; it cannot mix with these nested siblings.

`ui:` is reserved for scene controls. Bare `children={...}` is rejected; `children` remains the declaration-only final content parameter. Names beginning with `__ui_` are reserved. Unknown bare props reach ordinary setter resolution; duplicate, missing, and wrongly typed props are ordinary compile errors at their use sites.

## Root controls and evaluation

`ui:` is the only control namespace. It has exactly these forms:

| Attribute | Meaning |
| --- | --- |
| `ui:insert={bundle}` | `.insert(bundle)` on the returned scene root |
| `ui:style={patch}` | `.style(patch)` on the returned scene root |
| `ui:observe={observer}` | `.observe(observer)` on the returned scene root |
| `ui:on_click={observer}` | `.on_click(observer)` on the returned scene root |
| `ui:entity={entity}` | `.entity(entity)` assigns the returned scene root to an exact `Entity` reserved by its spawn invocation |
| `ui:children={items}` | Explicit source for a child-taking widget |

The four root modifiers may repeat and are applied in written order. `ui:entity` and `ui:children` each appear at most once; `ui:children` cannot mix with nested content and is invalid on a leaf such as `text`. `ui:entity` is valid only for an ID reserved by the current spawn invocation, and an inner scoped root plus an outer assignment is a duplicate. Root modifiers target the scene returned by that element, not an internal widget child. They do not add pointer components, bubbling, or new observer rules. Bare names such as `style` and `entity` are widget props when the widget declares them.

Only an exact `ui:name` identifier pair is a control. Unknown or malformed namespaces are macro errors; `prop:name`, `p:name`, `:name`, and `$name` are unsupported. Raw identifiers are normalized for classification, so `r#ui:r#style` is the same control as `ui:style`.

For every tag, supplied bare prop expressions evaluate once in written order, then core text evaluates its payload, then the child source is constructed, then the implementation function runs once, and finally `ui:` controls, including `ui:entity`, evaluate and apply once in written order. Nested child content is eager and ordered: each expression evaluates once and appends all of its scenes before the next sibling evaluates. In the mixed action list above, the optional hint is resolved before the button iterator is fully consumed, and both finish before the final label is constructed. An absent optional scene appends nothing and creates no entity. An explicit `ui:children` iterator expression evaluates once and advances only when its consumer uses it. A `ui:style` closure expression is created during construction, but its patch body remains deferred by the existing scene API.

This is an intentional migration from the earlier primitive lowering order, where root modifiers preceded child construction. If side effects depended on that order, move them into named Rust locals or update them to the sequence above; RSX does not preserve the old primitive-specific order.

## Migration

The RSX authoring interface changed before release. Migrate old templates explicitly:

| Before | After |
| --- | --- |
| `<Panel prop:title={"Title".to_string()}/>` | `<Panel title={"Title"}/>` |
| `style={patch}` as a root control | `ui:style={patch}` |
| `insert`, `observe`, `on_click` root controls | `ui:insert`, `ui:observe`, `ui:on_click` |
| `children={items}` as a source | `ui:children={items}` |
| `<container props={UIContainer { bg_color: Some(color), ..Default::default() }}/>` | `<container bg_color={color}/>` |
| `<image props={UIImage::new(sprite)}/>` | `<image sprite={sprite}/>` |
| `<rich_text props={UIRichText::new(layout)}/>` | `<rich_text layout={layout}/>` |

There is no legacy props bag or alternate namespace. Component-valued expressions remain available through scene holes such as `{ui::container(component)}`.

## Diagnostics and formatting

RSX deliberately accepts a narrow Rust-oriented grammar. Typical diagnostics cover mismatched tags, multiple roots, non-braced attributes, unsupported or unknown `ui:` controls, mixing nested children with `ui:children={...}`, obsolete namespaces such as `prop:`, and text without exactly one braced payload. Widget declaration diagnostics cover unsupported signatures and reserved names. Rust still reports unresolved tag paths, missing required finalization, unknown setters, leaf child attachment, and all normal type and trait errors. Compiler wording is not part of the API.

`cargo fmt` formats surrounding Rust, but treats RSX input as opaque macro tokens: it can adjust the template's outer indentation without formatting expressions inside attributes or blocks. It does not provide RSX-aware wrapping or editor completion for tag names and bare attributes; keep complex templates readable with ordinary Rust helpers and use the builder API when it is clearer. No formatter or editor integration is bundled with this feature.
