use super::config::{IndicatorOptions, IndicatorState, IndicatorType};
use async_std::{channel::Sender, sync::Mutex};
use async_trait::async_trait;
use kapacitor_udf::{
    proto::{
        response, BeginBatch, EdgeType, EndBatch, InfoResponse, InitRequest, InitResponse, Point,
        Response, RestoreRequest, RestoreResponse, SnapshotResponse,
    },
    traits::Handler,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io, sync::Arc};
use thiserror::Error;
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Error)]
pub enum IndicatorError {
    #[error("Failed to send response: {0}")]
    ResponseSendError(String),
    #[error("Invalid field type: expected double, got {0}")]
    InvalidFieldType(String),
    #[error("Missing ticker field: {0}")]
    MissingTickerField(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndicatorData {
    states: HashMap<String, IndicatorState>,
}

pub struct IndicatorHandler {
    responses: Arc<Mutex<Sender<Response>>>,
    options: IndicatorOptions,
    data: IndicatorData,
}

impl IndicatorHandler {
    #[instrument(skip(responses, options))]
    pub fn new(responses: Arc<Mutex<Sender<Response>>>, options: IndicatorOptions) -> Self {
        info!("Creating new IndicatorHandler");
        IndicatorHandler {
            responses,
            options: options.clone(),
            data: IndicatorData {
                states: HashMap::new(),
            },
        }
    }

    #[inline]
    fn calculate_ema(&mut self, ticker: &str, value: f64) -> f64 {
        let state = self
            .data
            .states
            .entry(ticker.to_string())
            .or_insert(IndicatorState {
                current_value: None,
                values: Vec::new(),
                count: 0,
            });

        let alpha = 2.0 / (self.options.period as f64 + 1.0);
        let new_ema = match state.current_value {
            Some(ema) => alpha * value + (1.0 - alpha) * ema,
            None => value,
        };
        state.current_value = Some(new_ema);
        state.count += 1;
        new_ema
    }

    #[inline]
    fn calculate_sma(&mut self, ticker: &str, value: f64) -> f64 {
        let state = self
            .data
            .states
            .entry(ticker.to_string())
            .or_insert(IndicatorState {
                current_value: None,
                values: Vec::new(),
                count: 0,
            });

        state.values.push(value);
        if state.values.len() > self.options.period as usize {
            state.values.remove(0);
        }
        state.count += 1;
        if state.values.len() == self.options.period as usize {
            state.values.iter().sum::<f64>() / self.options.period as f64
        } else {
            value
        }
    }

    #[inline]
    fn calculate_indicator(&mut self, ticker: &str, value: f64) -> f64 {
        let result = match self.options.indicator_type {
            IndicatorType::EMA => self.calculate_ema(ticker, value),
            IndicatorType::SMA => self.calculate_sma(ticker, value),
        };
        println!(
            "Ticker: {}, Input: {}, Output: {}, Type: {:?}",
            ticker, value, result, self.options.indicator_type
        );
        result
    }

    async fn send_response(&self, response: Response) -> Result<(), IndicatorError> {
        self.responses
            .lock()
            .await
            .send(response)
            .await
            .map_err(|e| IndicatorError::ResponseSendError(e.to_string()))
    }
}

#[async_trait]
impl Handler for IndicatorHandler {
    #[instrument(skip(self))]
    async fn info(&self) -> io::Result<InfoResponse> {
        debug!("Info request received");
        let info = InfoResponse {
            wants: EdgeType::Stream as i32,
            provides: EdgeType::Stream as i32,
            options: self.options.to_option_info(),
        };
        trace!("Responding with info: {:?}", info);
        Ok(info)
    }

    #[instrument(skip(self, r))]
    async fn init(&mut self, r: &InitRequest) -> io::Result<InitResponse> {
        debug!("Init request received: {:?}", r);
        match IndicatorOptions::from_proto_options(&r.options) {
            Ok(options) => {
                self.options = options;
                self.data.states.clear();
                Ok(InitResponse {
                    success: true,
                    error: String::new(),
                })
            }
            Err(e) => {
                error!("Failed to initialize: {}", e);
                Ok(InitResponse {
                    success: false,
                    error: e.to_string(),
                })
            }
        }
    }

    #[instrument(skip(self))]
    async fn snapshot(&self) -> io::Result<SnapshotResponse> {
        debug!("Snapshot request received");
        let snapshot = serde_json::to_vec(&self.data).map_err(|e| {
            error!("Failed to serialize state: {}", e);
            io::Error::new(io::ErrorKind::Other, e)
        })?;
        Ok(SnapshotResponse { snapshot })
    }

    #[instrument(skip(self, req))]
    async fn restore(&mut self, req: &RestoreRequest) -> io::Result<RestoreResponse> {
        debug!("Restore request received");
        match serde_json::from_slice(&req.snapshot) {
            Ok(data) => {
                self.data = data;
                Ok(RestoreResponse {
                    success: true,
                    error: String::new(),
                })
            }
            Err(e) => {
                error!("Failed to restore state: {}", e);
                Ok(RestoreResponse {
                    success: false,
                    error: e.to_string(),
                })
            }
        }
    }

    #[instrument(skip(self, begin))]
    async fn begin_batch(&mut self, begin: &BeginBatch) -> io::Result<()> {
        warn!(
            "BeginBatch called, but batching is not supported: {:?}",
            begin
        );
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Batching is not supported",
        ))
    }

    #[instrument(skip(self, p), fields(point_name = %p.name, point_time = %p.time))]
    async fn point(&mut self, p: &Point) -> io::Result<()> {
        trace!("Processing point: {:?}", p);

        let ticker = p.tags.get(&self.options.ticker_field).ok_or_else(|| {
            let err = IndicatorError::MissingTickerField(self.options.ticker_field.clone());
            error!("{}", err);
            io::Error::new(io::ErrorKind::InvalidData, err)
        })?;

        let value = p.fields_double.get(&self.options.field).ok_or_else(|| {
            let err = IndicatorError::InvalidFieldType(self.options.field.clone());
            error!("{}", err);
            io::Error::new(io::ErrorKind::InvalidData, err)
        })?;

        let indicator_value = self.calculate_indicator(ticker, *value);

        let mut new_point = p.clone();
        new_point
            .fields_double
            .insert(self.options.as_field.clone(), indicator_value);

        self.send_response(Response {
            message: Some(response::Message::Point(new_point)),
        })
        .await
        .map_err(|e| {
            error!("Failed to send point response: {}", e);
            io::Error::new(io::ErrorKind::Other, e)
        })?;

        Ok(())
    }

    #[instrument(skip(self, end))]
    async fn end_batch(&mut self, end: &EndBatch) -> io::Result<()> {
        debug!("EndBatch called: {:?}", end);
        Ok(())
    }

    #[instrument(skip(self))]
    async fn stop(&mut self) {
        info!("Stop called, closing agent responses");
        let _ = self.responses.lock().await.close();
        info!("IndicatorHandler stopped");
    }
}
