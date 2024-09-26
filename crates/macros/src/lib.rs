use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Transform2D, attributes(transform_2d))]
pub fn ui_element_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let where_clause = &input.generics.where_clause;

    let mut transform_opt = None;

    // Find the field with the #[transform_2d] attribute
    if let Data::Struct(data_struct) = &input.data {
        if let Fields::Named(fields_named) = &data_struct.fields {
            for field in &fields_named.named {
                for attr in &field.attrs {
                    if attr.path().is_ident("transform_2d") {
                        transform_opt = Some(field.ident.clone().unwrap());
                    }
                }
            }
        }
    }

    // Handle the case where no field was found with #[transform_2d] attribute
    let t = transform_opt.expect("No field with #[transform_2d] attribute found");

    // Generate the implementation using the detected field
    let expanded = quote! {
        impl #generics #name #generics #where_clause {
            // - Transform
            pub fn translate(&mut self, pos: Vec2) -> &mut Self {
                let t = self.#t.get_or_insert_with(|| Transform2D::default());
                t.set_translation(pos);
                self
            }

            pub fn anchor(&mut self, point: Vec2) -> &mut Self {
                let t = self.#t.get_or_insert_with(|| Transform2D::default());
                t.set_anchor(point);
                self
            }

            pub fn pivot(&mut self, point: Vec2) -> &mut Self {
                let t = self.#t.get_or_insert_with(|| Transform2D::default());
                t.set_pivot(point);
                self
            }

            pub fn flip_x(&mut self, flip: bool) -> &mut Self {
                let t = self.#t.get_or_insert_with(|| Transform2D::default());
                t.set_flip(bvec2(flip, t.flip().y));
                self
            }

            pub fn flip_y(&mut self, flip: bool) -> &mut Self {
                let t = self.#t.get_or_insert_with(|| Transform2D::default());
                t.set_flip(bvec2(t.flip().x, flip));
                self
            }

            pub fn skew(&mut self, skew: Vec2) -> &mut Self {
                let t = self.#t.get_or_insert_with(|| Transform2D::default());
                t.set_skew(skew);
                self
            }

            pub fn scale(&mut self, scale: Vec2) -> &mut Self {
                let t = self.#t.get_or_insert_with(|| Transform2D::default());
                t.set_scale(scale);
                self
            }

            pub fn rotation(&mut self, rot: f32) -> &mut Self {
                let t = self.#t.get_or_insert_with(|| Transform2D::default());
                t.set_rotation(rot);
                self
            }
        }
    };

    TokenStream::from(expanded)
}
