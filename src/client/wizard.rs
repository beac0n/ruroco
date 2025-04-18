#[derive(Debug)]
pub struct Wizard {
    force: bool,
}

impl Wizard {
    pub fn create(force: bool) -> Self {
        Self { force }
    }

    pub fn run(&self) {}
}
