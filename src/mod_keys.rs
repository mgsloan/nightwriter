#[derive(Hash, PartialEq, Eq, Debug, Clone, Arbitrary)]
pub struct ModKeys {
    pub ctrl: bool,
    pub shift: bool,
}
