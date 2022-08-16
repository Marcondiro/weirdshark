pub enum Filter<T> {
    Single(T),
    Range(T, T),
    List(Vec<T>),
}

impl<T: PartialOrd + PartialEq> Filter<T> {
    pub fn from_vec(vec : Vec<T>) -> Self{
        Filter::List(vec)
    }

    pub fn from_range(starting: T, ending: T) -> Self{
        return if starting < ending {
            Filter::Range(starting,ending)
        } else {
            Filter::Range(ending,starting)
        }
    }

    pub (super) fn filter(&self, x: &T) -> bool {
        use Filter::{Single, Range, List};
        match self {
            Single(addr) => x == addr,
            Range(start_addr, end_addr) => start_addr <= x && x <= end_addr,
            List(list) => list.iter().any(|ip| { ip == x })
        }
    }
}

pub enum DirectedFilter<T> {
    Source(Filter<T>),
    Destination(Filter<T>),
    Both(Filter<T>),
}

impl<T: PartialOrd + PartialEq> DirectedFilter<T> {
    pub fn only_source(filter :Filter<T>) -> Self{
        DirectedFilter::Source(filter)
    }

    pub fn only_destination(filter :Filter<T>) -> Self{
        DirectedFilter::Destination(filter)
    }

    pub fn both_directions(filter :Filter<T>) -> Self{
        DirectedFilter::Both(filter)
    }

    pub (super) fn filter(&self, source: &T, destination: &T) -> bool {
        use DirectedFilter::{Both, Source, Destination};
        match self {
            Source(filter) => filter.filter(source),
            Destination(filter) => filter.filter(destination),
            Both(filter) => filter.filter(source) || filter.filter(destination),
        }
    }
}