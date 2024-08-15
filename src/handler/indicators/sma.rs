use super::Indicator;
use crate::handler::config::IndicatorState;
use async_trait::async_trait;

pub struct Sma;

#[async_trait]
impl Indicator for Sma {
    async fn calculate(&mut self, state: &mut IndicatorState, period: usize, value: f64) -> f64 {
        state.values.push(value);
        if state.values.len() > period {
            state.values.remove(0);
        }
        state.count += 1;
        if state.values.len() == period {
            state.values.iter().sum::<f64>() / period as f64
        } else {
            value
        }
    }
}
