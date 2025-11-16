#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    number: u32,
    size: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PageError {
    NumberBelowOne,
    SizeBelowOne,
    SizeTooLarge,
}

pub const DEFAULT_SIZE: u32 = 25;
pub const MAX_SIZE: u32 = 100;

impl Page {
    pub fn parse(number: u32, size: u32) -> Result<Self, PageError> {
        if number < 1 {
            return Err(PageError::NumberBelowOne);
        }
        if size < 1 {
            return Err(PageError::SizeBelowOne);
        }
        if size > MAX_SIZE {
            return Err(PageError::SizeTooLarge);
        }
        Ok(Self { number, size })
    }

    pub fn first() -> Self {
        Self {
            number: 1,
            size: DEFAULT_SIZE,
        }
    }

    pub fn number(&self) -> u32 {
        self.number
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn offset(&self) -> u64 {
        (self.number as u64 - 1) * self.size as u64
    }

    pub fn limit(&self) -> u64 {
        self.size as u64
    }

    pub fn total_pages(&self, total: u64) -> u64 {
        if total == 0 {
            return 1;
        }
        total.div_ceil(self.size as u64)
    }

    pub fn has_next(&self, total: u64) -> bool {
        (self.number as u64) < self.total_pages(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_page_starts_at_zero() {
        let page = Page::first();
        assert_eq!(page.number(), 1);
        assert_eq!(page.size(), DEFAULT_SIZE);
        assert_eq!(page.offset(), 0);
        assert_eq!(page.limit(), DEFAULT_SIZE as u64);
    }

    #[test]
    fn a_later_page_skips_the_pages_before_it() {
        let page = Page::parse(3, 10).unwrap();
        assert_eq!(page.offset(), 20);
        assert_eq!(page.limit(), 10);
    }

    #[test]
    fn rejects_a_page_before_the_first() {
        assert_eq!(Page::parse(0, 10), Err(PageError::NumberBelowOne));
    }

    #[test]
    fn rejects_a_size_that_is_empty_or_unbounded() {
        assert_eq!(Page::parse(1, 0), Err(PageError::SizeBelowOne));
        assert_eq!(Page::parse(1, MAX_SIZE + 1), Err(PageError::SizeTooLarge));
        assert!(Page::parse(1, MAX_SIZE).is_ok());
    }

    #[test]
    fn counts_pages_from_a_total() {
        let page = Page::parse(1, 10).unwrap();
        assert_eq!(page.total_pages(0), 1);
        assert_eq!(page.total_pages(1), 1);
        assert_eq!(page.total_pages(10), 1);
        assert_eq!(page.total_pages(11), 2);
        assert_eq!(page.total_pages(25), 3);
    }

    #[test]
    fn knows_whether_another_page_follows() {
        let first = Page::parse(1, 10).unwrap();
        assert!(first.has_next(11));
        assert!(!first.has_next(10));
        assert!(!first.has_next(0));
        let second = Page::parse(2, 10).unwrap();
        assert!(!second.has_next(20));
        assert!(second.has_next(21));
    }

    #[test]
    fn a_page_past_the_end_simply_has_no_next() {
        let far = Page::parse(99, 10).unwrap();
        assert!(!far.has_next(20));
        assert_eq!(far.offset(), 980);
    }
}
