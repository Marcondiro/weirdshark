///Generic undirected filter
///
///Model a filter on sortable element, where the filter select a List of accepted values or a Range. The filter on single value can be brought as one element list.
///
/// Note: range filters adopt the IP addresses convention for ranges, where both extremes are included in it
///Example: Range(192.168.0.0 - 192.168.0.255) will include 192.168.0.255
#[derive(Clone)]
pub struct Filter<T> {
    inner: InnerFilter<T>,
}

///Generic directed filter
///
/// Model the direction of a `Filter`, by applying it only on the source or destination. To have all kinds of filters the `Both` version works like `Filter`
#[derive(Clone)]
pub struct DirectedFilter<T> {
    inner: InnerDirectedFilter<T>,
}

impl<T: PartialOrd + PartialEq> Filter<T> {

    ///Filter list builder
    pub fn from_vec(vec: Vec<T>) -> Self {
        Self { inner: InnerFilter::List(vec) }
    }

    ///Filter range builder
    /// starting and ending will be automatically ordered if written not in order
    pub fn from_range(starting: T, ending: T) -> Self {
        return if starting < ending {
            Self { inner: InnerFilter::Range(starting, ending) }
        } else {
            Self { inner: InnerFilter::Range(ending, starting) }
        };
    }
}

impl<T: PartialOrd + PartialEq> DirectedFilter<T> {
    ///Build a filter that only look at source field
    pub fn only_source(filter: Filter<T>) -> Self {
        Self { inner: InnerDirectedFilter::Source(filter.inner) }
    }

    ///Build a filter that only look at destination field
    pub fn only_destination(filter: Filter<T>) -> Self {
        Self { inner: InnerDirectedFilter::Destination(filter.inner) }
    }

    ///Build an bidirectional `DirectedFilter`
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

#[cfg(test)]
mod tests {
    use crate::{DirectedFilter, Filter};

    #[test]
    fn filter_single_element() {
        let filter = DirectedFilter::both_directions(Filter::from_vec(vec![10]));
        assert_eq!(filter.filter(&10, &11), true);
        assert_eq!(filter.filter(&12, &11), false);
    }

    #[test]
    fn filter_list() {
        let filter = DirectedFilter::only_source(Filter::from_vec(vec![10, 20, 30, 40]));
        assert_eq!(filter.filter(&0, &10), false);
        assert_eq!(filter.filter(&30, &35), true);
        assert_eq!(filter.filter(&40, &20), true);
    }

    #[test]
    fn filter_range() {
        let filter = DirectedFilter::only_destination(Filter::from_range(20, 40));
        assert_eq!(filter.filter(&0, &50), false);
        assert_eq!(filter.filter(&30, &35), true);
        assert_eq!(filter.filter(&40, &20), true);
    }

    #[test]
    fn range_limits_are_included() {
        let filter = Filter::from_range(20, 40);
        assert_eq!(filter.inner.filter(&20), true);
        assert_eq!(filter.inner.filter(&40), true);
        assert_eq!(filter.inner.filter(&41), false);
    }

    #[test]
    fn range_is_never_inverted() {
        let filter1 = Filter::from_range(20, 40);
        let filter2 = Filter::from_range(40, 20);
        assert_eq!(filter1.inner.filter(&21), true);
        assert_eq!(filter1.inner.filter(&41), false);
        assert_eq!(filter2.inner.filter(&21), true);
        assert_eq!(filter2.inner.filter(&41), false);
    }
}