/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::entity_channel::*;

///
/// Data about a log message
///
#[derive(Clone, Debug, PartialEq)]
pub struct LogMessage {
    pub message: String,
}

///
/// The level of a log request
///
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

///
/// A request to log a message
///
#[derive(Clone, Debug, PartialEq)]
pub enum LogRequest {
    Trace(LogMessage),
    Debug(LogMessage),
    Info(LogMessage),
    Warning(LogMessage),
    Error(LogMessage),
}

///
/// A request
///
pub enum LogControlRequest {
    /// Send a log message to anything that's listening
    Log(LogRequest),

    /// Send all log messages at the specified log level or above to the specified channel
    Monitor(BoxedEntityChannel<'static, LogRequest>, LogLevel),
}

impl From<LogRequest> for LogControlRequest {
    fn from(req: LogRequest) -> LogControlRequest {
        LogControlRequest::Log(req)
    }
}
