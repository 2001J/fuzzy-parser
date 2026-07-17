pub fn formats_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_formats_test() {
        assert!(formats_ready());
    }
}
