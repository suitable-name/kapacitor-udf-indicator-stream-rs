use super::config::IndicatorState;
use async_trait::async_trait;

pub mod ema;
pub mod sma;

#[async_trait]
pub trait Indicator: Send + Sync {
    async fn calculate(&mut self, state: &mut IndicatorState, period: usize, value: f64) -> f64;
}
