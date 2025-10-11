use core::f32;
use samples_core::{Samples, LONG_STRING, SHORT_STRING};
use samples_derive::Samples;
use std::collections::{BTreeSet, HashSet};

#[derive(Samples, Clone, Debug, PartialEq, Hash, Eq, PartialOrd, Ord)]
struct MyStruct {
    #[samples(Default::default())]
    bool: bool,
    #[samples([-3, -1, 5])]
    i8: i8,
    s: String,
}

#[test]
fn try_from_syntax_is_possible() {
    /// A non zero interval in milliseconds
    #[derive(Clone, Debug, PartialEq, Eq, Hash, Samples)]
    pub struct Interval {
        #[samples([500, 1000] -> const_try_from)]
        ms: u64,
    }

    impl Interval {
        pub const fn const_try_from(value: u64) -> Result<Self, &'static str> {
            if value == 0 {
                Err("Interval must be non-zero")
            } else {
                Ok(Interval { ms: value })
            }
        }
    }

    let samples: Vec<Interval> = Interval::sample_vec();
    assert_eq!(samples.len(), 2);
    assert_eq!(samples[0], Interval::const_try_from(500).unwrap());
    assert_eq!(samples[1], Interval::const_try_from(1000).unwrap());
}

#[test]
fn overrides_are_respected_in_nested_types() {
    #[derive(Samples, Clone, Debug, PartialEq)]
    struct WithoutOverrideLevel0 {
        a: bool,
    }
    assert_eq!(WithoutOverrideLevel0::samples().count(), 2);

    #[derive(Samples, Clone, Debug, PartialEq)]
    struct WithOverrideLevel0 {
        #[samples(false)]
        b: bool,
    }
    assert_eq!(WithOverrideLevel0::samples().count(), 1);

    #[derive(Samples, Clone, Debug, PartialEq)]
    struct WithoutOverrideLevel1 {
        c: bool,
        d: WithoutOverrideLevel0,
    }
    assert_eq!(WithoutOverrideLevel1::samples().count(), 4);

    #[derive(Samples, Clone, Debug, PartialEq)]
    struct WithOverrideLevel1VariantA {
        #[samples(false)]
        e: bool,
        f: WithOverrideLevel0,
    }
    assert_eq!(WithOverrideLevel1VariantA::samples().count(), 1);

    #[derive(Samples, Clone, Debug, PartialEq)]
    struct WithOverrideLevel1VariantB {
        g: bool,
        h: WithOverrideLevel0,
    }
    assert_eq!(WithOverrideLevel1VariantB::samples().count(), 2);

    impl WithOverrideLevel0 {
        fn x() -> Self {
            Self { b: false }
        }
        fn y() -> Self {
            Self { b: true }
        }
    }

    #[derive(Samples, Clone, Debug, PartialEq)]
    struct WithOverrideLevel1VariantC {
        i: bool,
        #[samples([WithOverrideLevel0::x(), WithOverrideLevel0::y()])]
        j: WithOverrideLevel0,
    }
    assert_eq!(WithOverrideLevel1VariantC::samples().count(), 4);

    #[derive(Samples, Clone, Debug, PartialEq)]
    struct WithOverrideLevel1VariantD {
        #[samples(true)]
        i: bool,
        #[samples([WithOverrideLevel0::x(), WithOverrideLevel0::y()])]
        j: WithOverrideLevel0,
    }
    assert_eq!(WithOverrideLevel1VariantD::samples().count(), 2);
}

#[test]
fn test_my_struct_cartesian_samples() {
    let samples = MyStruct::sample_vec();
    assert_eq!(
        samples.len(),
        1 * 3 * 3 // one bool override, three i8 overrides, three strings
    );

    let expected = {
        let mut out = Vec::new();
        for &bool in &[false] {
            for &i8 in &[-3, -1, 5] {
                for s in &[
                    SHORT_STRING.to_string(),
                    LONG_STRING.to_string(),
                    "".to_string(),
                ] {
                    out.push(MyStruct {
                        bool,
                        i8,
                        s: s.clone(),
                    });
                }
            }
        }
        out
    };

    assert_eq!(samples, expected);
}

#[test]
fn test_floats() {
    #[derive(Samples, Clone, Debug, PartialEq)]
    struct FloatStruct {
        #[samples([-1.2, -0.5, 237.42])]
        a: f32,
        #[samples(std::f64::consts::PI)]
        b: f64,
    }
    let samples = FloatStruct::sample_vec();
    let expected = {
        let mut out = Vec::new();
        for &a in &[-1.2, -0.5, 237.42] {
            out.push(FloatStruct {
                a,
                b: std::f64::consts::PI,
            });
        }
        out
    };
    assert_eq!(samples, expected);
}

#[test]
fn test_huge_nested_structs_compiles() {
    #[allow(unused)]
    #[derive(Samples, Clone, Debug, PartialEq)]
    struct HugeStructCompiles {
        a: bool,
        b: Vec<i8>,
        c: Vec<u8>,
        d: Vec<i16>,
        e: Vec<u16>,
        f: Vec<i32>,
        g: Vec<u32>,
        h: Vec<i64>,
        i: Vec<u64>,
        j: Vec<f32>,
        k: Vec<f64>,
        l: HashSet<i8>,
        m: HashSet<u8>,
        n: HashSet<i16>,
        o: HashSet<u16>,
        p: HashSet<i32>,
        q: HashSet<u32>,
        r: HashSet<i64>,
        s: HashSet<u64>,
        t: BTreeSet<i8>,
        u: BTreeSet<u8>,
        v: BTreeSet<i16>,
        w: BTreeSet<u16>,
        x: Vec<MyStruct>,
        y: HashSet<MyStruct>,
        z: BTreeSet<MyStruct>,
    }
    let _iter = HugeStructCompiles::samples();
}

#[test]
fn test_big_struct_cartesian_samples() {
    #[derive(Samples, Clone, Debug, PartialEq)]
    struct BigStruct {
        a: bool,
        b: Vec<i8>,
        c: Vec<u8>,
        n: HashSet<i16>,
        o: HashSet<u16>,
        v: BTreeSet<i32>,
        w: BTreeSet<u64>,
        x: Vec<MyStruct>,
        y: HashSet<MyStruct>,
        z: BTreeSet<MyStruct>,
    }
    let count = BigStruct::samples().count();
    assert_eq!(count, 39366);
}
