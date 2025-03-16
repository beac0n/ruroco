#[cfg(test)]
mod tests {
    use std::{env, fs};

    use ruroco::common::common::get_blocklist_path;
    use ruroco::server::blocklist::Blocklist;

    fn create_blocklist() -> Blocklist {
        remove_blocklist();
        Blocklist::create(&env::current_dir().unwrap())
    }

    fn remove_blocklist() {
        let _ = fs::remove_file(get_blocklist_path(&env::current_dir().unwrap()));
    }

    #[test]
    fn test_add() {
        let mut blocklist = create_blocklist();
        let number: u128 = 42;
        let another_number: u128 = 1337;

        blocklist.add(number);
        assert_eq!(blocklist.get().len(), 1);
        assert_eq!(blocklist.get().first().unwrap().clone(), number);

        blocklist.add(another_number);
        assert_eq!(blocklist.get().len(), 2);
        assert_eq!(blocklist.get().first().unwrap().clone(), number);
        assert_eq!(blocklist.get().get(1).unwrap().clone(), another_number);

        remove_blocklist();
    }

    #[test]
    fn test_clean() {
        let mut blocklist = create_blocklist();

        blocklist.add(21);
        blocklist.add(42);
        blocklist.add(63);
        blocklist.add(84);
        blocklist.add(105);

        assert_eq!(blocklist.get().len(), 5);

        blocklist.clean(63);
        assert_eq!(blocklist.get().len(), 2);
        assert_eq!(blocklist.get().first().unwrap().clone(), 84);
        assert_eq!(blocklist.get().get(1).unwrap().clone(), 105);

        remove_blocklist();
    }

    #[test]
    fn test_save() {
        let mut blocklist = create_blocklist();

        blocklist.add(42);
        blocklist.save();
        blocklist.add(1337);

        let other_blocklist = Blocklist::create(&env::current_dir().unwrap());
        assert_eq!(other_blocklist.get().len(), 1);
        assert_eq!(other_blocklist.get().first().unwrap().clone(), 42);

        remove_blocklist();
    }

    #[test]
    fn test_is_blocked() {
        let mut blocklist = create_blocklist();

        blocklist.add(42);

        assert!(blocklist.is_blocked(42));
        assert!(!blocklist.is_blocked(1337));

        remove_blocklist();
    }
}
