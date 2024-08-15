use super::Indicator;
use crate::handler::config::IndicatorState;
use async_trait::async_trait;

pub struct Ema;

#[async_trait]
impl Indicator for Ema {
    async fn calculate(&mut self, state: &mut IndicatorState, period: usize, value: f64) -> f64 {
        let alpha = 2.0 / (period as f64 + 1.0);
        let new_ema = match state.current_value {
            Some(ema) => alpha * value + (1.0 - alpha) * ema,
            None => value,
        };
        state.current_value = Some(new_ema);
        state.count += 1;
        new_ema
    }
}
