use super::super::widgets::{UIContainer, UIImage, UIRichText, UIText};
use super::UIScene;

pub fn node() -> UIScene {
    UIScene::node()
}

pub fn row() -> UIScene {
    UIScene::node().style(|style| style.flex_row())
}

pub fn column() -> UIScene {
    UIScene::node().style(|style| style.flex_col())
}

pub fn container(container: UIContainer) -> UIScene {
    UIScene::node().insert(container)
}

pub fn text(text: impl Into<String>) -> UIScene {
    UIScene::node().insert(UIText {
        text: text.into(),
        ..Default::default()
    })
}

pub fn rich_text(text: UIRichText) -> UIScene {
    UIScene::node().insert(text)
}

pub fn image(image: UIImage) -> UIScene {
    UIScene::node().insert(image)
}
