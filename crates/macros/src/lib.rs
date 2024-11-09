use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitInt, Token, Type};

#[proc_macro_derive(Drawable2D, attributes(transform_2d, pipeline_id))]
pub fn ui_element_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let where_clause = &input.generics.where_clause;

    let mut transform_opt = None;
    let mut pipeline_opt = None;

    // Find the fields with the #[transform_2d] and #[pipeline_id] attributes
    if let Data::Struct(data_struct) = &input.data {
        if let Fields::Named(fields_named) = &data_struct.fields {
            for field in &fields_named.named {
                for attr in &field.attrs {
                    if attr.path().is_ident("transform_2d") {
                        transform_opt = Some(field.ident.clone().unwrap());
                    }
                    if attr.path().is_ident("pipeline_id") {
                        pipeline_opt = Some(field.ident.clone().unwrap());
                    }
                }
            }
        }
    }

    // Handle cases where attributes are missing
    let transform_field = transform_opt.expect("No field with #[transform_2d] attribute found");

    // Generate the methods conditionally
    let pipeline_method = if let Some(pipeline_field) = pipeline_opt {
        quote! {
            pub fn pipeline(&mut self, pip: &DrawPipelineId) -> &mut Self {
                self.#pipeline_field = *pip;
                self
            }
        }
    } else {
        quote! {}
    };

    // Generate the implementation using the detected fields
    let expanded = quote! {
        impl #generics #name #generics #where_clause {
            // - Transform
            pub fn translate(&mut self, pos: Vec2) -> &mut Self {
                let t = self.#transform_field.get_or_insert_with(|| Transform2D::default());
                t.set_translation(pos);
                self
            }

            pub fn anchor(&mut self, point: Vec2) -> &mut Self {
                let t = self.#transform_field.get_or_insert_with(|| Transform2D::default());
                t.set_anchor(point);
                self
            }

            pub fn pivot(&mut self, point: Vec2) -> &mut Self {
                let t = self.#transform_field.get_or_insert_with(|| Transform2D::default());
                t.set_pivot(point);
                self
            }

            pub fn flip_x(&mut self, flip: bool) -> &mut Self {
                let t = self.#transform_field.get_or_insert_with(|| Transform2D::default());
                t.set_flip(bvec2(flip, t.flip().y));
                self
            }

            pub fn flip_y(&mut self, flip: bool) -> &mut Self {
                let t = self.#transform_field.get_or_insert_with(|| Transform2D::default());
                t.set_flip(bvec2(t.flip().x, flip));
                self
            }

            pub fn skew(&mut self, skew: Vec2) -> &mut Self {
                let t = self.#transform_field.get_or_insert_with(|| Transform2D::default());
                t.set_skew(skew);
                self
            }

            pub fn scale(&mut self, scale: Vec2) -> &mut Self {
                let t = self.#transform_field.get_or_insert_with(|| Transform2D::default());
                t.set_scale(scale);
                self
            }

            pub fn rotation(&mut self, rot: f32) -> &mut Self {
                let t = self.#transform_field.get_or_insert_with(|| Transform2D::default());
                t.set_rotation(rot);
                self
            }

            #pipeline_method
        }
    };

    TokenStream::from(expanded)
}

// -- LocalPool

// Define a struct to parse the input arguments for the `init_local_pool` macro
struct InitLocalPoolInput {
    pool_name: Ident,
    size: LitInt,
    ty: Type,
    init_expr: syn::ExprClosure,
}

impl Parse for InitLocalPoolInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pool_name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;

        let size: LitInt = input.parse()?;
        input.parse::<Token![,]>()?;

        let ty: Type = input.parse()?;
        input.parse::<Token![,]>()?;

        let init_expr: syn::ExprClosure = input.parse()?;

        Ok(InitLocalPoolInput {
            pool_name,
            size,
            ty,
            init_expr,
        })
    }
}

/// A macro to initialize a thread-local object pool with a specified size.
///
/// # Example
///
/// ```rust,ignore
/// init_local_pool!(MY_POOL, 32, Vec<u8>, || Vec::with_capacity(100));
/// ```
///
/// This will create a thread-local pool named `MY_POOL` that can hold up to 32 `Vec<u8>`
/// elements, each initialized with a capacity of 100.
///
/// - `pool_name`: The name of the pool.
/// - `size`: The size of the pool (number of items).
/// - `type`: The type of items to be pooled.
/// - `init_expr`: A closure to initialize each item in the pool.
#[proc_macro]
pub fn init_local_pool(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a custom InitLocalPoolInput struct
    let input = parse_macro_input!(input as InitLocalPoolInput);

    let pool_name = input.pool_name;
    let size = input.size;
    let ty = input.ty;
    let init_expr = input.init_expr;

    // Generate unique identifiers for functions and thread-local storage
    let on_take_fn = format_ident!("on_take_{}", pool_name);
    let on_drop_fn = format_ident!("on_drop_{}", pool_name);
    let len_fn = format_ident!("len_{}", pool_name);
    let inner_pool = format_ident!("INNER_POOL_{}", pool_name);

    let expanded = quote! {
        thread_local! {
            static #inner_pool: std::cell::RefCell<InnerLocalPool<#ty, #size>> = std::cell::RefCell::new(InnerLocalPool::new(#init_expr));
        }

        fn #on_take_fn() -> Option<LocalPoolObserver<#ty>> {
            #inner_pool.with(|pool| {
                let mut pool = pool.borrow_mut();
                pool.take().map(|t| LocalPoolObserver::new(t, #on_drop_fn))
            })
        }

        fn #on_drop_fn(t: #ty) {
            #inner_pool.with(|pool| {
                pool.borrow_mut().put_back(t);
            });
        }

        fn #len_fn() -> usize {
            #inner_pool.with(|pool| {
                pool.borrow().len()
            })
        }

        pub static #pool_name: LocalPool<#ty, #size> = LocalPool::init(#on_take_fn, #on_drop_fn, #len_fn);
    };

    TokenStream::from(expanded)
}
