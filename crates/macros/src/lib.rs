use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

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
