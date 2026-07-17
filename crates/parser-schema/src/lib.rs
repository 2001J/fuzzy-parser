pub fn schema_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_schema_test() {
        assert!(schema_ready());
    }
}
