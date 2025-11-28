#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct SplitEveryOtherIter<'a> {
    s: &'a str,
    pat: &'a str,
    include: bool,
    done: bool,
}

impl<'a> SplitEveryOtherIter<'a> {
    pub fn new(s: &'a str, pat: &'a str) -> Self {
        Self {
            s,
            pat,
            include: false,
            done: false,
        }
    }
}

impl<'a> Iterator for SplitEveryOtherIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.s.is_empty() {
            return None;
        }

        match self.s.split_once(self.pat) {
            None => {
                self.done = true;
                Some(self.s)
            }
            Some((value, rest)) => {
                let value = if self.include {
                    self.include = !self.include;
                    self.s = rest;
                    self.pat
                } else {
                    if rest.starts_with(self.pat) {
                        self.include = !self.include;
                    }

                    self.s = rest;

                    if value.is_empty() {
                        return self.next();
                    }

                    value
                };

                Some(value)
            }
        }
    }
}

pub trait SplitEveryOtherIterator {
    fn split_every_other<'a>(&'a self, pat: &'a str) -> SplitEveryOtherIter<'a>;
}

impl SplitEveryOtherIterator for &str {
    fn split_every_other<'a>(&'a self, pat: &'a str) -> SplitEveryOtherIter<'a> {
        SplitEveryOtherIter::new(self, pat)
    }
}
