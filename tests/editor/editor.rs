#[cfg(test)]

mod tests {
    #[test]
    fn test_switch_tabs() {
        let args = Args::parse();
        let mut app = App::new(args).await;
    }
}
