use super::Metric;

fn weighted_sum(previous_value: u128, current_value: u128, previous_weight: f32, current_weight: f32) -> Result<u128, String> {
    if previous_weight + current_weight != 1.0 {
        Err(format!("previous_weight({}) + current_weight({}) != 1.0", previous_weight, current_weight))
    } else {
        let scale: u128 = 1_000_000;
        let prev_weight = (previous_weight * scale as f32).round() as u128;
        let curr_weight = (current_weight * scale as f32).round() as u128;
        Ok(
            previous_value
                .checked_mul(prev_weight)
                .and_then(|x| x.checked_div(scale))
                .ok_or("overflow when calculating previous value")?
            +
            current_value
                .checked_mul(curr_weight)
                .and_then(|x| x.checked_div(scale))
                .ok_or("overflow when calculating current_value")?
        )
    }
}

pub (crate) struct TwapInput {
    pub (crate) timestamp: u64,
    pub (crate) price: u128
}

pub(crate) struct TwapValue {
    pub (crate) timestamp: u64,
    pub (crate) value: u128
}

pub(crate) struct TwapMetric {
    period: u64,
    current_value: u128,
    last_timestamp: u64
}

impl TwapMetric {
    pub(crate) fn new(period: u64) -> Self {
        Self {
            period,
            current_value: 0,
            last_timestamp: 0
        }
    }
}

impl Metric<TwapValue, TwapInput> for TwapMetric {
    fn update(&mut self, new_value: TwapInput) -> Result<Option<TwapValue>, String> {
        
        let previous_value = self.current_value;
        let current_value = new_value.price;
        let previous_timestamp = self.last_timestamp;
        let current_timestamp = new_value.timestamp;

        if previous_timestamp == 0 && current_timestamp == 0 {
            return Ok(None);
        }

        let previous_hour = previous_timestamp.div_euclid(self.period) * self.period;
        let current_hour = current_timestamp.div_euclid(self.period) * self.period;
        
        match previous_hour.cmp(&current_hour) {
            std::cmp::Ordering::Equal => {
                let observed_period = current_timestamp - previous_hour;
                let previous_weight = (previous_timestamp - previous_hour) as f32 / observed_period as f32;
    
                self.last_timestamp = current_timestamp;
                self.current_value = weighted_sum(previous_value, current_value, previous_weight, 1.0 - previous_weight).unwrap();
                Ok(None)
            }

            std::cmp::Ordering::Less => {
                let previous_weight = (previous_timestamp - previous_hour) as f32 / self.period as f32;
                let previous_closed_value = weighted_sum(previous_value, current_value, previous_weight, 1.0 - previous_weight).unwrap();
                
                self.last_timestamp = current_timestamp;
                self.current_value = current_value;
                Ok(Some(TwapValue{timestamp: previous_hour, value: previous_closed_value}))
            }

            std::cmp::Ordering::Greater => {
                Err(format!("previous_hour({}, {}) > current_hour({}, {})", previous_hour, previous_timestamp, current_hour, current_timestamp))
            }
        }
    }

    fn current(&self) -> TwapValue {
        TwapValue{timestamp: self.last_timestamp, value: self.current_value}
    }
}


#[cfg(test)]
mod tests {
    use rstest::rstest;
    use crate::metrics::{twap::{TwapInput, TwapMetric}, Metric};

    use super::weighted_sum;

    #[rstest]
    #[case(100, 200, 0.5, 0.5, 150)]
    #[case(100, 200, 0.0, 1.0, 200)]
    #[case(100, 200, 1.0, 0.0, 100)]
    fn test_weighted_sum(
        #[case] previous_value: u128,
        #[case] current_value: u128,
        #[case] previous_weight: f32,
        #[case] current_weight: f32,
        #[case] expected: u128
    ) {
        let result = weighted_sum(previous_value, current_value, previous_weight, current_weight).unwrap();
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case(TwapInput{timestamp: 100, price: 100}, TwapInput{timestamp: 200, price: 120}, TwapInput{timestamp: 400, price: 130}, 120, None)]
    #[case(TwapInput{timestamp: 1800, price: 100}, TwapInput{timestamp: 3599, price: 120}, TwapInput{timestamp: 3800, price: 130}, 130, Some(108))]
    #[case(TwapInput{timestamp: 1800, price: 100}, TwapInput{timestamp: 3800, price: 120}, TwapInput{timestamp: 4000, price: 130}, 125, None)]
    #[case(TwapInput{timestamp: 1800, price: 100}, TwapInput{timestamp: 1900, price: 120}, TwapInput{timestamp: 7300, price: 130}, 130, Some(113))]
    fn test_update(
        #[case] input_a: TwapInput,
        #[case] input_b: TwapInput,
        #[case] input_c: TwapInput,
        #[case] expected_curr: u128,
        #[case] expected_prev: Option<u128>
    ) {
        let mut twap_metric = TwapMetric::new(3600);
        twap_metric.update(input_a).unwrap();
        twap_metric.update(input_b).unwrap();
        let output_c = twap_metric.update(input_c).unwrap();

        let result = twap_metric.current();
        assert_eq!(result.value, expected_curr);
        
        match expected_prev {
            Some(value) => {
                match output_c {
                    Some(value_c) => {
                        assert_eq!(value_c.value, value);
                    }
                    None => {
                        assert_eq!(expected_prev.is_none(), output_c.is_none());
                    }
                }
                
            }
            None => {
                assert!(output_c.is_none())
            }
        }
    }
}

