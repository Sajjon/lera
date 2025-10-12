use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::hash::Hash;

pub use itertools;

pub type SampleIter<T> = Box<dyn Iterator<Item = T>>;

pub trait Samples: Sized + Clone + 'static {
    fn samples() -> SampleIter<Self>;

    /// Collects at max 255 samples into a Vec.
    /// This is useful for types that are used in collections, where we want to limit the
    /// number of samples to avoid combinatorial explosion.
    /// If the type has fewer samples, all samples will be collected.
    /// If the type has no samples, an empty Vec will be returned.
    /// If the type has more than 255 samples, only the first 255 will be collected.
    /// This is primarily intended for use in implementations of Samples for collections.
    /// For other use cases, consider using `Self::samples().take(n).collect()` directly.
    fn sample_vec_n(max_count: u8) -> Vec<Self> {
        Self::samples().take(max_count.into()).collect()
    }

    fn sample_vec() -> Vec<Self> {
        Self::sample_vec_n(255)
    }
}

impl Samples for bool {
    fn samples() -> SampleIter<Self> {
        Box::new([true, false].into_iter())
    }
}

impl Samples for char {
    fn samples() -> SampleIter<Self> {
        Box::new(['a', 'Z', '0', ' ', '~'].into_iter())
    }
}

impl Samples for () {
    fn samples() -> SampleIter<Self> {
        Box::new([()].into_iter())
    }
}

macro_rules! impl_samples_signed {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Samples for $ty {
                fn samples() -> SampleIter<Self> {
                    let divisor = <$ty>::BITS as $ty;
                    Box::new([
                        <$ty>::MIN / divisor,
                        0 as $ty,
                        <$ty>::MAX / divisor,
                    ].into_iter())
                }
            }
        )*
    };
}

macro_rules! impl_samples_unsigned {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Samples for $ty {
                fn samples() -> SampleIter<Self> {
                    let divisor = <$ty>::BITS as $ty;
                    Box::new([
                        <$ty>::MIN / divisor,
                        (<$ty>::MAX / 2) as $ty,
                        <$ty>::MAX / divisor,
                    ].into_iter())
                }
            }
        )*
    };
}

impl_samples_signed!(i8, i16, i32, i64, i128, isize);
impl_samples_unsigned!(u8, u16, u32, u64, u128, usize);

impl Samples for f32 {
    fn samples() -> SampleIter<Self> {
        Box::new([-std::f32::consts::E, 0.0, std::f32::consts::PI].into_iter())
    }
}

impl Samples for f64 {
    fn samples() -> SampleIter<Self> {
        Box::new([-std::f64::consts::E, 0.0, std::f64::consts::PI].into_iter())
    }
}

pub const SHORT_STRING: &str = "hello";
pub const LONG_STRING: &str = "super long string that tests that UI is smart enough to make accommodations for such long strings like this";

impl Samples for String {
    fn samples() -> SampleIter<Self> {
        Box::new(
            vec![
                SHORT_STRING.to_string(),
                LONG_STRING.to_string(),
                String::new(),
            ]
            .into_iter(),
        )
    }
}

impl Samples for &'static str {
    fn samples() -> SampleIter<Self> {
        Box::new(vec![SHORT_STRING, LONG_STRING, ""].into_iter())
    }
}

impl<T: Samples> Samples for Option<T> {
    fn samples() -> SampleIter<Self> {
        let mut source = T::samples();
        if let Some(first) = source.next() {
            Box::new(vec![Some(first), None].into_iter())
        } else {
            Box::new(std::iter::once(None))
        }
    }
}

impl<T: Samples, E: Samples> Samples for Result<T, E> {
    fn samples() -> SampleIter<Self> {
        let ok_samples: Vec<T> = T::samples().take(2).collect();
        let err_samples: Vec<E> = E::samples().take(2).collect();
        let mut out = Vec::new();
        if let Some(ok) = ok_samples.first() {
            out.push(Ok(ok.clone()));
        }
        if let Some(err) = err_samples.first() {
            out.push(Err(err.clone()));
        }
        if out.is_empty() {
            if let Some(err) = err_samples.get(1).or_else(|| err_samples.first()) {
                out.push(Err(err.clone()));
            } else if let Some(ok) = ok_samples.get(1).or_else(|| ok_samples.first()) {
                out.push(Ok(ok.clone()));
            }
        }
        Box::new(out.into_iter())
    }
}

impl<T: Samples> Samples for Vec<T> {
    fn samples() -> SampleIter<Self> {
        let elems: Vec<T> = T::samples().take(3).collect();
        if elems.is_empty() {
            return Box::new(std::iter::once(Vec::new()));
        }
        let mut out = Vec::new();
        out.push(vec![elems[0].clone()]);
        out.push(Vec::new());
        if elems.len() > 1 {
            let aggregated: Vec<_> = elems.iter().take(3).cloned().collect();
            if aggregated.len() > 1 {
                out.push(aggregated);
            }
        }
        Box::new(out.into_iter())
    }
}

impl<T: Samples> Samples for VecDeque<T> {
    fn samples() -> SampleIter<Self> {
        let elems: Vec<T> = T::samples().take(3).collect();
        if elems.is_empty() {
            return Box::new(std::iter::once(VecDeque::new()));
        }
        let mut out = Vec::new();
        out.push(VecDeque::from(vec![elems[0].clone()]));
        out.push(VecDeque::new());
        if elems.len() > 1 {
            let collected: Vec<_> = elems.iter().take(3).cloned().collect();
            if collected.len() > 1 {
                out.push(VecDeque::from(collected));
            }
        }
        Box::new(out.into_iter())
    }
}

impl<T: Samples> Samples for LinkedList<T> {
    fn samples() -> SampleIter<Self> {
        let elems: Vec<T> = T::samples().take(3).collect();
        if elems.is_empty() {
            return Box::new(std::iter::once(LinkedList::new()));
        }
        let mut out = Vec::new();
        let mut singleton = LinkedList::new();
        singleton.push_back(elems[0].clone());
        out.push(singleton);
        out.push(LinkedList::new());
        if elems.len() > 1 {
            let mut list = LinkedList::new();
            for item in elems.iter().take(3) {
                list.push_back(item.clone());
            }
            if list.len() > 1 {
                out.push(list);
            }
        }
        Box::new(out.into_iter())
    }
}

pub mod __private {
    /// Trait implemented for acceptable return types of validation functions used by the
    /// `#[samples(... -> const_fn)]` attribute.
    pub trait ConstValidateReturn<Parent> {
        type Error: ::core::fmt::Debug;
        const IS_RESULT: bool;
    }

    impl<Parent> ConstValidateReturn<Parent> for Parent {
        type Error = ::core::convert::Infallible;
        const IS_RESULT: bool = false;
    }

    impl<Parent, E> ConstValidateReturn<Parent> for ::core::result::Result<Parent, E>
    where
        E: ::core::fmt::Debug,
    {
        type Error = E;
        const IS_RESULT: bool = true;
    }

    pub const fn const_result_is_err<Parent, Return>(value: &Return) -> bool
    where
        Return: ConstValidateReturn<Parent>,
    {
        if <Return as ConstValidateReturn<Parent>>::IS_RESULT {
            let result_ref: &::core::result::Result<
                Parent,
                <Return as ConstValidateReturn<Parent>>::Error,
            > = unsafe {
                &*(value as *const Return
                    as *const ::core::result::Result<
                        Parent,
                        <Return as ConstValidateReturn<Parent>>::Error,
                    >)
            };
            (*result_ref).is_err()
        } else {
            false
        }
    }
}

impl<T: Samples + Eq + Hash> Samples for HashSet<T> {
    fn samples() -> SampleIter<Self> {
        let elems: Vec<T> = T::samples().take(3).collect();
        if elems.is_empty() {
            return Box::new(std::iter::once(HashSet::new()));
        }
        let mut out = Vec::new();
        out.push(HashSet::from([elems[0].clone()]));
        out.push(HashSet::new());
        if elems.len() > 1 {
            let aggregated: HashSet<_> = elems.iter().take(3).cloned().collect();
            if aggregated.len() > 1 {
                out.push(aggregated);
            }
        }
        Box::new(out.into_iter())
    }
}

impl<T: Samples + Ord> Samples for BTreeSet<T> {
    fn samples() -> SampleIter<Self> {
        let elems: Vec<T> = T::samples().take(3).collect();
        if elems.is_empty() {
            return Box::new(std::iter::once(BTreeSet::new()));
        }
        let mut out = Vec::new();
        out.push(BTreeSet::from([elems[0].clone()]));
        out.push(BTreeSet::new());
        if elems.len() > 1 {
            let aggregated: BTreeSet<_> = elems.iter().take(3).cloned().collect();
            if aggregated.len() > 1 {
                out.push(aggregated);
            }
        }
        Box::new(out.into_iter())
    }
}

impl<T: Samples + Ord> Samples for BinaryHeap<T> {
    fn samples() -> SampleIter<Self> {
        let elems: Vec<T> = T::samples().take(3).collect();
        if elems.is_empty() {
            return Box::new(std::iter::once(BinaryHeap::new()));
        }
        let mut out = Vec::new();
        out.push(BinaryHeap::from(vec![elems[0].clone()]));
        out.push(BinaryHeap::new());
        if elems.len() > 1 {
            let collected: Vec<_> = elems.iter().take(3).cloned().collect();
            if collected.len() > 1 {
                out.push(BinaryHeap::from(collected));
            }
        }
        Box::new(out.into_iter())
    }
}

impl<K, V> Samples for HashMap<K, V>
where
    K: Samples + Eq + Hash,
    V: Samples,
{
    fn samples() -> SampleIter<Self> {
        let keys: Vec<K> = K::samples().take(3).collect();
        let values: Vec<V> = V::samples().take(3).collect();
        if keys.is_empty() || values.is_empty() {
            return Box::new(std::iter::once(HashMap::new()));
        }
        let mut out = Vec::new();
        out.push(HashMap::from([(keys[0].clone(), values[0].clone())]));
        let mut map = HashMap::new();
        for (k, v) in keys.iter().cloned().zip(values.iter().cloned()) {
            map.insert(k, v);
            if map.len() == 3 {
                break;
            }
        }
        if map.len() > 1 {
            out.push(map);
        }
        out.push(HashMap::new());
        Box::new(out.into_iter())
    }
}

impl<K, V> Samples for BTreeMap<K, V>
where
    K: Samples + Ord,
    V: Samples,
{
    fn samples() -> SampleIter<Self> {
        let keys: Vec<K> = K::samples().take(3).collect();
        let values: Vec<V> = V::samples().take(3).collect();
        if keys.is_empty() || values.is_empty() {
            return Box::new(std::iter::once(BTreeMap::new()));
        }
        let mut out = Vec::new();
        out.push(BTreeMap::from([(keys[0].clone(), values[0].clone())]));
        let mut map = BTreeMap::new();
        for (k, v) in keys.iter().cloned().zip(values.iter().cloned()) {
            map.insert(k, v);
            if map.len() == 3 {
                break;
            }
        }
        if map.len() > 1 {
            out.push(map);
        }
        out.push(BTreeMap::new());
        Box::new(out.into_iter())
    }
}
