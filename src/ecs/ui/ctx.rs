use bevy_ecs::prelude::*;

#[derive(Debug, Clone, Copy)]
pub(super) struct NodeContext {
    pub entity: Entity,
}

#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UINodeType {
    #[default]
    Container,
    Text,
    RichText,
    Image,
}
