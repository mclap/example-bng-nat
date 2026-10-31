//! Minimal REST server exposing stats & sessions, with OpenAPI document.

use crate::cgnat::Nat;
use crate::stats::StatsState;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

const OPENAPI_DOC: &str = r#"{
  "openapi": "3.0.0",
  "info": { "title": "BNG Operations API", "version": "0.1.0" },
  "paths": {
    "/stats": { "get": { "summary": "Get traffic statistics", "responses": { "200": { "description": "OK" } } } },
    "/sessions": { "get": { "summary": "Get NAT sessions", "responses": { "200": { "description": "OK" } } } }
  }
}"#;

pub fn serve(addr: &str, nat: Arc<Mutex<Nat>>, stats: Arc<StatsState>) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut reader = BufReader::new(&mut stream);
        let mut request_line = String::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                break;
            }
            if request_line.is_empty() {
                request_line = line.to_string();
            }
        }
        let path = request_line.split_whitespace().nth(1).unwrap_or("/").to_string();

        let (status, content_type, body) = if path == "/openapi.json" {
            ("200 OK".to_string(), "application/json".to_string(), OPENAPI_DOC.to_string())
        } else if path == "/stats" {
            let (rx, tx, ports) = stats.snapshot();
            let mut body = String::new();
            body.push_str("{\"rx_packets\":");
            body.push_str(&rx.to_string());
            body.push_str(",\"tx_packets\":");
            body.push_str(&tx.to_string());
            body.push_str("}");
            body.push(',');
            body.push_str("\"ports\":[");
            for (i, (pid, pr, pt)) in ports.iter().enumerate() {
                if i > 0 {
                    body.push(',');
                }
                body.push_str("{\"port\":");
                body.push_str(&pid.to_string());
                body.push_str(",\"rx\":");
                body.push_str(&pr.to_string());
                body.push_str(",\"tx\":");
                body.push_str(&pt.to_string());
                body.push('}');
            }
            body.push(']');
            ("200 OK".to_string(), "application/json".to_string(), body)
        } else if path == "/sessions" {
            let nat_guard = nat.lock().unwrap();
            let sessions = nat_guard.sessions();
            let mut arr = String::from("[");
            for (i, s) in sessions.iter().enumerate() {
                if i > 0 {
                    arr.push(',');
                }
                arr.push_str("{\"nat_src\":");
                arr.push_str(&s.nat_src.to_string());
                arr.push_str(",\"nat_port\":");
                arr.push_str(&s.nat_port.to_string());
                arr.push_str(",\"orig_src\":");
                arr.push_str(&s.orig_src.to_string());
                arr.push_str(",\"orig_dst\":");
                arr.push_str(&s.orig_dst.to_string());
                arr.push_str(",\"orig_proto\":");
                arr.push_str(&s.orig_proto.to_string());
                arr.push_str(",\"orig_sport\":");
                arr.push_str(&s.orig_sport.to_string());
                arr.push_str(",\"orig_dport\":");
                arr.push_str(&s.orig_dport.to_string());
                arr.push('}');
            }
            arr.push(']');
            ("200 OK".to_string(), "application/json".to_string(), arr)
        } else {
            (
                "404 NOT FOUND".to_string(),
                "application/json".to_string(),
                "{}".to_string(),
            )
        };

        let response = format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status,
            content_type,
            body.len(),
            body
        );
        stream.write_all(response.as_bytes())?;
        stream.flush()?;
    }
    Ok(())
}
