use crate::counter::CounterState;
use crate::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[lera::state]
pub struct ManualOnlyCounterState {
    pub count: i64,
}

#[lera::model(state = ManualOnlyCounterState)]
pub struct ManualOnlyCounter {}

// Exported API
#[lera::api]
impl ManualOnlyCounter {
    pub fn increment_button_tapped(self: &Arc<Self>) {
        debug!("Incrementing counter");
        self.mutate(|state| {
            state.count += 1;
        });
    }

    pub fn decrement_button_tapped(self: &Arc<Self>) {
        self.mutate(|state| {
            state.count -= 1;
        });
    }

    pub fn reset_button_tapped(self: &Arc<Self>) {
        self.mutate(|state| {
            state.count = 0;
        });
    }

    pub fn tell_full_name(&self, first_name: &str, last_name: &str) -> String {
        format!("{} {}", first_name, last_name)
    }

    #[allow(clippy::too_many_arguments)]
    #[lera::default_params(
        should_throw = false,
        i8 = -8,
        optional_i8 = None,
        u8 = 8,
        i16,
        u16 = 16,
        i32 = -32,
        u32,
        i64 = -64,
        u64,
        s = "hello",
        string = "world",
        bytes,
        vec,
        hash_map,
    )]
    pub async fn cover_all(
        &self,
        should_throw: bool,
        i8: i8,
        optional_i8: Option<i8>,
        u8: u8,
        i16: i16,
        u16: u16,
        i32: i32,
        u32: u32,
        i64: i64,
        u64: u64,
        f32: f32,
        f64: f64,
        s: &str,
        string: String,
        bytes: &[u8],
        vec: Vec<u16>,
        hash_map: HashMap<String, u16>,
        custom_record: ManualOnlyCounterState,
        optional_other_custom_record: Option<CounterState>,
        deep_map: HashMap<ManualOnlyCounterState, Vec<Option<Vec<Option<CounterState>>>>>,
    ) -> Result<HashState, HashStateError> {
        use std::hash::Hash;
        use std::hash::Hasher;
        if should_throw {
            return Err(HashStateError::Unknown);
        }
        // hash all types such that unique input of all types produces unique output
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        i8.hash(&mut hasher);
        if let Some(optional_i8) = optional_i8 {
            optional_i8.hash(&mut hasher);
        }
        u8.hash(&mut hasher);
        i16.hash(&mut hasher);
        u16.hash(&mut hasher);
        i32.hash(&mut hasher);
        u32.hash(&mut hasher);
        i64.hash(&mut hasher);
        u64.hash(&mut hasher);
        f32.to_bits().hash(&mut hasher);
        f64.to_bits().hash(&mut hasher);
        s.hash(&mut hasher);
        string.hash(&mut hasher);
        bytes.hash(&mut hasher);
        vec.hash(&mut hasher);
        let mut sorted_map: Vec<(&String, &u16)> = hash_map.iter().collect();
        sorted_map.sort_by(|a, b| a.0.cmp(b.0));
        sorted_map.iter().for_each(|(k, v)| {
            k.hash(&mut hasher);
            v.hash(&mut hasher);
        });
        custom_record.hash(&mut hasher);
        if let Some(optional_other_custom_record) = optional_other_custom_record {
            optional_other_custom_record.hash(&mut hasher);
        }
        let mut sorted_deep: Vec<_> = deep_map.iter().collect();
        sorted_deep
            .sort_by(|(a_key, _), (b_key, _)| format!("{:?}", a_key).cmp(&format!("{:?}", b_key)));
        for (key, value) in sorted_deep {
            key.hash(&mut hasher);
            value.hash(&mut hasher);
        }
        let hash_str = format!("{:016x}", hasher.finish());
        Ok(HashState { hash: hash_str })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, uniffi::Record)]
pub struct HashState {
    pub hash: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, uniffi::Error, thiserror::Error)]
pub enum HashStateError {
    #[default]
    #[error("Unknown error")]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counter::Interval;

    #[allow(clippy::approx_constant)]
    #[actix_rt::test]
    async fn test_cover_all() {
        let model = ManualOnlyCounter::default();

        let result = model
            .cover_all(
                false,
                -8,
                Some(-8),
                8,
                -16,
                16,
                -32,
                32,
                -64,
                64,
                3.14,
                2.718,
                "hello",
                "world".to_string(),
                &[1, 2, 3],
                vec![1, 2, 3],
                [("one".to_string(), 1), ("two".to_string(), 2)]
                    .iter()
                    .cloned()
                    .collect(),
                ManualOnlyCounterState { count: 42 },
                Some(CounterState {
                    count: 100,
                    is_auto_incrementing: true,
                    auto_increment_interval_ms: Interval::try_from(500).unwrap(),
                }),
                [
                    (
                        ManualOnlyCounterState { count: 1 },
                        vec![
                            Some(vec![
                                Some(CounterState {
                                    count: 5,
                                    is_auto_incrementing: false,
                                    auto_increment_interval_ms: Interval::try_from(250).unwrap(),
                                }),
                                None,
                            ]),
                            None,
                        ],
                    ),
                    (
                        ManualOnlyCounterState { count: 2 },
                        vec![Some(vec![Some(CounterState::default())])],
                    ),
                ]
                .into_iter()
                .collect(),
            )
            .await;

        assert!(result.is_ok());
        let hash_state = result.unwrap();
        debug!("HashState: {:?}", hash_state);
        assert_eq!(hash_state.hash, "95e1c734f1ee2f27".to_owned());
    }
}
