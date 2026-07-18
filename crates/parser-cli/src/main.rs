fn main() {
    println!("parser-cli ready");
}

#[cfg(test)]
mod tests {
    #[test]
    fn empty_cli_test() {
        assert_eq!(2 + 2, 4);
    }
}
