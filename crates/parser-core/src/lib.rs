pub fn core_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_core_test() {
        assert!(core_ready());
    }
}
