//! Raw and JSON subprocess reader loops for runner output streams.
//!
//! Responsibilities:
//! - Read stdout/stderr incrementally from child processes.
//! - Apply shared buffer truncation rules.
//! - Parse JSON lines and forward rendered output.
//!
//! Does not handle:
//! - Event formatting internals (see `stream_events`).
//! - Sink rendering policy (see `stream_render`).

use anyhow::Context;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::constants::buffers::MAX_LINE_LENGTH;
use crate::debuglog::{self, DebugStream};
use crate::runner::{OutputHandler, OutputStream};

use super::super::json::{extract_session_id_from_json, parse_json_line};
use super::StreamSink;
use super::stream_buffer::append_to_buffer;
use super::stream_events::ToolCallTracker;
use super::stream_render::display_filtered_json;

pub(crate) fn spawn_reader<R: Read + Send + 'static>(
    mut reader: R,
    sink: StreamSink,
    buffer: Arc<Mutex<String>>,
    output_handler: Option<OutputHandler>,
    output_stream: OutputStream,
) -> thread::JoinHandle<anyhow::Result<()>> {
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut buffer_exceeded_logged = false;
        loop {
            let read = reader.read(&mut buf).context("read child output")?;
            if read == 0 {
                break;
            }
            let text = String::from_utf8_lossy(&buf[..read]);
            debuglog::write_runner_chunk(DebugStream::Stderr, text.as_ref());
            sink.write_all(&buf[..read], output_stream)
                .context("stream child output")?;
            let mut guard = buffer
                .lock()
                .map_err(|_| anyhow::anyhow!("lock output buffer"))?;
            append_to_buffer(&mut guard, &text, &mut buffer_exceeded_logged);

            if let Some(handler) = &output_handler {
                handler(&text);
            }
        }
        Ok(())
    })
}

pub(crate) fn spawn_json_reader<R: Read + Send + 'static>(
    mut reader: R,
    sink: StreamSink,
    buffer: Arc<Mutex<String>>,
    output_handler: Option<OutputHandler>,
    output_stream: OutputStream,
    session_id_buf: Arc<Mutex<Option<String>>>,
) -> thread::JoinHandle<anyhow::Result<()>> {
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut line_buf = String::new();
        let mut line_length_exceeded = false;
        let mut buffer_exceeded_logged = false;
        let mut tool_tracker = ToolCallTracker::default();

        loop {
            let read = reader.read(&mut buf).context("read child output")?;
            if read == 0 {
                break;
            }

            let text = String::from_utf8_lossy(&buf[..read]);
            debuglog::write_runner_chunk(DebugStream::Stdout, text.as_ref());
            for ch in text.chars() {
                if ch == '\n' {
                    if line_length_exceeded {
                        log::warn!(
                            "Runner output line exceeded {}MB limit; truncating",
                            MAX_LINE_LENGTH / (1024 * 1024)
                        );
                        line_length_exceeded = false;
                    }
                    handle_completed_line(
                        &line_buf,
                        &sink,
                        output_handler.as_ref(),
                        output_stream,
                        &session_id_buf,
                        &mut tool_tracker,
                    )?;
                    line_buf.clear();
                } else if line_buf.len() >= MAX_LINE_LENGTH {
                    line_length_exceeded = true;
                } else {
                    line_buf.push(ch);
                }
            }

            let mut guard = buffer
                .lock()
                .map_err(|_| anyhow::anyhow!("lock output buffer"))?;
            append_to_buffer(&mut guard, &text, &mut buffer_exceeded_logged);
        }

        if !line_buf.trim().is_empty() {
            if line_length_exceeded {
                log::warn!(
                    "Runner output line exceeded {}MB limit; truncating",
                    MAX_LINE_LENGTH / (1024 * 1024)
                );
            }
            handle_plain_line(&line_buf, &sink, output_handler.as_ref(), output_stream)?;
        }
        Ok(())
    })
}

fn handle_completed_line(
    line_buf: &str,
    sink: &StreamSink,
    output_handler: Option<&OutputHandler>,
    output_stream: OutputStream,
    session_id_buf: &Arc<Mutex<Option<String>>>,
    tool_tracker: &mut ToolCallTracker,
) -> anyhow::Result<()> {
    if let Some(mut json) = parse_json_line(line_buf) {
        tool_tracker.correlate(&mut json);
        if let Some(id) = extract_session_id_from_json(&json)
            && let Ok(mut guard) = session_id_buf.lock()
        {
            *guard = Some(id);
        }
        display_filtered_json(&json, sink, output_handler, output_stream)?;
    } else if !line_buf.trim().is_empty() {
        handle_plain_line(line_buf, sink, output_handler, output_stream)?;
    }

    Ok(())
}

fn handle_plain_line(
    line: &str,
    sink: &StreamSink,
    output_handler: Option<&OutputHandler>,
    output_stream: OutputStream,
) -> anyhow::Result<()> {
    let mut emitted = line.to_string();
    sink.write_all(emitted.as_bytes(), output_stream)?;
    sink.write_all(b"\n", output_stream)?;
    if let Some(handler) = output_handler {
        emitted.push('\n');
        handler(&emitted);
    }
    Ok(())
}
