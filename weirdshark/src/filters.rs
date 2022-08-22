#[derive(Clone)]
pub struct Filter<T> {
    inner: InnerFilter<T>,
}

#[derive(Clone)]
pub struct DirectedFilter<T> {
    inner: InnerDirectedFilter<T>,
}

impl<T: PartialOrd + PartialEq> Filter<T> {
    pub fn from_vec(vec: Vec<T>) -> Self {
        Self { inner: InnerFilter::List(vec) }
    }

    pub fn from_range(starting: T, ending: T) -> Self {
        return if starting < ending {
            Self { inner: InnerFilter::Range(starting, ending) }
        } else {
            Self { inner: InnerFilter::Range(ending, starting) }
        };
    }
}

impl<T: PartialOrd + PartialEq> DirectedFilter<T> {
    pub fn only_source(filter: Filter<T>) -> Self {
        Self { inner: InnerDirectedFilter::Source(filter.inner) }
    }

    pub fn only_destination(filter: Filter<T>) -> Self {
        Self { inner: InnerDirectedFilter::Destination(filter.inner) }
    }

    pub fn both_directions(filter: Filter<T>) -> Self {
        Self { inner: InnerDirectedFilter::Both(filter.inner) }
    }
    pub(super) fn filter(&self, source: &T, destination: &T) -> bool {
        self.inner.filter(source, destination)
    }
}

#[derive(Clone)]
enum InnerFilter<T> {
    Range(T, T),
    List(Vec<T>),
}

impl<T: PartialOrd + PartialEq> InnerFilter<T> {
    fn filter(&self, x: &T) -> bool {
        use InnerFilter::{Range, List};
        match self {
            Range(start_addr, end_addr) => start_addr <= x && x <= end_addr,
            List(list) => list.iter().any(|ip| { ip == x })
        }
    }
}

#[derive(Clone)]
enum InnerDirectedFilter<T> {
    Source(InnerFilter<T>),
    Destination(InnerFilter<T>),
    Both(InnerFilter<T>),
}

impl<T: PartialOrd + PartialEq> InnerDirectedFilter<T> {
    fn filter(&self, source: &T, destination: &T) -> bool {
        use InnerDirectedFilter::{Both, Source, Destination};
        match self {
            Source(filter) => filter.filter(source),
            Destination(filter) => filter.filter(destination),
            Both(filter) => filter.filter(source) || filter.filter(destination),
        }
    }
}