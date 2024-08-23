use crate::input::MouseButtonList;
use crate::option_usize_env;
use crate::utils::{next_pot2, EnumSet};
use arrayvec::ArrayVec;
use math::Vec2;
use nohash_hasher::IsEnabled;
use smol_str::SmolStr;
use std::hash::Hasher;
use strum::EnumCount;
use strum_macros::EnumCount;

// Passing this env variable we can control the size of the hashset to reduce memory consume.
// 16 keys at once seems more than enough, most keyboard are 6kro (6 at once), some gaming
// devices use NKRO (no-limit), but at the end of the day the human hands only have 10 fingers
// still, we can pass as var in the build.rs a higher value if the game will require more keys
const MAX_KEYS_PRESSED: usize = option_usize_env!("GK_MAX_KEYS_PRESSED", 16);
const KEYS_COUNT_POT2: usize = next_pot2(MAX_KEYS_PRESSED);

/// This enum comes from winit, we only add Unknown
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumCount)]
#[repr(u16)]
pub enum KeyCode {
    /// <kbd>`</kbd> on a US keyboard. This is also called a backtick or grave.
    /// This is the <kbd>半角</kbd>/<kbd>全角</kbd>/<kbd>漢字</kbd>
    /// (hankaku/zenkaku/kanji) key on Japanese keyboards
    Backquote,
    /// Used for both the US <kbd>\\</kbd> (on the 101-key layout) and also for the key
    /// located between the <kbd>"</kbd> and <kbd>Enter</kbd> keys on row C of the 102-,
    /// 104- and 106-key layouts.
    /// Labeled <kbd>#</kbd> on a UK (102) keyboard.
    Backslash,
    /// <kbd>[</kbd> on a US keyboard.
    BracketLeft,
    /// <kbd>]</kbd> on a US keyboard.
    BracketRight,
    /// <kbd>,</kbd> on a US keyboard.
    Comma,
    /// <kbd>0</kbd> on a US keyboard.
    Digit0,
    /// <kbd>1</kbd> on a US keyboard.
    Digit1,
    /// <kbd>2</kbd> on a US keyboard.
    Digit2,
    /// <kbd>3</kbd> on a US keyboard.
    Digit3,
    /// <kbd>4</kbd> on a US keyboard.
    Digit4,
    /// <kbd>5</kbd> on a US keyboard.
    Digit5,
    /// <kbd>6</kbd> on a US keyboard.
    Digit6,
    /// <kbd>7</kbd> on a US keyboard.
    Digit7,
    /// <kbd>8</kbd> on a US keyboard.
    Digit8,
    /// <kbd>9</kbd> on a US keyboard.
    Digit9,
    /// <kbd>=</kbd> on a US keyboard.
    Equal,
    /// Located between the left <kbd>Shift</kbd> and <kbd>Z</kbd> keys.
    /// Labeled <kbd>\\</kbd> on a UK keyboard.
    IntlBackslash,
    /// Located between the <kbd>/</kbd> and right <kbd>Shift</kbd> keys.
    /// Labeled <kbd>\\</kbd> (ro) on a Japanese keyboard.
    IntlRo,
    /// Located between the <kbd>=</kbd> and <kbd>Backspace</kbd> keys.
    /// Labeled <kbd>¥</kbd> (yen) on a Japanese keyboard. <kbd>\\</kbd> on a
    /// Russian keyboard.
    IntlYen,
    /// <kbd>a</kbd> on a US keyboard.
    /// Labeled <kbd>q</kbd> on an AZERTY (e.g., French) keyboard.
    KeyA,
    /// <kbd>b</kbd> on a US keyboard.
    KeyB,
    /// <kbd>c</kbd> on a US keyboard.
    KeyC,
    /// <kbd>d</kbd> on a US keyboard.
    KeyD,
    /// <kbd>e</kbd> on a US keyboard.
    KeyE,
    /// <kbd>f</kbd> on a US keyboard.
    KeyF,
    /// <kbd>g</kbd> on a US keyboard.
    KeyG,
    /// <kbd>h</kbd> on a US keyboard.
    KeyH,
    /// <kbd>i</kbd> on a US keyboard.
    KeyI,
    /// <kbd>j</kbd> on a US keyboard.
    KeyJ,
    /// <kbd>k</kbd> on a US keyboard.
    KeyK,
    /// <kbd>l</kbd> on a US keyboard.
    KeyL,
    /// <kbd>m</kbd> on a US keyboard.
    KeyM,
    /// <kbd>n</kbd> on a US keyboard.
    KeyN,
    /// <kbd>o</kbd> on a US keyboard.
    KeyO,
    /// <kbd>p</kbd> on a US keyboard.
    KeyP,
    /// <kbd>q</kbd> on a US keyboard.
    /// Labeled <kbd>a</kbd> on an AZERTY (e.g., French) keyboard.
    KeyQ,
    /// <kbd>r</kbd> on a US keyboard.
    KeyR,
    /// <kbd>s</kbd> on a US keyboard.
    KeyS,
    /// <kbd>t</kbd> on a US keyboard.
    KeyT,
    /// <kbd>u</kbd> on a US keyboard.
    KeyU,
    /// <kbd>v</kbd> on a US keyboard.
    KeyV,
    /// <kbd>w</kbd> on a US keyboard.
    /// Labeled <kbd>z</kbd> on an AZERTY (e.g., French) keyboard.
    KeyW,
    /// <kbd>x</kbd> on a US keyboard.
    KeyX,
    /// <kbd>y</kbd> on a US keyboard.
    /// Labeled <kbd>z</kbd> on a QWERTZ (e.g., German) keyboard.
    KeyY,
    /// <kbd>z</kbd> on a US keyboard.
    /// Labeled <kbd>w</kbd> on an AZERTY (e.g., French) keyboard, and <kbd>y</kbd> on a
    /// QWERTZ (e.g., German) keyboard.
    KeyZ,
    /// <kbd>-</kbd> on a US keyboard.
    Minus,
    /// <kbd>.</kbd> on a US keyboard.
    Period,
    /// <kbd>'</kbd> on a US keyboard.
    Quote,
    /// <kbd>;</kbd> on a US keyboard.
    Semicolon,
    /// <kbd>/</kbd> on a US keyboard.
    Slash,
    /// <kbd>Alt</kbd>, <kbd>Option</kbd>, or <kbd>⌥</kbd>.
    AltLeft,
    /// <kbd>Alt</kbd>, <kbd>Option</kbd>, or <kbd>⌥</kbd>.
    /// This is labeled <kbd>AltGr</kbd> on many keyboard layouts.
    AltRight,
    /// <kbd>Backspace</kbd> or <kbd>⌫</kbd>.
    /// Labeled <kbd>Delete</kbd> on Apple keyboards.
    Backspace,
    /// <kbd>CapsLock</kbd> or <kbd>⇪</kbd>
    CapsLock,
    /// The application context menu key, which is typically found between the right
    /// <kbd>Super</kbd> key and the right <kbd>Control</kbd> key.
    ContextMenu,
    /// <kbd>Control</kbd> or <kbd>⌃</kbd>
    ControlLeft,
    /// <kbd>Control</kbd> or <kbd>⌃</kbd>
    ControlRight,
    /// <kbd>Enter</kbd> or <kbd>↵</kbd>. Labeled <kbd>Return</kbd> on Apple keyboards.
    Enter,
    /// The Windows, <kbd>⌘</kbd>, <kbd>Command</kbd>, or other OS symbol key.
    SuperLeft,
    /// The Windows, <kbd>⌘</kbd>, <kbd>Command</kbd>, or other OS symbol key.
    SuperRight,
    /// <kbd>Shift</kbd> or <kbd>⇧</kbd>
    ShiftLeft,
    /// <kbd>Shift</kbd> or <kbd>⇧</kbd>
    ShiftRight,
    /// <kbd> </kbd> (space)
    Space,
    /// <kbd>Tab</kbd> or <kbd>⇥</kbd>
    Tab,
    /// Japanese: <kbd>変</kbd> (henkan)
    Convert,
    /// Japanese: <kbd>カタカナ</kbd>/<kbd>ひらがな</kbd>/<kbd>ローマ字</kbd>
    /// (katakana/hiragana/romaji)
    KanaMode,
    /// Korean: HangulMode <kbd>한/영</kbd> (han/yeong)
    ///
    /// Japanese (Mac keyboard): <kbd>か</kbd> (kana)
    Lang1,
    /// Korean: Hanja <kbd>한</kbd> (hanja)
    ///
    /// Japanese (Mac keyboard): <kbd>英</kbd> (eisu)
    Lang2,
    /// Japanese (word-processing keyboard): Katakana
    Lang3,
    /// Japanese (word-processing keyboard): Hiragana
    Lang4,
    /// Japanese (word-processing keyboard): Zenkaku/Hankaku
    Lang5,
    /// Japanese: <kbd>無変換</kbd> (muhenkan)
    NonConvert,
    /// <kbd>⌦</kbd>. The forward delete key.
    /// Note that on Apple keyboards, the key labelled <kbd>Delete</kbd> on the main part of
    /// the keyboard is encoded as [`Backspace`].
    ///
    /// [`Backspace`]: Self::Backspace
    Delete,
    /// <kbd>Page Down</kbd>, <kbd>End</kbd>, or <kbd>↘</kbd>
    End,
    /// <kbd>Help</kbd>. Not present on standard PC keyboards.
    Help,
    /// <kbd>Home</kbd> or <kbd>↖</kbd>
    Home,
    /// <kbd>Insert</kbd> or <kbd>Ins</kbd>. Not present on Apple keyboards.
    Insert,
    /// <kbd>Page Down</kbd>, <kbd>PgDn</kbd>, or <kbd>⇟</kbd>
    PageDown,
    /// <kbd>Page Up</kbd>, <kbd>PgUp</kbd>, or <kbd>⇞</kbd>
    PageUp,
    /// <kbd>↓</kbd>
    ArrowDown,
    /// <kbd>←</kbd>
    ArrowLeft,
    /// <kbd>→</kbd>
    ArrowRight,
    /// <kbd>↑</kbd>
    ArrowUp,
    /// On the Mac, this is used for the numpad <kbd>Clear</kbd> key.
    NumLock,
    /// <kbd>0 Ins</kbd> on a keyboard. <kbd>0</kbd> on a phone or remote control
    Numpad0,
    /// <kbd>1 End</kbd> on a keyboard. <kbd>1</kbd> or <kbd>1 QZ</kbd> on a phone or remote
    /// control
    Numpad1,
    /// <kbd>2 ↓</kbd> on a keyboard. <kbd>2 ABC</kbd> on a phone or remote control
    Numpad2,
    /// <kbd>3 PgDn</kbd> on a keyboard. <kbd>3 DEF</kbd> on a phone or remote control
    Numpad3,
    /// <kbd>4 ←</kbd> on a keyboard. <kbd>4 GHI</kbd> on a phone or remote control
    Numpad4,
    /// <kbd>5</kbd> on a keyboard. <kbd>5 JKL</kbd> on a phone or remote control
    Numpad5,
    /// <kbd>6 →</kbd> on a keyboard. <kbd>6 MNO</kbd> on a phone or remote control
    Numpad6,
    /// <kbd>7 Home</kbd> on a keyboard. <kbd>7 PQRS</kbd> or <kbd>7 PRS</kbd> on a phone
    /// or remote control
    Numpad7,
    /// <kbd>8 ↑</kbd> on a keyboard. <kbd>8 TUV</kbd> on a phone or remote control
    Numpad8,
    /// <kbd>9 PgUp</kbd> on a keyboard. <kbd>9 WXYZ</kbd> or <kbd>9 WXY</kbd> on a phone
    /// or remote control
    Numpad9,
    /// <kbd>+</kbd>
    NumpadAdd,
    /// Found on the Microsoft Natural Keyboard.
    NumpadBackspace,
    /// <kbd>C</kbd> or <kbd>A</kbd> (All Clear). Also for use with numpads that have a
    /// <kbd>Clear</kbd> key that is separate from the <kbd>NumLock</kbd> key. On the Mac, the
    /// numpad <kbd>Clear</kbd> key is encoded as [`NumLock`].
    ///
    /// [`NumLock`]: Self::NumLock
    NumpadClear,
    /// <kbd>C</kbd> (Clear Entry)
    NumpadClearEntry,
    /// <kbd>,</kbd> (thousands separator). For locales where the thousands separator
    /// is a "." (e.g., Brazil), this key may generate a <kbd>.</kbd>.
    NumpadComma,
    /// <kbd>. Del</kbd>. For locales where the decimal separator is "," (e.g.,
    /// Brazil), this key may generate a <kbd>,</kbd>.
    NumpadDecimal,
    /// <kbd>/</kbd>
    NumpadDivide,
    NumpadEnter,
    /// <kbd>=</kbd>
    NumpadEqual,
    /// <kbd>#</kbd> on a phone or remote control device. This key is typically found
    /// below the <kbd>9</kbd> key and to the right of the <kbd>0</kbd> key.
    NumpadHash,
    /// <kbd>M</kbd> Add current entry to the value stored in memory.
    NumpadMemoryAdd,
    /// <kbd>M</kbd> Clear the value stored in memory.
    NumpadMemoryClear,
    /// <kbd>M</kbd> Replace the current entry with the value stored in memory.
    NumpadMemoryRecall,
    /// <kbd>M</kbd> Replace the value stored in memory with the current entry.
    NumpadMemoryStore,
    /// <kbd>M</kbd> Subtract current entry from the value stored in memory.
    NumpadMemorySubtract,
    /// <kbd>*</kbd> on a keyboard. For use with numpads that provide mathematical
    /// operations (<kbd>+</kbd>, <kbd>-</kbd> <kbd>*</kbd> and <kbd>/</kbd>).
    ///
    /// Use `NumpadStar` for the <kbd>*</kbd> key on phones and remote controls.
    NumpadMultiply,
    /// <kbd>(</kbd> Found on the Microsoft Natural Keyboard.
    NumpadParenLeft,
    /// <kbd>)</kbd> Found on the Microsoft Natural Keyboard.
    NumpadParenRight,
    /// <kbd>*</kbd> on a phone or remote control device.
    ///
    /// This key is typically found below the <kbd>7</kbd> key and to the left of
    /// the <kbd>0</kbd> key.
    ///
    /// Use <kbd>"NumpadMultiply"</kbd> for the <kbd>*</kbd> key on
    /// numeric keypads.
    NumpadStar,
    /// <kbd>-</kbd>
    NumpadSubtract,
    /// <kbd>Esc</kbd> or <kbd>⎋</kbd>
    Escape,
    /// <kbd>Fn</kbd> This is typically a hardware key that does not generate a separate code.
    Fn,
    /// <kbd>FLock</kbd> or <kbd>FnLock</kbd>. Function Lock key. Found on the Microsoft
    /// Natural Keyboard.
    FnLock,
    /// <kbd>PrtScr SysRq</kbd> or <kbd>Print Screen</kbd>
    PrintScreen,
    /// <kbd>Scroll Lock</kbd>
    ScrollLock,
    /// <kbd>Pause Break</kbd>
    Pause,
    /// Some laptops place this key to the left of the <kbd>↑</kbd> key.
    ///
    /// This also the "back" button (triangle) on Android.
    BrowserBack,
    BrowserFavorites,
    /// Some laptops place this key to the right of the <kbd>↑</kbd> key.
    BrowserForward,
    /// The "home" button on Android.
    BrowserHome,
    BrowserRefresh,
    BrowserSearch,
    BrowserStop,
    /// <kbd>Eject</kbd> or <kbd>⏏</kbd>. This key is placed in the function section on some Apple
    /// keyboards.
    Eject,
    /// Sometimes labelled <kbd>My Computer</kbd> on the keyboard
    LaunchApp1,
    /// Sometimes labelled <kbd>Calculator</kbd> on the keyboard
    LaunchApp2,
    LaunchMail,
    MediaPlayPause,
    MediaSelect,
    MediaStop,
    MediaTrackNext,
    MediaTrackPrevious,
    /// This key is placed in the function section on some Apple keyboards, replacing the
    /// <kbd>Eject</kbd> key.
    Power,
    Sleep,
    AudioVolumeDown,
    AudioVolumeMute,
    AudioVolumeUp,
    WakeUp,
    // Legacy modifier key. Also called "Super" in certain places.
    Meta,
    // Legacy modifier key.
    Hyper,
    Turbo,
    Abort,
    Resume,
    Suspend,
    /// Found on Sun’s USB keyboard.
    Again,
    /// Found on Sun’s USB keyboard.
    Copy,
    /// Found on Sun’s USB keyboard.
    Cut,
    /// Found on Sun’s USB keyboard.
    Find,
    /// Found on Sun’s USB keyboard.
    Open,
    /// Found on Sun’s USB keyboard.
    Paste,
    /// Found on Sun’s USB keyboard.
    Props,
    /// Found on Sun’s USB keyboard.
    Select,
    /// Found on Sun’s USB keyboard.
    Undo,
    /// Use for dedicated <kbd>ひらがな</kbd> key found on some Japanese word processing keyboards.
    Hiragana,
    /// Use for dedicated <kbd>カタカナ</kbd> key found on some Japanese word processing keyboards.
    Katakana,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F1,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F2,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F3,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F4,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F5,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F6,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F7,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F8,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F9,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F10,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F11,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F12,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F13,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F14,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F15,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F16,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F17,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F18,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F19,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F20,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F21,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F22,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F23,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F24,
    /// General-purpose function key.
    F25,
    /// General-purpose function key.
    F26,
    /// General-purpose function key.
    F27,
    /// General-purpose function key.
    F28,
    /// General-purpose function key.
    F29,
    /// General-purpose function key.
    F30,
    /// General-purpose function key.
    F31,
    /// General-purpose function key.
    F32,
    /// General-purpose function key.
    F33,
    /// General-purpose function key.
    F34,
    /// General-purpose function key.
    F35,

    Unknown,
}

#[derive(Default, Clone)]
pub struct KeyCodeList {
    set: EnumSet<UniqueKeyCode, KEYS_COUNT_POT2>,
}

impl KeyCodeList {
    pub fn insert(&mut self, v: KeyCode) -> bool {
        self.set.insert(UniqueKeyCode(v)).unwrap_or_default()
    }

    pub fn contains(&self, k: KeyCode) -> bool {
        self.set.contains(&UniqueKeyCode(k))
    }

    pub fn iter(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.set.iter().map(|unique_btn| unique_btn.0)
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    pub fn remove(&mut self, k: KeyCode) -> bool {
        self.set.remove(&UniqueKeyCode(k))
    }

    pub fn clear(&mut self) {
        self.set.clear()
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct UniqueKeyCode(KeyCode);
impl std::hash::Hash for UniqueKeyCode {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_u16(self.0 as _)
    }
}

impl IsEnabled for UniqueKeyCode {}

impl std::fmt::Debug for KeyCodeList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

#[derive(Default, Clone)]
pub struct TextList {
    list: ArrayVec<SmolStr, MAX_KEYS_PRESSED>,
}

impl TextList {
    pub fn insert(&mut self, txt: &str) {
        self.list.push(SmolStr::new(txt))
    }

    pub fn contains(&self, txt: &str) -> bool {
        self.list.contains(&SmolStr::new(txt))
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> + '_ {
        self.list.iter().map(|s| s.as_str())
    }

    pub fn into_iter(self) -> impl Iterator<Item = SmolStr> + 'static {
        self.list.into_iter()
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn remove(&mut self, txt: &str) -> bool {
        let idx = self.list.iter().position(|cc| cc.as_str() == txt);
        match idx {
            None => false,
            Some(idx) => {
                self.list.remove(idx);
                true
            }
        }
    }

    pub fn clear(&mut self) {
        self.list.clear()
    }
}

impl std::fmt::Debug for TextList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct KeyboardState {
    pub pressed: KeyCodeList,
    pub released: KeyCodeList,
    pub down: KeyCodeList,

    pub text: TextList,
}

impl KeyboardState {
    pub fn add_text(&mut self, c: &str) {
        self.text.insert(c);
    }

    pub fn press(&mut self, btn: KeyCode) {
        self.pressed.insert(btn);
        self.down.insert(btn);
        self.released.remove(btn);
    }

    pub fn release(&mut self, btn: KeyCode) {
        self.released.insert(btn);
        self.down.remove(btn);
        self.pressed.remove(btn);
    }

    pub fn are_pressed<const N: usize>(&self, btns: &[KeyCode; N]) -> [bool; N] {
        let mut res = [false; N];
        btns.iter().enumerate().for_each(|(i, &btn)| {
            res[i] = self.is_pressed(btn);
        });
        res
    }

    pub fn is_pressed(&self, btn: KeyCode) -> bool {
        self.pressed.contains(btn)
    }

    pub fn are_released<const N: usize>(&self, btns: &[KeyCode; N]) -> [bool; N] {
        let mut res = [false; N];
        btns.iter().enumerate().for_each(|(i, &btn)| {
            res[i] = self.is_released(btn);
        });
        res
    }

    pub fn is_released(&self, btn: KeyCode) -> bool {
        self.released.contains(btn)
    }

    pub fn are_down<const N: usize>(&self, btns: &[KeyCode; N]) -> [bool; N] {
        let mut res = [false; N];
        btns.iter().enumerate().for_each(|(i, &btn)| {
            res[i] = self.is_down(btn);
        });
        res
    }

    pub fn is_down(&self, btn: KeyCode) -> bool {
        self.down.contains(btn)
    }

    pub fn tick(&mut self) {
        self.pressed.clear();
        self.released.clear();
        self.text.clear();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_list_insert_contains() {
        let mut list = KeyCodeList::default();
        assert!(!list.contains(KeyCode::KeyA));

        list.insert(KeyCode::KeyA);
        assert!(list.contains(KeyCode::KeyA));
    }

    #[test]
    fn test_list_remove() {
        let mut list = KeyCodeList::default();
        list.insert(KeyCode::KeyB);
        assert!(list.contains(KeyCode::KeyB));

        list.remove(KeyCode::KeyB);
        assert!(!list.contains(KeyCode::KeyB));
    }

    #[test]
    fn test_list_len_and_empty() {
        let mut list = KeyCodeList::default();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);

        list.insert(KeyCode::KeyC);
        assert!(!list.is_empty());
        assert_eq!(list.len(), 1);

        list.insert(KeyCode::KeyD);
        assert_eq!(list.len(), 2);

        list.remove(KeyCode::KeyC);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_list_iter() {
        let mut list = KeyCodeList::default();
        list.insert(KeyCode::KeyE);
        list.insert(KeyCode::KeyF);

        let keys: Vec<_> = list.iter().collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&KeyCode::KeyE));
        assert!(keys.contains(&KeyCode::KeyF));
    }

    #[test]
    fn test_text_list_insert_contains() {
        let mut text_list = TextList::default();
        assert!(!text_list.contains("Hello"));

        text_list.insert("Hello");
        assert!(text_list.contains("Hello"));
    }

    #[test]
    fn test_text_list_remove() {
        let mut text_list = TextList::default();
        text_list.insert("World");
        assert!(text_list.contains("World"));

        text_list.remove("World");
        assert!(!text_list.contains("World"));
    }

    #[test]
    fn test_text_list_len_and_empty() {
        let mut text_list = TextList::default();
        assert!(text_list.is_empty());
        assert_eq!(text_list.len(), 0);

        text_list.insert("Rust");
        assert!(!text_list.is_empty());
        assert_eq!(text_list.len(), 1);

        text_list.insert("Language");
        assert_eq!(text_list.len(), 2);

        text_list.remove("Rust");
        assert_eq!(text_list.len(), 1);
    }

    #[test]
    fn test_text_list_iter() {
        let mut text_list = TextList::default();
        text_list.insert("Hello");
        text_list.insert("World");

        let texts: Vec<_> = text_list.iter().collect();
        assert_eq!(texts.len(), 2);
        assert!(texts.contains(&"Hello"));
        assert!(texts.contains(&"World"));
    }

    #[test]
    fn test_state_add_text() {
        let mut state = KeyboardState::default();
        assert!(!state.text.contains("Typing"));

        state.add_text("Typing");
        assert!(state.text.contains("Typing"));
    }

    #[test]
    fn test_state_press_and_release() {
        let mut state = KeyboardState::default();

        assert!(!state.is_pressed(KeyCode::KeyG));
        assert!(!state.is_down(KeyCode::KeyG));
        assert!(!state.is_released(KeyCode::KeyG));

        state.press(KeyCode::KeyG);
        assert!(state.is_pressed(KeyCode::KeyG));
        assert!(state.is_down(KeyCode::KeyG));
        assert!(!state.is_released(KeyCode::KeyG));

        state.release(KeyCode::KeyG);
        assert!(!state.is_pressed(KeyCode::KeyG));
        assert!(!state.is_down(KeyCode::KeyG));
        assert!(state.is_released(KeyCode::KeyG));
    }

    #[test]
    fn test_state_are_pressed_are_released_are_down() {
        let mut state = KeyboardState::default();
        state.press(KeyCode::KeyH);
        state.press(KeyCode::KeyI);

        let pressed = state.are_pressed(&[KeyCode::KeyH, KeyCode::KeyJ, KeyCode::KeyI]);
        assert_eq!(pressed, [true, false, true]);

        state.release(KeyCode::KeyI);

        let released = state.are_released(&[KeyCode::KeyH, KeyCode::KeyI]);
        assert_eq!(released, [false, true]);

        let down = state.are_down(&[KeyCode::KeyH, KeyCode::KeyI]);
        assert_eq!(down, [true, false]);
    }

    #[test]
    fn test_state_tick() {
        let mut state = KeyboardState::default();
        state.press(KeyCode::KeyL);
        state.press(KeyCode::KeyM);
        state.add_text("Clearing");

        state.tick();

        assert!(state.pressed.is_empty());
        assert!(state.released.is_empty());
        assert!(state.text.is_empty());
    }
}
