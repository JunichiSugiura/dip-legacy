use crate::Mode;

#[derive(Debug, Clone)]
pub enum CoreCommand {
    Click,
    Exit,
}

#[derive(Debug, Clone, Copy)]
pub enum UICommand {
    Mode(Mode),
}
