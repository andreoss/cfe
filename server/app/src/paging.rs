use domain::Page;

pub struct Paged<T> {
    pub items: Vec<T>,
    pub page: Page,
    pub total: u64,
}

impl<T> Paged<T> {
    pub fn new(items: Vec<T>, page: Page, total: u64) -> Self {
        Self { items, page, total }
    }

    pub fn total_pages(&self) -> u64 {
        self.page.total_pages(self.total)
    }

    pub fn has_next(&self) -> bool {
        self.page.has_next(self.total)
    }

    pub fn has_previous(&self) -> bool {
        self.page.number() > 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paged(items: usize, number: u32, size: u32, total: u64) -> Paged<u32> {
        Paged::new(
            (0..items as u32).collect(),
            Page::parse(number, size).unwrap(),
            total,
        )
    }

    #[test]
    fn reports_where_it_sits_in_the_whole_list() {
        let first = paged(10, 1, 10, 25);
        assert_eq!(first.total_pages(), 3);
        assert!(first.has_next());
        assert!(!first.has_previous());

        let middle = paged(10, 2, 10, 25);
        assert!(middle.has_next());
        assert!(middle.has_previous());

        let last = paged(5, 3, 10, 25);
        assert!(!last.has_next());
        assert!(last.has_previous());
    }

    #[test]
    fn an_empty_list_is_one_page_with_nothing_after_it() {
        let empty = paged(0, 1, 10, 0);
        assert_eq!(empty.total_pages(), 1);
        assert!(!empty.has_next());
        assert!(!empty.has_previous());
    }
}
