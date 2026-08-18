#[derive(Clone, Copy)]
pub(crate) enum ButtonEdge {
    Pressed,
    Released,
}

pub(crate) fn ordered_button_edges(
    pressed: bool,
    released: bool,
    down: bool,
) -> impl Iterator<Item = ButtonEdge> {
    let edges = if down {
        [ButtonEdge::Released, ButtonEdge::Pressed]
    } else {
        [ButtonEdge::Pressed, ButtonEdge::Released]
    };

    edges.into_iter().filter(move |edge| match edge {
        ButtonEdge::Pressed => pressed,
        ButtonEdge::Released => released,
    })
}
