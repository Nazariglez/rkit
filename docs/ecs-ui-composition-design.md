# ECS UI composition ergonomics

## Goal

Remove three construction-time restrictions from RSX without changing the owned `UIScene` model:

1. Allow widgets to borrow resources while constructing a scene.
2. Allow a nested child expression to contribute a scene, optional scene, or iterator of scenes alongside other children.
3. Allow optional props to receive either their inner value or an existing `Option<T>`.

These are authoring improvements, not a new UI runtime. Keep ordinary Rust functions, explicit spawning, scoped entity allocation, and ECS-driven updates.

## Scope and current facts

Baseline: `0271e87` (`simplified rsx`). Sessions 1.1 through 4.2 are implemented and accepted following final review. The plan is self-contained and changes RKit only. No game migration or earlier design-document phase is a prerequisite.

`docs/ecs-ui-rsx.md` is existing untracked user content. Update relevant sections in place during implementation; do not replace or stage it. Preserve unrelated working-tree changes.

| Owner | Current behavior |
| --- | --- |
| `crates/macros/src/ui_widget.rs` | Generates transient lifetime-aware type-state adapters for lifetime-only generic signatures and construction borrows, invokes the original function synchronously, and leaves that function directly callable. |
| `ui_widget.rs` prop classification | Recognizes a syntactic outer standard `Option<T>` as optional. Omission passes `None`; non-string setters accept an inner value or `Option<T>`, while optional strings additionally accept bare `Into<String>` inputs and exact `Option<String>`. |
| `crates/macros/src/rsx.rs` | Uses one primitive/custom-widget lowering path. Nested child expressions eagerly flatten one level into owned scenes. `ui:children` remains the exclusive lazy single-source form. |
| `src/ecs/ui/rsx_widgets.rs` | Houses the feature-gated, doc-hidden primitive adapters. Primitive defaults come from existing component defaults, not an independent RSX default model. |
| `src/ecs/ui/experimental/scene.rs` | Owns child scenes. `.children(...)` already consumes `IntoIterator<Item = UIScene>`, including optional scenes and lazy iterators. Deferred operations and factories retain their existing ownership requirements. |
| `examples/app_ecs_ui_rsx.rs` | Demonstrates custom `Panel`/`Label` widgets, optional color, and a list of action buttons. It is the primary example to extend. |

## Non-goals

- Prop-default or conversion annotations such as `#[prop(default)]` or `#[prop(into)]`.
- New style controls, CSS-like syntax, semantic buttons, scrollbars, or focus/navigation systems.
- Virtual DOM, reconciliation, hooks, automatic state subscriptions, or reactive expressions.
- Lifetimes on `UIScene`, borrowed ECS components, relaxed observer/factory lifetimes, or implicit resource cloning.
- Generic widget type/const parameters or general `impl Trait` props.
- Recursive collection flattening, collection-valued top-level RSX, or wrapper entities for absent/list content.
- A formatter project, new permanent test suites, or changes to scoped allocation and spawn validation.

## User-facing contract

The forms below describe the implemented and accepted API.

### 1. Borrowed construction props

A widget may borrow inputs, resolve owned text or handles, and return a scene independent of those borrows:

```rust
use rkit::ecs::ui::{UIScene, rsx, ui, ui_widget};

struct Locales {
    wallet: String,
}

#[ui_widget(Wallet)]
fn wallet(locales: &Locales) -> UIScene {
    ui::text(locales.wallet.as_str())
}

fn screen() -> UIScene {
    let locales = Locales { wallet: String::from("Gold") };
    rsx! { <Wallet locales={&locales}/> }
}
```

`screen` may return before its scene is spawned. `wallet` copies only the text it needs; it does not clone the resource or store its reference.

Rules:

- Support shared and mutable construction references, elided or explicitly named. Apply the same rule inside supported field-compatible types, including `Option<&T>`, slices, and tuples.
- Support lifetime-only function generics and lifetime-outlives bounds. Type/const generics and type predicates in `where` clauses remain unsupported.
- Separate elided input borrows must not be rewritten to `'static` or unnecessarily tied to one another. Preserve explicit relationships chosen by the function author.
- Preserve bound lifetimes inside function-pointer and callback types. Do not turn their local binders into adapter lifetimes.
- Named types requiring lifetime arguments must provide arguments that can be represented in generated fields. Support explicit `'_` arguments; do not attempt type resolution to discover lifetimes hidden behind aliases or omitted path arguments.
- Do not add `Clone`, `Send`, `Sync`, or `'static` bounds to construction props or child iterators. Normal Rust borrowing and coercion rules apply at the generated setter boundaries.
- The adapter is temporary storage during construction. The returned RSX value remains one owned `UIScene` with no lifetime parameter.
- Retaining a non-`'static` input in `ui::with_entities`, `.style`, `.observe`, `.on_click`, or a component remains a compile error under the existing APIs. Resolve owned inputs before creating deferred work.
- Ordinary calls to the original function retain its original signature and Rust semantics.

### 2. Mixed optional and list children

```rust
fn content(warning: Option<String>, cards: Vec<String>) -> UIScene {
    rsx! {
        <column>
            <text>{"Upgrades"}</text>
            {warning.map(ui::text)}
            {cards.into_iter().map(ui::text)}
            <text>{"Choose an upgrade"}</text>
        </column>
    }
}
```

A nested child expression accepts exactly:

- `UIScene`, contributing one scene;
- any `IntoIterator<Item = UIScene>`, contributing its scenes in iteration order.

The iterator rule already covers `Option<UIScene>`, `Vec<UIScene>`, arrays, and mapped/chained iterators. No separate option/list syntax is needed. Borrowed iterators are allowed when they yield owned scenes, such as `names.iter().map(|name| ui::text(name.as_str()))`.

Rules:

- Flatten one level only. Reject `&UIScene`, iterators of references, nested optional/list items, strings, and unrelated values. Callers may use ordinary `.flatten()` or `.map(...)` to produce owned scenes themselves. Do not make `UIScene` cloneable or iterable for this feature.
- An empty iterator or `None::<UIScene>` contributes zero nodes. Use a typed `None` or an existing typed option where Rust cannot infer the item type.
- No placeholder, grouping, or wrapper entity is created.
- Keep the top-level result exactly one `UIScene`. A top-level `Option<UIScene>` or list is still an error.
- Core `text` retains its separate string-payload grammar. A leaf custom widget still rejects child content, including syntactically present content that would happen to be empty.
- Keep `ui:children={items}` as the explicit single-source form, including its existing lazy handoff to the widget. It still cannot mix with nested children. Mixed content uses child expressions instead.
- Keep `.child(UIScene)` and `.children(IntoIterator<Item = UIScene>)` unchanged. They already cover ordinary Rust composition.

#### Evaluation order

For a tag with nested content:

1. Evaluate supplied bare props once in their written order.
2. Construct nested content left to right. Evaluate each child expression once and fully append its scenes before evaluating the next sibling.
3. Invoke the widget once with the flattened child vector.
4. Evaluate/apply root controls in their existing relative order.

This deliberately makes mixed nested content eager. A mapped iterator runs before the following sibling is constructed, even if the receiving widget discards its children. The explicit `ui:children` form remains available when the widget should control iterator consumption.

No deferred scene factory runs merely because its scene is appended. Existing factory expansion, style-patch execution, and observer installation timing remain unchanged.

### 3. Optional-prop forwarding

For a widget parameter `color: Option<Color>`:

| RSX input | Function receives |
| --- | --- |
| Prop omitted | `None` |
| `color={None}` | `None` |
| `color={color}` with `color: Color` | `Some(color)` |
| `color={Some(color)}` | `Some(color)` |
| `color={maybe_color}` with `maybe_color: Option<Color>` | `maybe_color`, moved unchanged |

A wrapper can forward a field without branching around its child construction. Passing an option does not create a nested option unless the declared parameter itself is nested.

Rules:

- Continue recognizing only syntactic standard `Option<T>` forms. Type aliases do not acquire implicit optionality.
- Required props stay required at compile time. Do not weaken type-state finalization or duplicate-prop diagnostics.
- Defaults remain in the widget/component. `None` means exactly what omission currently means; it does not add a distinction between “absent” and “explicitly clear.”
- Convert once at the setter boundary. Do not clone values or rerun expressions.
- For ordinary optional types, use Rust's `Into<Option<T>>` contract: identity preserves an option, and the standard `From<T> for Option<T>` wraps an inner value. This is not a general conversion-annotation feature.
- Preserve the existing `Into<String>` convenience for required strings and bare optional strings, including string literals and custom types implementing that conversion.
- An optional string additionally accepts exact `Option<String>`, including inferred `None` and `Some(String)`. Do not add element-wise conversion of `Option<&str>` or arbitrary optional inputs; callers can use `.map(str::to_owned)`.
- Generic conversion bounds do not guarantee every coercion accepted by an exact-typed argument. Callers may need an explicit reborrow or concrete value type; do not add a coercion-dispatch subsystem.

#### Nested options and inference

Do not flatten nested optional props or inspect the spelling of `Some`/`None` in macro input. For a declared `Option<Option<Color>>`:

- `None::<Option<Color>>` is the exact outer option and remains `None`.
- `None::<Color>` is an inner value and becomes `Some(None)`.
- A variable already typed `Option<Option<Color>>` passes through unchanged.

A bare `None` can be ambiguous here because both input types are legal. Document the explicit types rather than choose a meaning silently. Existing nested-option call sites relying on untyped `None` may require this source migration. Ordinary `Option<Color>` and `Option<String>` must accept untyped `None`.

## Implementation ownership

### Lifetime analysis and adapter generation

Keep one normalized widget-signature representation in `ui_widget.rs`. It owns validated props, their adapter-facing types, and the lifetimes/bounds required by those types. All generated definitions, setters, child setters, and finalization impls consume that representation.

Normalize elided reference lifetimes and explicit `'_` arguments in copies used by the adapter, without rewriting the original callable function. Use scope-aware traversal so bound callback lifetimes remain local. Add `syn/visit-mut` only under the existing `rsx` feature if needed.

Carry named and synthesized lifetimes through type-state transitions. Where state-generic fields would otherwise leave a declared lifetime unused, a generated `PhantomData` lifetime witness is legitimate lifetime bookkeeping, not a warning suppression. Emit it only when needed, with no ownership or thread-safety bounds beyond the actual fields. Use the existing collision-avoidance machinery for generated names, including lifetime and conversion-kind names.

Do not generate independent signature-policy paths for primitives, custom widgets, required props, and optional props. Their input policies can differ; lifetime analysis and adapter construction have one owner.

### Nested child normalization

Add one doc-hidden `ChildContent` helper in the existing feature-gated `rsx_widgets` module. It appends into `Vec<UIScene>`, with one implementation for `UIScene` and one blanket implementation for `IntoIterator<Item = UIScene>`.

Have `rsx.rs::child_source` use that helper for every nested child, including tags. A nested child source becomes one vector built in lexical order, then passes through the existing generated child setter. Do not append children to the returned root outside the widget: a custom widget owns where its children belong.

Use the framework path already supplied through the `$crate` RSX wrapper. Do not add a crate-name lookup dependency. Keep the helper out of normal preludes and treat it as generated-code support, not a user extension API.

Accept one temporary vector for a nonempty nested child source. Do not add a parallel fixed-array fast path, boxed iterators, or per-child temporary vectors. The cost is construction-time allocation, not per-frame work. The explicit iterator source remains uncollected by RSX.

### Optional input conversion

Keep optional classification and setter conversion generation in `ui_widget.rs`; all primitive/custom adapters use it.

Use the standard `Into<Option<T>>` path for non-string optional types. Optional strings need a narrow generated helper because `&str` does not implement `Into<Option<String>>`, and blanket string conversion plus option identity cannot be expressed as overlapping implementations of one unmarked trait.

Generate the string helper only for widgets that need it, once per widget expansion. Use distinct inferred kind parameters for the bare `Into<String>` branch and exact `Option<String>` branch. Isolate helper names in an anonymous generated scope; give referenced items appropriate visibility so cross-crate calls compile without lint suppressions. No user-supplied kind argument, new prop annotation, or runtime registry is involved.

Keep this helper emission centralized in the macro implementation. `ui_widget` currently emits self-contained adapters using ordinary Rust paths; preserve that property rather than introduce an RKit-path dependency just for conversion.

Stable Rust feasibility snippets for the cross-crate string helper, ordinary option conversion, and the child blanket implementation were compiled during planning under `.pi/tmp/ecs-ui-composition-design/`. These support the chosen approach but are not acceptance evidence for the eventual generated adapters.

## Implementation plan

Each session has one primary outcome and must leave its supported feature combinations compiling. No session may accept a signature or syntax that its generator cannot yet implement. Run disposable checks under `.pi/tmp/ecs-ui-composition/`; do not add permanent unit/integration suites or compiler-fixture frameworks. Do not add cases to legacy unit tests; modify existing cases only when a production behavior change requires it.

### Phase 1: Borrowed construction props

Goal: generated adapters support temporary input borrows while scenes and deferred work remain owned.

#### Session 1.1: Lifetime-aware widget adapters

- Replace the blanket lifetime rejection with the scoped signature model described above.
- Thread its normalized lifetimes and permitted bounds through all type-state transitions, including optional fields and children.
- Preserve the original callable function and all unrelated signature restrictions.
- Verify positive downstream snippets for independent borrows, mutable references, lifetime-only generics/bounds, nested references, optional references using the current inner-value setter, bound callbacks, and owned scene return after input drop.
- Verify negative snippets for escaping borrows into deferred work, overlapping mutable borrows, unsupported type/const generics, and signature macros. Check diagnostic spans at the declaration or offending expression.
- Gate: macro checks with/without RSX, minimal RKit RSX check, and all-example compilation.

#### Session 1.2: Borrowed-prop example and phase acceptance

- Extend `app_ecs_ui_rsx.rs` with construction props borrowed from local owned inputs. Keep it an application example, not a lifetime test harness.
- Update the signature and ownership sections of `docs/ecs-ui-rsx.md`. Show that the returned scene can outlive its input resource borrow.
- Compile all examples, run the updated example, run RSX doctests, and check formatting.
- Acceptance: borrowing is confined to construction; direct calls still work; no `UIScene` or deferred API bound changed.

### Phase 2: Mixed nested child content

Goal: optional and list expressions compose with ordinary siblings without wrapper nodes.

#### Session 2.1: One-level nested child lowering

- Add the hidden child helper and replace nested-array lowering with eager, ordered flattening.
- Preserve top-level scene checking, text-payload handling, custom leaf rejection, and the explicit iterator-source contract.
- Verify zero/one/many children, arrays/vectors/borrowed mapped iterators, nested custom widgets, and typed absent content.
- Use a disposable consumer to verify expression/iteration order, exactly-once evaluation, parent placement, absence of extra entities, and deferred factories remaining deferred.
- Verify rejection of non-scene items, recursive collections, collection-valued roots, leaf children, and mixed `ui:children` plus nested content.
- Gate: minimal RKit RSX check and all-example compilation.

#### Session 2.2: Mixed-content example and phase acceptance

- Extend the existing RSX action list to place a conditional scene and mapped buttons between ordinary siblings. Remove any example grouping node needed only to satisfy the old child-source restriction.
- Update the guide's children and evaluation-order sections, including the distinction from explicit `ui:children` consumption.
- Compile all examples, run the updated example, run RSX doctests, and check formatting.
- Acceptance: visual order and widget-owned placement are correct; empty content creates no entity; there is one nested normalization path.

### Phase 3: Optional-prop forwarding

Goal: optional state passes through adapters without duplicating child construction or losing existing string convenience.

#### Session 3.1: Ordinary optional-value forwarding

- Change non-string optional setters to convert once through `Into<Option<T>>`.
- Preserve omitted-field initialization, required-prop type states, syntactic option recognition, and primitive defaults.
- Verify bare/optional values, untyped `None` for simple props, qualified options, non-`Copy` inputs, borrowed optional props, and explicit nested-option meanings.
- Verify missing/duplicate required props and wrong input types still fail. Record the nested untyped-`None` inference diagnostic as a documented limitation, not a runtime fallback.
- Gate: macro RSX check, minimal RKit RSX check, and all-example compilation. Optional strings retain their current behavior until Session 3.2.

#### Session 3.2: Optional strings without conversion regressions

- Add the isolated, kind-disambiguated helper to generated optional-string setters; leave required-string conversion unchanged.
- Verify omission, literal/owned/custom-convertible bare strings, `Some(String)`, untyped `None`, and an existing `Option<String>` in a downstream crate.
- Verify conversion and expression evaluation occur once. Check that unrelated optional element types and `Option<&str>` do not acquire new conversion rules.
- Verify cross-module tag aliases, renamed RKit dependency use, hygienic generated names, and no new visibility/dead-code warnings from helper items.
- Gate: macro RSX check, minimal RKit RSX check, and all-example compilation.

#### Session 3.3: Optional-prop examples and phase acceptance

- Extend the existing example with a wrapper forwarding optional color and an optional owned subtitle. Use mixed children for absent subtitle content where appropriate; do not branch around the whole child tag.
- Update the guide's prop rules, input table, string conversion limits, and nested-option migration guidance. Retain component-owned defaults.
- Compile all examples, run the example, run RSX doctests, and check formatting.
- Acceptance: `T` and `Option<T>` reach the same optional parameter; original bare string calls still compile; required props remain compile-time checked.

### Phase 4: Cross-feature acceptance

Goal: the three features work together under the existing scene lifecycle and supported build targets.

#### Session 4.1: Ownership and diagnostics verification

- Use disposable public-API consumers to combine borrowed resources, optional forwarding, mixed children, and `ui::with_entities` that captures only resolved owned inputs.
- Drop input resources before spawning; verify content, child order, expected node count, and instance-local entity links after commands apply.
- Exercise deferred style/observer/factory borrow rejection, standalone macro use, missing required props, and unsupported child content. Fix diagnostics in the owning macro layer, not with runtime checks or broad fallback bounds.
- Audit for duplicated normalization, copied primitive defaults, helper imports required of users, and changes outside the stated scope.
- Gate: downstream pass/fail probes, all-example compilation, and formatting.

#### Session 4.2: Target verification and closeout

- Run the final verification matrix below and record command results and unavailable runtime coverage under `.pi/tmp/ecs-ui-composition/`.
- Run the affected native examples with bounded startup/exit handling; ensure processes are reaped. Perform available browser/Windows runtime checks without treating missing environments as failures.
- Run `just check-health` as the final non-trivial-change gate when its tools are available.
- Reconcile the guide and this document's progress checklist with completed work. Leave unavailable coverage in implementation notes; do not imply compile checks prove interaction behavior.
- Acceptance: available checks pass, or actual failures have explicit maintainer acceptance. No unapproved feature expansion or staged-file changes.

## Progress

- [x] Phase 1: Borrowed construction props
  - [x] Session 1.1: Lifetime-aware widget adapters
  - [x] Session 1.2: Borrowed-prop example and phase acceptance
- [x] Phase 2: Mixed nested child content
  - [x] Session 2.1: One-level nested child lowering
  - [x] Session 2.2: Mixed-content example and phase acceptance
- [x] Phase 3: Optional-prop forwarding
  - [x] Session 3.1: Ordinary optional-value forwarding
  - [x] Session 3.2: Optional strings without conversion regressions
  - [x] Session 3.3: Optional-prop examples and phase acceptance
- [x] Phase 4: Cross-feature acceptance
  - [x] Session 4.1: Ownership and diagnostics verification
  - [x] Session 4.2: Target verification and closeout

## Acceptance and verification

Run the narrow gates while iterating, then this final matrix:

```bash
rtk cargo check -p macros --no-default-features
rtk cargo check -p macros --no-default-features --features rsx
rtk cargo check -p rkit --no-default-features
rtk cargo check -p rkit --no-default-features --features ecs,ui
rtk cargo check -p rkit --no-default-features --features ecs-ui-experimental
rtk cargo check -p rkit --no-default-features --features ecs-ui-rsx
rtk cargo check --workspace --all-features
rtk cargo check --workspace --all-features --examples
rtk cargo test -p rkit --doc --no-default-features --features ecs-ui-rsx
rtk cargo doc -p rkit --no-deps --no-default-features --features ecs-ui-rsx
rtk cargo fmt --all -- --check
rtk git diff --check
```

Compile the updated example on available release targets:

```bash
rtk cargo check --example app_ecs_ui_rsx --target x86_64-pc-windows-msvc --no-default-features --features ecs-ui-rsx,draw-default-font
rtk cargo check --example app_ecs_ui_rsx --target wasm32-unknown-unknown --no-default-features --features ecs-ui-rsx,draw-default-font
rtk cargo check --example app_ecs_ui_rsx --target wasm32-unknown-unknown --no-default-features --features ecs-ui-rsx,draw-default-font,webgl
```

Run `app_ecs_ui_rsx`, `app_ecs_ui_rsx_content`, and `app_ecs_ui_rsx_entities` on available runtime environments. For native runs:

```bash
rtk cargo run --example app_ecs_ui_rsx --no-default-features --features ecs-ui-rsx,draw-default-font
rtk cargo run --example app_ecs_ui_rsx_content --no-default-features --features ecs-ui-rsx,draw-default-font
rtk cargo run --example app_ecs_ui_rsx_entities --no-default-features --features ecs-ui-rsx,draw-default-font
```

Use bounded/reaped runs in automation rather than leaving these commands unattended. Confirm relevant interactions manually or through disposable public-API probes; label startup-only evidence accurately.

Finally:

```bash
just check-health
```

Do not suppress pre-existing warnings or fix unrelated failures silently. Missing target/runtime tools are residual coverage gaps; failures on available required checks must be fixed or explicitly accepted. New code must not introduce warning suppressions or additional unexplained warnings.

## Risks and limits

- Lifetime normalization must respect binder scopes and generated type-state lifetimes. Removing validation without changing adapter generation is insufficient.
- Generic optional conversion can change type-inference/coercion behavior. Preserve supported string forms and document the narrow nested-option migration.
- Eager nested flattening consumes mapped iterators before widget invocation and uses a temporary vector. This is an explicit construction contract; retain the existing explicit iterator form for consumer-controlled iteration.
- More useful syntax does not solve macro formatting. The current formatter limitations remain documented, and ordinary Rust composition remains a supported alternative.
